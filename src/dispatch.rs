use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio::time::{Duration, sleep};

use crate::config::{Config, Handler, SpawnMode};
use crate::protocol::socket_json::SocketLine;
use crate::template::{matches_when, substitute};

const MAX_CONCURRENT_HANDLERS: usize = 8;

pub struct Dispatcher {
    config: Arc<Config>,
    debounce: Arc<Mutex<HashMap<usize, tokio::task::JoinHandle<()>>>>,
    semaphore: Arc<tokio::sync::Semaphore>,
}

impl Dispatcher {
    pub fn new(config: Config) -> Self {
        Self {
            config: Arc::new(config),
            debounce: Arc::new(Mutex::new(HashMap::new())),
            semaphore: Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_HANDLERS)),
        }
    }

    pub async fn handle_event(&self, event: SocketLine) -> Result<()> {
        let event_name = match event.event.as_deref() {
            Some(name) => name,
            None => return Ok(()),
        };

        for (index, handler) in self.config.handlers.iter().enumerate() {
            if handler.event != event_name {
                continue;
            }
            if let Some(when) = &handler.when
                && !matches_when(when, &event)
            {
                continue;
            }

            let debounce_ms = handler.effective_debounce_ms(&self.config.handler_defaults);
            if debounce_ms == 0 {
                self.run_handler(handler, &event).await?;
            } else {
                self.schedule_debounced(index, handler, event.clone(), debounce_ms)
                    .await;
            }
        }
        Ok(())
    }

    async fn schedule_debounced(
        &self,
        index: usize,
        handler: &Handler,
        event: SocketLine,
        debounce_ms: u64,
    ) {
        let mut debounce = self.debounce.lock().await;
        if let Some(pending) = debounce.remove(&index) {
            pending.abort();
        }

        let config = Arc::clone(&self.config);
        let handler = handler.clone();
        let semaphore = Arc::clone(&self.semaphore);
        let debounce_map = Arc::clone(&self.debounce);

        let sleep_handle = tokio::spawn(async move {
            sleep(Duration::from_millis(debounce_ms)).await;
            let _ = run_handler_inner(&config, &handler, &event, &semaphore).await;
            debounce_map.lock().await.remove(&index);
        });
        debounce.insert(index, sleep_handle);
    }

    async fn run_handler(&self, handler: &Handler, event: &SocketLine) -> Result<()> {
        run_handler_inner(&self.config, handler, event, &self.semaphore).await
    }
}

async fn run_handler_inner(
    config: &Config,
    handler: &Handler,
    event: &SocketLine,
    semaphore: &Arc<tokio::sync::Semaphore>,
) -> Result<()> {
    let command = substitute(&handler.command, event);
    let spawn = handler.effective_spawn(&config.handler_defaults);
    log::debug!("handler {}: {command}", handler.event);

    match spawn {
        SpawnMode::Sync => {
            let status = Command::new("sh")
                .arg("-c")
                .arg(&command)
                .status()
                .await?;
            if !status.success() {
                log::warn!(
                    "handler {} exited with {}",
                    handler.event,
                    status.code().unwrap_or(-1)
                );
            }
        }
        SpawnMode::Async => {
            let permit = semaphore.clone().acquire_owned().await?;
            let event_name = handler.event.clone();
            tokio::spawn(async move {
                let _permit = permit;
                match Command::new("sh").arg("-c").arg(&command).status().await {
                    Ok(status) if status.success() => {
                        log::debug!("handler {event_name} completed");
                    }
                    Ok(status) => {
                        log::warn!(
                            "handler {event_name} exited with {}",
                            status.code().unwrap_or(-1)
                        );
                    }
                    Err(err) => {
                        log::warn!("handler {event_name} failed: {err:#}");
                    }
                }
            });
        }
    }
    Ok(())
}

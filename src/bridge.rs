use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, BufReader as AsyncBufReader};
use tokio::process::Command;

use crate::cli::Cli;
use crate::config::Config;
use crate::dispatch::Dispatcher;
use crate::protocol::helix_cli::FLAG_SUBSCRIBE;
use crate::protocol::socket_json::SocketLine;

pub async fn run(cli: &Cli) -> Result<()> {
    let config_path = cli.config_path()?;
    let config = Config::load(&config_path)?;
    let events = config.subscribe_event_list();
    let dispatcher = Dispatcher::new(config);

    log::info!(
        "bridge starting (config={}, events={events})",
        config_path.display()
    );

    let mut child = Command::new(cli.helix_bin())
        .arg(FLAG_SUBSCRIBE)
        .arg(&events)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .with_context(|| format!("failed to spawn {} --subscribe", cli.helix_bin()))?;

    let stdout = child
        .stdout
        .take()
        .context("hx --subscribe did not provide stdout")?;
    let mut lines = AsyncBufReader::new(stdout).lines();

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<SocketLine>(&line) {
            Ok(parsed) => {
                if parsed.is_error() {
                    log::warn!(
                        "helix error: {}",
                        parsed.error.as_deref().unwrap_or("unknown")
                    );
                    continue;
                }
                if parsed.is_event() {
                    dispatcher.handle_event(parsed).await?;
                }
            }
            Err(err) => {
                log::warn!("invalid event line: {err}: {line}");
            }
        }
    }

    let status = child.wait().await?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!(
            "{} --subscribe exited with {}",
            cli.helix_bin(),
            status.code().unwrap_or(-1)
        )
    }
}

/// Sync helper for unit tests without a live Helix process.
#[cfg(test)]
pub fn dispatch_lines(config: &Config, lines: &[&str]) -> Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let dispatcher = Dispatcher::new(config.clone());
    rt.block_on(async {
        for line in lines {
            let parsed: SocketLine = serde_json::from_str(line)?;
            if parsed.is_event() {
                dispatcher.handle_event(parsed).await?;
            }
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatches_configured_handler_line() {
        let raw = r#"
[subscribe]
events = ["document-focus"]

[[handlers]]
event = "document-focus"
command = "true"
"#;
        let config: Config = toml::from_str(raw).unwrap();
        let line = r#"{"v":1,"op":"event","event":"document-focus","path":"/tmp/a.rs","line":1,"column":1}"#;
        dispatch_lines(&config, &[line]).unwrap();
    }
}

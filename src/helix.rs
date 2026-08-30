use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};

use crate::cli::Cli;
use crate::protocol::helix_cli::{FLAG_REMOTE, FLAG_SUBSCRIBE};

pub const SOCKET_RETRY_ATTEMPTS: u32 = 75;
pub const SOCKET_RETRY_DELAY_MS: u64 = 200;

pub fn remote(cli: &Cli, socket: &Path, cmd: &str) -> Result<()> {
    let mut last_err = None;
    for attempt in 1..=SOCKET_RETRY_ATTEMPTS {
        match try_remote(cli, socket, cmd) {
            Ok(()) => return Ok(()),
            Err(err) => {
                if attempt == SOCKET_RETRY_ATTEMPTS {
                    last_err = Some(err);
                    break;
                }
                log::debug!(
                    "hx --remote attempt {attempt}/{SOCKET_RETRY_ATTEMPTS} failed: {err:#}"
                );
                thread::sleep(Duration::from_millis(SOCKET_RETRY_DELAY_MS));
            }
        }
    }
    Err(last_err.unwrap_or_else(|| {
        anyhow::anyhow!("helix socket not ready at {}", socket.display())
    }))
}

fn try_remote(cli: &Cli, _socket: &Path, cmd: &str) -> Result<()> {
    let status = Command::new(cli.helix_bin())
        .arg(FLAG_REMOTE)
        .arg(cmd)
        .status()
        .with_context(|| format!("failed to spawn {}", cli.helix_bin()))?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!(
            "{} --remote exited with {}",
            cli.helix_bin(),
            status.code().unwrap_or(-1)
        )
    }
}

pub fn spawn_subscribe(cli: &Cli, events: &str) -> Result<std::process::Child> {
    Command::new(cli.helix_bin())
        .arg(FLAG_SUBSCRIBE)
        .arg(events)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .with_context(|| format!("failed to spawn {} --subscribe", cli.helix_bin()))
}

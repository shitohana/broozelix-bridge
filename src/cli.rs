use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "broozelix-bridge", about = "Thin Helix socket bridge for broozelix")]
pub struct Cli {
    /// Path to handler config (overrides BROOZELIX_BRIDGE_CONFIG).
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    /// Helix binary (default: hx on PATH).
    #[arg(long, global = true)]
    pub hx: Option<PathBuf>,

    /// Helix Unix socket (overrides HELIX_SOCKET_PATH).
    #[arg(long, global = true)]
    pub helix_socket: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Open a file in Helix (optional :line[:column] suffix).
    Open {
        /// Path, optionally with :line or :line:column suffix.
        path: String,
    },
    /// Pass a typable or JSON command to Helix via hx --remote.
    Remote {
        /// Command string (hx adds leading ':' for typable commands).
        cmd: String,
    },
    /// Subscribe to Helix events and run configured handlers.
    Bridge,
}

impl Cli {
    pub fn helix_bin(&self) -> &str {
        self.hx
            .as_ref()
            .and_then(|p| p.to_str())
            .unwrap_or(crate::protocol::helix_cli::HX_BIN)
    }

    pub fn helix_socket(&self) -> anyhow::Result<PathBuf> {
        if let Some(path) = &self.helix_socket {
            return Ok(path.clone());
        }
        let path = std::env::var("HELIX_SOCKET_PATH")
            .map(PathBuf::from)
            .map_err(|_| anyhow::anyhow!("HELIX_SOCKET_PATH is not set (use --helix-socket)"))?;
        Ok(path)
    }

    pub fn config_path(&self) -> anyhow::Result<PathBuf> {
        if let Some(path) = &self.config {
            return Ok(path.clone());
        }
        if let Ok(path) = std::env::var("BROOZELIX_BRIDGE_CONFIG") {
            return Ok(PathBuf::from(path));
        }
        let default = PathBuf::from("broozelix-bridge.toml");
        if default.is_file() {
            return Ok(default);
        }
        anyhow::bail!(
            "config not specified (use --config, BROOZELIX_BRIDGE_CONFIG, or ./broozelix-bridge.toml)"
        )
    }
}

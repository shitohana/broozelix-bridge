use anyhow::Context;
use broozelix_bridge::cli::{Cli, Command};
use clap::Parser;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    match &cli.command {
        Command::Open { path } => broozelix_bridge::open::run(&cli, path),
        Command::Remote { cmd } => broozelix_bridge::remote::run(&cli, cmd),
        Command::Bridge => broozelix_bridge::bridge::run(&cli).await,
    }
    .context("broozelix-bridge failed")
}

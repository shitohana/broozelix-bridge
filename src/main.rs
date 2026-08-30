use anyhow::Context;
use broozelix_bridge::cli::{Cli, Command};
use clap::Parser;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let default_level = if cli.verbose { "info" } else { "warn" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(default_level))
        .init();

    match &cli.command {
        Command::Open { path } => broozelix_bridge::open::run(&cli, path),
        Command::Remote { cmd } => broozelix_bridge::remote::run(&cli, cmd),
        Command::Bridge => broozelix_bridge::bridge::run(&cli).await,
    }
    .context("broozelix-bridge failed")
}

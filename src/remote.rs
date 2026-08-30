use anyhow::Result;

use crate::cli::Cli;
use crate::helix;

pub fn run(cli: &Cli, cmd: &str) -> Result<()> {
    let socket = cli.helix_socket()?;
    helix::remote(cli, &socket, cmd)
}

use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
pub struct Cli {
    /// Path to an alternate configuration file
    #[clap(long)]
    pub config: Option<PathBuf>,
    /// Path to the socket to use for ipc
    #[clap(long)]
    pub socket: Option<PathBuf>,
}

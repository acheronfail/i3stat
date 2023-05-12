use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
pub struct Cli {
    #[clap(long)]
    pub config: Option<PathBuf>,
    #[clap(long)]
    pub socket: Option<PathBuf>,
}

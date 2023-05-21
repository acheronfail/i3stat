use std::path::PathBuf;

use clap::Parser;

/// A lightweight and batteries-included status_command for i3 and sway.
///
/// To learn more about configuration options and their possible values, see the `sample_config.toml`
/// that's provided with this program.
#[derive(Debug, Parser)]
#[clap(author, version, about, long_about)]
pub struct Cli {
    /// Path to an alternate configuration file.
    #[clap(long)]
    pub config: Option<PathBuf>,
    /// Path to the socket to use for ipc. Takes precedence over the same option in the config file.
    #[clap(long)]
    pub socket: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::Cli;
    use crate::test_utils::generate_manpage;

    #[test]
    fn manpage() {
        generate_manpage(Cli::command());
    }
}

use clap::{ColorChoice, Parser};
use i3stat::error::Result;
use i3stat::util::{local_block_on, netlink_acpi_listen};

#[derive(Debug, Parser)]
#[clap(author, version, long_about, name = "i3stat-acpi", color = ColorChoice::Always)]
/// A command which uses netlink and listens for acpi events, and prints them to
/// stdout as they're received.
///
/// The events are output in JSON format, one line per event.
struct Cli;

fn main() -> Result<()> {
    Cli::parse();

    let (output, _) = local_block_on(async {
        let mut acpi = netlink_acpi_listen().await?;
        while let Some(event) = acpi.recv().await {
            println!("{}", serde_json::to_string(&event)?);
        }

        Err("unexpected end of acpi event stream".into())
    })?;

    output
}

#[cfg(test)]
#[path = "../src/test_utils.rs"]
mod test_utils;

#[cfg(test)]
crate::gen_manpage!(Cli);

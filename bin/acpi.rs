use clap::{ColorChoice, Parser};
use i3stat::error::Result;
use i3stat::util::{local_block_on, netlink_acpi_listen};
use tokio::io::{stdout, AsyncWriteExt};

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
            let line = format!("{}", serde_json::to_string(&event)?);

            // flush output each time to facilitate common usage patterns, such as
            // `i3stat-acpi | while read x; do ... done`, etc.
            let mut stdout = stdout();
            stdout.write_all(line.as_bytes()).await?;
            stdout.flush().await?;
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

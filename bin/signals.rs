use clap::{ColorChoice, Parser};
use libc::{SIGRTMAX, SIGRTMIN};
use serde_json::json;

#[derive(Debug, Parser)]
#[clap(author, version, long_about, name = "istat-signals", color = ColorChoice::Always)]
/// Outputs the available realtime signals on the current machine.
///
/// Format is JSON.
struct Cli;

fn main() {
    Cli::parse();

    let rt_min = SIGRTMIN();
    let rt_max = SIGRTMAX();
    println!(
        "{}",
        json!({
            "count": rt_max - rt_min,
            "sigrtmin": rt_min,
            "sigrtmax": rt_max
        })
    );
}

#[cfg(test)]
#[path = "../src/test_utils.rs"]
mod test_utils;

#[cfg(test)]
crate::gen_manpage!(Cli);

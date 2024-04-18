use clap::{ColorChoice, Parser};
use sysinfo::Components;

#[derive(Debug, Parser)]
#[clap(author, version, long_about, name = "i3stat-sensors", color = ColorChoice::Always)]
/// Outputs a list of system temperature sensors.
///
/// Each line contains a sensor and its temperature, in the following format:
///
///     TEMP:COMPONENT
///
/// Where TEMP is the temperature in Celsius, and COMPONENT is the name of the sensor.
/// The COMPONENT property can by used to configure bar items with type "sensors".
struct Cli;

fn main() {
    Cli::parse();

    Components::new_with_refreshed_list()
        .iter()
        .for_each(|c| println!("{:>width$.2}°C:{}", c.temperature(), c.label(), width = 6));
}

#[cfg(test)]
#[path = "../src/test_utils.rs"]
mod test_utils;

#[cfg(test)]
crate::gen_manpage!(Cli);

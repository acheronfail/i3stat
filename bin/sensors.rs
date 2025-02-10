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

    Components::new_with_refreshed_list().iter().for_each(|c| {
        if let Some(temp) = c.temperature() {
            println!("{:>width$.2}°C:{}", temp, c.label(), width = 6)
        }
    });
}

#[cfg(test)]
#[path = "../src/test_utils.rs"]
mod test_utils;

#[cfg(test)]
crate::gen_manpage!(Cli);

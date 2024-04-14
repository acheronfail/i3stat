use clap::{ColorChoice, Parser};
use sysinfo::{ComponentExt, RefreshKind, System, SystemExt};

#[derive(Debug, Parser)]
#[clap(author, version, long_about, name = "i3stat-sensors", color = ColorChoice::Always)]
/// Outputs a list of system temperature sensors.
///
/// Each line contains a sensor and its temperature, in the following format:
///
///     TEMP:LABEL
///
/// Where TEMP is the temperature in Celsius, and LABEL is the name of the sensor.
/// The LABEL property can by used to configure bar items with type "sensors".
struct Cli;

fn main() {
    Cli::parse();

    let sys = System::new_with_specifics(RefreshKind::new().with_components_list());
    sys.components()
        .iter()
        .for_each(|c| println!("{:>width$.2}°C:{}", c.temperature(), c.label(), width = 6));
}

#[cfg(test)]
#[path = "../src/test_utils.rs"]
mod test_utils;

#[cfg(test)]
crate::gen_manpage!(Cli);

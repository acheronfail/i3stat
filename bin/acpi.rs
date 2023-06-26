use istat::error::Result;
use istat::util::{local_block_on, netlink_acpi_listen};

fn main() -> Result<()> {
    let (output, _) = local_block_on(async {
        let mut acpi = netlink_acpi_listen().await?;
        while let Some(event) = acpi.recv().await {
            println!("{}", serde_json::to_string(&event)?);
        }

        Err("unexpected end of acpi event stream".into())
    })?;

    output
}

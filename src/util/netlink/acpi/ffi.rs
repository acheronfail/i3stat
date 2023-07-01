use ::std::os::raw::{c_char, c_uint};
use serde_derive::{Deserialize, Serialize};

use crate::error::{Error, Result};

// https://github.com/torvalds/linux/blob/f8dba31b0a826e691949cd4fdfa5c30defaac8c5/drivers/acpi/event.c#L77
pub const ACPI_EVENT_FAMILY_NAME: &str = "acpi_event";
// https://github.com/torvalds/linux/blob/f8dba31b0a826e691949cd4fdfa5c30defaac8c5/drivers/acpi/event.c#L79
pub const ACPI_EVENT_MCAST_GROUP_NAME: &str = "acpi_mc_group";

// linux:  https://github.com/torvalds/linux/blob/f8dba31b0a826e691949cd4fdfa5c30defaac8c5/drivers/acpi/event.c#L62
#[repr(u16)]
#[allow(unused)]
pub enum AcpiAttrType {
    Unspecified = 0,
    Event = 1,
}

// linux: https://github.com/torvalds/linux/blob/f8dba31b0a826e691949cd4fdfa5c30defaac8c5/drivers/acpi/event.c#L54
#[repr(C)]
#[derive(Debug, Copy, Clone)]
#[allow(non_camel_case_types)]
pub struct acpi_genl_event {
    // https://github.com/torvalds/linux/blob/f8dba31b0a826e691949cd4fdfa5c30defaac8c5/include/acpi/acpi_bus.h#L223
    device_class: [c_char; 20usize],
    bus_id: [c_char; 15usize],
    r#type: c_uint,
    data: c_uint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpiGenericNetlinkEvent {
    /// Describes the device from where the event was emitted, see struct's associated constants.
    /// Sometimes also completely empty - `""` - in some cases (such as changing display brightness).
    pub device_class: String,
    pub bus_id: String,
    pub r#type: u32,
    pub data: u32,
}

impl AcpiGenericNetlinkEvent {
    /// https://github.com/torvalds/linux/blob/f8dba31b0a826e691949cd4fdfa5c30defaac8c5/include/acpi/battery.h#L7
    pub const DEVICE_CLASS_BATTERY: &str = "battery";
    /// https://github.com/torvalds/linux/blob/f8dba31b0a826e691949cd4fdfa5c30defaac8c5/drivers/acpi/ac.c#L23
    pub const DEVICE_CLASS_AC: &str = "ac_adapter";
    /// https://github.com/torvalds/linux/blob/f8dba31b0a826e691949cd4fdfa5c30defaac8c5/include/acpi/processor.h#L17
    pub const DEVICE_CLASS_PROCESSOR: &str = "processor";
}

/// Checks a slice of C's chars to ensure they're not signed, needed because C's `char` type could
/// be either signed or unsigned unless specified. See: https://stackoverflow.com/a/2054941/5552584
fn get_u8_bytes(slice: &[c_char]) -> Result<Vec<u8>> {
    slice
        .into_iter()
        .take_while(|c| **c != 0)
        .map(|c| -> Result<u8> {
            if *c < 0 {
                Err(format!("slice contained signed char: {}", c).into())
            } else {
                Ok(*c as u8)
            }
        })
        .collect::<Result<Vec<_>>>()
}

impl<'a> TryFrom<&'a acpi_genl_event> for AcpiGenericNetlinkEvent {
    type Error = Error;

    fn try_from(value: &'a acpi_genl_event) -> std::result::Result<Self, Self::Error> {
        Ok(AcpiGenericNetlinkEvent {
            device_class: String::from_utf8(get_u8_bytes(&value.device_class)?)?,
            bus_id: String::from_utf8(get_u8_bytes(&value.bus_id)?)?,
            r#type: value.r#type,
            data: value.data,
        })
    }
}

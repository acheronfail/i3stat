use std::error::Error;

use ::std::os::raw::{c_char, c_uint};

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
pub struct acpi_generic_netlink_event {
    // https://github.com/torvalds/linux/blob/f8dba31b0a826e691949cd4fdfa5c30defaac8c5/include/acpi/acpi_bus.h#L223
    device_class: [c_char; 20usize],
    bus_id: [c_char; 15usize],
    r#type: c_uint,
    data: c_uint,
}

#[derive(Debug, Clone)]
pub struct AcpiGenericNetlinkEvent {
    pub device_class: String,
    pub bus_id: String,
    pub r#type: u32,
    pub data: u32,
}

/// Checks a slice of C's chars to ensure they're not signed, needed because:
/// https://stackoverflow.com/a/2054941/5552584
fn get_u8_bytes(slice: &[c_char]) -> Result<Vec<u8>, Box<dyn Error>> {
    slice
        .into_iter()
        .take_while(|c| **c != 0)
        .map(|c| -> Result<u8, Box<dyn Error>> {
            if *c < 0 {
                Err(format!("slice contained signed char: {}", c).into())
            } else {
                Ok(*c as u8)
            }
        })
        .collect::<Result<Vec<_>, _>>()
}

impl<'a> TryFrom<&'a acpi_generic_netlink_event> for AcpiGenericNetlinkEvent {
    type Error = Box<dyn Error>;

    fn try_from(value: &'a acpi_generic_netlink_event) -> Result<Self, Self::Error> {
        Ok(AcpiGenericNetlinkEvent {
            device_class: String::from_utf8(get_u8_bytes(&value.device_class)?)?,
            bus_id: String::from_utf8(get_u8_bytes(&value.bus_id)?)?,
            r#type: value.r#type,
            data: value.data,
        })
    }
}

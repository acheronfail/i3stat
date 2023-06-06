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

pub struct AcpiGenericNetlinkEvent {
    pub device_class: String,
    pub bus_id: String,
    pub r#type: u32,
    pub data: u32,
}

/// Checks a slice of C's chars to ensure they're not signed, needed because:
/// https://stackoverflow.com/a/2054941/5552584
fn check_char_sign(slice: &[c_char]) -> Result<&[u8], Box<dyn Error>> {
    if slice.iter().all(|c| *c >= 0) {
        Ok(unsafe { &*(slice.as_ptr() as *const &[u8]) })
    } else {
        Err(format!("slice contained signed chars: {:?}", slice).into())
    }
}

impl<'a> TryFrom<&'a acpi_generic_netlink_event> for AcpiGenericNetlinkEvent {
    type Error = Box<dyn Error>;

    fn try_from(value: &'a acpi_generic_netlink_event) -> Result<Self, Self::Error> {
        Ok(AcpiGenericNetlinkEvent {
            device_class: std::str::from_utf8(&check_char_sign(&value.device_class)?)?.to_string(),
            bus_id: std::str::from_utf8(&check_char_sign(&value.bus_id)?)?.to_string(),
            r#type: value.r#type,
            data: value.data,
        })
    }
}

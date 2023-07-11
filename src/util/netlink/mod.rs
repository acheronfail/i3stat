pub mod acpi;
pub mod nl80211;
pub mod route;

use std::array::TryFromSliceError;
use std::fmt::Debug;
use std::net::IpAddr;
use std::sync::Arc;

pub use acpi::netlink_acpi_listen;
use indexmap::IndexSet;
pub use route::netlink_ipaddr_listen;

#[derive(Clone)]
pub struct MacAddr {
    octets: [u8; 6],
}

impl Debug for MacAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "MacAddr({})",
            self.octets.map(|o| format!("{:02x}", o)).join(":")
        ))
    }
}

impl From<&MacAddr> for neli::types::Buffer {
    fn from(value: &MacAddr) -> Self {
        Self::from(&value.octets[..])
    }
}

impl From<&[u8; 6]> for MacAddr {
    fn from(value: &[u8; 6]) -> Self {
        MacAddr { octets: *value }
    }
}

impl TryFrom<Vec<u8>> for MacAddr {
    type Error = TryFromSliceError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        let octets: &[u8; 6] = (&value[..]).try_into()?;
        Ok(MacAddr { octets: *octets })
    }
}

impl TryFrom<&[u8]> for MacAddr {
    type Error = TryFromSliceError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let octets: &[u8; 6] = value.try_into()?;
        Ok(MacAddr { octets: *octets })
    }
}

impl TryFrom<&str> for MacAddr {
    type Error = crate::error::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let parts = value.split(':').collect::<Vec<_>>();
        if parts.len() != 6 {
            bail!("expected 6 parts");
        }

        let parts = parts
            .into_iter()
            .map(|s| u8::from_str_radix(s, 16))
            .collect::<Result<Vec<u8>, _>>()?;

        Ok(parts.try_into()?)
    }
}

#[derive(Debug, Clone)]
pub struct NetlinkInterface {
    pub index: i32,
    // NOTE: `Arc` rather than `Rc` here because `Send` is needed by `tokio::sync::broadcast`
    pub name: Arc<str>,
    pub mac_address: Option<MacAddr>,
    pub ip_addresses: IndexSet<IpAddr>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug() {
        let mac = MacAddr::from(&[1, 42, 83, 124, 165, 206]);
        assert_eq!(format!("{:?}", mac), "MacAddr(01:2a:53:7c:a5:ce)");
    }
}

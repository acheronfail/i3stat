use std::cmp::Ordering;
use std::error::Error;
use std::net::{SocketAddrV4, SocketAddrV6};
use std::str::FromStr;

use iwlib::{get_wireless_info, WirelessInfo};
use nix::ifaddrs::getifaddrs;
use nix::net::if_::InterfaceFlags;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceKind {
    V4,
    V6,
}

impl ToString for InterfaceKind {
    fn to_string(&self) -> String {
        match self {
            InterfaceKind::V4 => "v4".into(),
            InterfaceKind::V6 => "v6".into(),
        }
    }
}

impl FromStr for InterfaceKind {
    type Err = Box<dyn Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "v4" => Ok(Self::V4),
            "v6" => Ok(Self::V6),
            _ => Err(format!("unrecognised InterfaceKind, expected v4 or v6, got: {}", s).into()),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Interface {
    pub name: String,
    pub addr: String,
    pub kind: InterfaceKind,
    pub flags: InterfaceFlags,
    pub is_wireless: Option<bool>,
}

impl PartialOrd for Interface {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.name.partial_cmp(&other.name) {
            Some(Ordering::Equal) => {}
            ord => return ord,
        }
        match self.addr.partial_cmp(&other.addr) {
            Some(Ordering::Equal) => {}
            ord => return ord,
        }
        self.flags.partial_cmp(&other.flags)
    }
}

impl Ord for Interface {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

impl Interface {
    pub fn new(
        name: impl AsRef<str>,
        addr: impl AsRef<str>,
        kind: InterfaceKind,
        flags: InterfaceFlags,
    ) -> Interface {
        Interface {
            name: name.as_ref().into(),
            addr: addr.as_ref().into(),
            kind,
            flags,
            is_wireless: None,
        }
    }

    pub fn is_vpn(&self) -> bool {
        self.flags.contains(InterfaceFlags::IFF_TAP) || self.flags.contains(InterfaceFlags::IFF_TUN)
    }

    pub fn is_wireless(&mut self) -> bool {
        match self.is_wireless {
            Some(b) => b,
            None => self.get_wireless_info().is_some(),
        }
    }

    pub fn get_wireless_info(&mut self) -> Option<WirelessInfo> {
        // TODO: contribute AsRef upstream to https://github.com/psibi/iwlib-rs
        // See: https://github.com/psibi/iwlib-rs/pull/2
        let name = self.name.clone();

        // check if this is a wireless network
        match self.is_wireless {
            // not a wireless interface, just return defaults
            Some(false) => None,
            // SAFETY: we've previously checked if this is a wireless network
            Some(true) => get_wireless_info(name),
            // check if we're a wireless network and remember for next time
            None => match get_wireless_info(name) {
                Some(i) => {
                    self.is_wireless = Some(true);
                    Some(i)
                }
                None => {
                    self.is_wireless = Some(false);
                    None
                }
            },
        }
    }

    pub fn get_interfaces() -> Result<Vec<Interface>, Box<dyn Error>> {
        let if_addrs = match getifaddrs() {
            Ok(if_addrs) => if_addrs,
            Err(e) => return Err(format!("call to `getifaddrs` failed: {}", e).into()),
        };

        let mut interfaces = vec![];
        for if_addr in if_addrs.into_iter() {
            // skip any interfaces that aren't active
            if !if_addr.flags.contains(InterfaceFlags::IFF_UP) {
                continue;
            }

            // skip the local loopback interface
            if if_addr.flags.contains(InterfaceFlags::IFF_LOOPBACK) {
                continue;
            }

            // skip any unsupported entry (see nix's `getifaddrs` documentation)
            let addr = match if_addr.address {
                Some(addr) => addr,
                None => continue,
            };

            // extract ip address
            let (addr, kind) = match (addr.as_sockaddr_in(), addr.as_sockaddr_in6()) {
                (Some(ipv4), _) => (
                    format!("{}", SocketAddrV4::from(*ipv4).ip()),
                    InterfaceKind::V4,
                ),
                (_, Some(ipv6)) => (
                    format!("{}", SocketAddrV6::from(*ipv6).ip()),
                    InterfaceKind::V6,
                ),
                (None, None) => continue,
            };

            interfaces.push(Interface::new(
                if_addr.interface_name,
                addr,
                kind,
                if_addr.flags,
            ));
        }

        interfaces.sort();

        Ok(interfaces)
    }
}

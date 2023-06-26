use std::error::Error;
use std::net::{SocketAddrV4, SocketAddrV6};
use std::str::FromStr;

use iwlib::{get_wireless_info, WirelessInfo};
use nix::ifaddrs::getifaddrs;
use nix::net::if_::InterfaceFlags;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

// TODO: cache these? pass them all around by reference? interior mutability for wireless or not?
//  cache list of wireless ones?
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Interface {
    pub name: String,
    pub addr: String,
    pub kind: InterfaceKind,
    pub flags: InterfaceFlags,
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
        }
    }

    pub fn is_vpn(&self) -> bool {
        self.flags.contains(InterfaceFlags::IFF_TAP) || self.flags.contains(InterfaceFlags::IFF_TUN)
    }

    /// If this is a wireless network, then return info from `iwlib`
    pub fn get_wireless_info(&self) -> Option<WirelessInfo> {
        get_wireless_info(&self.name)
    }

    pub fn get_interfaces() -> Result<Vec<Interface>, Box<dyn Error>> {
        let if_addrs = match getifaddrs() {
            Ok(if_addrs) => if_addrs,
            Err(e) => bail!("call to `getifaddrs` failed: {}", e),
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
                (_, Some(ipv6)) => {
                    // filter out non-global ipv6 addresses
                    if !ipv6.ip().is_global() {
                        continue;
                    }

                    (
                        format!("{}", SocketAddrV6::from(*ipv6).ip()),
                        InterfaceKind::V6,
                    )
                }
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

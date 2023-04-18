use std::net::{
    SocketAddrV4,
    SocketAddrV6,
};

use nix::ifaddrs::getifaddrs;
use sysinfo::System;

use crate::item::{
    Item,
    ToItem,
};

#[derive(Debug)]
struct Interface {
    name: String,
    addr: String,
    // TODO: network name for wifi?
}

pub struct Nic {
    interfaces: Vec<Interface>,
}

impl Default for Nic {
    fn default() -> Self {
        Nic { interfaces: vec![] }
    }
}

impl ToItem for Nic {
    fn to_item(&self) -> Item {
        Item::text(
            self.interfaces
                .iter()
                .map(|i| format!("{}: {}", i.name, i.addr))
                .collect::<Vec<_>>()
                .join(", "),
        )
    }

    fn update(&mut self, _: &mut System) {
        // TODO: no need to update this every single time... (only if the network changed?)

        let if_addrs = match getifaddrs() {
            Ok(if_addrs) => if_addrs,
            Err(_) => todo!(),
        };

        let mut interfaces = vec![];
        for if_addr in if_addrs.into_iter() {
            if if_addr.interface_name == "lo" {
                continue;
            }

            let addr = match if_addr.address {
                Some(addr) => addr,
                None => continue,
            };

            let addr = match (addr.as_sockaddr_in(), addr.as_sockaddr_in6()) {
                (Some(ipv4), _) => format!("{}", SocketAddrV4::from(*ipv4).ip()),
                (_, Some(ipv6)) => format!("{}", SocketAddrV6::from(*ipv6).ip()),
                (None, None) => continue,
            };

            interfaces.push(Interface {
                name: if_addr.interface_name,
                addr,
            });
        }

        self.interfaces = interfaces;
    }
}

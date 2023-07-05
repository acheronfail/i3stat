//! Use rtnetlink (route netlink) for the following:
//!     - fetching information about all current network interfaces
//!     - be notified when ip addresses change
//!
//! Useful things when developing this:
//!     - https://man7.org/linux/man-pages/man7/rtnetlink.7.html
//!     - https://docs.kernel.org/userspace-api/netlink/intro.html
//!     - `genl-ctrl-list` returns generic families
//!     - simulate ipv4 activity: `ip a add 10.0.0.254 dev wlan0 && sleep 1 && ip a del 10.0.0.254/32 dev wlan0`
//!     - simulate ipv6 activity: `ip -6 addr add 2001:0db8:0:f101::1/64 dev lo && sleep 1 && ip -6 addr del 2001:0db8:0:f101::1/64 dev lo`

use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use libc::{RTNLGRP_IPV4_IFADDR, RTNLGRP_IPV6_IFADDR};
use neli::consts::nl::NlmF;
use neli::consts::rtnl::{Arphrd, Ifa, Ifla, RtAddrFamily, RtScope, Rtm};
use neli::consts::socket::NlFamily;
use neli::err::RouterError;
use neli::genl::Genlmsghdr;
use neli::nl::{NlPayload, Nlmsghdr};
use neli::router::asynchronous::{NlRouter, NlRouterReceiverHandle};
use neli::rtnl::{Ifaddrmsg, IfaddrmsgBuilder, Ifinfomsg, IfinfomsgBuilder};
use neli::utils::Groups;
use tokio::sync::mpsc::{self, Receiver, Sender};

use super::NetlinkInterface;
use crate::error::Result;

pub type InterfaceUpdate = HashMap<i32, NetlinkInterface>;

pub async fn netlink_ipaddr_listen() -> Result<Receiver<InterfaceUpdate>> {
    // setup socket for netlink route
    let (socket, multicast) = NlRouter::connect(NlFamily::Route, None, Groups::empty()).await?;

    // enable strict checking
    // https://docs.kernel.org/userspace-api/netlink/intro.html#strict-checking
    socket.enable_strict_checking(true)?;

    // add multicast membership for ipv4-addr updates
    socket
        .add_mcast_membership(Groups::new_groups(&[
            RTNLGRP_IPV4_IFADDR,
            RTNLGRP_IPV6_IFADDR,
        ]))
        .unwrap();

    let (tx, rx) = mpsc::channel(8);
    tokio::task::spawn_local(async move {
        if let Err(e) = handle_netlink_route_messages(&socket, multicast, tx).await {
            log::error!("fatal error handling netlink route messages: {}", e);
        }

        // make sure socket is kept alive while we're reading messages
        drop(socket);
    });

    Ok(rx)
}

async fn handle_netlink_route_messages(
    socket: &NlRouter,
    mut multicast: NlRouterReceiverHandle<u16, Genlmsghdr<u8, u16>>,
    tx: Sender<InterfaceUpdate>,
) -> Result<Infallible> {
    // listen for multicast events
    type Next = Option<std::result::Result<Nlmsghdr<Rtm, Ifaddrmsg>, RouterError<Rtm, Ifaddrmsg>>>;
    loop {
        match multicast.next().await as Next {
            None => bail!("Unexpected end of netlink route stream"),
            // we got a multicast event
            Some(response) => {
                // check we got a message (not an error)
                let response = match response {
                    Ok(response) => response,
                    Err(e) => {
                        log::error!("error receiving netlink message: {}", e);
                        continue;
                    }
                };

                // check we have a payload
                match response.nl_payload() {
                    // parse payload and send event
                    NlPayload::Payload(_ifaddrmsg) => {
                        // request all interfaces from netlink again - we request it each time because we get ifaddrmsg
                        // events even when the address is deleted (but we can't tell that is was deleted)
                        // TODO: in the future it would be nice to only update the interface which emitted the event
                        tx.send(get_all_interfaces(&socket).await?).await?
                    }
                    // not payload, something is wrong
                    payload => {
                        log::error!("unexpected nl message payload type: {:?}", payload);
                        continue;
                    }
                }
            }
        }
    }
}

/// Request all interfaces with their addresses from rtnetlink(7)
async fn get_all_interfaces(socket: &NlRouter) -> Result<HashMap<i32, NetlinkInterface>> {
    let mut interface_map = HashMap::<i32, NetlinkInterface>::new();

    // first, get all the interfaces: we need this for the interface names
    {
        let ifinfomsg = IfinfomsgBuilder::default()
            // this is layer 2, so family is unspecified
            .ifi_family(RtAddrFamily::Unspecified)
            .ifi_type(Arphrd::Netrom)
            // when index is zero, it fetches them all
            .ifi_index(0)
            .build()?;

        let mut recv = socket
            .send::<Rtm, Ifinfomsg, Rtm, Ifinfomsg>(
                Rtm::Getlink,
                NlmF::REQUEST | NlmF::DUMP | NlmF::ACK,
                NlPayload::Payload(ifinfomsg),
            )
            .await?;

        type Next =
            Option<std::result::Result<Nlmsghdr<Rtm, Ifinfomsg>, RouterError<Rtm, Ifinfomsg>>>;
        while let Some(response) = recv.next().await as Next {
            let header = match response {
                Ok(header) => header,
                Err(e) => bail!("an error occurred receiving rtnetlink message: {}", e),
            };

            if let NlPayload::Payload(ifinfomsg) = header.nl_payload() {
                // handle to the attributes of this message
                let attr_handle = ifinfomsg.rtattrs().get_attr_handle();

                // extract interface name
                let mut interface_info = NetlinkInterface {
                    index: *ifinfomsg.ifi_index(),
                    name: match attr_handle.get_attr_payload_as_with_len::<String>(Ifla::Ifname) {
                        Ok(interface) => interface.into(),
                        Err(e) => {
                            log::error!(
                                "failed to parse interface name from ifinfomsg: {} :: {:?}",
                                e,
                                ifinfomsg
                            );
                            continue;
                        }
                    },
                    mac_address: None,
                    ip_addresses: HashSet::new(),
                };

                // extract mac address if set
                if let Ok(bytes) =
                    attr_handle.get_attr_payload_as_with_len_borrowed::<&[u8]>(Ifla::Address)
                {
                    if let Ok(array) = bytes.try_into() {
                        interface_info.mac_address = Some(array);
                    }
                }

                interface_map.insert(*ifinfomsg.ifi_index(), interface_info);
            }
        }
    }

    // ... next, get v4 & v6 addresses of each interface
    {
        for family in [RtAddrFamily::Inet, RtAddrFamily::Inet6] {
            let ifaddrmsg = IfaddrmsgBuilder::default()
                .ifa_family(family)
                .ifa_index(0)
                .ifa_prefixlen(0)
                .ifa_scope(RtScope::Universe)
                .build()?;

            let mut recv = socket
                .send::<Rtm, Ifaddrmsg, Rtm, Ifaddrmsg>(
                    Rtm::Getaddr,
                    NlmF::REQUEST | NlmF::DUMP | NlmF::ACK,
                    NlPayload::Payload(ifaddrmsg),
                )
                .await?;

            type Next =
                Option<std::result::Result<Nlmsghdr<Rtm, Ifaddrmsg>, RouterError<Rtm, Ifaddrmsg>>>;
            while let Some(response) = recv.next().await as Next {
                let header = match response {
                    Ok(header) => header,
                    Err(e) => bail!("an error occurred receiving rtnetlink message: {}", e),
                };

                if let NlPayload::Payload(ifaddrmsg) = header.nl_payload() {
                    match interface_map.get_mut(ifaddrmsg.ifa_index()) {
                        Some(if_info) => {
                            // handle to the attributes of this message
                            let attr_handle = ifaddrmsg.rtattrs().get_attr_handle();

                            // extract address
                            match ifaddrmsg.ifa_family() {
                                RtAddrFamily::Inet => {
                                    if let Ok(addr) =
                                        attr_handle.get_attr_payload_as::<u32>(Ifa::Address)
                                    {
                                        if_info
                                            .ip_addresses
                                            .insert(IpAddr::V4(Ipv4Addr::from(u32::from_be(addr))));
                                    }
                                }
                                RtAddrFamily::Inet6 => {
                                    if let Ok(addr) =
                                        attr_handle.get_attr_payload_as::<u128>(Ifa::Address)
                                    {
                                        if_info.ip_addresses.insert(IpAddr::V6(Ipv6Addr::from(
                                            u128::from_be(addr),
                                        )));
                                    }
                                }
                                _ => {
                                    continue;
                                }
                            }
                        }
                        None => {
                            log::error!(
                                "received ifaddrmsg for unknown interface: {:?}",
                                ifaddrmsg
                            );
                            continue;
                        }
                    }
                }
            }
        }
    }

    Ok(interface_map)
}

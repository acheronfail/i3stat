pub mod filter;

use std::net::IpAddr;

use tokio::sync::{broadcast, mpsc, OnceCell};

use self::filter::InterfaceFilter;
use super::route::InterfaceUpdate;
use super::NetlinkInterface;
use crate::error::Result;
use crate::util::netlink_ipaddr_listen;

static NET_RX: OnceCell<Net> = OnceCell::const_new();

// structs ---------------------------------------------------------------------

#[derive(Debug)]
pub struct Net {
    tx: mpsc::Sender<()>,
    rx: broadcast::Receiver<InterfaceUpdate>,
}

impl Net {
    fn new(tx: mpsc::Sender<()>, rx: broadcast::Receiver<InterfaceUpdate>) -> Net {
        Net { tx, rx }
    }

    pub async fn wait_for_change(&mut self) -> Result<Interfaces> {
        Ok(self.rx.recv().await?.into())
    }

    pub async fn trigger_update(&self) -> Result<()> {
        Ok(self.tx.send(()).await?)
    }

    pub async fn update_now(&mut self) -> Result<Interfaces> {
        self.trigger_update().await?;
        self.wait_for_change().await
    }
}

impl Clone for Net {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            rx: self.rx.resubscribe(),
        }
    }
}

#[derive(Debug, Default)]
pub struct Interfaces {
    inner: InterfaceUpdate,
}

impl Interfaces {
    /// Count of total interfaces
    pub fn len_interfaces(&self) -> usize {
        self.inner.len()
    }

    /// Count of total addresses across all interfaces
    pub fn len_addresses(&self) -> usize {
        self.inner
            .iter()
            .map(|(_, int)| int.ip_addresses.len())
            .sum()
    }

    /// Checks if there are any addresses (and thus interfaces) at all
    pub fn is_empty(&self) -> bool {
        self.len_addresses() == 0
    }

    /// Get a specific interface by its index
    pub fn get_interface(&self, index: i32) -> Option<&NetlinkInterface> {
        self.inner.get(&index)
    }

    /// Get an address by its index (where index is `0..interfaces.len_addresses()`)
    pub fn get_address_at(&self, address_index: usize) -> Option<(&NetlinkInterface, &IpAddr)> {
        self.inner
            .iter()
            .flat_map(|(_, int)| {
                int.ip_addresses
                    .iter()
                    .map(|addr| (int, addr))
                    .collect::<Vec<_>>()
            })
            .nth(address_index)
    }

    /// Apply a set of filters to this struct and return a new struct
    pub fn filtered(mut self, filters: &[InterfaceFilter]) -> Interfaces {
        if filters.is_empty() {
            return self;
        }

        self.inner.retain(|_, interface| {
            interface
                .ip_addresses
                .retain(|addr| filters.iter().any(|f| f.matches(&interface.name, addr)));

            !interface.ip_addresses.is_empty()
        });

        self
    }
}

impl From<InterfaceUpdate> for Interfaces {
    fn from(inner: InterfaceUpdate) -> Self {
        Interfaces { inner }
    }
}

// subscribe -------------------------------------------------------------------

pub async fn net_subscribe() -> Result<Net> {
    Ok(NET_RX.get_or_try_init(start_task).await?.clone())
}

async fn start_task() -> Result<Net> {
    let (iface_tx, iface_rx) = broadcast::channel(2);
    let (manual_tx, manual_rx) = mpsc::channel(1);

    // spawn task to watch for network updates
    tokio::task::spawn_local(watch_net_updates(iface_tx, manual_rx));

    // trigger an initial update
    manual_tx.send(()).await?;

    Ok(Net::new(manual_tx, iface_rx))
}

async fn watch_net_updates(
    tx: broadcast::Sender<InterfaceUpdate>,
    manual_trigger: mpsc::Receiver<()>,
) -> Result<()> {
    let mut rx = netlink_ipaddr_listen(manual_trigger).await?;
    loop {
        if let Some(mut interfaces) = rx.recv().await {
            interfaces.retain(|_, int| {
                log::trace!("found interface: {:?}", int);

                // some address filtering
                int.ip_addresses.retain(|addr| match addr {
                    // get rid of loopback addresses
                    any if any.is_loopback() => false,
                    // get rid of link local addresses
                    IpAddr::V4(v4) if v4.is_link_local() => false,
                    IpAddr::V6(v6) if v6.is_unicast_link_local() => false,
                    _ => true,
                });

                // filter out interfaces without any addresses
                !int.ip_addresses.is_empty()
            });

            tx.send(interfaces)?;
        }
    }
}

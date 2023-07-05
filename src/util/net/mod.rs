pub mod filter;
pub mod interface;

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

#[derive(Debug)]
pub struct Interfaces {
    inner: Vec<NetlinkInterface>,
}

impl Interfaces {
    pub fn filtered(self, filters: &[InterfaceFilter]) -> Vec<NetlinkInterface> {
        if filters.is_empty() {
            return self.inner;
        }

        let mut filtered = vec![];
        for mut interface in self.inner {
            interface
                .ip_addresses
                .retain(|addr| filters.iter().any(|f| f.matches(&interface.name, addr)));

            if !interface.ip_addresses.is_empty() {
                filtered.push(interface);
            }
        }

        filtered
    }
}

impl From<InterfaceUpdate> for Interfaces {
    fn from(value: InterfaceUpdate) -> Self {
        let mut inner = value.into_values().collect::<Vec<_>>();
        inner.sort_unstable_by_key(|int| int.index);
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
            // filter out loopback
            interfaces.retain(|_, int| int.name.as_ref() != "lo");
            tx.send(interfaces)?;
        }
    }
}

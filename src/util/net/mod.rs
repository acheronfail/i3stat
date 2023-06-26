pub mod filter;
pub mod interface;

use std::ops::Deref;

use futures::StreamExt;
use tokio::sync::{broadcast, mpsc, OnceCell};

use self::filter::InterfaceFilter;
pub use self::interface::Interface;
use crate::dbus::network_manager::NetworkManagerProxy;
use crate::dbus::{dbus_connection, BusType};
use crate::error::Result;

// FIXME: I don't like this interface list thing
#[derive(Debug, Clone)]
pub struct InterfaceList {
    inner: Vec<Interface>,
}

impl InterfaceList {
    pub fn filtered(self, filter: &[InterfaceFilter]) -> Vec<Interface> {
        self.inner
            .into_iter()
            .filter(|i| {
                if filter.is_empty() {
                    true
                } else {
                    filter.iter().any(|filter| filter.matches(i))
                }
            })
            .collect()
    }
}

impl Deref for InterfaceList {
    type Target = Vec<Interface>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

static NET_RX: OnceCell<Net> = OnceCell::const_new();

#[derive(Debug)]
pub struct Net {
    tx: mpsc::Sender<()>,
    rx: broadcast::Receiver<InterfaceList>,
}

impl Net {
    fn new(tx: mpsc::Sender<()>, rx: broadcast::Receiver<InterfaceList>) -> Net {
        Net { tx, rx }
    }

    pub async fn wait_for_change(&mut self) -> Result<InterfaceList> {
        Ok(self.rx.recv().await?)
    }

    pub async fn trigger_update(&self) -> Result<()> {
        Ok(self.tx.send(()).await?)
    }

    pub async fn update_now(&mut self) -> Result<InterfaceList> {
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

pub async fn net_subscribe() -> Result<Net> {
    Ok(NET_RX.get_or_try_init(start_task).await?.clone())
}

async fn start_task() -> Result<Net> {
    let (iface_tx, iface_rx) = broadcast::channel(2);
    let (manual_tx, manual_rx) = mpsc::channel(1);
    tokio::task::spawn_local(watch_net_updates(iface_tx, manual_rx));

    Ok(Net::new(manual_tx, iface_rx))
}

async fn watch_net_updates(
    tx: broadcast::Sender<InterfaceList>,
    mut rx: mpsc::Receiver<()>,
) -> Result<()> {
    // TODO: investigate effort of checking network state with netlink rather than dbus
    let connection = dbus_connection(BusType::System).await?;
    let nm = NetworkManagerProxy::new(&connection).await?;
    // this captures all network connect/disconnect events
    let mut state_changed = nm.receive_state_changed().await?;
    // this captures all vpn interface connect/disconnect events
    let mut active_con_change = nm.receive_active_connections_objpath_changed().await;

    let mut force_update = true;
    let mut last_value = vec![];
    loop {
        // check current interfaces
        let interfaces = Interface::get_interfaces()?;

        // send updates to subscribers only if it's changed since last time
        if force_update || last_value != interfaces {
            force_update = false;
            last_value = interfaces.clone();
            tx.send(InterfaceList { inner: interfaces })?;
        }

        tokio::select! {
            // callers can manually trigger updates
            Some(()) = rx.recv() => {
                force_update = true;
                continue;
            },
            // catch updates from NetworkManager via dbus
            opt = state_changed.next() => if opt.is_none() {
                bail!("unexpected end of NetworkManagerProxy::receive_state_changed stream");
            },
            opt = active_con_change.next() => if opt.is_none() {
                bail!("unexpected end of NetworkManagerProxy::receive_active_connections_objpath_changed stream");
            }
        }
    }
}

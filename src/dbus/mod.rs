pub mod dunst;
pub mod network_manager;
pub mod notifications;

use std::error::Error;

use tokio::sync::OnceCell;
use zbus::Connection;

#[derive(Debug, Copy, Clone)]
pub enum BusType {
    Session,
    System,
}

static DBUS_SYSTEM: OnceCell<Connection> = OnceCell::const_new();
static DBUS_SESSION: OnceCell<Connection> = OnceCell::const_new();

pub async fn dbus_connection(bus: BusType) -> Result<&'static Connection, Box<dyn Error>> {
    Ok(match bus {
        BusType::Session => {
            DBUS_SESSION
                .get_or_try_init(|| async { Connection::session().await })
                .await?
        }
        BusType::System => {
            DBUS_SYSTEM
                .get_or_try_init(|| async { Connection::system().await })
                .await?
        }
    })
}

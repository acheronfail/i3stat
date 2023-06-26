mod client;
pub mod protocol;
mod server;

use std::env;
use std::path::PathBuf;

use tokio_util::sync::CancellationToken;

pub use self::server::{create_ipc_socket, handle_ipc_events};
use crate::config::AppConfig;
use crate::dispatcher::Dispatcher;
use crate::error::Result;
use crate::i3::I3Item;
use crate::util::RcCell;

#[derive(Debug, Clone)]
pub struct IpcContext {
    bar: RcCell<Vec<I3Item>>,
    token: CancellationToken,
    config: RcCell<AppConfig>,
    dispatcher: RcCell<Dispatcher>,
}

impl IpcContext {
    pub fn new(
        bar: RcCell<Vec<I3Item>>,
        token: CancellationToken,
        config: RcCell<AppConfig>,
        dispatcher: RcCell<Dispatcher>,
    ) -> IpcContext {
        IpcContext {
            bar,
            token,
            config,
            dispatcher,
        }
    }
}

pub fn get_socket_path(socket_path: Option<&PathBuf>) -> Result<PathBuf> {
    socket_path.map_or_else(
        || {
            let i3_socket = PathBuf::from(match env::var("I3SOCK") {
                Ok(v) => v,
                Err(e) => bail!("I3SOCK: {}", e),
            });

            let my_socket = PathBuf::from(&i3_socket).with_extension(
                i3_socket
                    .extension()
                    .map(|ext| format!("{}.istat", ext.to_string_lossy()))
                    .unwrap(),
            );

            Ok(my_socket)
        },
        |p| Ok(p.clone()),
    )
}

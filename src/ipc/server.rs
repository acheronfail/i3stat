use std::convert::Infallible;
use std::error::Error;
use std::io::ErrorKind;

use tokio::net::{UnixListener, UnixStream};

use super::client::handle_ipc_client;
use crate::config::AppConfig;
use crate::ipc::protocol::{encode_ipc_msg, IpcReply};
use crate::ipc::IpcContext;
use crate::util::RcCell;

pub async fn create_ipc_socket(config: &RcCell<AppConfig>) -> Result<UnixListener, Box<dyn Error>> {
    let socket_path = config.socket();

    // try to remove socket if one exists
    match tokio::fs::remove_file(&socket_path).await {
        Ok(_) => {}
        Err(e) if e.kind() == ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }

    Ok(UnixListener::bind(&socket_path)?)
}

pub async fn handle_ipc_events(
    listener: UnixListener,
    ctx: IpcContext,
) -> Result<Infallible, Box<dyn Error>> {
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let ipc_ctx = ctx.clone();
                tokio::task::spawn_local(async move {
                    match handle_ipc_client(stream, ipc_ctx).await {
                        Ok(_) => {}
                        Err(e) => log::error!("ipc error: {}", e),
                    }
                });
            }
            Err(e) => return Err(format!("failed to setup ipc connection: {}", e).into()),
        }
    }
}

pub async fn send_ipc_response(stream: &UnixStream, resp: &IpcReply) -> Result<(), Box<dyn Error>> {
    let data = encode_ipc_msg(resp)?;
    let mut idx = 0;
    loop {
        stream.writable().await?;
        match stream.try_write(&data[idx..]) {
            Ok(n) => {
                idx += n;
                if idx == data.len() {
                    break Ok(());
                }
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => continue,
            Err(e) => return Err(e.into()),
        }
    }
}

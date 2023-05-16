use std::collections::HashMap;
use std::convert::Infallible;
use std::env;
use std::error::Error;
use std::io::ErrorKind;
use std::path::PathBuf;

use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::oneshot;

use crate::context::{BarEvent, CustomResponse};
use crate::dispatcher::Dispatcher;
use crate::i3::I3ClickEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IpcBarEvent {
    Click(I3ClickEvent),
    Signal,
    Custom(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IpcMessage {
    Info,
    BarEvent {
        instance: String,
        event: IpcBarEvent,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IpcReply {
    Result(IpcResult),
    Response(Value),
    // NOTE: ANSI text
    Help(String),
    Info(HashMap<usize, String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "detail")]
pub enum IpcResult {
    Success(Option<String>),
    Failure(String),
}

pub fn get_socket_path(socket_path: Option<PathBuf>) -> Result<PathBuf, Box<dyn Error>> {
    socket_path.map_or_else(
        || {
            let i3_socket = PathBuf::from(env::var("I3SOCK")?);
            let my_socket = PathBuf::from(&i3_socket).with_extension(
                i3_socket
                    .extension()
                    .map(|ext| format!("{}.staturs", ext.to_string_lossy()))
                    .unwrap(),
            );

            Ok(my_socket)
        },
        |p| Ok(p),
    )
}

pub async fn handle_ipc_events(
    socket_path: Option<PathBuf>,
    dispatcher: Dispatcher,
) -> Result<Infallible, Box<dyn Error>> {
    let socket_path = get_socket_path(socket_path)?;

    // try to remove socket if one exists
    match tokio::fs::remove_file(&socket_path).await {
        Ok(_) => {}
        Err(e) if e.kind() == ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }

    let listener = UnixListener::bind(&socket_path)?;
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let bar_txs = dispatcher.clone();
                tokio::task::spawn_local(async move {
                    match handle_ipc_client(stream, bar_txs).await {
                        Ok(_) => {}
                        Err(e) => log::error!("ipc error: {}", e),
                    }
                });
            }
            Err(e) => {
                todo!("{}", e);
            }
        }
    }
}

async fn handle_ipc_client(
    stream: UnixStream,
    dispatcher: Dispatcher,
) -> Result<(), Box<dyn Error>> {
    // TODO: upper limit? error if too big? how to handle that? add len in ipc protocol?
    let mut buf = vec![0; 1024];
    loop {
        stream.readable().await?;

        // there may be false positives readiness events
        match stream.try_read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let msg = serde_json::from_slice::<IpcMessage>(&buf[0..n])?;
                // log::trace!("ipc message: {}", msg);
                match msg {
                    IpcMessage::Info => {
                        send_ipc_response(&stream, &IpcReply::Info(dispatcher.instance_mapping()))
                            .await?;
                    }
                    IpcMessage::BarEvent { instance, event } => {
                        // NOTE: special considerations here for `instance`: if it's a number, then it maps to the item at the index
                        // otherwise, it's interpreted as a tag and the first item with that tag will be sent the item
                        let instance = match instance.parse::<usize>() {
                            // ipc message contained an index
                            Ok(idx) => idx,
                            Err(e) => match dispatcher.iter().find_map(|(idx, (_, item))| {
                                if instance == item.tag() {
                                    Some(idx)
                                } else {
                                    None
                                }
                            }) {
                                // ipc message contained a tag
                                Some(idx) => *idx,
                                // error
                                None => {
                                    let err =
                                        format!("failed to parse ipc instance property: {}", e);
                                    log::warn!("{}", err);
                                    send_ipc_response(
                                        &stream,
                                        &IpcReply::Result(IpcResult::Failure(err)),
                                    )
                                    .await?;
                                    break;
                                }
                            },
                        };

                        let (event, rx) = match event {
                            IpcBarEvent::Signal => (BarEvent::Signal, None),
                            IpcBarEvent::Click(click) => (BarEvent::Click(click), None),
                            IpcBarEvent::Custom(payload) => {
                                let (responder, receiver) = oneshot::channel();
                                (BarEvent::Custom { payload, responder }, Some(receiver))
                            }
                        };

                        let reply = match dispatcher.send_bar_event(instance, event).await {
                            Ok(()) => match rx {
                                Some(rx) => match rx.await {
                                    Ok(CustomResponse::Help(help)) => {
                                        IpcReply::Help(help.ansi().to_string())
                                    }
                                    Ok(CustomResponse::Json(value)) => IpcReply::Response(value),
                                    Err(_) => IpcReply::Result(IpcResult::Failure(
                                        "not listening for events".into(),
                                    )),
                                },
                                None => IpcReply::Result(IpcResult::Success(None)),
                            },
                            Err(e) => {
                                log::warn!("{}", e);
                                IpcReply::Result(IpcResult::Failure(e.to_string()))
                            }
                        };
                        send_ipc_response(&stream, &reply).await?;
                    }
                }

                break;
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => continue,
            Err(e) => return Err(e.into()),
        }
    }

    Ok(())
}

async fn send_ipc_response(stream: &UnixStream, resp: &IpcReply) -> Result<(), Box<dyn Error>> {
    let data = serde_json::to_vec(resp)?;
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

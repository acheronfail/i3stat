use std::convert::Infallible;
use std::env;
use std::error::Error;
use std::io::ErrorKind;
use std::path::PathBuf;

use indexmap::IndexMap;
use serde::Serialize;
use serde_derive::Deserialize;
use serde_json::Value;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use crate::config::AppConfig;
use crate::context::{BarEvent, CustomResponse};
use crate::dispatcher::Dispatcher;
use crate::i3::{I3ClickEvent, I3Item};
use crate::theme::Theme;
use crate::util::RcCell;

pub const IPC_HEADER_LEN: usize = std::mem::size_of::<u64>();

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
    RefreshAll,
    GetBar,
    GetConfig,
    GetTheme,
    SetTheme(Value),
    BarEvent {
        instance: String,
        event: IpcBarEvent,
    },
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IpcReply {
    Result(IpcResult),
    // NOTE: ANSI text
    Help(String),
    Info(IndexMap<usize, String>),
    Value(Value),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type", content = "detail")]
pub enum IpcResult {
    Success(Option<String>),
    Failure(String),
}

pub fn get_socket_path(socket_path: Option<&PathBuf>) -> Result<PathBuf, Box<dyn Error>> {
    socket_path.map_or_else(
        || {
            let i3_socket = PathBuf::from(match env::var("I3SOCK") {
                Ok(v) => v,
                Err(e) => return Err(format!("I3SOCK: {}", e).into()),
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

pub fn encode_ipc_msg<T: Serialize>(t: T) -> Result<Vec<u8>, Box<dyn Error>> {
    let msg = serde_json::to_vec(&t)?;
    // header is a u64 of length
    let mut payload = (msg.len() as u64).to_le_bytes().to_vec();
    // followed by bytes of the body encoded as json
    payload.extend(msg);
    Ok(payload)
}

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
    config: RcCell<AppConfig>,
    dispatcher: RcCell<Dispatcher>,
    bar: RcCell<Vec<I3Item>>,
    token: CancellationToken,
) -> Result<Infallible, Box<dyn Error>> {
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let dispatcher = dispatcher.clone();
                let config = config.clone();
                let bar = bar.clone();
                let token = token.clone();
                tokio::task::spawn_local(async move {
                    match handle_ipc_client(stream, config, dispatcher, bar, token).await {
                        Ok(_) => {}
                        Err(e) => log::error!("ipc error: {}", e),
                    }
                });
            }
            Err(e) => return Err(format!("failed to setup ipc connection: {}", e).into()),
        }
    }
}

async fn handle_ipc_client(
    stream: UnixStream,
    config: RcCell<AppConfig>,
    dispatcher: RcCell<Dispatcher>,
    bar: RcCell<Vec<I3Item>>,
    token: CancellationToken,
) -> Result<(), Box<dyn Error>> {
    // first read the length header of the IPC message
    let mut buf = [0; IPC_HEADER_LEN];
    loop {
        stream.readable().await?;
        match stream.try_read(&mut buf) {
            Ok(0) => break,
            Ok(IPC_HEADER_LEN) => {
                let len = u64::from_le_bytes(buf);
                handle_ipc_request(&stream, config, dispatcher, bar, len as usize, token).await?;
                break;
            }
            Ok(n) => {
                return Err(format!(
                    "failed reading ipc header, read {} bytes, expected {}",
                    n, IPC_HEADER_LEN
                )
                .into())
            }
            // there may be false positives readiness events
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => continue,
            Err(e) => return Err(e.into()),
        }
    }

    Ok(())
}

async fn handle_ipc_request(
    stream: &UnixStream,
    mut config: RcCell<AppConfig>,
    dispatcher: RcCell<Dispatcher>,
    bar: RcCell<Vec<I3Item>>,
    len: usize,
    cancel_token: CancellationToken,
) -> Result<(), Box<dyn Error>> {
    // read ipc message entirely
    let mut buf = vec![0; len];
    let mut idx = 0;
    loop {
        stream.readable().await?;
        match stream.try_read(&mut buf) {
            Ok(0) => {
                return Err(format!(
                    "unexpected end of ipc stream, read {} bytes, expected: {}",
                    idx, len
                )
                .into())
            }
            Ok(n) => {
                idx += n;
                if idx == len {
                    break;
                }
            }
            // there may be false positives readiness events
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => continue,
            Err(e) => return Err(e.into()),
        }
    }

    // handle ipc message
    let msg = serde_json::from_slice::<IpcMessage>(&buf)?;
    match msg {
        IpcMessage::Shutdown => {
            send_ipc_response(&stream, &IpcReply::Result(IpcResult::Success(None))).await?;
            cancel_token.cancel();
        }
        IpcMessage::GetBar => {
            send_ipc_response(&stream, &IpcReply::Value(serde_json::to_value(&*bar)?)).await?;
        }
        IpcMessage::Info => {
            send_ipc_response(&stream, &IpcReply::Info(config.item_name_map())).await?;
        }
        IpcMessage::GetConfig => {
            send_ipc_response(&stream, &IpcReply::Value(serde_json::to_value(&*config)?)).await?;
        }
        IpcMessage::GetTheme => {
            send_ipc_response(
                &stream,
                &IpcReply::Value(serde_json::to_value(&config.theme)?),
            )
            .await?;
        }
        IpcMessage::SetTheme(json) => {
            let reply = match serde_json::from_value::<Theme>(json) {
                Ok(new) => {
                    config.theme = new;
                    IpcReply::Result(IpcResult::Success(None))
                }
                Err(e) => IpcReply::Result(IpcResult::Failure(e.to_string())),
            };
            send_ipc_response(&stream, &reply).await?;
        }
        IpcMessage::RefreshAll => {
            dispatcher.signal_all().await?;
            send_ipc_response(&stream, &IpcReply::Result(IpcResult::Success(None))).await?;
        }
        IpcMessage::BarEvent { instance, event } => {
            // NOTE: special considerations here for `instance`: if it's a number, then it maps to the item at the index
            // otherwise, it's interpreted as a name and the first item with that name is chosen
            let instance = match instance.parse::<usize>() {
                // ipc message contained an index
                Ok(idx) => idx,
                Err(e) => {
                    match config.item_name_map().into_iter().find_map(|(idx, name)| {
                        if instance == name {
                            Some(idx)
                        } else {
                            None
                        }
                    }) {
                        // ipc message contained a tag
                        Some(idx) => idx,
                        // error
                        None => {
                            let err = format!("failed to parse ipc instance property: {}", e);
                            log::warn!("{}", err);
                            send_ipc_response(&stream, &IpcReply::Result(IpcResult::Failure(err)))
                                .await?;
                            return Ok(());
                        }
                    }
                }
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
                        Ok(CustomResponse::Help(help)) => IpcReply::Help(help.ansi().to_string()),
                        Ok(CustomResponse::Json(value)) => IpcReply::Value(value),
                        Err(_) => {
                            IpcReply::Result(IpcResult::Failure("not listening for events".into()))
                        }
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

    Ok(())
}

async fn send_ipc_response(stream: &UnixStream, resp: &IpcReply) -> Result<(), Box<dyn Error>> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_ipc_msg() {
        let bytes = encode_ipc_msg::<IpcMessage>(IpcMessage::Info).unwrap();
        let header = &bytes[0..IPC_HEADER_LEN];
        let body = &bytes[IPC_HEADER_LEN..];
        assert_eq!(header, 6_u64.to_le_bytes());
        assert_eq!(body, br#""info""#);
    }

    #[test]
    fn test_encode_ipc_reply() {
        let bytes = encode_ipc_msg::<IpcReply>(IpcReply::Result(IpcResult::Success(None))).unwrap();
        let header = &bytes[0..IPC_HEADER_LEN];
        let body = &bytes[IPC_HEADER_LEN..];
        assert_eq!(header, 43_u64.to_le_bytes());
        assert_eq!(body, br#"{"result":{"type":"success","detail":null}}"#);
    }
}

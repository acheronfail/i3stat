use std::io::ErrorKind;

use tokio::net::UnixStream;
use tokio::sync::oneshot;

use crate::context::{BarEvent, CustomResponse};
use crate::error::Result;
use crate::ipc::protocol::{IpcBarEvent, IpcMessage, IpcReply, IpcResult, IPC_HEADER_LEN};
use crate::ipc::server::send_ipc_response;
use crate::ipc::IpcContext;
use crate::theme::Theme;

pub async fn handle_ipc_client(stream: UnixStream, ctx: IpcContext) -> Result<()> {
    // first read the length header of the IPC message
    let mut buf = [0; IPC_HEADER_LEN];
    loop {
        stream.readable().await?;
        match stream.try_read(&mut buf) {
            Ok(0) => break,
            Ok(IPC_HEADER_LEN) => {
                let len = u64::from_le_bytes(buf);
                handle_ipc_request(&stream, ctx, len as usize).await?;
                break;
            }
            Ok(n) => {
                bail!(
                    "failed reading ipc header, read {} bytes, expected {}",
                    n,
                    IPC_HEADER_LEN
                )
            }
            // there may be false positives readiness events
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => continue,
            Err(e) => bail!(e),
        }
    }

    Ok(())
}

async fn handle_ipc_request(stream: &UnixStream, mut ctx: IpcContext, len: usize) -> Result<()> {
    // read ipc message entirely
    let mut buf = vec![0; len];
    let mut idx = 0;
    loop {
        stream.readable().await?;
        match stream.try_read(&mut buf) {
            Ok(0) => {
                bail!(
                    "unexpected end of ipc stream, read {} bytes, expected: {}",
                    idx,
                    len
                )
            }
            Ok(n) => {
                idx += n;
                if idx == len {
                    break;
                }
            }
            // there may be false positives readiness events
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => continue,
            Err(e) => bail!(e),
        }
    }

    // handle ipc message
    let msg = serde_json::from_slice::<IpcMessage>(&buf)?;
    match msg {
        IpcMessage::Shutdown => {
            send_ipc_response(&stream, &IpcReply::Result(IpcResult::Success(None))).await?;
            ctx.token.cancel();
        }
        IpcMessage::GetBar => {
            send_ipc_response(&stream, &IpcReply::Value(serde_json::to_value(&*ctx.bar)?)).await?;
        }
        IpcMessage::Info => {
            let info = serde_json::to_value(ctx.config.item_idx_to_name())?;
            send_ipc_response(&stream, &IpcReply::Value(info)).await?;
        }
        IpcMessage::GetConfig => {
            send_ipc_response(
                &stream,
                &IpcReply::Value(serde_json::to_value(&*ctx.config)?),
            )
            .await?;
        }
        IpcMessage::GetTheme => {
            send_ipc_response(
                &stream,
                &IpcReply::Value(serde_json::to_value(&ctx.config.theme)?),
            )
            .await?;
        }
        IpcMessage::SetTheme(json) => {
            let reply = match serde_json::from_value::<Theme>(json) {
                Ok(new) => {
                    ctx.config.theme = new;
                    IpcReply::Result(IpcResult::Success(None))
                }
                Err(e) => IpcReply::Result(IpcResult::Failure(e.to_string())),
            };
            send_ipc_response(&stream, &reply).await?;
            ctx.dispatcher.manual_bar_update().await?;
        }
        IpcMessage::RefreshAll => {
            ctx.dispatcher.signal_all().await?;
            send_ipc_response(&stream, &IpcReply::Result(IpcResult::Success(None))).await?;
        }
        IpcMessage::BarEvent { instance, event } => {
            // NOTE: special considerations here for `instance`: if it's a number, then it maps to the item at the index
            // otherwise, it's interpreted as a name and the first item with that name is chosen
            let instance = match instance.parse::<usize>() {
                // ipc message contained an index
                Ok(idx) => idx,
                Err(e) => {
                    match ctx
                        .config
                        .item_idx_to_name()
                        .iter()
                        .find_map(
                            |(idx, name)| {
                                if instance == *name {
                                    Some(idx)
                                } else {
                                    None
                                }
                            },
                        ) {
                        // ipc message contained a tag
                        Some(idx) => *idx,
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

            let reply = match ctx.dispatcher.send_bar_event(instance, event).await {
                Ok(()) => match rx {
                    Some(rx) => match rx.await {
                        Ok(CustomResponse::Help(help)) => IpcReply::Help(help.ansi().to_string()),
                        Ok(CustomResponse::Json(value)) => IpcReply::Value(value),
                        Err(_) => IpcReply::Result(IpcResult::Failure(
                            "bar item not listening for response".into(),
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

    Ok(())
}

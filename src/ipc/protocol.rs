use serde::Serialize;
use serde_derive::Deserialize;
use serde_json::Value;

use crate::error::Result;
use crate::i3::I3ClickEvent;

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
    Value(Value),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type", content = "detail")]
pub enum IpcResult {
    Success(Option<String>),
    Failure(String),
}

pub fn encode_ipc_msg<T: Serialize>(t: T) -> Result<Vec<u8>> {
    let msg = serde_json::to_vec(&t)?;
    // header is a u64 of length
    let mut payload = (msg.len() as u64).to_le_bytes().to_vec();
    // followed by bytes of the body encoded as json
    payload.extend(msg);
    Ok(payload)
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

use clap::{Parser, ValueEnum};
use num_traits::ToPrimitive;
use serde_derive::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::oneshot;

use super::{Object, Port, PulseState, Vol};
use crate::context::CustomResponse;
use crate::util::RcCell;

#[derive(Debug, Copy, Clone, ValueEnum)]
enum Bool {
    On,
    Off,
    Yes,
    No,
    True,
    False,
    Enabled,
    Disabled,
}

impl From<Bool> for bool {
    fn from(value: Bool) -> Self {
        match value {
            Bool::On => true,
            Bool::Off => false,
            Bool::Yes => true,
            Bool::No => false,
            Bool::True => true,
            Bool::False => false,
            Bool::Enabled => true,
            Bool::Disabled => false,
        }
    }
}

#[derive(Debug, Parser)]
#[command(name = "pulse", no_binary_name = true)]
enum PulseCommand {
    Info,
    List { what: Object },
    VolumeUp { what: Object },
    VolumeDown { what: Object },
    VolumeSet { what: Object, vol: u32 },
    Mute { what: Object, mute: Bool },
    MuteToggle { what: Object },
    // TODO: set default object with idx or name
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "detail")]
pub enum PulseResponse {
    Info(Value),
    List(Value),
    Success,
    Failure(String),
}

impl Port {
    fn to_value(&self) -> Value {
        json!({
            "index": self.index,
            "name": self.name,
            "volume": self.volume_pct(),
            "mute": self.mute,
            "port_type": self.port_type.map(|t| t.to_i64()).flatten()
        })
    }
}

impl RcCell<PulseState> {
    pub fn handle_custom_message(&self, args: Vec<String>, tx: oneshot::Sender<CustomResponse>) {
        let resp = match PulseCommand::try_parse_from(args) {
            Ok(cmd) => {
                let resp = match cmd {
                    PulseCommand::Info => PulseResponse::Info(json!({
                        "default_sink": &*self.default_sink,
                        "default_source": &*self.default_source,
                        "sinks": self.sinks.iter().map(|p| p.to_value()).collect::<Value>(),
                        "sources": self.sources.iter().map(|p| p.to_value()).collect::<Value>(),
                    })),
                    PulseCommand::List { what } => match what {
                        Object::Sink => {
                            PulseResponse::List(self.sinks.iter().map(|p| p.to_value()).collect())
                        }
                        Object::Source => {
                            PulseResponse::List(self.sources.iter().map(|p| p.to_value()).collect())
                        }
                    },
                    PulseCommand::VolumeUp { what } => {
                        self.set_volume(what, Vol::Incr(self.increment));
                        PulseResponse::Success
                    }
                    PulseCommand::VolumeDown { what } => {
                        self.set_volume(what, Vol::Decr(self.increment));
                        PulseResponse::Success
                    }
                    PulseCommand::VolumeSet { what, vol } => {
                        self.set_volume(what, Vol::Set(vol));
                        PulseResponse::Success
                    }
                    PulseCommand::Mute { what, mute } => {
                        self.set_mute(what, mute.into());
                        PulseResponse::Success
                    }
                    PulseCommand::MuteToggle { what } => {
                        self.toggle_mute(what);
                        PulseResponse::Success
                    }
                };

                CustomResponse::Json(json!(resp))
            }
            Err(e) => CustomResponse::Help(e.render()),
        };

        let _ = tx.send(resp);
    }
}

use clap::{Parser, ValueEnum};
use num_traits::ToPrimitive;
use serde_derive::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::oneshot;

use super::{InOut, Object, PulseState, Vol};
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
    SetDefault { what: Object, name: String },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "detail")]
pub enum PulseResponse {
    Info(Value),
    List(Value),
    Success,
    Failure(String),
}

impl InOut {
    fn to_value(&self) -> Value {
        json!({
            "index": self.index,
            "name": self.name,
            "volume": self.volume_pct(),
            "mute": self.mute,
            "port_type": self.active_port.as_ref().map(|t| t.port_type.to_i64()).flatten()
        })
    }
}

impl RcCell<PulseState> {
    // NOTE: since pulse's callback API requires `FnMut`, but `oneshot::tx.send` consumes itself
    // we wrap it in an option so it's only send once. This should be fine, because pulse only runs
    // this callback once anyway.
    fn custom_responder<F>(tx: oneshot::Sender<CustomResponse>, f: F) -> impl FnMut(bool) + 'static
    where
        F: FnOnce() -> String + 'static,
    {
        let mut tx = Some(tx);
        let mut f = Some(f);
        move |success| match (tx.take(), f.take()) {
            (Some(tx), Some(f)) => {
                let _ = tx.send(CustomResponse::Json(json!(match success {
                    true => PulseResponse::Success,
                    false => PulseResponse::Failure(f()),
                })));
            }
            _ => {}
        }
    }

    pub fn handle_custom_message(
        &mut self,
        args: Vec<String>,
        tx: oneshot::Sender<CustomResponse>,
    ) {
        // TODO: send pulse response from pulse success callbacks for all "success" responses
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
                        return self.set_volume(
                            what,
                            Vol::Incr(self.increment),
                            Self::custom_responder(tx, move || {
                                format!("failed to increment {} volume", what)
                            }),
                        );
                    }
                    PulseCommand::VolumeDown { what } => {
                        return self.set_volume(
                            what,
                            Vol::Decr(self.increment),
                            Self::custom_responder(tx, move || {
                                format!("failed to decrement {} volume", what)
                            }),
                        );
                    }
                    PulseCommand::VolumeSet { what, vol } => {
                        return self.set_volume(
                            what,
                            Vol::Set(vol),
                            Self::custom_responder(tx, move || {
                                format!("failed to set {} volume", what)
                            }),
                        );
                    }
                    PulseCommand::Mute { what, mute } => {
                        return self.set_mute(
                            what,
                            mute.into(),
                            Self::custom_responder(tx, move || {
                                format!("failed to set mute for {}", what)
                            }),
                        );
                    }
                    PulseCommand::MuteToggle { what } => {
                        return self.toggle_mute(
                            what,
                            Self::custom_responder(tx, move || {
                                format!("failed to toggle mute for {}", what)
                            }),
                        );
                    }
                    PulseCommand::SetDefault { what, name } => {
                        return self.set_default(
                            what,
                            name.clone(),
                            Self::custom_responder(tx, move || {
                                format!(
                                    "failed to set default {} to {}, is the name right?",
                                    what, name
                                )
                            }),
                        );
                    }
                };

                CustomResponse::Json(json!(resp))
            }
            Err(e) => CustomResponse::Help(e.render()),
        };

        let _ = tx.send(resp);
    }
}

mod custom;

use std::cell::RefCell;
use std::error::Error;
use std::fmt::Debug;
use std::process;
use std::rc::Rc;

use async_trait::async_trait;
use libpulse_binding::callbacks::ListResult;
use libpulse_binding::context::introspect::{Introspector, SinkInfo, SourceInfo};
use libpulse_binding::context::subscribe::{Facility, InterestMaskSet, Operation};
use libpulse_binding::context::{Context as PAContext, FlagSet, State};
use libpulse_binding::def::DevicePortType;
use libpulse_binding::proplist::properties::{APPLICATION_NAME, APPLICATION_PROCESS_ID};
use libpulse_binding::proplist::Proplist;
use libpulse_binding::volume::{ChannelVolumes, Volume};
use libpulse_tokio::TokioMain;
use serde_derive::{Deserialize, Serialize};
use tokio::sync::mpsc::{self, UnboundedSender};

use crate::context::{BarEvent, BarItem, Context};
use crate::exec::exec;
use crate::i3::{I3Button, I3Item, I3Modifier};
use crate::theme::Theme;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Vol {
    Incr(u32),
    Decr(u32),
    Set(u32),
}

/// Information about a `Sink` or a `Source`
#[derive(Debug, Default, Clone)]
struct Port {
    index: u32,
    name: String,
    volume: ChannelVolumes,
    mute: bool,
    port_type: Option<DevicePortType>,
}

impl Port {
    fn volume_pct(&self) -> u32 {
        let normal = Volume::NORMAL.0;
        (self.volume.max().0 * 100 + normal / 2) / normal
    }

    fn port_symbol(&self) -> Option<&str> {
        match self.port_type {
            Some(DevicePortType::Bluetooth) => Some("󰂰 "),
            Some(DevicePortType::Headphones) => Some("󰋋 "),
            Some(DevicePortType::Headset) => Some("󰋎 "),
            _ => None,
        }
    }
}

macro_rules! impl_port_from {
    ($ty:ty) => {
        impl<'a> From<&'a $ty> for Port {
            fn from(value: &'a $ty) -> Self {
                Port {
                    index: value.index,
                    name: value.name.as_deref().unwrap_or("").to_owned(),
                    volume: value.volume,
                    mute: value.mute,
                    port_type: value.active_port.as_ref().map(|port| port.r#type),
                }
            }
        }
    };
}

impl_port_from!(SinkInfo<'a>);
impl_port_from!(SourceInfo<'a>);

#[derive(Debug)]
enum CtxCommand {
    UpdateItem(I3Item),
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Pulse {
    increment: Option<u32>,
    max_volume: Option<u32>,
    // TODO: a sample to play each time the volume is changed?
    // See: https://docs.rs/libpulse-binding/2.26.0/libpulse_binding/mainloop/threaded/index.html#example
}

pub struct PulseState {
    tx: UnboundedSender<CtxCommand>,
    theme: Theme,
    increment: u32,
    max_volume: Option<u32>,
    // NOTE: wrapped in `RefCell`s so they can be easily shared between tokio tasks on the same thread
    pa_ctx: RefCell<PAContext>,
    default_sink: RefCell<String>,
    default_source: RefCell<String>,
    sinks: RefCell<Vec<Port>>,
    sources: RefCell<Vec<Port>>,
}

macro_rules! impl_pa_methods {
    ($name:ident) => {
        paste::paste! {
            fn [<add_ $name>](&self, result: ListResult<&[<$name:camel Info>]>) {
                match result {
                    ListResult::Item(info) => {
                        let mut items = self.[<$name s>].borrow_mut();
                        match items.iter_mut().find(|s| s.index == info.index) {
                            Some(s) => *s = info.into(),
                            None => items.push(info.into()),
                        }
                    },
                    ListResult::Error => log::warn!("add_{} failed", stringify!($name)),
                    ListResult::End => {}
                }
            }

            fn [<remove_ $name>](&self, idx: u32) {
                self.[<$name s>].borrow_mut().retain(|s| s.index == idx);
            }

            fn [<set_mute_ $name>](self: &Rc<Self>, idx: u32, mute: bool) {
                let inner = self.clone();
                let mut inspect = self.pa_ctx.borrow_mut().introspect();
                inspect.[<set_ $name _mute_by_index>](idx, mute, Some(Box::new(move |success| {
                    if !success {
                        let port = inner.get_port_by_idx(idx);
                        log::error!("set_mute_{} failed: idx={}, port={:?}", stringify!(name), idx, port);
                    }
                })));
            }

            fn [<set_volume_ $name>](self: &Rc<Self>, idx: u32, cv: &ChannelVolumes) {
                let inner = self.clone();
                let mut inspect = self.pa_ctx.borrow_mut().introspect();
                inspect.[<set_ $name _volume_by_index>](idx, cv, Some(Box::new(move |success| {
                    if !success {
                        let port = inner.get_port_by_idx(idx);
                        log::error!("set_volume_{} failed: idx={}, port={:?}", stringify!(name), idx, port);
                    }
                })));
            }
        }
    };
}

impl PulseState {
    impl_pa_methods!(sink);
    impl_pa_methods!(source);

    fn get_port_by_idx(&self, idx: u32) -> Option<Port> {
        self.sinks
            .borrow()
            .iter()
            .chain(self.sources.borrow().iter())
            .find(|p| p.index == idx)
            .cloned()
    }

    fn default_sink(&self) -> Option<Port> {
        self.sinks
            .borrow()
            .iter()
            .find(|s| s.name == *self.default_sink.borrow())
            .cloned()
    }

    fn default_source(&self) -> Option<Port> {
        self.sources
            .borrow()
            .iter()
            .find(|s| s.name == *self.default_source.borrow())
            .cloned()
    }

    fn update_volume<'a, 'b>(
        &'a self,
        cv: &'b mut ChannelVolumes,
        vol: Vol,
    ) -> &'b mut ChannelVolumes {
        let step = Volume::NORMAL.0 / 100;
        let current_pct = cv.max().0 / step;
        match vol {
            Vol::Decr(inc_pct) => {
                if cv
                    .decrease(Volume((inc_pct - (current_pct % inc_pct)) * step))
                    .is_none()
                {
                    log::error!("failed to decrease ChannelVolumes");
                }
            }
            Vol::Incr(inc_pct) => {
                let tgt = Volume((inc_pct - (current_pct % inc_pct)) * step);
                if (match self.max_volume {
                    Some(max_pct) => cv.inc_clamp(tgt, Volume(max_pct * step)),
                    None => cv.increase(tgt),
                })
                .is_none()
                {
                    log::error!("failed to increase ChannelVolumes");
                }
            }
            Vol::Set(pct) => {
                cv.set(cv.len(), Volume(pct * step));
            }
        }

        cv
    }

    fn update_item(self: &Rc<Self>) {
        let (default_sink, default_source) = match (self.default_sink(), self.default_source()) {
            (Some(sink), Some(source)) => (sink, source),
            _ => {
                log::warn!("tried to update, but failed to find default sink and source");
                return;
            }
        };

        let sink_fg = if default_sink.mute {
            format!(r#" foreground="{}""#, self.theme.dark4)
        } else {
            "".into()
        };

        let sink_text = format!(
            "<span{}>{}{}%</span>",
            sink_fg,
            default_sink
                .port_symbol()
                .unwrap_or_else(|| if default_sink.mute { " " } else { " " }),
            default_sink.volume_pct(),
        );

        let full = format!(
            r#"{} <span foreground="{}">[{}{}%]</span>"#,
            sink_text,
            if default_source.mute {
                self.theme.dark4
            } else {
                self.theme.light1
            },
            default_source.port_symbol().unwrap_or(""),
            default_source.volume_pct(),
        );

        let item = I3Item::new(full)
            .short_text(sink_text)
            .name("pulse")
            .markup(crate::i3::I3Markup::Pango);

        let _ = self.tx.send(CtxCommand::UpdateItem(item));
    }

    fn subscribe_cb(
        self: &Rc<Self>,
        inspect: &Introspector,
        facility: Facility,
        op: Operation,
        idx: u32,
    ) {
        use Facility::*;
        use Operation::*;
        // TODO: these events come in fast with many per type, can we debounce the `fetch_server_state` call?
        macro_rules! impl_handler {
            ($(($obj:ty, $get:ident)),*) => {
                paste::paste! {
                    match (facility, op) {
                        $(
                            ($obj, New) | ($obj, Changed) => {
                                let state = self.clone();
                                inspect.$get(idx, move |result| {
                                    let should_refetch = matches!(&result, ListResult::End | ListResult::Error);
                                    state.[<add_ $obj:snake>](result);
                                    if should_refetch {
                                        state.fetch_server_state();
                                    }
                                });
                            }
                            ($obj, Removed) => {
                                self.[<remove_ $obj:snake>](idx);
                                self.fetch_server_state();
                            },
                        )*
                        // triggered when the defaults change
                        (Server, _) => self.fetch_server_state(),
                        // ignore other events
                        _ => {}
                    }
                }
            }
        }

        impl_handler!(
            (Sink, get_sink_info_by_index),
            (Source, get_source_info_by_index)
        );
    }

    fn fetch_server_state(self: &Rc<Self>) {
        let inspect = self.pa_ctx.borrow().introspect();

        let state = self.clone();
        inspect.get_sink_info_list(move |item| {
            state.add_sink(item);
        });

        let state = self.clone();
        inspect.get_source_info_list(move |item| {
            state.add_source(item);
        });

        let state = self.clone();
        inspect.get_server_info(move |info| {
            if let Some(name) = &info.default_sink_name {
                state.default_sink.replace((**name).to_owned());
            }
            if let Some(name) = &info.default_source_name {
                state.default_source.replace((**name).to_owned());
            }

            state.update_item();
        });
    }
}

#[async_trait(?Send)]
impl BarItem for Pulse {
    async fn start(self: Box<Self>, mut ctx: Context) -> Result<(), Box<dyn Error>> {
        // setup pulse main loop
        let (mut main_loop, pa_ctx) = {
            let mut main_loop = TokioMain::new();

            let app_name = env!("CARGO_PKG_NAME");
            let mut props = Proplist::new().ok_or("Failed to create PulseAudio Proplist")?;
            let _ = props.set_str(APPLICATION_NAME, app_name);
            let _ = props.set_str(APPLICATION_PROCESS_ID, &process::id().to_string());

            let mut pa_ctx = PAContext::new_with_proplist(&main_loop, app_name, &props)
                .ok_or("Failed to create PulseAudio context")?;

            pa_ctx.connect(None, FlagSet::NOFAIL, None)?;
            match main_loop.wait_for_ready(&pa_ctx).await {
                Ok(State::Ready) => {}
                Ok(c) => {
                    return Err(format!("Pulse context {:?}, not continuing", c).into());
                }
                Err(_) => {
                    return Err(
                        "Pulse mainloop exited while waiting on context, not continuing".into(),
                    );
                }
            }

            (main_loop, pa_ctx)
        };

        let inspect = pa_ctx.introspect();

        // this is shared between all the async tasks
        let (tx, mut rx) = mpsc::unbounded_channel();
        let inner = Rc::new(PulseState {
            tx,
            theme: ctx.theme.clone(),
            increment: self.increment.unwrap_or(2),
            max_volume: self.max_volume,

            pa_ctx: RefCell::new(pa_ctx),
            default_sink: RefCell::new("?".into()),
            default_source: RefCell::new("?".into()),
            sinks: RefCell::new(vec![]),
            sources: RefCell::new(vec![]),
        });

        // subscribe to changes
        {
            let state = inner.clone();
            let mut pa_ctx = inner.pa_ctx.borrow_mut();
            pa_ctx.set_subscribe_callback(Some(Box::new(move |fac, op, idx| {
                // SAFETY: `libpulse_binding` decodes these values from an integer, and explains
                // that it's probably safe to always unwrap them
                state.subscribe_cb(&inspect, fac.unwrap(), op.unwrap(), idx);
            })));

            let mask = InterestMaskSet::SERVER | InterestMaskSet::SINK | InterestMaskSet::SOURCE;
            pa_ctx.subscribe(mask, |success| {
                if !success {
                    log::error!("subscribe failed");
                }
            });
        }

        // request initial state
        {
            inner.fetch_server_state();
        }

        // run pulse main loop
        let (exit_tx, mut exit_rx) = mpsc::channel(1);
        tokio::task::spawn_local(async move {
            let _ = exit_tx.send(main_loop.run().await).await;
        });

        loop {
            tokio::select! {
                // handle events
                Some(event) = ctx.raw_event_rx().recv() => match event {
                    BarEvent::Custom { payload, responder } => inner.handle_custom_message(payload, responder),
                    BarEvent::Click(click) => match click.button {
                        // open control panel
                        I3Button::Left => exec("i3-msg exec pavucontrol").await,

                        // show a popup with information about the current state
                        I3Button::Right => {
                            let s = |s: &str| s.chars().filter(char::is_ascii).collect::<String>();
                            let m = |p: Port| format!("name: {}\nvolume: {}\n", s(&p.name), p.volume_pct());
                            let sink = inner.default_sink().map(m).unwrap_or("???".into());
                            let source = inner.default_source().map(m).unwrap_or("???".into());
                            exec(
                                format!(
                                    r#"zenity --info --text='[sink]\n{}\n\n[source]\n{}'"#,
                                    sink,
                                    source
                                )
                            ).await
                        },

                        // source
                        I3Button::Middle if click.modifiers.contains(&I3Modifier::Shift) => {
                            inner.default_source().map(|x| inner.set_mute_source(x.index, !x.mute));
                        },
                        I3Button::ScrollUp if click.modifiers.contains(&I3Modifier::Shift) => {
                            inner.default_source().map(|mut x| {
                                inner.set_volume_source(x.index, inner.update_volume(&mut x.volume, Vol::Incr(inner.increment)));
                            });
                        }
                        I3Button::ScrollDown if click.modifiers.contains(&I3Modifier::Shift) => {
                            inner.default_source().map(|mut x| {
                                inner.set_volume_source(x.index, inner.update_volume(&mut x.volume, Vol::Decr(inner.increment)));
                            });
                        }
                        // sink
                        I3Button::Middle  => {
                            inner.default_sink().map(|x| inner.set_mute_sink(x.index, !x.mute));
                        },
                        I3Button::ScrollUp  => {
                            inner.default_sink().map(|mut x| {
                                inner.set_volume_sink(x.index, inner.update_volume(&mut x.volume, Vol::Incr(inner.increment)));
                            });
                        }
                        I3Button::ScrollDown  => {
                            inner.default_sink().map(|mut x| {
                                inner.set_volume_sink(x.index, inner.update_volume(&mut x.volume, Vol::Decr(inner.increment)));
                            });
                        }
                    }
                    _ => {}
                },

                // whenever we want to refresh our item, an event it send on this channel
                Some(cmd) = rx.recv() => match cmd {
                    CtxCommand::UpdateItem(item) => {
                        let _ = ctx.update_item(item).await;
                    }
                },

                // handle pulse main loop exited
                Some(ret_val) = exit_rx.recv() => {
                    break Err(format!("mainloop exited unexpectedly with value: {}", ret_val.0).into());
                }
            }
        }
    }
}

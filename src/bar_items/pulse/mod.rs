mod pulse_tokio;

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
use serde_derive::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;

use self::pulse_tokio::TokioMain;
use crate::context::{BarItem, Context};
use crate::exec::exec;
use crate::i3::{I3Button, I3Item, I3Modifier};
use crate::theme::Theme;
use crate::BarEvent;

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
            Some(DevicePortType::Bluetooth) => Some("󰂰"),
            Some(DevicePortType::Headphones) => Some("󰋋"),
            Some(DevicePortType::Headset) => Some("󰋎"),
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

fn update_volume(cv: &mut ChannelVolumes, delta: i64) -> &mut ChannelVolumes {
    let step = Volume::NORMAL.0 / 100;
    let v = Volume(((delta.abs() as u32) * step) as u32);
    if delta < 0 {
        cv.decrease(v).unwrap()
    } else {
        cv.increase(v).unwrap()
    }
}

#[derive(Debug)]
enum CtxCommand {
    UpdateItem(I3Item),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pulse {}

pub struct PulseState {
    tx: UnboundedSender<CtxCommand>,
    theme: Theme,
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
                    ListResult::Error => log::warn!("pulse::add_{} failed:", stringify!($name)),
                    ListResult::End => {}
                }
            }

            fn [<remove_ $name>](&self, idx: u32) {
                self.[<$name s>].borrow_mut().retain(|s| s.index == idx);
            }

            fn [<set_mute_ $name>](self: &Rc<Self>, idx: u32, mute: bool) {
                let mut inspect = self.pa_ctx.borrow_mut().introspect();
                inspect.[<set_ $name _mute_by_index>](idx, mute, Some(Box::new(move |success| {
                    if !success {
                        log::error!("pulse::set_mute_{} failed", stringify!(name));
                    }
                })));
            }

            fn [<set_volume_ $name>](self: &Rc<Self>, idx: u32, cv: &ChannelVolumes) {
                let mut inspect = self.pa_ctx.borrow_mut().introspect();
                inspect.[<set_ $name _volume_by_index>](idx, cv, Some(Box::new(move |success| {
                    if !success {
                        log::error!("pulse::set_volume_{} failed", stringify!(name));
                    }
                })));
            }
        }
    };
}

impl PulseState {
    impl_pa_methods!(sink);
    impl_pa_methods!(source);

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

    fn update_item(self: &Rc<Self>) {
        let default_sink = self.default_sink().unwrap();
        let default_source = self.default_source().unwrap();

        let sink_fg = if default_sink.mute {
            format!(r#" foreground="{}""#, self.theme.dark4)
        } else {
            "".into()
        };

        let sink_text = format!(
            "<span{}>{} {}%</span>",
            sink_fg,
            default_sink
                .port_symbol()
                .unwrap_or_else(|| if default_sink.mute { "" } else { "" }),
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

        self.tx.send(CtxCommand::UpdateItem(item)).unwrap();
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
        macro_rules! impl_handler {
            ($(($obj:ty, $get:ident)),*) => {
                paste::paste! {
                    match (facility, op) {
                        $(
                            ($obj, New) | ($obj, Changed) => {
                                let state = self.clone();
                                inspect.$get(idx, move |result| {
                                    state.[<add_ $obj:snake>](result);
                                    state.update_item();
                                });
                            }
                            ($obj, Removed) => self.[<remove_ $obj:snake>](idx),
                        )*
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

        let inspect_sub = pa_ctx.introspect();
        let inspect = pa_ctx.introspect();

        // this is shared between all the async tasks
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let inner = Rc::new(PulseState {
            tx,
            theme: ctx.theme.clone(),
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
                state.subscribe_cb(&inspect_sub, fac.unwrap(), op.unwrap(), idx);
            })));

            pa_ctx.subscribe(InterestMaskSet::ALL, |success| {
                if !success {
                    log::error!("pulse::subscribe failed");
                }
            });
        }

        // request initial state
        {
            let state = inner.clone();
            inspect.get_sink_info_list(move |item| {
                state.add_sink(item);
            });

            let state = inner.clone();
            inspect.get_source_info_list(move |item| {
                state.add_source(item);
            });

            let state = inner.clone();
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

        // run pulse main loop
        tokio::task::spawn_local(async move {
            let code = main_loop.run().await;
            // TODO: potentially try to reconnect? test it out with restarting pulse, etc
            log::error!("pulse::mainloop exited unexpectedly with value: {}", code.0);
        });

        loop {
            tokio::select! {
                // handle click events
                Some(BarEvent::Click(click)) = ctx.raw_event_rx().recv() => {
                    match click.button {
                        // open control panel
                        I3Button::Left => exec("i3-msg exec pavucontrol").await,

                        // show a popup with information about the current state
                        I3Button::Right => {
                            let s = |s: &str| s.chars().filter(char::is_ascii).collect::<String>();

                            let sink = inner.default_sink().unwrap();
                            let source = inner.default_source().unwrap();
                            exec(
                                format!(
                                    r#"zenity --info --text='{}\n\n{}'"#,
                                    format!("[sink]\nname: {}\nvolume: {}\n", s(&sink.name), sink.volume_pct()),
                                    format!("[source]\nname: {}\nvolume: {}\n", s(&source.name), source.volume_pct())
                                )
                            ).await
                        },

                        // source
                        I3Button::Middle if click.modifiers.contains(&I3Modifier::Shift) => {
                            inner.default_source().map(|x| inner.set_mute_source(x.index, !x.mute));
                        },
                        I3Button::ScrollUp if click.modifiers.contains(&I3Modifier::Shift) => {
                            inner.default_source().map(|mut x| inner.set_volume_source(x.index, update_volume(&mut x.volume, 2)));
                        }
                        I3Button::ScrollDown if click.modifiers.contains(&I3Modifier::Shift) => {
                            inner.default_source().map(|mut x| inner.set_volume_source(x.index, update_volume(&mut x.volume, -2)));
                        }
                        // sink
                        I3Button::Middle  => {
                            inner.default_sink().map(|x| inner.set_mute_sink(x.index, !x.mute));
                        },
                        I3Button::ScrollUp  => {
                            inner.default_sink().map(|mut x| inner.set_volume_sink(x.index, update_volume(&mut x.volume, 2)));
                        }
                        I3Button::ScrollDown  => {
                            inner.default_sink().map(|mut x| inner.set_volume_sink(x.index, update_volume(&mut x.volume, -2)));
                        }
                    }
                },

                // handle item updates
                Some(cmd) = rx.recv() => match cmd {
                    CtxCommand::UpdateItem(item) => {
                        ctx.update_item(item).await.unwrap();
                    }
                }
            }
        }
    }
}

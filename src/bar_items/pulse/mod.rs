mod custom;

use std::borrow::Cow;
use std::cell::RefCell;
use std::error::Error;
use std::fmt::Debug;
use std::process;
use std::rc::Rc;

use async_trait::async_trait;
use clap::ValueEnum;
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
use zbus::Connection;

use crate::context::{BarEvent, BarItem, Context};
use crate::dbus::notifications::NotificationsProxy;
use crate::exec::exec;
use crate::i3::{I3Button, I3Item, I3Markup, I3Modifier};
use crate::theme::Theme;

#[derive(Debug, Copy, Clone, ValueEnum)]
pub enum Object {
    Source,
    Sink,
}

impl ToString for Object {
    fn to_string(&self) -> String {
        match self {
            Object::Sink => "sink".into(),
            Object::Source => "source".into(),
        }
    }
}

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

    fn notify_volume_mute(&self) -> Command {
        Command::NotifyVolume {
            name: self.name.clone(),
            volume: self.volume_pct(),
            mute: self.mute,
        }
    }

    fn notify_new(&self, r#type: &'static str) -> Command {
        Command::NotifyNewSourceSink {
            name: self.name.clone(),
            what: r#type.into(),
        }
    }

    fn format(&self, what: Object, theme: &Theme) -> String {
        format!(
            r#"<span foreground="{}">{} {}%</span>"#,
            if self.mute { theme.dim } else { theme.fg },
            self.port_symbol()
                .unwrap_or_else(|| match (what, self.mute) {
                    (Object::Sink, false) => "",
                    (Object::Sink, true) => "",
                    (Object::Source, false) => "󰍬",
                    (Object::Source, true) => "󰍭",
                }),
            self.volume_pct(),
        )
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

enum Command {
    UpdateItem(Box<dyn FnOnce(&Theme) -> I3Item>),
    NotifyVolume {
        name: String,
        volume: u32,
        mute: bool,
    },
    NotifyNewSourceSink {
        name: String,
        what: String,
    },
    NotifyDefaultsChange {
        name: String,
        what: String,
    },
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum NotificationSetting {
    /// No notifications are sent (the default)
    #[default]
    None,
    /// When volumes are changed
    VolumeMute,
    /// When a source or sink is added
    NewSourceSink,
    /// When the default source or sink has changed
    DefaultsChange,
    /// All notifications
    All,
}

impl NotificationSetting {
    pub fn should_notify(&self, ask: Self) -> bool {
        match self {
            NotificationSetting::All => true,
            other => *other == ask,
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Pulse {
    /// How much to increment when increasing/decreasing the volume; measured in percent
    increment: Option<u32>,
    /// The maximum allowed volume; measured in percent
    max_volume: Option<u32>,
    /// Whether to send notifications on server state changes
    #[serde(default)]
    notify: NotificationSetting,
    // TODO: a sample to play each time the volume is changed?
    // See: https://docs.rs/libpulse-binding/2.26.0/libpulse_binding/mainloop/threaded/index.html#example
}

pub struct PulseState {
    tx: UnboundedSender<Command>,
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
                            None => {
                                let obj = info.into();
                                if matches!(obj, Port { .. }) {
                                    if !obj.name.contains("auto_null") {
                                        let _ = self.tx.send(obj.notify_new(stringify!($name)));
                                    }
                                }

                                items.push(obj);
                            },
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

    fn set_volume(self: &Rc<Self>, what: Object, vol: Vol) {
        (match what {
            Object::Sink => self.default_sink().map(|mut p| {
                self.set_volume_sink(p.index, self.update_volume(&mut p.volume, vol));
                p
            }),
            Object::Source => self.default_source().map(|mut p| {
                self.set_volume_source(p.index, self.update_volume(&mut p.volume, vol));
                p
            }),
        })
        .map(|p| {
            let _ = self.tx.send(p.notify_volume_mute());
        });
    }

    fn set_mute(self: &Rc<Self>, what: Object, mute: bool) {
        (match what {
            Object::Sink => self.default_sink().map(|mut p| {
                p.mute = mute;
                self.set_mute_sink(p.index, p.mute);
                p
            }),
            Object::Source => self.default_source().map(|mut p| {
                p.mute = mute;
                self.set_mute_source(p.index, p.mute);
                p
            }),
        })
        .map(|p| {
            let _ = self.tx.send(p.notify_volume_mute());
        });
    }

    fn toggle_mute(self: &Rc<Self>, what: Object) {
        (match what {
            Object::Sink => self.default_sink().map(|mut p| {
                p.mute = !p.mute;
                self.set_mute_sink(p.index, p.mute);
                p
            }),
            Object::Source => self.default_source().map(|mut p| {
                p.mute = !p.mute;
                self.set_mute_source(p.index, p.mute);
                p
            }),
        })
        .map(|p| {
            let _ = self.tx.send(p.notify_volume_mute());
        });
    }

    fn update_item(self: &Rc<Self>) {
        let (default_sink, default_source) = match (self.default_sink(), self.default_source()) {
            (Some(sink), Some(source)) => (sink, source),
            _ => {
                log::warn!("tried to update, but failed to find default sink and source");
                return;
            }
        };

        let _ = self.tx.send(Command::UpdateItem(Box::new(move |theme| {
            let sink_text = default_sink.format(Object::Sink, theme);
            let source_text = default_source.format(Object::Source, theme);

            I3Item::new(format!(r#"{} {}"#, sink_text, source_text))
                .short_text(sink_text)
                .markup(I3Markup::Pango)
        })));
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
                                let inner = self.clone();
                                inspect.$get(idx, move |result| {
                                    let should_refetch = matches!(&result, ListResult::End | ListResult::Error);
                                    inner.[<add_ $obj:snake>](result);
                                    if should_refetch {
                                        inner.fetch_server_state();
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

        let inner = self.clone();
        inspect.get_sink_info_list(move |item| {
            inner.add_sink(item);
        });

        let inner = self.clone();
        inspect.get_source_info_list(move |item| {
            inner.add_source(item);
        });

        let inner = self.clone();
        inspect.get_server_info(move |info| {
            let update_default = |name: &Cow<str>, default: &RefCell<String>, what: Object| {
                let name = (**name).to_owned();
                let prev = default.replace(name.clone());
                if &*prev != &*name {
                    let _ = inner.tx.send(Command::NotifyDefaultsChange {
                        name,
                        what: what.to_string(),
                    });
                }
            };

            if let Some(name) = &info.default_sink_name {
                update_default(name, &inner.default_sink, Object::Sink);
            }

            if let Some(name) = &info.default_source_name {
                update_default(name, &inner.default_source, Object::Source);
            }

            inner.update_item();
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

        let dbus = Connection::session().await?;
        let notifications = NotificationsProxy::new(&dbus).await?;
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
                            inner.toggle_mute(Object::Source);
                        },
                        I3Button::ScrollUp if click.modifiers.contains(&I3Modifier::Shift) => {
                            inner.set_volume(Object::Source, Vol::Incr(inner.increment));
                        }
                        I3Button::ScrollDown if click.modifiers.contains(&I3Modifier::Shift) => {
                            inner.set_volume(Object::Source, Vol::Decr(inner.increment));
                        }
                        // sink
                        I3Button::Middle  => {
                            inner.toggle_mute(Object::Sink);
                        },
                        I3Button::ScrollUp  => {
                            inner.set_volume(Object::Sink, Vol::Incr(inner.increment));
                        }
                        I3Button::ScrollDown  => {
                            inner.set_volume(Object::Sink, Vol::Decr(inner.increment));
                        }
                    }
                    _ => {}
                },

                // whenever we want to refresh our item, an event it send on this channel
                Some(cmd) = rx.recv() => match cmd {
                    Command::UpdateItem(cb) => {
                        ctx.update_item(cb(&ctx.theme())).await?;
                    }
                    Command::NotifyVolume { name, volume, mute } => {
                        if self.notify.should_notify(NotificationSetting::VolumeMute) {
                            let _ = notifications.volume_mute(name, volume, mute).await;
                        }
                    }
                    Command::NotifyNewSourceSink { name, what } => {
                        if self.notify.should_notify(NotificationSetting::NewSourceSink) {
                            let _ = notifications.new_source_sink(name, what).await;
                        }
                    }
                    Command::NotifyDefaultsChange { name, what } => {
                        if self.notify.should_notify(NotificationSetting::DefaultsChange) {
                            let _ = notifications.defaults_change(name, what).await;
                        }
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

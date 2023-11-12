//! This bar item connects to pulseaudio/pipewire directly for the lowest latency
//! possible when interacting with it (for example, changing volume happens extremely
//! fast, since it's not invoking `pactl` each time, it's communicating directly with
//! the audio server).
//!
//! The following were great resources:
//! * https://gavv.net/articles/pulseaudio-under-the-hood/
//! * https://www.freedesktop.org/wiki/Software/PulseAudio/Documentation/Developer/
//! * https://github.com/danieldg/rwaybar/blob/master/src/pulse.rs
//! * https://gitlab.freedesktop.org/pulseaudio/pavucontrol/-/blob/master/src/sinkwidget.cc
//! * https://gitlab.gnome.org/GNOME/libgnome-volume-control/-/blob/master/gvc-mixer-control.c

mod audio;
mod custom;

use std::fmt::{Debug, Display};
use std::ops::Add;
use std::path::{Path, PathBuf};
use std::process;
use std::rc::Rc;

use async_trait::async_trait;
use clap::ValueEnum;
use libpulse_binding::callbacks::ListResult;
use libpulse_binding::context::introspect::{
    Introspector, SinkInfo, SinkPortInfo, SourceInfo, SourcePortInfo,
};
use libpulse_binding::context::subscribe::{Facility, InterestMaskSet, Operation};
use libpulse_binding::context::{Context as PAContext, FlagSet, State};
use libpulse_binding::def::{DevicePortType, PortAvailable};
use libpulse_binding::error::{Code, PAErr};
use libpulse_binding::proplist::properties::{APPLICATION_NAME, APPLICATION_PROCESS_ID};
use libpulse_binding::proplist::Proplist;
use libpulse_binding::stream::{SeekMode, Stream};
use libpulse_binding::volume::{ChannelVolumes, Volume};
use libpulse_tokio::TokioMain;
use num_traits::ToPrimitive;
use serde_derive::{Deserialize, Serialize};
use tokio::sync::mpsc::{self, UnboundedSender};

use crate::context::{BarEvent, BarItem, Context, StopAction};
use crate::dbus::notifications::NotificationsProxy;
use crate::dbus::{dbus_connection, BusType};
use crate::error::Result;
use crate::i3::{I3Button, I3Item, I3Markup, I3Modifier};
use crate::theme::Theme;
use crate::util::{expand_path, RcCell};

#[derive(Debug, Copy, Clone, ValueEnum)]
pub enum Object {
    Source,
    Sink,
}

impl Display for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = <Object as Into<std::rc::Rc<str>>>::into(*self);
        f.write_str(&s)
    }
}

impl From<Object> for Rc<str> {
    fn from(value: Object) -> Self {
        match value {
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

#[derive(Debug, Clone, PartialEq)]
struct Port {
    name: Rc<str>,
    description: Rc<str>,
    available: PortAvailable,
    port_type: DevicePortType,
}

macro_rules! impl_port_from {
    ($ty:ty) => {
        impl<'a> From<&'a $ty> for Port {
            fn from(value: &'a $ty) -> Self {
                Port {
                    name: value.name.as_deref().unwrap_or("").into(),
                    description: value.description.as_deref().unwrap_or("").into(),
                    available: value.available,
                    port_type: value.r#type,
                }
            }
        }
    };
}

impl_port_from!(SinkPortInfo<'a>);
impl_port_from!(SourcePortInfo<'a>);

/// Information about a `Sink` or a `Source` (input or output)
#[derive(Debug, Clone)]
struct InOut {
    index: u32,
    name: Rc<str>,
    volume: ChannelVolumes,
    mute: bool,
    ports: Rc<[Port]>,
    active_port: Option<Port>,
}

impl InOut {
    fn volume_pct(&self) -> u32 {
        let normal = Volume::NORMAL.0;
        (self.volume.max().0 * 100 + normal / 2) / normal
    }

    fn port_symbol(&self) -> Option<&str> {
        match &self.active_port {
            Some(port) => match port.port_type {
                DevicePortType::Bluetooth => Some("󰂰 "),
                DevicePortType::Headphones => Some("󰋋 "),
                DevicePortType::Headset => Some("󰋎 "),
                _ => None,
            },
            None => None,
        }
    }

    fn current_port_idx(&self) -> usize {
        self.active_port.as_ref().map_or(0, |active| {
            match self.ports.iter().position(|p| p == active) {
                Some(idx) => idx,
                None => {
                    log::warn!(
                        "failed to find active port: object={:?}, active_port={:?}",
                        self,
                        active
                    );

                    // default to 0
                    0
                }
            }
        })
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

macro_rules! impl_io_from {
    ($ty:ty) => {
        impl<'a> From<&'a $ty> for InOut {
            fn from(value: &'a $ty) -> Self {
                InOut {
                    index: value.index,
                    name: value.name.as_deref().unwrap_or("").into(),
                    volume: value.volume,
                    mute: value.mute,
                    ports: value.ports.iter().map(Port::from).collect(),
                    active_port: value.active_port.as_ref().map(|p| Port::from(p.as_ref())),
                }
            }
        }
    };
}

impl_io_from!(SinkInfo<'a>);
impl_io_from!(SourceInfo<'a>);

enum Command {
    UpdateItem(Box<dyn FnOnce(&Theme) -> I3Item>),
    NotifyVolume {
        name: Rc<str>,
        volume: u32,
        mute: bool,
    },
    NotifyNewSourceSink {
        name: Rc<str>,
        what: Rc<str>,
    },
    NotifyDefaultsChange {
        name: Rc<str>,
        what: Rc<str>,
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

const SAMPLE_NAME: &str = "i3stat-pulse-volume";

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Pulse {
    /// How much to increment when increasing/decreasing the volume; measured in percent
    #[serde(default = "Pulse::default_increment")]
    increment: u32,
    /// Path to a `.wav` file to play each time the sound is changed
    increment_sound: Option<PathBuf>,
    /// The maximum allowed volume; measured in percent
    max_volume: Option<u32>,
    /// Whether to send notifications on server state changes
    #[serde(default)]
    notify: NotificationSetting,
    /// Name of the audio server to try to connect to
    server_name: Option<String>,
}

impl Pulse {
    pub const fn default_increment() -> u32 {
        5
    }
}

pub struct PulseState {
    tx: UnboundedSender<Command>,
    increment: u32,
    increment_sound: bool,
    max_volume: Option<u32>,
    pa_ctx: PAContext,
    default_sink: Rc<str>,
    default_source: Rc<str>,
    sinks: Vec<InOut>,
    sources: Vec<InOut>,
}

macro_rules! impl_pa_methods {
    ($name:ident) => {
        paste::paste! {
            fn [<add_ $name>](&mut self, result: ListResult<&[<$name:camel Info>]>) {
                match result {
                    ListResult::Item(info) => {
                        match self.[<$name s>].iter_mut().find(|s| s.index == info.index) {
                            Some(s) => *s = info.into(),
                            None => {
                                let obj = info.into();

                                if matches!(obj, InOut { .. }) {
                                    // ignore any null sinks/sources - they're not useful for this bar item anyway
                                    if obj.name.contains("auto_null") {
                                        return;
                                    }

                                    let _ = self.tx.send(obj.notify_new(stringify!($name)));
                                }

                                self.[<$name s>].push(obj);
                            },
                        }
                    },
                    ListResult::Error => log::warn!("add_{} failed", stringify!($name)),
                    ListResult::End => {}
                }
            }

            fn [<remove_ $name>](&mut self, idx: u32) {
                self.[<$name s>].retain(|s| s.index == idx);
            }

            fn [<set_mute_ $name>]<F>(&self, idx: u32, mute: bool, f: F)
                where F: FnMut(bool) + 'static,
            {
                let mut inspect = self.pa_ctx.introspect();
                inspect.[<set_ $name _mute_by_index>](idx, mute, Some(Box::new(f)));
            }

            fn [<set_volume_ $name>]<F>(&self, idx: u32, cv: &ChannelVolumes, f: F)
                where F: FnMut(bool) + 'static,
            {
                let mut inspect = self.pa_ctx.introspect();
                inspect.[<set_ $name _volume_by_index>](idx, cv, Some(Box::new(f)));
            }
        }
    };
}

#[derive(Debug, Copy, Clone, ValueEnum)]
enum Dir {
    Prev,
    Next,
}

impl Add<usize> for Dir {
    type Output = usize;

    fn add(self, rhs: usize) -> Self::Output {
        match self {
            Dir::Prev => rhs - 1,
            Dir::Next => rhs + 1,
        }
    }
}

impl Add<Dir> for usize {
    type Output = usize;

    fn add(self, rhs: Dir) -> Self::Output {
        match rhs {
            Dir::Prev => self - 1,
            Dir::Next => self + 1,
        }
    }
}

impl RcCell<PulseState> {
    impl_pa_methods!(sink);
    impl_pa_methods!(source);

    fn default_sink(&self) -> Option<InOut> {
        self.sinks
            .iter()
            .find(|s| s.name == self.default_sink)
            .cloned()
    }

    fn default_source(&self) -> Option<InOut> {
        self.sources
            .iter()
            .find(|s| s.name == self.default_source)
            .cloned()
    }

    fn cycle_objects_and_ports<F>(&mut self, what: Object, dir: Dir, mut f: F)
    where
        F: FnMut(bool) + 'static,
    {
        let objects = match what {
            Object::Sink => &self.sinks,
            Object::Source => &self.sources,
        };

        let curr_obj_idx = match objects.iter().position(|s| match what {
            Object::Sink => s.name == self.default_sink,
            Object::Source => s.name == self.default_source,
        }) {
            Some(idx) => idx,
            None => {
                log::warn!("failed to find active {what}");
                return;
            }
        };

        let obj = &objects[curr_obj_idx];
        let next_obj_idx = (curr_obj_idx + objects.len() + dir) % objects.len();
        let curr_prt_idx = obj.current_port_idx();
        let next_prt_idx = (curr_prt_idx + obj.ports.len() + dir) % obj.ports.len();

        if curr_prt_idx < obj.ports.len() - 1 || next_obj_idx == curr_obj_idx {
            self.set_object_port(what, obj.index, &obj.ports[next_prt_idx].name, f);
        } else {
            let next_obj = &objects[next_obj_idx];
            let next_obj_index = next_obj.index;
            let next_obj_name = next_obj.name.clone();

            // I don't believe a pulse source/sink can ever have zero ports, but handle it gracefully
            // just in case this assumption is incorrect
            let next_prt_name = match next_obj.ports.first() {
                Some(port) => port.name.clone(),
                None => {
                    return self.set_default(what, next_obj_name.clone(), move |success| {
                        if !success {
                            log::warn!("failed to set default to {next_obj_name} while cycling");
                        }
                        f(false);
                    })
                }
            };

            let next_obj_port_already_set = next_obj
                .active_port
                .as_ref()
                .map(|p| p.name == next_prt_name)
                .unwrap_or(false);

            // if the object we're moving to already has the right port set, just set that object as
            // the new default
            if next_obj_port_already_set {
                return self.set_default(what, next_obj_name.clone(), move |success| {
                    if !success {
                        log::warn!("failed to set default to {next_obj_name} while cycling");
                    }
                    f(false);
                });
            }

            // otherwise, if the object we're moving to needs its active port changed, first change
            // the active port - under the hood pulse sometimes sets this object as the default when
            // we change the port (I believe there are some heuristics to do with port availability
            // groups, etc)
            let mut inner = self.clone();
            self.set_object_port(what, next_obj_index, &next_prt_name, move |success| {
                // sometimes setting the active port doesn't change the default, so check for
                // that and set it ourselves if needed
                let should_try_set_default = success
                    && match what {
                        Object::Sink => inner.default_sink != next_obj_name,
                        Object::Source => inner.default_source != next_obj_name,
                    };

                if should_try_set_default {
                    let next_obj_name = next_obj_name.clone();
                    inner.set_default(what, next_obj_name.clone(), move |success| {
                        if !success {
                            log::warn!("failed to set default to {next_obj_name} while cycling");
                        }
                    });
                }

                // it would be nice to call this after the above `set_default` is called (if it is)
                // rather than just here, but our closure bounds don't make that easy right now
                f(success);
            });
        }
    }

    fn set_object_port<F>(&self, what: Object, object_idx: u32, port_name: impl AsRef<str>, f: F)
    where
        F: FnMut(bool) + 'static,
    {
        let port_name = port_name.as_ref();
        let mut introspect = self.pa_ctx.introspect();
        match what {
            Object::Sink => {
                introspect.set_sink_port_by_index(object_idx, &port_name, Some(Box::new(f)));
            }
            Object::Source => {
                introspect.set_source_port_by_index(object_idx, &port_name, Some(Box::new(f)));
            }
        }
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
                    self.fetch_server_state();
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
                    self.fetch_server_state();
                }
            }
            Vol::Set(pct) => {
                cv.set(cv.len(), Volume(pct * step));
            }
        }

        cv
    }

    fn set_volume<F>(&mut self, what: Object, vol: Vol, f: F)
    where
        F: FnMut(bool) + 'static,
    {
        (match what {
            Object::Sink => self.default_sink().map(|mut p| {
                self.set_volume_sink(p.index, self.update_volume(&mut p.volume, vol), f);
                p
            }),
            Object::Source => self.default_source().map(|mut p| {
                self.set_volume_source(p.index, self.update_volume(&mut p.volume, vol), f);
                p
            }),
        })
        .map(|p| {
            // send notification
            let _ = self.tx.send(p.notify_volume_mute());
            self.play_volume_sample_if_enabled(what);
        });
    }

    fn set_mute<F>(&mut self, what: Object, mute: bool, f: F)
    where
        F: FnMut(bool) + 'static,
    {
        (match what {
            Object::Sink => self.default_sink().map(|mut p| {
                p.mute = mute;
                self.set_mute_sink(p.index, p.mute, f);
                p
            }),
            Object::Source => self.default_source().map(|mut p| {
                p.mute = mute;
                self.set_mute_source(p.index, p.mute, f);
                p
            }),
        })
        .map(|p| {
            let _ = self.tx.send(p.notify_volume_mute());
            self.play_volume_sample_if_enabled(what);
        });
    }

    fn toggle_mute<F>(&mut self, what: Object, f: F)
    where
        F: FnMut(bool) + 'static,
    {
        (match what {
            Object::Sink => self.default_sink().map(|mut p| {
                p.mute = !p.mute;
                self.set_mute_sink(p.index, p.mute, f);
                p
            }),
            Object::Source => self.default_source().map(|mut p| {
                p.mute = !p.mute;
                self.set_mute_source(p.index, p.mute, f);
                p
            }),
        })
        .map(|p| {
            let _ = self.tx.send(p.notify_volume_mute());
            self.play_volume_sample_if_enabled(what);
        });
    }

    fn play_volume_sample_if_enabled(&mut self, what: Object) {
        if matches!(what, Object::Sink) && self.increment_sound {
            self.pa_ctx.play_sample(SAMPLE_NAME, None, None, None);
        }
    }

    fn set_default<F>(&mut self, what: Object, name: impl AsRef<str>, f: F)
    where
        F: FnMut(bool) + 'static,
    {
        let name = name.as_ref();
        match what {
            Object::Sink => self.pa_ctx.set_default_sink(name, f),
            Object::Source => self.pa_ctx.set_default_source(name, f),
        };
    }

    fn update_item(&self) {
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

    /// Setup subscription to be notified of server change events
    fn subscribe_to_server_changes(&mut self) {
        let inspect = self.pa_ctx.introspect();
        let mut state = self.clone();
        self.pa_ctx
            .set_subscribe_callback(Some(Box::new(move |fac, op, idx| {
                // SAFETY: `libpulse_binding` decodes these values from an integer, and explains
                // that it's probably safe to always unwrap them
                state.subscribe_cb(&inspect, fac.unwrap(), op.unwrap(), idx);
            })));

        let mask = InterestMaskSet::SERVER | InterestMaskSet::SINK | InterestMaskSet::SOURCE;
        self.pa_ctx.subscribe(mask, |success| {
            if !success {
                log::error!("subscribe failed");
            }
        });
    }

    /// Callback used when server sends us change evnets
    fn subscribe_cb(
        &mut self,
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
                                let mut inner = self.clone();
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

    /// subscribe to state changes to detect if the server is terminated
    fn subscribe_to_state_changes(&mut self, exit_tx: UnboundedSender<()>) {
        // SAFETY: there's a test ensuring this doesn't panic
        let conn_terminated = Code::ConnectionTerminated.to_i32().unwrap();

        let state = self.clone();
        self.pa_ctx
            .set_state_callback(Some(Box::new(move || match state.pa_ctx.get_state() {
                State::Failed => match state.pa_ctx.errno() {
                    PAErr(code) if code == conn_terminated => {
                        let _ = exit_tx.send(());
                    }
                    pa_err => {
                        log::error!("unknown error occurred: {:?}", pa_err.to_string());
                    }
                },
                State::Terminated => {}
                _ => (),
            })));
    }

    fn fetch_server_state(&self) {
        let inspect = self.pa_ctx.introspect();

        let mut inner = self.clone();
        inspect.get_sink_info_list(move |item| {
            inner.add_sink(item);
        });

        let mut inner = self.clone();
        inspect.get_source_info_list(move |item| {
            inner.add_source(item);
        });

        let mut inner = self.clone();
        inspect.get_server_info(move |info| {
            let update_if_needed = |me: &mut PulseState, what: Object, name: Rc<str>| {
                match what {
                    Object::Sink if me.default_sink != name => me.default_sink = name.clone(),
                    Object::Source if me.default_source != name => me.default_source = name.clone(),
                    _ => return,
                }

                let _ = me.tx.send(Command::NotifyDefaultsChange {
                    what: what.into(),
                    name,
                });
            };

            info.default_sink_name
                .as_ref()
                .map(|name| update_if_needed(&mut inner, Object::Sink, name.to_string().into()));

            info.default_source_name
                .as_ref()
                .map(|name| update_if_needed(&mut inner, Object::Source, name.to_string().into()));

            inner.update_item();
        });
    }

    async fn setup_volume_sample(&mut self, wav_path: impl AsRef<Path>) -> Result<()> {
        let (spec, audio_data) = audio::read_wav_file(wav_path.as_ref()).await?;
        let audio_data_len = audio_data.len();

        // create stream
        let mut stream = match Stream::new(&mut self.pa_ctx, SAMPLE_NAME, &spec, None) {
            Some(stream) => RcCell::new(stream),
            None => bail!("failed to create new stream"),
        };

        // set up write callback for writing audio data to the stream
        let mut inner = self.clone();
        let mut stream_ref = stream.clone();
        let mut bytes_written = 0;

        // NOTE: calling `stream_ref.set_write_callback(None)` causes a segmentation fault
        // see: https://github.com/jnqnfe/pulse-binding-rust/issues/56
        stream.set_write_callback(Some(Box::new(move |len| {
            if let Err(e) = stream_ref.write(&audio_data, None, 0, SeekMode::Relative) {
                log::error!(
                    "failed to write to stream: {:?} - {:?}",
                    e,
                    inner.pa_ctx.errno().to_string()
                );
                return;
            }

            bytes_written += len;

            // we're finished writing the audio data, finish the upload, thereby saving the audio stream
            // as a sample in the audio server (so we can play it later)
            if bytes_written == audio_data_len {
                if let Ok(()) = stream_ref.finish_upload() {
                    // the upload to the audio server has completed - we're ready to use the sample now
                    inner.increment_sound = true;
                }
            }
        })));

        // connect the stream as an upload, which sends it to the audio server instead of playing it directly
        stream.connect_upload(audio_data_len)?;

        Ok(())
    }
}

#[async_trait(?Send)]
impl BarItem for Pulse {
    async fn start(&self, mut ctx: Context) -> Result<crate::context::StopAction> {
        // setup pulse main loop
        let (mut main_loop, pa_ctx) = {
            let mut main_loop = TokioMain::new();

            let app_name = env!("CARGO_PKG_NAME");
            let mut props = Proplist::new().ok_or("Failed to create PulseAudio Proplist")?;
            let _ = props.set_str(APPLICATION_NAME, app_name);
            let _ = props.set_str(APPLICATION_PROCESS_ID, &process::id().to_string());

            let mut pa_ctx = PAContext::new_with_proplist(&main_loop, app_name, &props)
                .ok_or("Failed to create PulseAudio context")?;

            pa_ctx.connect(self.server_name.as_deref(), FlagSet::NOFAIL, None)?;
            match main_loop.wait_for_ready(&pa_ctx).await {
                Ok(State::Ready) => {}
                Ok(state) => bail!(
                    "failed to connect: state={:?}, err={:?}",
                    state,
                    pa_ctx.errno().to_string()
                ),
                Err(_) => bail!("Pulse mainloop exited while waiting on context, not continuing"),
            }

            (main_loop, pa_ctx)
        };

        // this is shared between all the async tasks
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut inner = RcCell::new(PulseState {
            tx,
            increment: self.increment,
            increment_sound: false,
            max_volume: self.max_volume,

            pa_ctx,
            default_sink: "?".into(),
            default_source: "?".into(),
            sinks: vec![],
            sources: vec![],
        });

        // subscribe to server changes
        let (exit_tx, mut exit_rx) = mpsc::unbounded_channel();
        inner.subscribe_to_state_changes(exit_tx.clone());
        inner.subscribe_to_server_changes();
        inner.fetch_server_state();

        // if a sound file was given, then setup a sample
        if let Some(ref path) = self.increment_sound {
            if let Err(e) = inner.setup_volume_sample(expand_path(path)?).await {
                log::error!("failed to setup volume sample: {}", e);
            }
        }

        // run pulse main loop
        tokio::task::spawn_local(async move {
            let ret = main_loop.run().await;
            log::warn!("exited with return value: {}", ret.0);
            let _ = exit_tx.send(());
        });

        let dbus = dbus_connection(BusType::Session).await?;
        let notifications = NotificationsProxy::new(&dbus).await?;
        loop {
            tokio::select! {
                // handle events
                Some(event) = ctx.raw_event_rx().recv() => match event {
                    BarEvent::Custom { payload, responder } => inner.handle_custom_message(payload, responder),
                    BarEvent::Click(click) => match click.button {
                        // cycle source ports
                        I3Button::Left if click.modifiers.contains(&I3Modifier::Shift) => {
                            inner.cycle_objects_and_ports(Object::Source, Dir::Next, |success| {
                                if !success {
                                    log::warn!("failed to cycle {}", Object::Source);
                                }
                            });
                        }

                        // cycle sink ports
                        I3Button::Left => {
                            inner.cycle_objects_and_ports(Object::Sink, Dir::Next, |success| {
                                if !success {
                                    log::warn!("failed to cycle {}", Object::Sink);
                                }
                            });
                        }

                        // source
                        I3Button::Middle if click.modifiers.contains(&I3Modifier::Shift) => {
                            inner.toggle_mute(Object::Source, |success| {
                                if !success {
                                    log::warn!("failed to toggle mute for default {}", Object::Source);
                                }
                            });
                        },
                        I3Button::ScrollUp if click.modifiers.contains(&I3Modifier::Shift) => {
                            inner.set_volume(Object::Source, Vol::Incr(inner.increment), |success| {
                                if !success {
                                    log::warn!("failed to increment volume for default {}", Object::Source);
                                }
                            });
                        }
                        I3Button::ScrollDown if click.modifiers.contains(&I3Modifier::Shift) => {
                            inner.set_volume(Object::Source, Vol::Decr(inner.increment), |success| {
                                if !success {
                                    log::warn!("failed to decrement volume for default {}", Object::Source);
                                }
                            });
                        }

                        // sink
                        I3Button::Middle  => {
                            inner.toggle_mute(Object::Sink, |success| {
                                if !success {
                                    log::warn!("failed to toggle mute for default {}", Object::Sink);
                                }
                            });
                        },
                        I3Button::ScrollUp  => {
                            inner.set_volume(Object::Sink, Vol::Incr(inner.increment), |success| {
                                if !success {
                                    log::warn!("failed to increment volume for default {}", Object::Sink);
                                }
                            });
                        }
                        I3Button::ScrollDown  => {
                            inner.set_volume(Object::Sink, Vol::Decr(inner.increment), |success| {
                                if !success {
                                    log::warn!("failed to decrement volume for default {}", Object::Sink);
                                }
                            });
                        }

                        _ => {}
                    }
                    _ => {}
                },

                // whenever we want to refresh our item, an event is send on this channel
                Some(cmd) = rx.recv() => match cmd {
                    Command::UpdateItem(cb) => {
                        ctx.update_item(cb(&ctx.config.theme)).await?;
                    }
                    Command::NotifyVolume { name, volume, mute } => {
                        if self.notify.should_notify(NotificationSetting::VolumeMute) {
                            let _ = notifications.pulse_volume_mute(name, volume, mute).await;
                        }
                    }
                    Command::NotifyNewSourceSink { name, what } => {
                        if self.notify.should_notify(NotificationSetting::NewSourceSink) {
                            let _ = notifications.pulse_new_source_sink(name, what).await;
                        }
                    }
                    Command::NotifyDefaultsChange { name, what } => {
                        if self.notify.should_notify(NotificationSetting::DefaultsChange) {
                            let _ = notifications.pulse_defaults_change(name, what).await;
                        }
                    }
                },

                // handle pulse main loop exited
                Some(()) = exit_rx.recv() => {
                    log::warn!("connection to server closed");
                    break Ok(StopAction::Restart);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use libpulse_binding::error::Code;

    #[test]
    fn check_code_cast() {
        use num_traits::ToPrimitive;

        assert_eq!(Code::ConnectionTerminated.to_i32().unwrap(), 11);
    }
}

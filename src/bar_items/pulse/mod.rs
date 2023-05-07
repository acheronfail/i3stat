mod pulse_tokio;

use std::cell::RefCell;
use std::error::Error;
use std::process;
use std::rc::Rc;

use async_trait::async_trait;
use libpulse_binding::callbacks::ListResult;
use libpulse_binding::context::introspect::{
    ClientInfo,
    Introspector,
    SinkInfo,
    SinkInputInfo,
    SourceInfo,
    SourceOutputInfo,
};
use libpulse_binding::context::subscribe::{Facility, InterestMaskSet, Operation};
use libpulse_binding::context::{Context as PAContext, FlagSet, State};
use libpulse_binding::def::DevicePortType;
use libpulse_binding::proplist::properties::{APPLICATION_NAME, APPLICATION_PROCESS_ID};
use libpulse_binding::proplist::Proplist;
use libpulse_binding::volume::{ChannelVolumes, Volume};
use tokio::sync::mpsc::UnboundedSender;

use self::pulse_tokio::TokioMain;
use crate::context::{BarItem, Context};
use crate::i3::I3Item;

#[derive(Default)]
struct Cell<T>(pub std::cell::Cell<T>);

impl<T> Cell<T> {
    pub fn new(t: T) -> Cell<T> {
        Cell(std::cell::Cell::new(t))
    }
}

impl<T: Default> Cell<T> {
    pub fn inspect<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut t = self.0.take();
        let rv = f(&mut t);
        self.0.set(t);
        rv
    }
}

impl<T> std::ops::Deref for Cell<T> {
    type Target = std::cell::Cell<T>;
    fn deref(&self) -> &std::cell::Cell<T> {
        &self.0
    }
}

/// Information about a `Sink` or a `Source`
#[derive(Debug, Default)]
struct Port {
    index: u32,
    name: String,
    port: String,
    volume: ChannelVolumes,
    mute: bool,
    port_type: Option<DevicePortType>,
}

macro_rules! impl_port_from {
    ($ty:ty) => {
        impl<'a> From<&'a $ty> for Port {
            fn from(value: &'a $ty) -> Self {
                Port {
                    index: value.index,
                    name: value.name.as_deref().unwrap_or("").to_owned(),
                    port: value
                        .active_port
                        .as_ref()
                        .and_then(|p| p.description.as_deref())
                        .unwrap_or("")
                        .to_owned(),
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

/// Information about a `SinkInput` or a `SourceOutput`
#[derive(Debug, Default)]
struct Wire {
    index: u32,
    client: Option<u32>,
    port: u32,
    volume: ChannelVolumes,
    mute: bool,
}

macro_rules! impl_wire_from {
    ($ty:ty, $ident:ident) => {
        impl<'a> From<&'a $ty> for Wire {
            fn from(value: &'a $ty) -> Self {
                Wire {
                    index: value.index,
                    client: value.client,
                    port: value.$ident,
                    volume: value.volume,
                    mute: value.mute,
                }
            }
        }
    };
}

impl_wire_from!(SinkInputInfo<'a>, sink);
impl_wire_from!(SourceOutputInfo<'a>, source);

/// Information about a pulse `Client`
#[derive(Debug, Default)]
struct Client {
    index: u32,
    name: String,
}

impl<'a> From<&'a ClientInfo<'a>> for Client {
    fn from(value: &'a ClientInfo<'a>) -> Self {
        Client {
            index: value.index,
            name: value.name.as_deref().unwrap_or("").to_owned(),
        }
    }
}

#[derive(Debug)]
enum CtxCommand {
    UpdateItem(I3Item),
}

#[derive(Default)]
pub struct Pulse {}

pub struct PulseState {
    tx: UnboundedSender<CtxCommand>,
    // NOTE: wrapped in cells so it can be easily shared between tokio tasks on the same thread
    pa_ctx: RefCell<PAContext>,
    default_sink: Cell<String>,
    default_source: Cell<String>,
    sinks: Cell<Vec<Port>>,
    sources: Cell<Vec<Port>>,
    sink_inputs: Cell<Vec<Wire>>,
    source_outputs: Cell<Vec<Wire>>,
    clients: Cell<Vec<Client>>,
}

macro_rules! impl_add_remove {
    ($name:ident) => {
        paste::paste! {
            fn [<add_ $name>](&self, result: ListResult<&[<$name:camel Info>]>) {
                match result {
                    ListResult::Item(info) => self.[<$name s>].inspect(|items| {
                        match items.iter_mut().find(|s| s.index == info.index) {
                            Some(s) => *s = info.into(),
                            None => items.push(info.into()),
                        }
                    }),
                    ListResult::Error => todo!("add_{} failed", stringify!($name)),
                    ListResult::End => {}
                }
            }

            fn [<remove_ $name>](&self, idx: u32) {
                self.[<$name s>].inspect(|items| {
                    items.retain(|s| s.index == idx);
                });
            }
        }
    };
}

impl PulseState {
    impl_add_remove!(sink);
    impl_add_remove!(sink_input);
    impl_add_remove!(source);
    impl_add_remove!(source_output);
    impl_add_remove!(client);

    // TODO: update item representation
    fn update_item(self: &Rc<Self>) {
        let default_sink = self.default_sink.inspect(|s| s.to_string());
        let sink_vol_pct = self
            .sinks
            .inspect(|sinks| {
                let sink = sinks.iter().find(|s| s.name == default_sink);
                sink.map(|s| s.volume)
            })
            // TODO: volume wrappers for percentages
            .map(|cv| {
                let value = cv.avg().0 as f64;
                let pct = value / Volume::NORMAL.0 as f64 * 100.0;
                format!("{:.0}", pct)
            })
            .unwrap_or("?".into());

        let source_volume = 0; // TODO

        // TODO: pango markup for muted state?
        let item = I3Item::new(format!("IN: {}, OUT: {}%", source_volume, sink_vol_pct));

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
                                inspect.$get(idx, move |result| state.[<add_ $obj:snake>](result));
                                self.update_item();
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
            (Source, get_source_info_by_index),
            (SinkInput, get_sink_input_info),
            (SourceOutput, get_source_output_info),
            (Client, get_client_info)
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
            pa_ctx: RefCell::new(pa_ctx),

            default_sink: Cell::new("?".into()),
            default_source: Cell::new("?".into()),
            sinks: Cell::new(vec![]),
            sources: Cell::new(vec![]),
            sink_inputs: Cell::new(vec![]),
            source_outputs: Cell::new(vec![]),
            clients: Cell::new(vec![]),
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
                // TODO handle
                assert!(success, "failed to subscribe")
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
            inspect.get_client_info_list(move |item| {
                state.add_client(item);
            });

            let state = inner.clone();
            inspect.get_sink_input_info_list(move |item| {
                state.add_sink_input(item);
            });

            let state = inner.clone();
            inspect.get_source_output_info_list(move |item| {
                state.add_source_output(item);
            });

            let state = inner.clone();
            inspect.get_server_info(move |info| {
                if let Some(name) = &info.default_sink_name {
                    state.default_sink.set((**name).to_owned());
                }
                if let Some(name) = &info.default_source_name {
                    state.default_source.set((**name).to_owned());
                }

                state.update_item();
            });
        }

        // run pulse main loop
        tokio::task::spawn_local(async move {
            let code = main_loop.run().await;
            // TODO: potentially try to reconnect? test it out with restarting pulse, etc
            todo!("Handle pulse loop exit: {}", code.0);
        });

        loop {
            tokio::select! {
                // handle click events
                Some(click) = ctx.raw_click_rx().recv() => {
                    dbg!(click);
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

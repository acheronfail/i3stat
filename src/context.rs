use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use sysinfo::{System, SystemExt};
use tokio::sync::mpsc::error::{SendError, TryRecvError};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::sleep;

use crate::i3::bar_item::I3Item;
use crate::i3::click::I3ClickEvent;

pub struct SharedState {
    pub sys: System,
    // TODO: dbus? due to its limitations, I could setup a list of `MatchRule`s and use a channel to send them out?
}

impl SharedState {
    pub fn new() -> State {
        Arc::new(Mutex::new(SharedState {
            // TODO: only load what we need (depending on configuration, etc)
            sys: System::new_all(),
        }))
    }
}

pub type State = Arc<Mutex<SharedState>>;

pub struct Context {
    pub state: State,
    tx_item: Sender<(I3Item, usize)>,
    rx_event: Receiver<I3ClickEvent>,
    index: usize,
}

impl Context {
    pub fn new(
        state: State,
        tx_item: Sender<(I3Item, usize)>,
        rx_event: Receiver<I3ClickEvent>,
        index: usize,
    ) -> Context {
        Context {
            state,
            tx_item,
            rx_event,
            index,
        }
    }

    pub async fn update_item(&self, item: I3Item) -> Result<(), SendError<(I3Item, usize)>> {
        // TODO: check for diff from last? no need to update if the same item is sent multiple times
        self.tx_item.send((item, self.index)).await
    }

    pub async fn wait_for_click(&mut self) -> Option<I3ClickEvent> {
        self.rx_event.recv().await
    }

    pub async fn delay_with_click_handler<F>(&mut self, duration: Duration, mut closure: F)
    where
        F: FnMut(I3ClickEvent),
    {
        tokio::select! {
            Some(click) = self.rx_event.recv() => {
                closure(click);
                loop {
                    match self.rx_event.try_recv() {
                        Ok(click) => closure(click),
                            Err(TryRecvError::Empty) => break,
                            Err(TryRecvError::Disconnected) => todo!()
                    }
                }
            }
            _ = sleep(duration) => {}
        }
    }

    pub fn raw_click_rx(&mut self) -> &mut Receiver<I3ClickEvent> {
        &mut self.rx_event
    }
}

// TODO: it might be nice to optionally require `Send` so we can have a multi-threaded runtime
// right now it's not, since the PulseAudio item can't be `Send`
#[async_trait(?Send)]
pub trait BarItem: Send {
    async fn start(self: Box<Self>, ctx: Context) -> Result<(), Box<dyn Error>>;
}

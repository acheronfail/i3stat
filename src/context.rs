use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use sysinfo::{System, SystemExt};
use tokio::sync::mpsc::error::TryRecvError;
use tokio::time::sleep;

use crate::i3::I3ClickEvent;
use crate::item::Item;

pub struct SharedState {
    pub sys: System,
    // TODO: dbus
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
    tx_item: tokio::sync::mpsc::Sender<(Item, usize)>,
    rx_event: tokio::sync::mpsc::Receiver<I3ClickEvent>,
    index: usize,
}

impl Context {
    pub fn new(
        state: State,
        tx_item: tokio::sync::mpsc::Sender<(Item, usize)>,
        rx_event: tokio::sync::mpsc::Receiver<I3ClickEvent>,
        index: usize,
    ) -> Context {
        Context {
            state,
            tx_item,
            rx_event,
            index,
        }
    }

    pub async fn update_item(
        &self,
        item: Item,
    ) -> Result<(), tokio::sync::mpsc::error::SendError<(Item, usize)>> {
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
}

#[async_trait]
pub trait BarItem: Send {
    // TODO: have this return a result
    async fn start(&mut self, ctx: Context) -> Result<(), Box<dyn Error>>;
}

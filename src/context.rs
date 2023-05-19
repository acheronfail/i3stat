use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;
use std::time::Duration;

use async_trait::async_trait;
use clap::builder::StyledStr;
use futures_core::Future;
use serde_json::Value;
use sysinfo::{System, SystemExt};
use tokio::sync::mpsc::error::{SendError, TryRecvError};
use tokio::sync::{mpsc, oneshot};
use tokio::time::sleep;

use crate::i3::bar_item::I3Item;
use crate::i3::{I3Button, I3ClickEvent};
use crate::theme::Theme;

#[derive(Debug)]
pub enum CustomResponse {
    Help(StyledStr),
    Json(Value),
}

#[derive(Debug)]
pub enum BarEvent {
    Click(I3ClickEvent),
    Signal,
    Custom {
        payload: Vec<String>,
        responder: oneshot::Sender<CustomResponse>,
    },
}

pub struct SharedState {
    pub sys: System,
}

impl SharedState {
    pub fn new() -> Rc<RefCell<SharedState>> {
        Rc::new(RefCell::new(SharedState {
            // this loads nothing, it's up to each item to load what it needs
            sys: System::new(),
        }))
    }
}

pub struct Context {
    pub state: Rc<RefCell<SharedState>>,
    pub theme: Theme,
    // Used as an internal cache to prevent sending the same item multiple times
    last_item: RefCell<I3Item>,
    tx_item: mpsc::Sender<(I3Item, usize)>,
    tx_event: mpsc::Sender<BarEvent>,
    rx_event: mpsc::Receiver<BarEvent>,
    index: usize,
}

impl Context {
    pub fn new(
        state: Rc<RefCell<SharedState>>,
        tx_item: mpsc::Sender<(I3Item, usize)>,
        tx_event: mpsc::Sender<BarEvent>,
        rx_event: mpsc::Receiver<BarEvent>,
        index: usize,
    ) -> Context {
        Context {
            state,
            theme: Theme::NORD,
            last_item: RefCell::default(),
            tx_item,
            tx_event,
            rx_event,
            index,
        }
    }

    pub async fn update_item(&self, item: I3Item) -> Result<(), SendError<(I3Item, usize)>> {
        let mut last = self.last_item.borrow_mut();
        if *last == item {
            return Ok(());
        }

        *last = item.clone();
        self.tx_item.send((item, self.index)).await
    }

    pub async fn wait_for_event(&mut self) -> Option<BarEvent> {
        self.rx_event.recv().await
    }

    pub async fn delay_with_event_handler<F, R>(&mut self, duration: Duration, mut closure: F)
    where
        F: FnMut(BarEvent) -> R,
        R: Future<Output = ()>,
    {
        tokio::select! {
            Some(event) = self.rx_event.recv() => {
                closure(event).await;
                loop {
                    match self.rx_event.try_recv() {
                        Ok(event) => closure(event).await,
                        Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected)=> break,
                    }
                }
            }
            _ = sleep(duration) => {}
        }
    }

    pub fn get_event_tx(&self) -> mpsc::Sender<BarEvent> {
        self.tx_event.clone()
    }

    pub fn raw_event_rx(&mut self) -> &mut mpsc::Receiver<BarEvent> {
        &mut self.rx_event
    }

    // A utility to help with paginating items
    pub fn paginate(event: &BarEvent, len: usize, idx: &mut usize) {
        use I3Button::*;
        match event {
            BarEvent::Click(c) if matches!(c.button, Left | ScrollUp) => *idx += 1,
            BarEvent::Click(c) if matches!(c.button, Right | ScrollDown) => {
                if *idx == 0 {
                    *idx = len - 1
                } else {
                    *idx -= 1
                }
            }
            _ => {}
        }
    }
}

#[async_trait(?Send)]
pub trait BarItem: Send {
    async fn start(self: Box<Self>, ctx: Context) -> Result<(), Box<dyn Error>>;
}

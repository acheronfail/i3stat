use std::time::Duration;

use async_trait::async_trait;
use clap::builder::StyledStr;
use futures::Future;
use serde_json::Value;
use sysinfo::{System, SystemExt};
use tokio::sync::mpsc::error::{SendError, TryRecvError};
use tokio::sync::{mpsc, oneshot};
use tokio::time::sleep;

use crate::config::AppConfig;
use crate::error::Result;
use crate::i3::bar_item::I3Item;
use crate::i3::I3ClickEvent;
use crate::util::RcCell;

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

#[derive(Debug)]
pub struct SharedState {
    pub sys: System,
}

impl SharedState {
    pub fn new() -> RcCell<SharedState> {
        RcCell::new(SharedState {
            // this loads nothing, it's up to each item to load what it needs
            sys: System::new(),
        })
    }
}

#[derive(Debug)]
pub struct Context {
    pub config: RcCell<AppConfig>,
    pub state: RcCell<SharedState>,
    tx_item: mpsc::Sender<(I3Item, usize)>,
    rx_event: mpsc::Receiver<BarEvent>,
    index: usize,
}

impl Context {
    pub fn new(
        config: RcCell<AppConfig>,
        state: RcCell<SharedState>,
        tx_item: mpsc::Sender<(I3Item, usize)>,
        rx_event: mpsc::Receiver<BarEvent>,
        index: usize,
    ) -> Context {
        Context {
            config,
            state,
            tx_item,
            rx_event,
            index,
        }
    }

    pub async fn update_item(
        &self,
        item: I3Item,
    ) -> std::result::Result<(), SendError<(I3Item, usize)>> {
        self.tx_item.send((item, self.index)).await?;
        Ok(())
    }

    pub async fn wait_for_event(&mut self, delay: Option<Duration>) -> Option<BarEvent> {
        match delay {
            None => self.rx_event.recv().await,
            Some(delay) => tokio::select! {
                event = self.rx_event.recv() => event,
                _ = sleep(delay) => None
            },
        }
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

    pub fn raw_event_rx(&mut self) -> &mut mpsc::Receiver<BarEvent> {
        &mut self.rx_event
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub enum StopAction {
    /// The task finished, and the item will stay in the bar
    #[default]
    Complete,
    /// The task finished, and the item should be removed from the bar
    Remove,
    /// The task finished, and should be restarted
    Restart,
}

#[async_trait(?Send)]
pub trait BarItem: Send {
    async fn start(&self, ctx: Context) -> Result<StopAction>;
}

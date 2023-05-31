use std::error::Error;
use std::iter::Enumerate;
use std::slice::Iter;

use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::Sender;

use crate::context::BarEvent;

#[derive(Debug, Clone)]
pub struct Dispatcher {
    inner: Vec<Sender<BarEvent>>,
}

impl Dispatcher {
    pub fn new(inner: Vec<Sender<BarEvent>>) -> Dispatcher {
        Dispatcher { inner }
    }

    pub fn enumerate(&self) -> Enumerate<Iter<Sender<BarEvent>>> {
        self.inner.iter().enumerate()
    }

    pub fn get(&self, idx: usize) -> Result<&Sender<BarEvent>, Box<dyn Error>> {
        match self.inner.get(idx) {
            Some(tx) => Ok(tx),
            None => Err(format!("no item found with index: {}", idx).into()),
        }
    }

    pub async fn send_bar_event(&self, idx: usize, ev: BarEvent) -> Result<(), Box<dyn Error>> {
        match self.inner.get(idx) {
            Some(tx) => {
                // if the channel fills up (the bar never reads click events), since this is a bounded channel
                // sending the event would block forever, so just drop the event
                if tx.capacity() == 0 {
                    return Err(format!(
                        "failed to send event to item[{}]: dropping event (channel is full)",
                        idx
                    )
                    .into());
                }

                // send click event to its corresponding bar item
                if let Err(SendError(_)) = tx.send(ev).await {
                    return Err(format!(
                        "failed to send event to item[{}]: dropping event (receiver dropped)",
                        idx
                    )
                    .into());
                }

                Ok(())
            }
            None => Err(format!("no item found with index: {}", idx).into()),
        }
    }
}

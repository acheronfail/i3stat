use std::error::Error;

use futures::future::join_all;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::Sender;

use crate::context::BarEvent;

#[derive(Debug, Clone)]
pub struct Dispatcher {
    inner: Vec<Option<Sender<BarEvent>>>,
}

impl Dispatcher {
    pub fn new(capacity: usize) -> Dispatcher {
        Dispatcher {
            inner: vec![None; capacity],
        }
    }

    pub fn set(&mut self, idx: usize, tx: Sender<BarEvent>) {
        self.inner[idx] = Some(tx);
    }

    pub async fn signal_all(&self) -> Result<(), Box<dyn Error>> {
        Ok(join_all(
            self.inner
                .iter()
                .enumerate()
                .filter_map(|(i, o)| o.as_ref().map(|_| self.send_bar_event(i, BarEvent::Signal))),
        )
        .await
        .into_iter()
        .for_each(|r| {
            if let Err(e) = r {
                log::warn!("{}", e);
            }
        }))
    }

    pub async fn send_bar_event(&self, idx: usize, ev: BarEvent) -> Result<(), Box<dyn Error>> {
        match self.inner.get(idx) {
            Some(Some(tx)) => {
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
            None | Some(None) => Err(format!("no item found with index: {}", idx).into()),
        }
    }
}

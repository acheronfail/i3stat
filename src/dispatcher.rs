use std::collections::hash_map::{IntoIter, Iter};
use std::collections::HashMap;
use std::error::Error;

use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::Sender;

use crate::config::Item;
use crate::context::BarEvent;

type InnerItem = (Sender<BarEvent>, Item);

#[derive(Debug, Clone)]
pub struct Dispatcher {
    inner: HashMap<usize, InnerItem>,
}

impl Dispatcher {
    pub fn new(inner: HashMap<usize, InnerItem>) -> Dispatcher {
        Dispatcher { inner }
    }

    pub fn iter(&self) -> Iter<usize, InnerItem> {
        self.inner.iter()
    }

    pub fn get(&self, idx: usize) -> Option<&InnerItem> {
        self.inner.get(&idx)
    }

    pub fn instance_mapping(&self) -> HashMap<usize, String> {
        // TODO: consistent ordering (indexmap?)
        self.inner
            .iter()
            .map(|(idx, (_, item))| {
                (
                    *idx,
                    item.common
                        .name
                        .as_ref()
                        .map_or_else(|| item.tag().into(), |n| n.to_string()),
                )
            })
            .collect()
    }

    pub async fn send_bar_event(&self, idx: usize, ev: BarEvent) -> Result<(), Box<dyn Error>> {
        match self.inner.get(&idx) {
            Some((tx, _)) => {
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

impl IntoIterator for Dispatcher {
    type Item = (usize, InnerItem);

    type IntoIter = IntoIter<usize, InnerItem>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

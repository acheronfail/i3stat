use serde::Serialize;
use serde_derive::Serialize;
use sysinfo::System;

use crate::item::ToItem;

#[derive(Debug, Serialize)]
pub struct I3BarHeader {
    version: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_signal: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cont_signal: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    click_events: Option<bool>,
}

impl Default for I3BarHeader {
    fn default() -> Self {
        I3BarHeader {
            version: 1,
            stop_signal: None,
            cont_signal: None,
            click_events: Some(true),
        }
    }
}

// #[derive(Debug)]
pub struct Bar(pub Vec<Box<dyn ToItem>>);

impl Bar {
    pub fn update(&mut self, sys: &mut System) {
        self.0.iter_mut().for_each(|item| item.update(sys));
    }
}

impl Serialize for Bar {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        s.collect_seq(self.0.iter().map(|x| x.to_item()))
    }
}

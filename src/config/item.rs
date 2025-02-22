use std::cell::OnceCell;
use std::collections::HashSet;

use serde_derive::{Deserialize, Serialize};
use strum::EnumIter;

use crate::bar_items::*;
use crate::context::BarItem;
use crate::i3::{I3Item, I3Modifier};

/// Custom item action.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Action {
    /// Run the given command
    Simple(String),
    /// Run the given command, if the modifiers are present
    WithOptions {
        command: String,
        modifiers: HashSet<I3Modifier>,
    },
}

/// A wrapper struct to allow defining a single item or many.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ActionWrapper {
    Single(Action),
    Many(Vec<Action>),
}

/// Custom actions that are configurable per item.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Actions {
    #[serde(default)]
    pub left_click: Option<ActionWrapper>,
    #[serde(default)]
    pub middle_click: Option<ActionWrapper>,
    #[serde(default)]
    pub right_click: Option<ActionWrapper>,
}

/// Configuration that's common to every item.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Common {
    /// Name of the item. Used in the IPC protocol.
    /// Defaults to the item's type.
    pub name: Option<String>,
    /// Override the index of the item.
    pub index: Option<usize>,
    /// Provide a signal for the time.
    pub signal: Option<u32>,
    /// Optionally set or unset the separator for this item.
    pub separator: Option<bool>,
    /// Optionally configure actions for each item.
    pub actions: Option<Actions>,
    /// Optionally hide items. This is useful for some items which have a CLI
    /// interface; sometimes it's nice to provide the CLI without needing to
    /// take up space in the bar. (E.g., multiple "light" items, one for a laptop
    /// screen and another for the keyboard backlight, and you don't have to show
    /// the item for the keyboard backlight in the bar.)
    pub hidden: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone, EnumIter)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ItemInner {
    Raw(I3Item),
    Battery(Battery),
    Cpu(Cpu),
    Disk(Disk),
    Dunst(Dunst),
    Kbd(Kbd),
    Krb(Krb),
    Light(Light),
    Mako(Mako),
    Mem(Mem),
    NetUsage(NetUsage),
    Nic(Nic),
    Pulse(Pulse),
    Script(Script),
    Sensors(Sensors),
    Time(Time),
}

impl ItemInner {
    // Can't seem to use serde to access the tags, even though it's automatically derived the tags.
    // For now, we have a test ensuring this is accurate.
    // See: https://github.com/serde-rs/serde/issues/2455
    pub fn tag(&self) -> &'static str {
        match self {
            ItemInner::Raw(_) => "raw",
            ItemInner::Battery(_) => "battery",
            ItemInner::Cpu(_) => "cpu",
            ItemInner::Disk(_) => "disk",
            ItemInner::Dunst(_) => "dunst",
            ItemInner::Kbd(_) => "kbd",
            ItemInner::Krb(_) => "krb",
            ItemInner::Light(_) => "light",
            ItemInner::Mako(_) => "mako",
            ItemInner::Mem(_) => "mem",
            ItemInner::NetUsage(_) => "net_usage",
            ItemInner::Nic(_) => "nic",
            ItemInner::Pulse(_) => "pulse",
            ItemInner::Script(_) => "script",
            ItemInner::Sensors(_) => "sensors",
            ItemInner::Time(_) => "time",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    #[serde(flatten)]
    pub common: Common,
    #[serde(flatten)]
    inner: ItemInner,

    /// A runtime only cache for this item's name
    #[serde(skip)]
    name: OnceCell<String>,
}

impl Item {
    pub fn to_bar_item(&self) -> Box<dyn BarItem> {
        match &self.inner {
            ItemInner::Raw(inner) => Box::new(inner.clone()),
            ItemInner::Battery(inner) => Box::new(inner.clone()),
            ItemInner::Cpu(inner) => Box::new(inner.clone()),
            ItemInner::Disk(inner) => Box::new(inner.clone()),
            ItemInner::Dunst(inner) => Box::new(inner.clone()),
            ItemInner::Kbd(inner) => Box::new(inner.clone()),
            ItemInner::Krb(inner) => Box::new(inner.clone()),
            ItemInner::Light(inner) => Box::new(inner.clone()),
            ItemInner::Mako(inner) => Box::new(inner.clone()),
            ItemInner::Mem(inner) => Box::new(inner.clone()),
            ItemInner::NetUsage(inner) => Box::new(inner.clone()),
            ItemInner::Nic(inner) => Box::new(inner.clone()),
            ItemInner::Pulse(inner) => Box::new(inner.clone()),
            ItemInner::Script(inner) => Box::new(inner.clone()),
            ItemInner::Sensors(inner) => Box::new(inner.clone()),
            ItemInner::Time(inner) => Box::new(inner.clone()),
        }
    }

    pub fn name(&self) -> &String {
        self.name.get_or_init(|| match self.common.name {
            Some(ref name) => name.to_string(),
            None => self.inner.tag().into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use strum::IntoEnumIterator;

    use super::*;

    // only used in tests, in production code items are only created via deserialisation
    impl Item {
        pub fn new(common: Common, item: I3Item) -> Item {
            Item {
                common,
                inner: ItemInner::Raw(item),
                name: OnceCell::new(),
            }
        }
    }

    #[test]
    fn item_tags() {
        let assert_tag = |item: &ItemInner| {
            let v = json!(item);
            let serialised_tag = v.get("type").unwrap();
            let computed_tag = item.tag();
            assert_eq!(
                serialised_tag, computed_tag,
                "item tags did not match, expected {} got {}",
                serialised_tag, computed_tag
            );
        };

        // iterate over all enums and assert tags
        for variant in ItemInner::iter() {
            assert_tag(&variant);
        }
    }
}

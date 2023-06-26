use std::collections::HashMap;

use async_trait::async_trait;
use hex_color::HexColor;
use serde::Serialize;
use serde_derive::Deserialize;
use serde_json::Value;

use crate::context::{BarItem, Context, StopAction};
use crate::error::Result;

#[derive(Debug, Default, Serialize, Deserialize, Copy, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum I3Align {
    #[default]
    Center,
    Right,
    Left,
}

#[derive(Debug, Default, Serialize, Deserialize, Copy, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum I3Markup {
    #[default]
    None,
    Pango,
}

impl I3Markup {
    pub fn is_none(opt: &Option<Self>) -> bool {
        match opt {
            None => true,
            Some(inner) => matches!(inner, I3Markup::None),
        }
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(untagged, rename_all = "lowercase")]
pub enum I3MinWidth {
    Pixels(usize),
    StringCount(usize),
    String(String),
}

impl Serialize for I3MinWidth {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            I3MinWidth::Pixels(n) => serializer.serialize_u64(*n as u64),
            I3MinWidth::StringCount(n) => serializer.serialize_str(&"x".repeat(*n)),
            I3MinWidth::String(s) => serializer.serialize_str(&s),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct I3Item {
    pub full_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    short_text: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    instance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<HexColor>,
    #[serde(rename = "background", skip_serializing_if = "Option::is_none")]
    background_color: Option<HexColor>,
    #[serde(rename = "border", skip_serializing_if = "Option::is_none")]
    border_color: Option<HexColor>,

    #[serde(rename = "border_top", skip_serializing_if = "Option::is_none")]
    border_top_px: Option<usize>,
    #[serde(rename = "border_right", skip_serializing_if = "Option::is_none")]
    border_right_px: Option<usize>,
    #[serde(rename = "border_bottom", skip_serializing_if = "Option::is_none")]
    border_bottom_px: Option<usize>,
    #[serde(rename = "border_left", skip_serializing_if = "Option::is_none")]
    border_left_px: Option<usize>,

    #[serde(skip_serializing_if = "Option::is_none")]
    min_width: Option<I3MinWidth>,
    #[serde(skip_serializing_if = "Option::is_none")]
    align: Option<I3Align>,

    #[serde(skip_serializing_if = "Option::is_none")]
    urgent: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    separator: Option<bool>,
    #[serde(
        rename = "separator_block_width",
        skip_serializing_if = "Option::is_none"
    )]
    separator_block_width_px: Option<usize>,

    #[serde(skip_serializing_if = "I3Markup::is_none")]
    markup: Option<I3Markup>,

    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    additional_data: HashMap<String, Value>,
}

macro_rules! impl_get_set {
    ($(#[$outer:meta])* ($name:ident, String)) => {
        paste::paste! {
            $(#[$outer])*
            pub fn $name(mut self, $name: impl AsRef<str>) -> Self {
                self.$name = $name.as_ref().into();
                self
            }

            pub fn [<get_ $name>](&self) -> &String {
                &self.$name
            }
        }
    };
    ($(#[$outer:meta])* ($name:ident, Option<String>)) => {
        paste::paste! {
            $(#[$outer])*
            pub fn $name(mut self, $name: impl AsRef<str>) -> Self {
                self.$name = Some($name.as_ref().into());
                self
            }

            pub fn [<get_ $name>](&self) -> Option<&String> {
                self.$name.as_ref()
            }
        }
    };
    ($(#[$outer:meta])* ($name:ident, $ty:ty)) => {
        paste::paste! {
            $(#[$outer])*
            pub fn $name(mut self, $name: $ty) -> Self {
                self.$name = Some($name);
                self
            }

            pub fn [<get_ $name>](&self) -> Option<&$ty> {
                self.$name.as_ref()
            }
        }
    };
}

impl I3Item {
    pub fn new(full_text: impl AsRef<str>) -> I3Item {
        I3Item {
            full_text: full_text.as_ref().into(),
            short_text: None,
            name: None,
            instance: None,
            color: None,
            background_color: None,
            border_color: None,
            border_top_px: None,
            border_right_px: None,
            border_bottom_px: None,
            border_left_px: None,
            min_width: None,
            align: None,
            urgent: None,
            separator: None,
            separator_block_width_px: None,
            markup: None,
            additional_data: HashMap::new(),
        }
    }

    pub fn empty() -> I3Item {
        I3Item::new("")
    }

    pub fn with_data(mut self, key: impl AsRef<str>, value: Value) -> Self {
        let key = key.as_ref();

        // as per i3's protocol, additional fields must begin with an underscore
        let key = match key.starts_with('_') {
            true => key.into(),
            false => format!("_{}", key),
        };

        self.additional_data.insert(key, value);
        self
    }

    impl_get_set! (
        /// Set the name of the item. NOTE: setting this from within an item implementation will
        /// have no effect, since istat manages this property itself from config.
        (name, Option<String>)
    );
    impl_get_set!(
        /// Set the instance of the item. NOTE: setting this from within an item implementation ill
        /// have no effect, since istat manages this property itself from config.
        (instance, Option<String>)
    );

    impl_get_set!((full_text, String));
    impl_get_set!((short_text, Option<String>));
    impl_get_set!((color, HexColor));
    impl_get_set!((background_color, HexColor));
    impl_get_set!((border_color, HexColor));
    impl_get_set!((border_top_px, usize));
    impl_get_set!((border_right_px, usize));
    impl_get_set!((border_bottom_px, usize));
    impl_get_set!((border_left_px, usize));
    impl_get_set!((min_width, I3MinWidth));
    impl_get_set!((align, I3Align));
    impl_get_set!((urgent, bool));
    impl_get_set!((separator, bool));
    impl_get_set!((separator_block_width_px, usize));
    impl_get_set!((markup, I3Markup));
}

#[async_trait(?Send)]
impl BarItem for I3Item {
    async fn start(&self, ctx: Context) -> Result<StopAction> {
        ctx.update_item(self.clone()).await?;
        Ok(StopAction::Complete)
    }
}

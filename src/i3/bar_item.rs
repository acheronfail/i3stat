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
            I3MinWidth::String(s) => serializer.serialize_str(s),
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

    pub fn is_empty(&self) -> bool {
        self.full_text.is_empty()
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

    pub fn as_env_map(&self) -> Result<HashMap<String, String>> {
        use serde_json::{from_value, to_value};

        let mut env_map = HashMap::new();
        for (key, value) in from_value::<HashMap<String, Value>>(to_value(self)?)? {
            env_map.insert(
                key,
                match value {
                    Value::String(s) => s,
                    other => other.to_string(),
                },
            );
        }

        Ok(env_map)
    }

    impl_get_set! (
        /// Set the name of the item. NOTE: setting this from within an item implementation will
        /// have no effect, since i3stat manages this property itself from config.
        (name, Option<String>)
    );
    impl_get_set!(
        /// Set the instance of the item. NOTE: setting this from within an item implementation ill
        /// have no effect, since i3stat manages this property itself from config.
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

#[cfg(test)]
mod tests {
    use hex_color::HexColor;
    use serde_json::json;

    use super::I3Item;
    use crate::i3::{I3Align, I3Markup, I3MinWidth};

    #[test]
    fn as_env_map() {
        let item = I3Item::new("full_text")
            .with_data("custom_field", "custom_field".into())
            .align(I3Align::Center)
            .background_color(HexColor::MAGENTA)
            .border_bottom_px(1)
            .border_color(HexColor::CYAN)
            .border_left_px(2)
            .border_right_px(3)
            .border_top_px(4)
            .color(HexColor::GREEN)
            .full_text("full_text")
            .instance("instance")
            .markup(I3Markup::Pango)
            .min_width(I3MinWidth::Pixels(5))
            .separator_block_width_px(6)
            .separator(true)
            .short_text("short_text")
            .urgent(false);

        assert_eq!(
            serde_json::to_value(item.as_env_map().unwrap()).unwrap(),
            json!({
              "_custom_field": "custom_field",
              "align": "center",
              "background": "#FF00FF",
              "border": "#00FFFF",
              "border_bottom": "1",
              "border_left": "2",
              "border_right": "3",
              "border_top": "4",
              "color": "#00FF00",
              "full_text": "full_text",
              "instance": "instance",
              "markup": "pango",
              "min_width": "5",
              "separator": "true",
              "separator_block_width": "6",
              "short_text": "short_text",
              "urgent": "false"
            }
            )
        );
    }
}

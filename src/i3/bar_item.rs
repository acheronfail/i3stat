use std::error::Error;

use async_trait::async_trait;
use hex_color::HexColor;
use serde_derive::Serialize;

use crate::context::{BarItem, Context};

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum I3Align {
    Center,
    Right,
    Left,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum I3Markup {
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

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(untagged, rename_all = "lowercase")]
pub enum I3MinWidth {
    Pixels(usize),
    String(String),
}

#[derive(Debug, Default, Serialize, Clone, PartialEq, Eq)]
pub struct I3Item {
    full_text: String,
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
        }
    }

    pub fn empty() -> I3Item {
        I3Item::new("")
    }

    pub fn short_text(mut self, short_text: impl AsRef<str>) -> Self {
        self.short_text = Some(short_text.as_ref().into());
        self
    }

    pub fn color(mut self, color: HexColor) -> Self {
        self.color = Some(color);
        self
    }

    pub fn background_color(mut self, background_color: HexColor) -> Self {
        self.background_color = Some(background_color);
        self
    }

    pub fn border_color(mut self, border_color: HexColor) -> Self {
        self.border_color = Some(border_color);
        self
    }

    pub fn border_top_px(mut self, border_top_px: usize) -> Self {
        self.border_top_px = Some(border_top_px);
        self
    }

    pub fn border_right_px(mut self, border_right_px: usize) -> Self {
        self.border_right_px = Some(border_right_px);
        self
    }

    pub fn border_bottom_px(mut self, border_bottom_px: usize) -> Self {
        self.border_bottom_px = Some(border_bottom_px);
        self
    }

    pub fn border_left_px(mut self, border_left_px: usize) -> Self {
        self.border_left_px = Some(border_left_px);
        self
    }

    pub fn min_width(mut self, min_width: I3MinWidth) -> Self {
        self.min_width = Some(min_width);
        self
    }

    pub fn align(mut self, align: I3Align) -> Self {
        self.align = Some(align);
        self
    }

    pub fn name(mut self, name: impl AsRef<str>) -> Self {
        self.name = Some(name.as_ref().into());
        self
    }

    pub fn instance(mut self, instance: impl AsRef<str>) -> Self {
        self.instance = Some(instance.as_ref().into());
        self
    }

    pub fn urgent(mut self, urgent: bool) -> Self {
        self.urgent = Some(urgent);
        self
    }

    pub fn separator(mut self, separator: bool) -> Self {
        self.separator = Some(separator);
        self
    }

    pub fn separator_block_width_px(mut self, separator_block_width_px: usize) -> Self {
        self.separator_block_width_px = Some(separator_block_width_px);
        self
    }

    pub fn markup(mut self, markup: I3Markup) -> Self {
        self.markup = Some(markup);
        self
    }
}

#[async_trait(?Send)]
impl BarItem for I3Item {
    async fn start(self: Box<Self>, ctx: Context) -> Result<(), Box<dyn Error>> {
        ctx.update_item(self.as_ref().clone()).await?;
        Ok(())
    }
}

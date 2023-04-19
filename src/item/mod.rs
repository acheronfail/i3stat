pub mod battery;
pub mod cpu;
pub mod disk;
pub mod dunst;
pub mod mem;
pub mod net_usage;
pub mod nic;
pub mod script;
pub mod sensors;
pub mod time;

use hex_color::HexColor;
use serde_derive::Serialize;
use sysinfo::System;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
pub enum Align {
    Center,
    Right,
    Left,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
pub enum Markup {
    None,
    Pango,
}

impl Markup {
    pub fn is_none(opt: &Option<Self>) -> bool {
        match opt {
            None => true,
            Some(inner) => matches!(inner, Markup::None),
        }
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(untagged, rename_all = "lowercase")]
#[allow(dead_code)]
pub enum MinWidth {
    Pixels(usize),
    String(String),
}

#[derive(Debug, Serialize, Clone)]
pub struct Item {
    pub full_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_text: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<HexColor>,
    #[serde(rename = "background", skip_serializing_if = "Option::is_none")]
    pub background_color: Option<HexColor>,
    #[serde(rename = "border", skip_serializing_if = "Option::is_none")]
    pub border_color: Option<HexColor>,

    #[serde(rename = "border_top", skip_serializing_if = "Option::is_none")]
    pub border_top_px: Option<usize>,
    #[serde(rename = "border_right", skip_serializing_if = "Option::is_none")]
    pub border_right_px: Option<usize>,
    #[serde(rename = "border_bottom", skip_serializing_if = "Option::is_none")]
    pub border_bottom_px: Option<usize>,
    #[serde(rename = "border_left", skip_serializing_if = "Option::is_none")]
    pub border_left_px: Option<usize>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_width: Option<MinWidth>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<Align>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub urgent: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub separator: Option<bool>,
    #[serde(
        rename = "separator_block_width",
        skip_serializing_if = "Option::is_none"
    )]
    pub separator_block_width_px: Option<usize>,

    #[serde(skip_serializing_if = "Markup::is_none")]
    pub markup: Option<Markup>,
}

#[allow(dead_code)]
impl Item {
    pub fn new(full_text: impl AsRef<str>) -> Item {
        Item {
            full_text: full_text.as_ref().into(),
            short_text: None,
            color: None,
            background_color: None,
            border_color: None,
            border_top_px: None,
            border_right_px: None,
            border_bottom_px: None,
            border_left_px: None,
            min_width: None,
            align: None,
            name: None,
            instance: None,
            urgent: None,
            separator: None,
            separator_block_width_px: None,
            markup: None,
        }
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

    pub fn min_width(mut self, min_width: MinWidth) -> Self {
        self.min_width = Some(min_width);
        self
    }

    pub fn align(mut self, align: Align) -> Self {
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

    pub fn markup(mut self, markup: Markup) -> Self {
        self.markup = Some(markup);
        self
    }
}

impl ToItem for Item {
    fn to_item(&self) -> Item {
        self.clone()
    }
}

pub trait ToItem {
    fn to_item(&self) -> Item;
    fn update(&mut self, _sys: &mut System) {}
}

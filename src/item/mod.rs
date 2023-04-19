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

// TODO: builder struct to make it easy to create
#[derive(Debug, Serialize, Clone)]
pub struct Item {
    pub full_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_text: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<HexColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<HexColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border: Option<HexColor>,

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

impl Item {
    pub fn text(text: impl AsRef<str>) -> Item {
        Item {
            full_text: text.as_ref().to_string(),
            short_text: None,
            color: None,
            background: None,
            border: None,
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

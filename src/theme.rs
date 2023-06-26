use hex_color::HexColor;
use serde_derive::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorPair {
    pub fg: HexColor,
    pub bg: HexColor,
}

impl ColorPair {
    pub const fn new(fg: HexColor, bg: HexColor) -> ColorPair {
        ColorPair { fg, bg }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerlineSeparator {
    value: String,
    #[serde(default)]
    scale: Option<u32>,
}

impl PowerlineSeparator {
    pub fn to_span(&self) -> String {
        match self.scale {
            None => self.value.clone(),
            Some(pct) => format!(r#"<span size="{}%">{}</span>"#, pct, self.value),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    #[serde(default = "Theme::default_bg")]
    pub bg: HexColor,
    #[serde(default = "Theme::default_fg")]
    pub fg: HexColor,
    #[serde(default = "Theme::default_dim")]
    pub dim: HexColor,
    #[serde(default = "Theme::default_red")]
    pub red: HexColor,
    #[serde(default = "Theme::default_orange")]
    pub orange: HexColor,
    #[serde(default = "Theme::default_yellow")]
    pub yellow: HexColor,
    #[serde(default = "Theme::default_green")]
    pub green: HexColor,
    #[serde(default = "Theme::default_purple")]
    pub purple: HexColor,
    #[serde(default = "Theme::default_blue")]
    pub blue: HexColor,
    #[serde(default = "Theme::default_powerline")]
    pub powerline: Vec<ColorPair>,
    #[serde(default)]
    pub powerline_enable: bool,
    #[serde(default = "Theme::default_powerline_separator")]
    pub powerline_separator: PowerlineSeparator,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: Self::default_bg(),
            fg: Self::default_fg(),
            dim: Self::default_dim(),
            red: Self::default_red(),
            orange: Self::default_orange(),
            yellow: Self::default_yellow(),
            green: Self::default_green(),
            purple: Self::default_purple(),
            blue: Self::default_blue(),
            powerline: Self::default_powerline(),
            powerline_enable: false,
            powerline_separator: Self::default_powerline_separator(),
        }
    }
}

impl Theme {
    pub fn validate(&self) -> Result<()> {
        if self.powerline.len() <= 1 {
            bail!("theme.powerline must contain at least two values");
        }

        Ok(())
    }

    const DEFAULT_POWERLINE: &[ColorPair] = &[
        ColorPair::new(HexColor::rgb(216, 222, 233), HexColor::rgb(59, 66, 82)),
        ColorPair::new(HexColor::rgb(229, 233, 240), HexColor::rgb(67, 76, 94)),
        ColorPair::new(HexColor::rgb(236, 239, 244), HexColor::rgb(76, 86, 106)),
        ColorPair::new(HexColor::rgb(229, 233, 240), HexColor::rgb(67, 76, 94)),
    ];

    const fn default_bg() -> HexColor {
        HexColor::rgb(46, 52, 64)
    }

    const fn default_fg() -> HexColor {
        HexColor::rgb(216, 222, 233)
    }

    const fn default_dim() -> HexColor {
        HexColor::rgb(76, 86, 106)
    }

    const fn default_blue() -> HexColor {
        HexColor::rgb(143, 188, 187)
    }

    const fn default_red() -> HexColor {
        HexColor::rgb(191, 97, 106)
    }

    const fn default_orange() -> HexColor {
        HexColor::rgb(208, 135, 112)
    }

    const fn default_yellow() -> HexColor {
        HexColor::rgb(235, 203, 139)
    }

    const fn default_green() -> HexColor {
        HexColor::rgb(163, 190, 140)
    }

    const fn default_purple() -> HexColor {
        HexColor::rgb(180, 142, 173)
    }

    fn default_powerline() -> Vec<ColorPair> {
        Self::DEFAULT_POWERLINE.to_vec()
    }

    fn default_powerline_separator() -> PowerlineSeparator {
        PowerlineSeparator {
            value: "".into(),
            scale: None,
        }
    }
}

use hex_color::HexColor;
use serde_derive::{Deserialize, Serialize};

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
        }
    }
}

impl Theme {
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
}

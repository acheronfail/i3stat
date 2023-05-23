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
    #[serde(default = "Theme::default_error")]
    pub error: HexColor,
    #[serde(default = "Theme::default_danger")]
    pub danger: HexColor,
    #[serde(default = "Theme::default_warning")]
    pub warning: HexColor,
    #[serde(default = "Theme::default_good")]
    pub good: HexColor,
    #[serde(default = "Theme::default_special")]
    pub special: HexColor,
    #[serde(default = "Theme::default_accent")]
    pub accent: HexColor,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: Self::default_bg(),
            fg: Self::default_fg(),
            dim: Self::default_dim(),
            error: Self::default_error(),
            danger: Self::default_danger(),
            warning: Self::default_warning(),
            good: Self::default_good(),
            special: Self::default_special(),
            accent: Self::default_accent(),
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

    const fn default_accent() -> HexColor {
        HexColor::rgb(143, 188, 187)
    }

    const fn default_error() -> HexColor {
        HexColor::rgb(191, 97, 106)
    }

    const fn default_danger() -> HexColor {
        HexColor::rgb(208, 135, 112)
    }

    const fn default_warning() -> HexColor {
        HexColor::rgb(235, 203, 139)
    }

    const fn default_good() -> HexColor {
        HexColor::rgb(163, 190, 140)
    }

    const fn default_special() -> HexColor {
        HexColor::rgb(180, 142, 173)
    }
}

use hex_color::HexColor;

#[derive(Debug, Clone)]
pub struct Theme {
    pub dark1: HexColor,
    pub dark2: HexColor,
    pub dark3: HexColor,
    pub dark4: HexColor,
    pub light1: HexColor,
    pub light2: HexColor,
    pub light3: HexColor,
    pub accent1: HexColor,
    pub accent2: HexColor,
    pub accent3: HexColor,
    pub accent4: HexColor,
    pub error: HexColor,
    pub danger: HexColor,
    pub warning: HexColor,
    pub success: HexColor,
    pub special: HexColor,
}

impl Theme {
    pub const NORD: Theme = Theme {
        dark1: HexColor::rgb(46, 52, 64),
        dark2: HexColor::rgb(59, 66, 82),
        dark3: HexColor::rgb(67, 76, 94),
        dark4: HexColor::rgb(76, 86, 106),
        light1: HexColor::rgb(216, 222, 233),
        light2: HexColor::rgb(229, 233, 240),
        light3: HexColor::rgb(236, 239, 244),
        accent1: HexColor::rgb(143, 188, 187),
        accent2: HexColor::rgb(136, 192, 208),
        accent3: HexColor::rgb(129, 161, 193),
        accent4: HexColor::rgb(94, 129, 172),
        error: HexColor::rgb(191, 97, 106),
        danger: HexColor::rgb(208, 135, 112),
        warning: HexColor::rgb(235, 203, 139),
        success: HexColor::rgb(163, 190, 140),
        special: HexColor::rgb(180, 142, 173),
    };
}

use std::str::FromStr;

use crate::error::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InterfaceKind {
    V4,
    V6,
}

impl ToString for InterfaceKind {
    fn to_string(&self) -> String {
        match self {
            InterfaceKind::V4 => "v4".into(),
            InterfaceKind::V6 => "v6".into(),
        }
    }
}

// TODO replace with TryFrom
impl FromStr for InterfaceKind {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "v4" => Ok(Self::V4),
            "v6" => Ok(Self::V6),
            _ => Err(format!("unrecognised InterfaceKind, expected v4 or v6, got: {}", s).into()),
        }
    }
}

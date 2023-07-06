use std::net::IpAddr;
use std::str::FromStr;

use serde::{de, Deserialize, Serialize};

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

impl TryFrom<&str> for InterfaceKind {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "v4" => Ok(Self::V4),
            "v6" => Ok(Self::V6),
            s => bail!("unrecognised InterfaceKind, expected v4 or v6, got: {}", s),
        }
    }
}

/// This type is in the format of `interface[:type]`, where `interface` is the interface name, and
/// `type` is an optional part which is either `ipv4` or `ipv6`.
///
/// If `interface` is an empty string, then all interfaces are matched, for example:
/// - `vpn0:ipv4` will match ip4 addresses for the `vpn` interface
/// - `:ipv6`     will match all interfaces which have an ip6 address
// TODO: better filtering? don't match docker interfaces, or libvirtd ones, etc?
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceFilter {
    name: String,
    kind: Option<InterfaceKind>,
}

impl InterfaceFilter {
    pub fn new(name: impl AsRef<str>, kind: Option<InterfaceKind>) -> InterfaceFilter {
        InterfaceFilter {
            name: name.as_ref().to_owned(),
            kind,
        }
    }

    pub fn matches(&self, name: impl AsRef<str>, addr: &IpAddr) -> bool {
        let name_match = if self.name.is_empty() {
            true
        } else {
            self.name == name.as_ref()
        };

        match self.kind {
            None => name_match,
            Some(k) => {
                name_match
                    && match k {
                        InterfaceKind::V4 => addr.is_ipv4(),
                        InterfaceKind::V6 => addr.is_ipv6(),
                    }
            }
        }
    }
}

impl ToString for InterfaceFilter {
    fn to_string(&self) -> String {
        match self.kind {
            Some(kind) => format!("{}:{}", self.name, kind.to_string()),
            None => self.name.clone(),
        }
    }
}

impl FromStr for InterfaceFilter {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let d = ':';
        if !s.contains(d) {
            return Ok(InterfaceFilter::new(s, None));
        }

        // SAFETY: we just checked for the delimiter above
        let (name, kind) = s.split_once(d).unwrap();
        Ok(InterfaceFilter::new(name, Some(kind.try_into()?)))
    }
}

impl Serialize for InterfaceFilter {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for InterfaceFilter {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.parse::<InterfaceFilter>() {
            Ok(value) => Ok(value),
            Err(e) => Err(de::Error::custom(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    #[test]
    fn interface_filter_to_string() {
        use InterfaceFilter as F;

        assert_eq!(F::new("foo", None).to_string(), "foo");
        assert_eq!(F::new("bar", Some(InterfaceKind::V4)).to_string(), "bar:v4");
        assert_eq!(F::new("baz", Some(InterfaceKind::V6)).to_string(), "baz:v6");
        assert_eq!(F::new("", None).to_string(), "");
        assert_eq!(F::new("", Some(InterfaceKind::V4)).to_string(), ":v4");
        assert_eq!(F::new("", Some(InterfaceKind::V6)).to_string(), ":v6");
    }

    #[test]
    fn interface_filter_from_str() {
        use InterfaceFilter as F;

        let p = |s: &str| s.parse::<F>().unwrap();
        assert_eq!(p("foo"), F::new("foo", None));
        assert_eq!(p("bar:v4"), F::new("bar", Some(InterfaceKind::V4)));
        assert_eq!(p("baz:v6"), F::new("baz", Some(InterfaceKind::V6)));
        assert_eq!(p(""), F::new("", None));
        assert_eq!(p(":v4"), F::new("", Some(InterfaceKind::V4)));
        assert_eq!(p(":v6"), F::new("", Some(InterfaceKind::V6)));
    }

    #[test]
    fn interface_filter_ser() {
        let to_s = |i| serde_json::to_value(&i).unwrap();

        assert_eq!(to_s(InterfaceFilter::new("foo", None)), "foo");
        assert_eq!(
            to_s(InterfaceFilter::new("bar", Some(InterfaceKind::V4))),
            "bar:v4"
        );
        assert_eq!(
            to_s(InterfaceFilter::new("baz", Some(InterfaceKind::V6))),
            "baz:v6"
        );
        assert_eq!(to_s(InterfaceFilter::new("", None)), "");
        assert_eq!(
            to_s(InterfaceFilter::new("", Some(InterfaceKind::V4))),
            ":v4"
        );
        assert_eq!(
            to_s(InterfaceFilter::new("", Some(InterfaceKind::V6))),
            ":v6"
        );
    }

    #[test]
    fn interface_filter_de() {
        let from_s =
            |s: &str| match serde_json::from_value::<InterfaceFilter>(Value::String(s.into())) {
                Ok(x) => x,
                Err(e) => panic!("input: {}, error: {}", s, e),
            };

        assert_eq!(from_s("foo"), InterfaceFilter::new("foo", None));
        assert_eq!(
            from_s("bar:v4"),
            InterfaceFilter::new("bar", Some(InterfaceKind::V4))
        );
        assert_eq!(
            from_s("baz:v6"),
            InterfaceFilter::new("baz", Some(InterfaceKind::V6))
        );
        assert_eq!(from_s(""), InterfaceFilter::new("", None));
        assert_eq!(
            from_s(":v4"),
            InterfaceFilter::new("", Some(InterfaceKind::V4))
        );
        assert_eq!(
            from_s(":v6"),
            InterfaceFilter::new("", Some(InterfaceKind::V6))
        );
    }
}

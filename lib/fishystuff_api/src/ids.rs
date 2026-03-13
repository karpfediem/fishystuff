use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

pub type Timestamp = i64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[serde(transparent)]
pub struct PatchId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[serde(transparent)]
pub struct MapVersionId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[serde(transparent)]
pub struct TileSetId(pub String);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[serde(transparent)]
pub struct RgbKey(pub String);

impl Rgb {
    pub fn to_u32(self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | self.b as u32
    }

    pub fn from_u32(value: u32) -> Self {
        Self {
            r: ((value >> 16) & 0xff) as u8,
            g: ((value >> 8) & 0xff) as u8,
            b: (value & 0xff) as u8,
        }
    }

    pub fn key(self) -> RgbKey {
        RgbKey(format!("{},{},{}", self.r, self.g, self.b))
    }
}

impl FromStr for RgbKey {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut parts = input.split(',');
        let parse_part = |label: &str, value: Option<&str>| -> Result<u8, String> {
            let raw = value.ok_or_else(|| format!("rgb missing {label}"))?;
            raw.trim()
                .parse::<u8>()
                .map_err(|_| format!("invalid {label} component in rgb"))
        };
        let r = parse_part("red", parts.next())?;
        let g = parse_part("green", parts.next())?;
        let b = parse_part("blue", parts.next())?;
        if parts.next().is_some() {
            return Err("rgb has too many components".to_string());
        }
        Ok(RgbKey(format!("{},{},{}", r, g, b)))
    }
}

impl TryFrom<&str> for Rgb {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let key = RgbKey::from_str(value)?;
        key.as_rgb()
    }
}

impl RgbKey {
    pub fn as_rgb(&self) -> Result<Rgb, String> {
        let mut parts = self.0.split(',');
        let parse_part = |label: &str, value: Option<&str>| -> Result<u8, String> {
            let raw = value.ok_or_else(|| format!("rgb missing {label}"))?;
            raw.trim()
                .parse::<u8>()
                .map_err(|_| format!("invalid {label} component in rgb"))
        };
        let r = parse_part("red", parts.next())?;
        let g = parse_part("green", parts.next())?;
        let b = parse_part("blue", parts.next())?;
        if parts.next().is_some() {
            return Err("rgb has too many components".to_string());
        }
        Ok(Rgb { r, g, b })
    }

    pub fn to_u32(&self) -> Result<u32, String> {
        Ok(self.as_rgb()?.to_u32())
    }
}

impl fmt::Display for RgbKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

macro_rules! impl_string_id {
    ($ty:ty) => {
        impl From<String> for $ty {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $ty {
            fn from(value: &str) -> Self {
                Self(value.to_string())
            }
        }

        impl AsRef<str> for $ty {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

impl_string_id!(PatchId);
impl_string_id!(MapVersionId);
impl_string_id!(TileSetId);

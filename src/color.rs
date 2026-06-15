use std::{fmt::Display, str::FromStr};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseColorError {
    #[error("wrong hex length: {0}")]
    WrongHexLength(usize),
    #[error("invalid red component: {0}")]
    InvalidRedComponent(std::num::ParseIntError),
    #[error("invalid green component: {0}")]
    InvalidGreenComponent(std::num::ParseIntError),
    #[error("invalid blue component: {0}")]
    InvalidBlueComponent(std::num::ParseIntError),
    #[error("invalid alpha component: {0}")]
    InvalidAlphaComponent(std::num::ParseIntError),
}

#[derive(Debug, Clone)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl FromStr for Color {
    type Err = ParseColorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Color::hex(s)
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            r,
            g,
            b,
            a: u8::MAX,
        }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn hex(hex: &str) -> Result<Self, ParseColorError> {
        let hex = hex.trim().strip_prefix('#').unwrap_or(hex).trim();

        match hex.len() {
            6 | 8 => {}
            len => return Err(ParseColorError::WrongHexLength(len)),
        }

        let r = u8::from_str_radix(&hex[0..2], 16).map_err(ParseColorError::InvalidRedComponent)?;
        let g =
            u8::from_str_radix(&hex[2..4], 16).map_err(ParseColorError::InvalidGreenComponent)?;
        let b =
            u8::from_str_radix(&hex[4..6], 16).map_err(ParseColorError::InvalidBlueComponent)?;
        let a = if hex.len() == 8 {
            u8::from_str_radix(&hex[6..8], 16).map_err(ParseColorError::InvalidAlphaComponent)?
        } else {
            255
        };
        Ok(Self { r, g, b, a })
    }

    pub fn to_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
    }

    pub const fn get_bgra(&self) -> [u8; 4] {
        [self.b, self.g, self.r, self.a]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_6_digit_hex_without_hash() {
        let c = Color::from_str("ff0000").unwrap();
        assert_eq!(c.r, 0xff);
        assert_eq!(c.g, 0x00);
        assert_eq!(c.b, 0x00);
        assert_eq!(c.a, 0xff);
    }

    #[test]
    fn parses_6_digit_hex_with_hash() {
        let c = Color::from_str("#00ff00").unwrap();
        assert_eq!(c.r, 0x00);
        assert_eq!(c.g, 0xff);
        assert_eq!(c.b, 0x00);
        assert_eq!(c.a, 0xff);
    }

    #[test]
    fn parses_8_digit_hex_with_alpha() {
        let c = Color::from_str("#11223344").unwrap();
        assert_eq!(c.r, 0x11);
        assert_eq!(c.g, 0x22);
        assert_eq!(c.b, 0x33);
        assert_eq!(c.a, 0x44);
    }

    #[test]
    fn hex_roundtrip_to_hex() {
        let c = Color::rgb(10, 20, 30);
        let hex = c.to_hex();
        let parsed = Color::from_str(&hex).unwrap();

        assert_eq!(parsed.r, 10);
        assert_eq!(parsed.g, 20);
        assert_eq!(parsed.b, 30);
        assert_eq!(parsed.a, 255);
    }

    #[test]
    fn display_matches_to_hex() {
        let c = Color::rgba(1, 2, 3, 4);
        assert_eq!(format!("{}", c), c.to_hex());
    }

    #[test]
    fn invalid_hex_length() {
        let err = Color::from_str("#123").unwrap_err();

        match err {
            ParseColorError::WrongHexLength(len) => assert_eq!(len, 3),
            _ => panic!("expected WrongHexLength"),
        }
    }

    #[test]
    fn invalid_hex_characters() {
        let err = Color::from_str("zzzzzz").unwrap_err();

        match err {
            ParseColorError::InvalidRedComponent(_) => {}
            _ => panic!("expected InvalidRedComponent"),
        }
    }

    #[test]
    fn get_bgra_order_is_correct() {
        let c = Color::rgba(1, 2, 3, 4);
        assert_eq!(c.get_bgra(), [3, 2, 1, 4]);
    }

    #[test]
    fn whitespace_is_trimmed() {
        let c = Color::from_str("   #ff00ff   ").unwrap();
        assert_eq!(c.r, 0xff);
        assert_eq!(c.g, 0x00);
        assert_eq!(c.b, 0xff);
        assert_eq!(c.a, 0xff);
    }
}

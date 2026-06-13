use std::{fmt::Display, str::FromStr};

#[derive(Debug, Clone)]
pub struct Color {
    pub r: u8,
    pub b: u8,
    pub g: u8,
    pub a: u8,
}

impl FromStr for Color {
    type Err = anyhow::Error;

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
    pub const DEFAULT_ITEM_COLOR: Color = Color::rgb(255, 255, 255);

    pub const DEFAULT_BACKGROUND_COLOR: Color = Color::rgba(0, 0, 0, 200);

    pub const DEFAULT_PROMPT_COLOR: Color = Color::rgb(50, 255, 50);
    pub const DEFAULT_ARROW_COLOR: Color = Color::rgb(50, 255, 50);

    pub const DEFAULT_INPUT_COLOR: Color = Color::rgb(50, 50, 255);
    pub const DEFAULT_SELECTED_COLOR: Color = Color::rgb(50, 50, 255);

    pub const DEFAULT_EXTRA_TEXT_COLOR: Color = Color::rgb(255, 50, 50);

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

    // TODO: better error handling
    pub fn hex(hex: &str) -> anyhow::Result<Self> {
        let hex = match hex.strip_prefix('#') {
            Some(v) => v,
            None => hex,
        }
        .trim();

        let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
        let a = if hex.len() == 8 {
            u8::from_str_radix(&hex[6..8], 16).unwrap()
        } else {
            255
        };
        Ok(Self { r, g, b, a })
    }

    pub fn to_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
    }

    #[allow(unused)]
    pub const fn get_bgra(&self) -> [u8; 4] {
        [self.b, self.g, self.r, self.a]
    }

    #[allow(unused)]
    pub const fn get_bgr(&self) -> [u8; 3] {
        [self.b, self.g, self.r]
    }
}

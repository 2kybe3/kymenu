pub struct Color {
    pub r: u8,
    pub b: u8,
    pub g: u8,
    pub a: u8,
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

    pub fn hex(hex: &str) -> Self {
        let hex = match hex.strip_prefix('#') {
            Some(v) => v,
            None => hex,
        }
        .trim();
        tracing::info!("{hex}");

        let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
        let a = if hex.len() == 8 {
            u8::from_str_radix(&hex[6..8], 16).unwrap()
        } else {
            255
        };
        Self { r, g, b, a }
    }

    pub const fn get_bgra(&self) -> [u8; 4] {
        [self.b, self.g, self.r, self.a]
    }

    #[allow(unused)]
    pub const fn get_bgr(&self) -> [u8; 3] {
        [self.b, self.g, self.r]
    }
}

use std::{fs::OpenOptions, io::Read, path::Path};

use ab_glyph::{Font, FontVec, PxScale, PxScaleFont, ScaleFont};
use anyhow::Context;
use fontconfig::Fontconfig;

use crate::color;

pub struct TextFont(pub FontVec);

impl TextFont {
    pub fn new(family: &str, style: Option<&str>) -> Self {
        match Self::load_font(family, style) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    "failed to load font (family: '{family}', style: '{style:?}'): {e}\nusing fallback font"
                );
                match Self::new_fallback_font() {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::error!("failed to load fallback font: {e}");
                        std::process::exit(1);
                    }
                }
            }
        }
    }

    fn from_path(path: &Path) -> anyhow::Result<Self> {
        let mut file = OpenOptions::new()
            .read(true)
            .open(path)
            .context(format!("failed to open font file '{}'", path.display()))?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .context(format!("failed to read font file '{}'", path.display()))?;

        let font = FontVec::try_from_vec(buffer)
            .context(format!("failed to parse font file '{}'", path.display()))?;

        Ok(Self(font))
    }

    fn new_fallback_font() -> anyhow::Result<Self> {
        let font =
            FontVec::try_from_vec(include_bytes!("../assets/font/FiraCode-Regular.ttf").to_vec())
                .context("failed to parse fallback font file")?;
        Ok(Self(font))
    }

    fn load_font(family: &str, style: Option<&str>) -> anyhow::Result<Self> {
        let fc = Fontconfig::new().context("failed to create Fontconfig instance")?;
        let font = fc.find(family, style).context("font not found")?;
        Self::from_path(font.path.as_path())
    }

    fn scaled(&self, font_size: f32) -> PxScaleFont<&FontVec> {
        self.0.as_scaled(PxScale::from(font_size))
    }
}

pub struct TextRenderer(TextFont);

impl TextRenderer {
    pub fn new(font: TextFont) -> Self {
        Self(font)
    }

    pub fn text_width(&self, text: &str, font_size: f32) -> u32 {
        if text.is_empty() {
            return 0;
        }

        let font = self.0.scaled(font_size);

        let mut x = 0.0;
        let mut prev = None;

        for c in text.chars() {
            let glyph_id = font.glyph_id(c);

            if let Some(prev_id) = prev {
                x += font.kern(prev_id, glyph_id);
            }

            x += font.h_advance(glyph_id);
            prev = Some(glyph_id);
        }

        x as u32
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_text(
        &self,
        text: &str,
        font_size: f32,
        start_x: u32,
        color: &color::Color,
        buffer: &mut [u8],
        height: u32,
        width: u32,
    ) {
        if text.is_empty() {
            return;
        }

        let font = self.0.scaled(font_size);

        let text_height = font.ascent() - font.descent();
        let baseline = ((height as f32 - text_height) / 2.0) + font.ascent();

        let mut x = start_x as f32;

        for c in text.chars() {
            let glyph_id = font.glyph_id(c);
            let glyph =
                glyph_id.with_scale_and_position(font.scale(), ab_glyph::point(x, baseline));

            if let Some(outlined) = font.outline_glyph(glyph) {
                let bounds = outlined.px_bounds();

                outlined.draw(|gx, gy, coverage| {
                    let pixel_y = bounds.min.y + gy as f32;
                    let pixel_x = bounds.min.x + gx as f32;

                    if pixel_x < 0.0
                        || pixel_y < 0.0
                        || pixel_x as u32 >= width
                        || pixel_y as u32 >= height
                    {
                        return;
                    }

                    let base = (pixel_y * width as f32 + pixel_x) as usize * 4;
                    if base + crate::COLOR_SIZE as usize >= buffer.len() {
                        return;
                    }

                    let a = (coverage * 255.0).round() as u32;
                    let inv_a = 255 - a;

                    let bg_b = buffer[base] as u32;
                    let bg_g = buffer[base + 1] as u32;
                    let bg_r = buffer[base + 2] as u32;
                    let bg_a = buffer[base + 3] as u32;

                    let blended_r = ((color.r as u32 * a + bg_r * inv_a) / 255) as u8;
                    let blended_g = ((color.g as u32 * a + bg_g * inv_a) / 255) as u8;
                    let blended_b = ((color.b as u32 * a + bg_b * inv_a) / 255) as u8;

                    buffer[base] = blended_b;
                    buffer[base + 1] = blended_g;
                    buffer[base + 2] = blended_r;
                    buffer[base + 3] = bg_a as u8;
                });
            }

            x += font.h_advance(glyph_id);
        }
    }
}

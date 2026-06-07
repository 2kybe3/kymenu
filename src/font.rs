use std::{fs::OpenOptions, io::Read};

use ab_glyph::{Font, FontRef, PxScale, ScaleFont, point};
use anyhow::Context;
use fontconfig::Fontconfig;

use crate::color;

pub fn get_font(family: &str, style: Option<&str>) -> anyhow::Result<fontconfig::Font> {
    let fc = Fontconfig::new().context("failed to create Fontconfig instance")?;
    let font = fc
        .find(family, style)
        .context("fon't {family} with style {style:?} not found")?;
    Ok(font)
}

pub fn load_font(font: fontconfig::Font) -> anyhow::Result<TextRender> {
    let mut file = OpenOptions::new().read(true).open(font.path)?;

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    Ok(TextRender(buffer))
}

pub struct TextRender(Vec<u8>);

impl TextRender {
    fn get_font_ref(&self) -> FontRef<'_> {
        FontRef::try_from_slice(&self.0).unwrap()
    }

    pub fn text_width(&self, text: &str, font_size: f32) -> u32 {
        if text.is_empty() {
            return 0;
        }

        let font_ref = self.get_font_ref();

        let scale = PxScale::from(font_size);
        let scaled_font = font_ref.as_scaled(scale);

        let mut x = 0.0;
        let mut prev = None;

        for c in text.chars() {
            let glyph_id = scaled_font.glyph_id(c);

            if let Some(prev_id) = prev {
                x += scaled_font.kern(prev_id, glyph_id);
            }

            x += scaled_font.h_advance(glyph_id);
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

        let font_ref = self.get_font_ref();

        let scale = PxScale::from(font_size);
        let scaled_font = font_ref.as_scaled(scale);

        let text_height = scaled_font.ascent() - scaled_font.descent();
        let baseline = ((height as f32 - text_height) / 2.0) + scaled_font.ascent();

        let mut x = start_x as f32;

        for c in text.chars() {
            let glyph_id = scaled_font.glyph_id(c);
            let glyph = glyph_id.with_scale_and_position(scale, point(x, baseline));

            if let Some(outlined) = scaled_font.outline_glyph(glyph) {
                let bounds = outlined.px_bounds();

                outlined.draw(|gx, gy, coverage| {
                    let pixel_y = (bounds.min.y as u32 + gy) as i32;
                    let pixel_x = (bounds.min.x as u32 + gx) as i32;

                    if pixel_x < 0
                        || pixel_y < 0
                        || pixel_x as u32 >= width
                        || pixel_y as u32 >= height
                    {
                        return;
                    }

                    let base = (pixel_y as usize * width as usize + pixel_x as usize) * 4;
                    if base + 3 >= buffer.len() {
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

            let advance = scaled_font.h_advance(glyph_id);
            x += advance;
        }
    }
}

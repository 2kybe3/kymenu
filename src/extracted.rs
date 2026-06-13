use crate::{cli::Cli, color::Color};

#[derive(Debug)]
pub struct Extracted {
    // General
    pub font_family: String,
    pub font_style: Option<String>,
    pub height: u32,
    pub prompt: String,
    pub end_arrow: String,
    pub end_arrow_more: String,
    pub start_arrow: String,
    pub start_arrow_more: String,
    pub end_margin: u32,
    pub start_margin: u32,
    pub text_margin: u32,
    pub arrow_margin: u32,
    pub font_size: f32,
    pub default_bin_start_x: u32,
    pub bin_start_margin: u32,
    pub path_launcher: bool,
    pub json_out: bool,
    pub json_in: bool,

    // Colors
    pub item_color: Color,
    pub background_color: Color,
    pub prompt_color: Color,
    pub arrow_color: Color,
    pub input_color: Color,
    pub selected_color: Color,
    pub extra_text_color: Color,
}

impl Cli {
    pub fn extract(self) -> Extracted {
        Extracted {
            // General
            font_family: self.font_family,
            font_style: self.font_style,
            height: self.height,
            prompt: format!("{} ", self.prompt),
            end_arrow: self.end_arrow,
            end_arrow_more: self.end_arrow_more,
            start_arrow: self.start_arrow,
            start_arrow_more: self.start_arrow_more,
            end_margin: self.end_margin,
            start_margin: self.start_margin,
            text_margin: self.text_margin,
            arrow_margin: self.arrow_margin,
            font_size: self.font_size,
            default_bin_start_x: self.default_bin_start,
            bin_start_margin: self.bin_start_margin,
            path_launcher: self.path_launcher,
            json_out: self.json_out,
            json_in: self.json_in,

            // Colors
            item_color: self.item_color,
            background_color: self.background_color,
            prompt_color: self.prompt_color,
            arrow_color: self.arrow_color,
            input_color: self.input_color,
            selected_color: self.selected_color,
            extra_text_color: self.extra_text_color,
        }
    }
}

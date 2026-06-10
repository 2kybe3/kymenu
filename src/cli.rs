use crate::color::Color;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version)]
pub struct Cli {
    // General
    #[arg(long)]
    font_family: Option<String>,
    #[arg(long)]
    font_style: Option<String>,
    #[arg(long)]
    height: Option<u32>,
    #[arg(long)]
    prompt: Option<String>,
    #[arg(long)]
    end_arrow: Option<String>,
    #[arg(long)]
    start_arrow: Option<String>,
    #[arg(long)]
    end_margin: Option<u32>,
    #[arg(long)]
    start_margin: Option<u32>,
    #[arg(long)]
    text_margin: Option<u32>,
    #[arg(long)]
    arrow_margin: Option<u32>,
    #[arg(long)]
    font_size: Option<f32>,
    #[arg(long)]
    default_bin_start: Option<u32>,
    #[arg(long)]
    bin_start_margin: Option<u32>,

    // Colors
    #[arg(long)]
    item_color: Option<String>,
    #[arg(long)]
    background_color: Option<String>,
    #[arg(long)]
    prompt_color: Option<String>,
    #[arg(long)]
    arrow_color: Option<String>,
    #[arg(long)]
    input_color: Option<String>,
    #[arg(long)]
    selected_color: Option<String>,
    #[arg(long)]
    extra_text_color: Option<String>,
}

pub struct Extracted<'a> {
    // General
    pub font_family: &'a str,
    pub font_style: Option<&'a str>,
    pub height: u32,
    pub prompt: &'a str,
    pub end_arrow: &'a str,
    pub start_arrow: &'a str,
    pub end_margin: u32,
    pub start_margin: u32,
    pub text_margin: u32,
    pub arrow_margin: u32,
    pub font_size: f32,
    pub default_bin_start_x: u32,
    pub bin_start_offset: u32,

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
    pub fn extract(&self) -> Extracted<'_> {
        Extracted {
            // General
            font_family: self.font_family.as_deref().unwrap_or("monospace"),
            font_style: self.font_style.as_deref(),
            height: self.height.unwrap_or(20),
            prompt: self.prompt.as_deref().unwrap_or(">> "),
            end_arrow: self.end_arrow.as_deref().unwrap_or(">"),
            start_arrow: self.start_arrow.as_deref().unwrap_or("<"),
            end_margin: self.end_margin.unwrap_or(0),
            start_margin: self.start_margin.unwrap_or(0),
            text_margin: self.text_margin.unwrap_or(5),
            arrow_margin: self.arrow_margin.unwrap_or(8),
            font_size: self.font_size.unwrap_or(16.0),
            default_bin_start_x: self.default_bin_start.unwrap_or(200),
            bin_start_offset: self.bin_start_margin.unwrap_or(75),

            // Colors
            item_color: self
                .item_color
                .as_ref()
                .map(|c| Color::hex(c))
                .unwrap_or(Color::DEFAULT_ITEM_COLOR),
            background_color: self
                .background_color
                .as_ref()
                .map(|c| Color::hex(c))
                .unwrap_or(Color::DEFAULT_BACKGROUND_COLOR),
            prompt_color: self
                .prompt_color
                .as_ref()
                .map(|c| Color::hex(c))
                .unwrap_or(Color::DEFAULT_PROMPT_COLOR),
            arrow_color: self
                .arrow_color
                .as_ref()
                .map(|c| Color::hex(c))
                .unwrap_or(Color::DEFAULT_ARROW_COLOR),
            input_color: self
                .input_color
                .as_ref()
                .map(|c| Color::hex(c))
                .unwrap_or(Color::DEFAULT_INPUT_COLOR),
            selected_color: self
                .selected_color
                .as_ref()
                .map(|c| Color::hex(c))
                .unwrap_or(Color::DEFAULT_SELECTED_COLOR),
            extra_text_color: self
                .extra_text_color
                .as_ref()
                .map(|c| Color::hex(c))
                .unwrap_or(Color::DEFAULT_EXTRA_TEXT_COLOR),
        }
    }
}

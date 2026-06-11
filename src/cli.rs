use crate::color::Color;
use clap::{Parser, Subcommand};

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
    end_arrow_more: Option<String>,
    #[arg(long)]
    start_arrow: Option<String>,
    #[arg(long)]
    start_arrow_more: Option<String>,
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

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum Commands {
    GenerateZshCompletion,
    GenerateFishCompletion,
    GenerateBashCompletion,
}

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
    pub fn extract(&self) -> Extracted {
        Extracted {
            // General
            font_family: self
                .font_family
                .as_deref()
                .unwrap_or("monospace")
                .to_string(),
            font_style: self.font_style.as_deref().map(|x| x.to_string()),
            height: self.height.unwrap_or(20),
            prompt: self.prompt.as_deref().unwrap_or(">> ").to_string(),
            end_arrow: self.end_arrow.as_deref().unwrap_or(">").to_string(),
            end_arrow_more: self.end_arrow_more.as_deref().unwrap_or(">>").to_string(),
            start_arrow: self.start_arrow.as_deref().unwrap_or("<").to_string(),
            start_arrow_more: self.start_arrow_more.as_deref().unwrap_or("<<").to_string(),
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

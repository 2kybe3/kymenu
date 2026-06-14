use clap::{ArgAction, Parser};

use crate::color::Color;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Cli {
    // General
    #[arg(
        long,
        default_value = "monospace",
        help = "Font family used for rendering text"
    )]
    pub(crate) font_family: String,
    #[arg(long, help = "Font style (e.g. 'bold', 'italic', 'bold italic')")]
    pub(crate) font_style: Option<String>,
    #[arg(long, default_value_t = 20, help = "Height of the menu")]
    pub(crate) height: u32,
    #[arg(
        short,
        long,
        default_value = ">>",
        help = "Prompt symbol shown before user input",
        value_parser = |str: &str| Ok::<_, std::io::Error>(format!("{str} ")),
    )]
    pub(crate) prompt: String,
    #[arg(long, default_value = ">", help = "Arrow shown at the end of a line")]
    pub(crate) end_arrow: String,
    #[arg(
        long,
        default_value = ">>",
        help = "Arrow shown when there is more content after the line"
    )]
    pub(crate) end_arrow_more: String,
    #[arg(long, default_value = "<", help = "Arrow shown at the start of a line")]
    pub(crate) start_arrow: String,
    #[arg(
        long,
        default_value = "<<",
        help = "Arrow shown when there is more content before the line"
    )]
    pub(crate) start_arrow_more: String,
    #[arg(long, default_value_t = 0, help = "Right margin after the end arrow")]
    pub(crate) end_margin: u32,
    #[arg(long, default_value_t = 0, help = "Left margin before the start arrow")]
    pub(crate) start_margin: u32,
    #[arg(
        long,
        default_value_t = 15,
        help = "Horizontal margin around the text content"
    )]
    pub(crate) text_margin: u32,
    #[arg(long, default_value_t = 8, help = "Margin between arrows and text")]
    pub(crate) arrow_margin: u32,
    #[arg(long, default_value_t = 16.0, help = "Font size in pixels")]
    pub(crate) font_size: f32,
    #[arg(
        long,
        default_value_t = 200,
        help = "Default X position where the item list starts"
    )]
    pub(crate) default_bin_start: u32,
    #[arg(
        long,
        default_value_t = 75,
        help = "Margin after the input where the item list starts"
    )]
    pub(crate) bin_start_margin: u32,
    #[arg(
        long,
        num_args = 0..=1,
        require_equals = true,
        action = ArgAction::Set,
        default_missing_value = "true",
        default_value_t = false,
        help = "Lists all bins installed and launches the one you select"
    )]
    pub(crate) path_launcher: bool,
    #[arg(
        long,
        num_args = 0..=1,
        require_equals = true,
        action = ArgAction::Set,
        default_value_t = false,
        default_missing_value = "true",
    )]
    pub(crate) json_out: bool,
    #[arg(
        long,
        num_args = 0..=1,
        require_equals = true,
        action = ArgAction::Set,
        default_value_t = false,
        default_missing_value = "true",
    )]
    pub(crate) json_in: bool,
    #[arg(
        long,
        num_args = 0..=1,
        require_equals = true,
        action = ArgAction::Set,
        default_value_t = false,
        default_missing_value = "true",
        help = "Show only the input prompt and return the input after pressing enter"
    )]
    pub(crate) input: bool,

    // Colors
    #[arg(long, default_value_t = Color::DEFAULT_ITEM_COLOR)]
    pub(crate) item_color: Color,
    #[arg(long, default_value_t = Color::DEFAULT_BACKGROUND_COLOR)]
    pub(crate) background_color: Color,
    #[arg(long, default_value_t = Color::DEFAULT_PROMPT_COLOR)]
    pub(crate) prompt_color: Color,
    #[arg(long, default_value_t = Color::DEFAULT_ARROW_COLOR)]
    pub(crate) arrow_color: Color,
    #[arg(long, default_value_t = Color::DEFAULT_INPUT_COLOR)]
    pub(crate) input_color: Color,
    #[arg(long, default_value_t = Color::DEFAULT_SELECTED_COLOR)]
    pub(crate) selected_color: Color,
    #[arg(long, default_value_t = Color::DEFAULT_EXTRA_TEXT_COLOR)]
    pub(crate) extra_text_color: Color,
}

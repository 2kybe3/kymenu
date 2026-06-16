pub mod buffer;
pub mod input;
pub mod output;
pub mod wayland_globals;
pub mod xkb;

use std::time::Instant;

use xkeysym::KeyCode;

use crate::{
    appdata::{
        buffer::Buffer,
        input::{Input, InputItems},
        output::Output,
        wayland_globals::{Registries, WaylandGlobals},
        xkb::Xkb,
    },
    cli::Cli,
};

#[derive(Debug)]
pub struct RepeatState {
    pub key: KeyCode,
    pub started_at: Instant,
    pub last_repeat: Instant,
}

#[derive(Debug)]
pub struct RepeatConfig {
    pub rate: u32,
    pub delay: u32,
}

#[derive(Debug)]
pub struct AppData {
    pub wayland_globals: WaylandGlobals,
    pub registries: Option<Registries>,
    pub output: Option<Output>,
    pub buffer: Option<Buffer>,

    pub repeat_config: Option<RepeatConfig>,
    pub repeat_state: Option<RepeatState>,

    pub xkb: Option<Xkb>,

    pub configured: bool,
    pub callback_done: bool,

    pub inp: Input,

    pub cli: Cli,
}

impl AppData {
    pub fn new(cli: Cli) -> anyhow::Result<Self> {
        Ok(Self {
            wayland_globals: WaylandGlobals::default(),
            registries: None,
            output: None,
            buffer: None,

            repeat_config: None,
            repeat_state: None,

            xkb: None,

            configured: false,
            callback_done: false,

            inp: Input::new(InputItems::new(&cli))?,

            cli,
        })
    }
}

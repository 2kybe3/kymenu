pub mod buffer;
pub mod input;
pub mod output;
pub mod repeat;
pub mod wayland_globals;
pub mod xkb;

use crate::{
    appdata::{
        buffer::Buffer,
        input::{Input, InputItems},
        output::Output,
        repeat::Repeat,
        wayland_globals::{Registries, WaylandGlobals},
        xkb::Xkb,
    },
    cli::Cli,
};

#[derive(Debug)]
pub struct AppData {
    pub wayland_globals: WaylandGlobals,
    pub registries: Option<Registries>,
    pub output: Option<Output>,
    pub buffer: Option<Buffer>,

    pub repeat: Repeat,

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

            repeat: Repeat::default(),

            xkb: None,

            configured: false,
            callback_done: false,

            inp: Input::new(InputItems::new(&cli))?,

            cli,
        })
    }
}

pub mod buffer;
pub mod input;
pub mod output;
pub mod wayland_globals;

use std::{os::unix::process::CommandExt, process::Command, time::Instant};

use xkbcommon::xkb::{self, keysyms};
use xkeysym::KeyCode;

use crate::{
    appdata::{
        buffer::Buffer,
        input::{Input, InputItems},
        output::Output,
        wayland_globals::{Registries, WaylandGlobals},
    },
    cli::Cli,
};

pub struct Xkb(pub xkb::State);

impl std::fmt::Debug for Xkb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XKB").finish()
    }
}

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

    pub fn handle_key(&mut self, key: KeyCode) {
        let Some(xkb) = self.xkb.as_ref() else {
            tracing::error!("Tried to handle a key without xkb state being initialized");
            return;
        };

        let sym = xkb.0.key_get_one_sym(key);

        let execute = |index: usize| {
            if self.cli.input {
                println!("{}", self.inp.input());
                std::process::exit(0)
            }

            let result = match self.inp.filtered_inputs().get(index) {
                Some(v) => v.clone(),
                None => std::process::exit(0),
            };

            if let serde_json::Value::String(raw) = result.raw()
                && self.cli.path_launcher
            {
                let _ = Command::new(raw).exec();
                std::process::exit(1)
            } else if self.cli.json_out {
                match serde_json::to_string(&result) {
                    Ok(v) => {
                        print!("{v}");
                        std::process::exit(0)
                    }
                    Err(e) => {
                        tracing::error!("failed to convert result into json: {e}");
                        std::process::exit(1);
                    }
                };
            } else {
                match result.raw() {
                    serde_json::Value::String(s) => println!("{s}"),
                    _ => println!("{}", result.raw()),
                };
                std::process::exit(0)
            }
        };

        match sym.into() {
            keysyms::KEY_Return => execute(self.inp.selected_index() as usize),
            keysyms::KEY_BackSpace => self.inp.pop(),
            keysyms::KEY_Escape => std::process::exit(0),
            keysyms::KEY_Right => self.inp.move_right(),
            keysyms::KEY_Left => self.inp.move_left(),

            _ => {
                let alt_pressed = xkb.0.mod_name_is_active("Mod1", xkb::STATE_MODS_EFFECTIVE);

                let s = xkb.0.key_get_utf8(key);

                if alt_pressed && let Ok(digit) = s.parse::<u8>() {
                    let mapped = if digit == 0 { 9 } else { digit - 1 };

                    execute(self.inp.selected_index() as usize + mapped as usize)
                } else {
                    self.inp.push(&xkb.0.key_get_utf8(key));
                }
            }
        }
    }
}

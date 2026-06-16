use std::os::unix::process::CommandExt;

use xkbcommon::xkb::keysyms;

use crate::{appdata::input::Input, cli::Cli};

pub struct Xkb(pub xkbcommon::xkb::State);

impl Xkb {
    pub fn handle_key(&mut self, key: xkeysym::KeyCode, cli: &Cli, inp: &mut Input) {
        let sym = self.0.key_get_one_sym(key);

        let execute = |index: usize| {
            if cli.input {
                println!("{}", inp.input());
                std::process::exit(0);
            }

            let Some(result) = inp.filtered_inputs().get(index) else {
                std::process::exit(0);
            };

            if cli.path_launcher {
                match result.raw() {
                    serde_json::Value::String(raw) => {
                        let _ = std::process::Command::new(raw).exec();
                        std::process::exit(0);
                    }
                    _ => {
                        tracing::error!("path_launcher but raw is not a string");
                        std::process::exit(1);
                    }
                }
            }

            if cli.json_out {
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
            }

            match result.raw() {
                serde_json::Value::String(s) => println!("{s}"),
                v => println!("{}", v),
            };

            std::process::exit(0)
        };

        match sym.into() {
            keysyms::KEY_Return => execute(inp.selected_index() as usize),
            keysyms::KEY_BackSpace => inp.pop(),
            keysyms::KEY_Escape => std::process::exit(0),
            keysyms::KEY_Right => inp.move_right(),
            keysyms::KEY_Left => inp.move_left(),

            _ => {
                let alt_pressed = self
                    .0
                    .mod_name_is_active("Mod1", xkbcommon::xkb::STATE_MODS_EFFECTIVE);

                let s = self.0.key_get_utf8(key);

                if alt_pressed && let Ok(digit) = s.parse::<u8>() {
                    let mapped = if digit == 0 { 9 } else { digit - 1 };

                    execute(inp.selected_index() as usize + mapped as usize)
                } else {
                    inp.push(&self.0.key_get_utf8(key));
                }
            }
        }
    }
}

impl std::fmt::Debug for Xkb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XKB").finish()
    }
}

use std::{
    env, fs,
    io::{self, IsTerminal, Read},
    os::unix::{fs::PermissionsExt, process::CommandExt},
    process::Command,
    time::Instant,
};

use anyhow::Context;
use memfd::Memfd;
use memmap2::MmapMut;
use serde::{Deserialize, Serialize};
use wayland_client::{
    QueueHandle,
    protocol::{wl_compositor, wl_registry::WlRegistry, wl_seat, wl_shm},
};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1;
use xkbcommon::xkb::{self, keysyms};
use xkeysym::KeyCode;

use crate::cli::Cli;

#[derive(Default, Debug)]
pub struct WaylandGlobal {
    name: u32,
    version: u32,
}

impl WaylandGlobal {
    pub fn new(name: u32, version: u32) -> Self {
        Self { name, version }
    }
}

#[derive(Default, Debug)]
pub struct WaylandGlobals {
    pub compositor: Option<WaylandGlobal>,
    pub layer_shell: Option<WaylandGlobal>,
    pub wl_seat: Option<WaylandGlobal>,
    pub shm: Option<WaylandGlobal>,
}

impl WaylandGlobals {
    pub fn bind_registries(&self, registry: &WlRegistry, qh: &QueueHandle<AppData>) -> Registries {
        let shm = if let Some(shm) = &self.shm {
            registry.bind::<wl_shm::WlShm, _, _>(shm.name, shm.version, qh, ())
        } else {
            panic!("No shared memory support");
        };

        let compositor = if let Some(compositor) = &self.compositor {
            registry.bind::<wl_compositor::WlCompositor, _, _>(
                compositor.name,
                compositor.version,
                qh,
                (),
            )
        } else {
            panic!("No compositor");
        };

        let layer_shell = if let Some(layer_shell) = &self.layer_shell {
            registry.bind::<zwlr_layer_shell_v1::ZwlrLayerShellV1, _, _>(
                layer_shell.name,
                layer_shell.version,
                qh,
                (),
            )
        } else {
            panic!("No layer shell");
        };

        let seat = if let Some(wl_seat) = &self.wl_seat {
            registry.bind::<wl_seat::WlSeat, _, _>(wl_seat.name, wl_seat.version, qh, ())
        } else {
            panic!("No Seat");
        };

        Registries {
            shm,
            seat,
            compositor,
            layer_shell,
        }
    }
}

#[derive(Default, Debug)]
pub struct Output {
    pub width: u32,
    pub height: u32,
}

#[derive(Default, Debug)]
pub struct Input {
    pub dirty: bool,

    input: String,

    inputs: InputItems,
    filtered_inputs: Vec<InputItem>,

    selected_index: u32,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct InputItems(pub Vec<InputItem>);

impl InputItems {
    pub fn new(cli: &Cli) -> Self {
        if cli.input {
            Self(vec![])
        } else if cli.path_launcher {
            Self::from_path()
        } else if cli.json_in {
            Self::from_json_in()
        } else {
            Self::from_input()
        }
    }

    fn get_stdin() -> String {
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .context("failed to read from stdin")
            .unwrap();

        input
    }

    fn from_input() -> InputItems {
        if io::stdin().is_terminal() {
            tracing::warn!("expected input from a pipe, you might wanna run --path-launcher");
            std::process::exit(1);
        }

        InputItems(
            Self::get_stdin()
                .lines()
                .map(|v| InputItem::new(v.to_string(), serde_json::Value::String(v.to_string())))
                .collect(),
        )
    }

    fn from_json_in() -> InputItems {
        if io::stdin().is_terminal() {
            tracing::warn!(
                r#"expected input from a pipe in form of [{{"display": "Name", "raw": 69}}]"#
            );

            std::process::exit(1);
        }

        let input = Self::get_stdin();

        serde_json::from_str(&input)
            .context("failed to parse input")
            .unwrap()
    }

    fn from_path() -> InputItems {
        let mut bins = Vec::new();

        let path = match env::var("PATH") {
            Ok(v) => v,
            Err(_) => {
                tracing::error!("No PATH set");
                std::process::exit(1);
            }
        };
        for dir in path.split(':') {
            let entries = match fs::read_dir(dir) {
                Ok(entries) => entries,
                Err(_) => continue,
            };

            for entry in entries {
                let entry = match entry {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::warn!("{e:?}");
                        continue;
                    }
                };
                let path = entry.path();

                if path.is_file() {
                    let metadata = match fs::metadata(&path) {
                        Ok(v) => v,
                        Err(e) => {
                            tracing::warn!("{e:?}");
                            continue;
                        }
                    };
                    let permissions = metadata.permissions();
                    if permissions.mode() & 0o111 != 0
                        && let Some(name) = entry.file_name().to_str()
                    {
                        bins.push(InputItem::new(
                            name.to_owned(),
                            serde_json::Value::String(path.to_str().unwrap().to_string()),
                        ));
                    }
                }
            }
        }

        InputItems(bins)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InputItem {
    display: String,
    raw: serde_json::Value,
}

impl InputItem {
    pub fn new(display: String, raw: serde_json::Value) -> Self {
        Self { display, raw }
    }

    pub fn display(&self) -> &str {
        &self.display
    }
}

impl Input {
    pub fn new(inputs: InputItems) -> anyhow::Result<Self> {
        let mut new = Self {
            dirty: true,

            input: String::new(),

            inputs,
            filtered_inputs: vec![],

            selected_index: 0,
        };
        new.update_bins();
        Ok(new)
    }

    pub fn pop(&mut self) {
        if self.input().is_empty() {
            return;
        }

        self.input.pop();
        self.selected_index = 0;
        self.update_bins();

        self.dirty = true;
    }

    pub fn push(&mut self, str: &str) {
        if str.is_empty() {
            return;
        }

        self.input.push_str(str);
        self.selected_index = 0;
        self.update_bins();

        self.dirty = true;
    }

    pub fn move_left(&mut self) {
        let old = self.selected_index();

        self.selected_index = self.selected_index().saturating_sub(1);

        if old != self.selected_index() {
            self.dirty = true;
        }
    }

    pub fn move_right(&mut self) {
        let old = self.selected_index();

        let max_index = self.filtered_inputs().len().saturating_sub(1) as u32;
        self.selected_index = (self.selected_index() + 1).min(max_index);

        if old != self.selected_index() {
            self.dirty = true;
        }
    }

    pub fn update_bins(&mut self) {
        let input = self.input.to_lowercase();

        let mut bins: Vec<(InputItem, String)> = self
            .inputs
            .0
            .iter()
            .filter(|s| {
                if input.is_empty() {
                    true
                } else {
                    self.input.is_empty() || s.display.contains(&self.input)
                }
            })
            .map(|s| (s.clone(), s.display.to_lowercase()))
            .collect();

        bins.sort_by(|a, b| {
            let score = |s: &str| {
                if !input.is_empty() && s.starts_with(&input) {
                    0
                } else if !input.is_empty() && s.contains(&input) {
                    1
                } else {
                    2
                }
            };

            score(&a.1)
                .cmp(&score(&b.1))
                .then_with(|| a.0.display.cmp(&b.0.display))
        });

        let bins = bins.into_iter().map(|(orig, _)| orig).collect();

        self.filtered_inputs = bins;
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn filtered_inputs(&self) -> &[InputItem] {
        &self.filtered_inputs
    }

    pub fn selected_index(&self) -> u32 {
        self.selected_index
    }
}

pub struct Xkb(pub xkb::State);

impl std::fmt::Debug for Xkb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XKB").finish()
    }
}

#[derive(Debug)]
pub struct RepeatState {
    pub key: Option<KeyCode>,
    pub started_at: Instant,
    pub last_repeat: Instant,
    pub rate: i32,
    pub delay: i32,
}

#[derive(Debug)]
pub struct Registries {
    pub shm: wl_shm::WlShm,
    pub seat: wl_seat::WlSeat,
    pub compositor: wl_compositor::WlCompositor,
    pub layer_shell: zwlr_layer_shell_v1::ZwlrLayerShellV1,
}

#[derive(Debug)]
pub struct AppData {
    pub repeat: Option<RepeatState>,
    pub wayland_globals: WaylandGlobals,
    pub output: Option<Output>,
    pub xkb: Option<Xkb>,

    pub configured: bool,
    pub callback_done: bool,

    pub buffer_memfd: Option<Memfd>,
    pub buffer_mmap: Option<MmapMut>,

    pub inp: Input,

    pub cli: Cli,
}

impl AppData {
    pub fn new(cli: Cli, inputs: InputItems) -> anyhow::Result<Self> {
        Ok(Self {
            repeat: None,
            wayland_globals: WaylandGlobals::default(),
            output: None,
            xkb: None,

            configured: false,
            callback_done: false,

            buffer_memfd: None,
            buffer_mmap: None,

            inp: Input::new(inputs)?,

            cli,
        })
    }

    pub fn handle_key(&mut self, key: KeyCode) {
        let xkb = self.xkb.as_ref().unwrap();

        let sym = xkb.0.key_get_one_sym(key);

        let execute = |index: usize| {
            if self.cli.input {
                println!("{}", self.inp.input);
                std::process::exit(0)
            }

            let result = self.inp.filtered_inputs().get(index).unwrap().clone();

            if let serde_json::Value::String(ref raw) = result.raw
                && self.cli.path_launcher
            {
                let _ = Command::new(raw).exec();
                std::process::exit(1)
            } else if self.cli.json_out {
                println!("{}", serde_json::to_string(&result).unwrap());
                std::process::exit(0)
            } else {
                match result.raw {
                    serde_json::Value::String(ref s) => println!("{s}"),
                    _ => println!("{}", result.raw),
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
                let xkb_state = &self.xkb.as_ref().unwrap().0;

                let alt_pressed = xkb_state.mod_name_is_active("Mod1", xkb::STATE_MODS_EFFECTIVE);

                let s = xkb_state.key_get_utf8(key);

                if alt_pressed && let Ok(digit) = s.parse::<u8>() {
                    let mapped = if digit == 0 { 9 } else { digit - 1 };

                    execute(self.inp.selected_index() as usize + mapped as usize)
                } else {
                    self.inp
                        .push(&self.xkb.as_ref().unwrap().0.key_get_utf8(key));
                }
            }
        }
    }
}

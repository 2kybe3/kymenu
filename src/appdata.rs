use std::{os::unix::process::CommandExt, process::Command, time::Instant};

use memfd::Memfd;
use memmap2::MmapMut;
use regex::Regex;
use wayland_client::{
    QueueHandle,
    protocol::{wl_compositor, wl_registry::WlRegistry, wl_seat, wl_shm},
};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1;
use xkbcommon::xkb::{self, keysyms};
use xkeysym::KeyCode;

#[derive(Default, Debug)]
pub struct WaylandGlobals {
    pub compositor_name: Option<u32>,
    pub compositor_version: Option<u32>,
    pub layer_shell_name: Option<u32>,
    pub layer_shell_version: Option<u32>,
    pub shm_name: Option<u32>,
    pub shm_version: Option<u32>,
    pub wl_seat_name: Option<u32>,
    pub wl_seat_version: Option<u32>,
}

impl WaylandGlobals {
    pub fn bind_registries(&self, registry: &WlRegistry, qh: &QueueHandle<AppData>) -> Registries {
        let shm = if let (Some(name), Some(version)) = (self.shm_name, self.shm_version) {
            registry.bind::<wl_shm::WlShm, _, _>(name, version, qh, ())
        } else {
            panic!("No shared memory support");
        };

        let compositor =
            if let (Some(name), Some(version)) = (self.compositor_name, self.compositor_version) {
                registry.bind::<wl_compositor::WlCompositor, _, _>(name, version, qh, ())
            } else {
                panic!("No compositor");
            };

        let layer_shell = if let (Some(name), Some(version)) =
            (self.layer_shell_name, self.layer_shell_version)
        {
            registry.bind::<zwlr_layer_shell_v1::ZwlrLayerShellV1, _, _>(name, version, qh, ())
        } else {
            panic!("No layer shell");
        };

        let seat = if let (Some(name), Some(version)) = (self.wl_seat_name, self.wl_seat_version) {
            registry.bind::<wl_seat::WlSeat, _, _>(name, version, qh, ())
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
    pub input: String,

    bins: Vec<String>,
    pub filtered_bins: Vec<String>,

    pub selected_index: u32,
}

impl Input {
    pub fn new() -> anyhow::Result<Self> {
        let mut new = Self {
            input: Default::default(),
            bins: crate::path::get_bin_names()?,
            filtered_bins: Default::default(),
            selected_index: Default::default(),
        };
        new.update_bins();
        Ok(new)
    }
}

impl Input {
    pub fn update_bins(&mut self) {
        let input = self.input.to_lowercase();

        let regex = Regex::new(&self.input).ok();
        let mut bins: Vec<(String, String)> = self
            .bins
            .iter()
            .filter(|s| {
                if input.is_empty() {
                    true
                } else if let Some(regex) = &regex {
                    regex.is_match(s)
                } else {
                    self.input.is_empty() || s.contains(&self.input)
                }
            })
            .map(|s| (s.to_string(), s.to_lowercase()))
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

            score(&a.1).cmp(&score(&b.1)).then_with(|| a.0.cmp(&b.0))
        });

        let bins = bins.into_iter().map(|(orig, _)| orig).collect();

        self.filtered_bins = bins;
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

#[derive(Default, Debug)]
pub struct AppData {
    pub repeat: Option<RepeatState>,
    pub wayland_globals: WaylandGlobals,
    pub output: Option<Output>,
    pub xkb: Option<Xkb>,

    pub configured: bool,
    pub callback_done: bool,
    pub redraw_needed: bool,

    pub buffer_memfd: Option<Memfd>,
    pub buffer_mmap: Option<MmapMut>,

    pub inp: Input,
}

impl AppData {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            inp: Input::new()?,
            ..Default::default()
        })
    }

    pub fn handle_key(&mut self, key: KeyCode) {
        let xkb = self.xkb.as_ref().unwrap();

        let sym = xkb.0.key_get_one_sym(key);

        match sym.into() {
            keysyms::KEY_Return => {
                let program = self
                    .inp
                    .filtered_bins
                    .get(self.inp.selected_index as usize)
                    .unwrap()
                    .clone();
                let _ = Command::new(program).exec();
                unreachable!()
            }
            keysyms::KEY_BackSpace => {
                self.inp.input.pop();
                self.inp.selected_index = 0;
                self.inp.update_bins();
            }
            keysyms::KEY_Escape => std::process::exit(0),
            keysyms::KEY_Right => {
                let max_index = self.inp.filtered_bins.len().saturating_sub(1) as u32;
                if self.inp.selected_index < max_index {
                    self.inp.selected_index += 1;
                }
            }
            keysyms::KEY_Left => {
                if self.inp.selected_index != 0 {
                    self.inp.selected_index -= 1;
                }
            }
            _ => {
                let text = self.xkb.as_ref().unwrap().0.key_get_utf8(key);

                if !text.is_empty() {
                    self.inp.input.push_str(&text);
                    self.inp.selected_index = 0;
                    self.inp.update_bins();
                }
            }
        }
    }
}

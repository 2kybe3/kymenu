mod color;
mod dispatch;
mod font;
mod path;

use std::{
    os::{
        fd::{AsRawFd, BorrowedFd},
        unix::process::CommandExt,
    },
    process::Command,
    time::{Duration, Instant},
};

use memfd::Memfd;
use memmap2::{Mmap, MmapMut};
use wayland_client::{
    Connection, QueueHandle,
    protocol::{wl_compositor, wl_registry::WlRegistry, wl_seat, wl_shm},
};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};
use xkbcommon::xkb::{self, keysyms};
use xkeysym::KeyCode;

use crate::{color::Color, path::get_bin_names};

#[derive(Default, Debug)]
pub struct WaylandGlobals {
    compositor_name: Option<u32>,
    compositor_version: Option<u32>,
    layer_shell_name: Option<u32>,
    layer_shell_version: Option<u32>,
    shm_name: Option<u32>,
    shm_version: Option<u32>,
    wl_seat_name: Option<u32>,
    wl_seat_version: Option<u32>,
}

#[derive(Default, Debug)]
pub struct Output {
    width: u32,
    height: u32,
}

#[derive(Default, Debug)]
pub struct Input {
    input: String,
    bins: Vec<String>,
    selected_index: u32,
}

impl Input {
    pub fn get_bins(&self) -> Vec<String> {
        let input = self.input.to_lowercase();

        let mut bins: Vec<(String, String)> = self
            .bins
            .iter()
            .filter(|s| {
                if let Ok(regex) = regex::Regex::new(&self.input) {
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

        bins.into_iter().map(|(orig, _)| orig).collect()
    }
}

pub struct XKB {
    state: xkb::State,
}

#[derive(Debug)]
struct RepeatState {
    key: Option<KeyCode>,
    started_at: Instant,
    last_repeat: Instant,
    rate: i32,
    delay: i32,
}

impl std::fmt::Debug for XKB {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XKB").finish()
    }
}

#[derive(Default, Debug)]
pub struct AppData {
    repeat: Option<RepeatState>,
    wayland_globals: WaylandGlobals,
    output: Option<Output>,
    xkb: Option<XKB>,

    configured: bool,
    callback_done: bool,
    redraw_needed: bool,

    buffer_memfd: Option<Memfd>,
    buffer_mmap: Option<MmapMut>,

    inp: Input,
}

impl AppData {
    pub fn handle_key(&mut self, key: KeyCode) {
        let xkb = self.xkb.as_ref().unwrap();

        let sym = xkb.state.key_get_one_sym(key);

        match sym.into() {
            keysyms::KEY_Return => {
                let program = self
                    .inp
                    .get_bins()
                    .get(self.inp.selected_index as usize)
                    .unwrap()
                    .clone();
                let _ = Command::new(program).exec();
                unreachable!()
            }
            keysyms::KEY_BackSpace => {
                self.inp.input.pop();
            }
            keysyms::KEY_Escape => std::process::exit(0),
            keysyms::KEY_Right => {
                if self.inp.selected_index < self.inp.get_bins().len() as u32 - 1 {
                    self.inp.selected_index += 1;
                }
            }
            keysyms::KEY_Left => {
                if self.inp.selected_index != 0 {
                    self.inp.selected_index -= 1;
                }
            }
            _ => {
                let text = self.xkb.as_ref().unwrap().state.key_get_utf8(key);

                if !text.is_empty() {
                    self.inp.input.push_str(&text);
                    self.inp.selected_index = 0;
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct Registries {
    pub shm: wl_shm::WlShm,
    pub seat: wl_seat::WlSeat,
    pub compositor: wl_compositor::WlCompositor,
    pub layer_shell: zwlr_layer_shell_v1::ZwlrLayerShellV1,
}

static DEFAULT_HEIGHT: u32 = 20;

static PROMPT: &str = ">> ";

static END_ARROW: &str = ">";
static END_MARGIN: u32 = 5;

static START_ARROW: &str = "<";

static TEXT_MARGIN: u32 = 5;

static ARROW_MARGIN: u32 = 8;

static FONT: &str = "monospace";
static FONT_SIZE: f32 = 16.0;

static DEFAULT_BIN_START: u32 = 200;

/*
 * blue:  [base]
 * green: [base + 1]
 * red:   [base + 2]
 * alpha: [base + 3]
 */
static COLOR_FORMAT: wl_shm::Format = wl_shm::Format::Argb8888;
static COLOR_SIZE: u32 = 4;

pub fn bind_registries(
    registry: &WlRegistry,
    qh: &QueueHandle<AppData>,
    state: &AppData,
) -> Registries {
    let shm = if let (Some(name), Some(version)) = (
        state.wayland_globals.shm_name,
        state.wayland_globals.shm_version,
    ) {
        registry.bind::<wl_shm::WlShm, _, _>(name, version, qh, ())
    } else {
        panic!("No shared memory support");
    };

    let compositor = if let (Some(name), Some(version)) = (
        state.wayland_globals.compositor_name,
        state.wayland_globals.compositor_version,
    ) {
        registry.bind::<wl_compositor::WlCompositor, _, _>(name, version, qh, ())
    } else {
        panic!("No compositor");
    };

    let layer_shell = if let (Some(name), Some(version)) = (
        state.wayland_globals.layer_shell_name,
        state.wayland_globals.layer_shell_version,
    ) {
        registry.bind::<zwlr_layer_shell_v1::ZwlrLayerShellV1, _, _>(name, version, qh, ())
    } else {
        panic!("No layer shell");
    };

    let seat = if let (Some(name), Some(version)) = (
        state.wayland_globals.wl_seat_name,
        state.wayland_globals.wl_seat_version,
    ) {
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

fn main() -> anyhow::Result<()> {
    // INIT
    tracing_subscriber::fmt().init();

    let mut state = AppData {
        inp: Input {
            bins: get_bin_names()?,
            ..Default::default()
        },
        ..Default::default()
    };

    let font = font::load_font(font::get_font(FONT, None)?)?;

    let conn = Connection::connect_to_env()?;
    let display = conn.display();

    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let registry = display.get_registry(&qh, ());

    // Get registries
    display.sync(&qh, ());

    loop {
        event_queue.roundtrip(&mut state)?;

        if state.callback_done {
            state.callback_done = false;
            break;
        }
    }

    let registries = bind_registries(&registry, &qh, &state);
    tracing::info!(registries = ?registries, "loaded registries");

    // Create a surface and layered_surface
    let surface = registries.compositor.create_surface(&qh, ());
    let layered_surface = registries.layer_shell.get_layer_surface(
        &surface,
        None,
        zwlr_layer_shell_v1::Layer::Overlay,
        "my_layer".into(),
        &qh,
        (),
    );

    // configure layered_surface and commit surface to get the configured response and also
    // output_width set
    layered_surface.set_size(0, DEFAULT_HEIGHT);
    layered_surface.set_anchor(
        zwlr_layer_surface_v1::Anchor::Top
            | zwlr_layer_surface_v1::Anchor::Left
            | zwlr_layer_surface_v1::Anchor::Right,
    );
    layered_surface.set_exclusive_zone(-1);
    layered_surface
        .set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::Exclusive);
    surface.commit();

    // Wait for output_width to be set from wayland server
    loop {
        event_queue.roundtrip(&mut state)?;

        if state.output.is_some() {
            break;
        }
    }

    let pool = {
        // We scope this in the rare case height or width changes and size could be inaccurate
        let output = state.output.as_ref().unwrap();
        let size = (output.width * output.height * COLOR_SIZE) as usize;
        let pool_size = size * 2;

        // Create memory file for the buffer
        let fd = memfd::MemfdOptions::default().create("memfd_create")?;
        fd.as_file().set_len(pool_size as u64)?;

        let mmap = unsafe { Mmap::map(fd.as_raw_fd())?.make_mut()? };

        // Pool to create buffers
        let pool = registries.shm.create_pool(
            unsafe { BorrowedFd::borrow_raw(fd.as_raw_fd()) },
            pool_size as i32,
            &qh,
            (),
        );

        state.buffer_memfd = Some(fd);
        state.buffer_mmap = Some(mmap);

        pool
    };

    registries.seat.get_keyboard(&qh, ());

    let mut frame = 0usize;
    let mut surface_attached = false;

    loop {
        let mut repeated_key = None;

        if let Some(repeat) = state.repeat.as_mut() {
            let now = Instant::now();

            if now.duration_since(repeat.started_at) >= Duration::from_millis(repeat.delay as u64) {
                let interval = Duration::from_secs_f64(1.0 / repeat.rate as f64);

                if now.duration_since(repeat.last_repeat) >= interval {
                    repeat.last_repeat = now;

                    repeated_key = repeat.key;
                }
            }
        }

        if let Some(key) = repeated_key {
            state.handle_key(key);
        }

        if state.configured && state.redraw_needed {
            state.redraw_needed = false;

            let output = state.output.as_ref().unwrap();
            let width = output.width;
            let height = output.height;

            let size = (width * height * COLOR_SIZE) as usize;
            let buffer_size = size * 2;

            state
                .buffer_memfd
                .as_mut()
                .unwrap()
                .as_file()
                .set_len(buffer_size as u64)?;

            let size = (width * height * COLOR_SIZE) as usize;

            // We swap between 2 buffers because 1 is in use by swayland and should not be used
            let offset = (frame % 2) * size;
            let buffer = &mut state.buffer_mmap.as_mut().unwrap()[offset..offset + size];

            // Background
            for pixel in buffer.chunks_exact_mut(4) {
                pixel[..4].copy_from_slice(&Color::BACKGROUND_COLOR.get_bgra());
            }

            // Rendering from left to right
            let mut index = 0;
            {
                // Prompt
                let size = font.text_width(PROMPT, FONT_SIZE);

                font.render_text(
                    PROMPT,
                    FONT_SIZE,
                    index,
                    &Color::PROMPT_COLOR,
                    buffer,
                    height,
                    width,
                );

                index += size;
            }
            {
                // Current Input
                let size = font.text_width(&state.inp.input, FONT_SIZE);

                font.render_text(
                    &state.inp.input,
                    FONT_SIZE,
                    index,
                    &Color::INPUT_COLOR,
                    buffer,
                    height,
                    width,
                );

                index += size;
            }

            if index < DEFAULT_BIN_START {
                index = DEFAULT_BIN_START;
            } else {
                index += TEXT_MARGIN * 15;
            }

            {
                // Start Arrow
                let size = font.text_width(START_ARROW, FONT_SIZE) + ARROW_MARGIN;

                font.render_text(
                    START_ARROW,
                    FONT_SIZE,
                    index,
                    &Color::ARROW_COLOR,
                    buffer,
                    height,
                    width,
                );
                index += size;
            }

            // Packages
            let mut all_bins_shown = true;
            let end_arrow_size = font.text_width(END_ARROW, FONT_SIZE) + END_MARGIN;

            for (i, bin) in state.inp.get_bins().iter().enumerate() {
                let size = font.text_width(bin, FONT_SIZE) + TEXT_MARGIN;
                if (index + size) > width - end_arrow_size {
                    all_bins_shown = false;
                    break;
                }

                font.render_text(
                    bin,
                    FONT_SIZE,
                    index,
                    if i == state.inp.selected_index as usize {
                        &Color::SELECTED_COLOR
                    } else {
                        &Color::ITEM_COLOR
                    },
                    buffer,
                    height,
                    width,
                );

                index += size;
            }

            {
                // End Arrow
                font.render_text(
                    END_ARROW,
                    FONT_SIZE,
                    if all_bins_shown {
                        index
                    } else {
                        width - end_arrow_size
                    },
                    &Color::ARROW_COLOR,
                    buffer,
                    height,
                    width,
                );
            }

            let buffer = pool.create_buffer(
                offset as i32,
                width as i32,
                height as i32,
                (width * COLOR_SIZE) as i32,
                COLOR_FORMAT,
                &qh,
                (),
            );

            surface.attach(Some(&buffer), 0, 0);
            surface.damage(0, 0, width as i32, height as i32);

            if !surface_attached {
                surface_attached = true;
            }

            surface.frame(&qh, ());

            surface.commit();
            frame += 1;
        };

        event_queue.flush()?;

        let _ = conn.prepare_read().unwrap().read().ok();

        event_queue.dispatch_pending(&mut state)?;
    }
}

mod appdata;
mod cli;
mod color;
mod dispatch;
mod font;
mod path;

use std::{
    io,
    os::fd::{AsRawFd, BorrowedFd},
    time::{Duration, Instant},
};

use clap::{CommandFactory, Parser};
use memmap2::Mmap;
use wayland_client::{Connection, protocol::wl_shm};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

use crate::{appdata::AppData, cli::Cli};

/*
 * blue:  [base]
 * green: [base + 1]
 * red:   [base + 2]
 * alpha: [base + 3]
 */
static COLOR_FORMAT: wl_shm::Format = wl_shm::Format::Argb8888;
static COLOR_SIZE: u32 = 4;

static NAME: &str = "kymenu";

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if let Some(cmd) = cli.command {
        match cmd {
            cli::Commands::GenerateZshCompletion => clap_complete::generate(
                clap_complete::shells::Zsh,
                &mut Cli::command(),
                NAME,
                &mut io::stdout(),
            ),
            cli::Commands::GenerateFishCompletion => clap_complete::generate(
                clap_complete::shells::Fish,
                &mut Cli::command(),
                NAME,
                &mut io::stdout(),
            ),
            cli::Commands::GenerateBashCompletion => clap_complete::generate(
                clap_complete::shells::Bash,
                &mut Cli::command(),
                NAME,
                &mut io::stdout(),
            ),
        };
        std::process::exit(0);
    };

    let mut state = AppData::new(cli.extract())?;

    let font = font::load_font(
        font::get_font(
            &state.extracted.font_family,
            state.extracted.font_style.as_deref(),
        )
        .ok(),
    )?;

    // Connect to beloved wayland
    let conn = Connection::connect_to_env()?;
    let display = conn.display();

    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let registry = display.get_registry(&qh, ());

    {
        // Make sure all registries are received
        display.sync(&qh, ());

        loop {
            event_queue.roundtrip(&mut state)?;

            if state.callback_done {
                state.callback_done = false;
                break;
            }
        }
    }

    let registries = state.wayland_globals.bind_registries(&registry, &qh);

    // Create a surface and layered_surface
    let surface = registries.compositor.create_surface(&qh, ());
    let layered_surface = registries.layer_shell.get_layer_surface(
        &surface,
        None,
        zwlr_layer_shell_v1::Layer::Overlay,
        NAME.to_string(),
        &qh,
        (),
    );

    layered_surface.set_size(0, state.extracted.height);
    layered_surface.set_anchor(
        zwlr_layer_surface_v1::Anchor::Top
            | zwlr_layer_surface_v1::Anchor::Left
            | zwlr_layer_surface_v1::Anchor::Right,
    );
    layered_surface.set_exclusive_zone(-1);
    layered_surface
        .set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::Exclusive);
    surface.commit();

    // Wait for output to be set from wayland event from the layered_surface
    loop {
        event_queue.roundtrip(&mut state)?;

        if state.output.is_some() {
            break;
        }
    }

    let pool = {
        let output = state.output.as_ref().unwrap();
        let size = (output.width * output.height * COLOR_SIZE) as usize;
        let pool_size = size * 2;

        // Create memory file for the buffer
        let fd = memfd::MemfdOptions::default().create("memfd_create")?;
        fd.as_file().set_len(pool_size as u64)?;

        let mmap = unsafe { Mmap::map(fd.as_raw_fd())?.make_mut()? };

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

    // Start getting keyboard events
    registries.seat.get_keyboard(&qh, ());

    // Wait for keyboard to be set up
    loop {
        event_queue.roundtrip(&mut state)?;

        if state.xkb.is_some() {
            break;
        }
    }

    // No frame is request so the loop would just sit there so we first have to set the dirty state
    // once so we start sending frames (just adding surface.frame,commit now would also work but
    // seems bloat)
    state.inp.dirty = true;

    let mut frame = 0usize;

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

        if state.configured && state.inp.dirty {
            eprintln!("render");

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
            let bgra = u32::from_le_bytes(state.extracted.background_color.get_bgra());
            let pixels = bytemuck::cast_slice_mut::<u8, u32>(buffer);
            pixels.fill(bgra);

            // Rendering from left to right
            let mut index = state.extracted.start_margin;
            {
                // Prompt
                let size = font.text_width(&state.extracted.prompt, state.extracted.font_size);

                font.render_text(
                    &state.extracted.prompt,
                    state.extracted.font_size,
                    index,
                    &state.extracted.prompt_color,
                    buffer,
                    height,
                    width,
                );

                index += size;
            }

            {
                // Current Input
                let mut extra = false;
                let mut input = state.inp.input().to_owned();

                let mut size = font.text_width(&input, state.extracted.font_size);
                if size >= width / 4 {
                    let mut truncated = String::new();

                    for c in state.inp.input().chars() {
                        let next = format!("{}...", truncated.clone() + &c.to_string());
                        if font.text_width(&next, state.extracted.font_size) >= width / 4 {
                            extra = true;
                            break;
                        }
                        truncated.push(c);
                    }

                    input = truncated;
                    size = font.text_width(&input, state.extracted.font_size);
                }

                font.render_text(
                    &input,
                    state.extracted.font_size,
                    index,
                    &state.extracted.input_color,
                    buffer,
                    height,
                    width,
                );

                index += size;

                if extra {
                    font.render_text(
                        "...",
                        state.extracted.font_size,
                        index,
                        &state.extracted.extra_text_color,
                        buffer,
                        height,
                        width,
                    );

                    index += font.text_width("...", state.extracted.font_size);
                }
            }

            if index + state.extracted.bin_start_offset < state.extracted.default_bin_start_x {
                index = state.extracted.default_bin_start_x;
            } else {
                index += state.extracted.bin_start_offset;
            }

            {
                // Start Arrow
                let arrow = if state.inp.selected_index() == 0 {
                    &state.extracted.start_arrow
                } else {
                    &state.extracted.start_arrow_more
                };

                let size = font.text_width(arrow, state.extracted.font_size)
                    + state.extracted.arrow_margin;

                font.render_text(
                    arrow,
                    state.extracted.font_size,
                    index,
                    &state.extracted.arrow_color,
                    buffer,
                    height,
                    width,
                );
                index += size;
            }

            // Packages
            let mut all_bins_shown = true;

            let end_arrow_size = font
                .text_width(&state.extracted.end_arrow, state.extracted.font_size)
                + state.extracted.end_margin;

            let end_arrow_size_more = font
                .text_width(&state.extracted.end_arrow_more, state.extracted.font_size)
                + state.extracted.end_margin;

            for (i, bin) in state
                .inp
                .filtered_bins()
                .iter()
                .enumerate()
                .skip(state.inp.selected_index() as usize)
            {
                let size =
                    font.text_width(bin, state.extracted.font_size) + state.extracted.text_margin;

                let last = i == state.inp.filtered_bins().len() - 1;

                if (index + size)
                    > width
                        - if last {
                            end_arrow_size
                        } else {
                            end_arrow_size_more
                        }
                {
                    all_bins_shown = false;
                    break;
                }

                font.render_text(
                    bin,
                    state.extracted.font_size,
                    index,
                    if i == state.inp.selected_index() as usize {
                        &state.extracted.selected_color
                    } else {
                        &state.extracted.item_color
                    },
                    buffer,
                    height,
                    width,
                );

                index += size;
            }

            {
                // End Arrow
                let arrow = if all_bins_shown {
                    &state.extracted.end_arrow
                } else {
                    &state.extracted.end_arrow_more
                };

                let size =
                    font.text_width(arrow, state.extracted.font_size) + state.extracted.end_margin;

                font.render_text(
                    arrow,
                    state.extracted.font_size,
                    if all_bins_shown { index } else { width - size },
                    &state.extracted.arrow_color,
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

            surface.frame(&qh, ());

            surface.commit();

            state.inp.dirty = false;
            frame += 1;
        };

        event_queue.flush()?;

        let _ = conn.prepare_read().unwrap().read().ok();

        event_queue.dispatch_pending(&mut state)?;
    }
}

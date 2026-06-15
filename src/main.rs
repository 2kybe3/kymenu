mod appdata;
mod cli;
mod color;
mod dispatch;
mod font;

use std::{
    os::fd::{AsRawFd, BorrowedFd},
    time::{Duration, Instant},
};

use clap::Parser;
use memmap2::Mmap;
use wayland_client::{Connection, protocol::wl_shm};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

use crate::{
    appdata::{AppData, Buffer},
    cli::Cli,
    font::{TextFont, TextRenderer},
};

/*
 * blue:  [base]
 * green: [base + 1]
 * red:   [base + 2]
 * alpha: [base + 3]
 */
static COLOR_FORMAT: wl_shm::Format = wl_shm::Format::Argb8888;
static COLOR_SIZE: u32 = 4;

pub static NAME: &str = "kymenu";

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let mut state = AppData::new(Cli::parse())?;

    let font = TextRenderer::new(TextFont::new(
        &state.cli.font_family,
        state.cli.font_style.as_deref(),
    ));

    // Connect to beloved wayland
    let conn = Connection::connect_to_env()?;
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let display = conn.display();
    let registry = display.get_registry(&qh, ());
    display.sync(&qh, ());

    // Make sure all registries are received
    loop {
        event_queue.roundtrip(&mut state)?;

        if state.callback_done {
            state.callback_done = false;
            break;
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

    layered_surface.set_size(0, state.cli.height);
    layered_surface.set_anchor(
        zwlr_layer_surface_v1::Anchor::Top
            | zwlr_layer_surface_v1::Anchor::Left
            | zwlr_layer_surface_v1::Anchor::Right,
    );
    layered_surface.set_exclusive_zone(-1);
    layered_surface
        .set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::Exclusive);
    surface.commit();

    // Wait for output sizes to be sent from wayland via an event from the layered_surface
    let output = loop {
        event_queue.roundtrip(&mut state)?;

        if let Some(output) = state.output.as_ref() {
            break output;
        }
    };

    let pool = {
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

        state.buffer = Some(Buffer::new(fd, mmap));

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
            let Some(output) = state.output.as_ref() else {
                tracing::error!("warning somehow got unset. this is an bug!");
                std::process::exit(1);
            };

            let width = output.width;
            let height = output.height;

            let size = (width * height * COLOR_SIZE) as usize;
            let buffer_size = size * 2;

            let Some(state_buffer) = &mut state.buffer else {
                tracing::error!("state_buffer somehow got unset. this is an bug!");
                std::process::exit(1);
            };

            state_buffer.memfd.as_file().set_len(buffer_size as u64)?;

            let size = (width * height * COLOR_SIZE) as usize;

            // We swap between 2 buffers because 1 is in use by swayland and should not be used
            let offset = (frame % 2) * size;
            let buffer = &mut state_buffer.mmap[offset..offset + size];

            // Background
            let bgra = u32::from_le_bytes(state.cli.background_color.get_bgra());
            let pixels = bytemuck::cast_slice_mut::<u8, u32>(buffer);
            pixels.fill(bgra);

            // Rendering from left to right
            let mut index = state.cli.start_margin;
            {
                // Prompt
                let size = font.text_width(&state.cli.prompt, state.cli.font_size);

                font.render_text(
                    &state.cli.prompt,
                    state.cli.font_size,
                    index,
                    &state.cli.prompt_color,
                    buffer,
                    height,
                    width,
                );

                index += size;
            }

            {
                // Current Input
                let mut extra = false;

                let mut input = if state.cli.hidden_input {
                    "*".repeat(state.inp.input().chars().count())
                } else {
                    state.inp.input().to_string()
                };

                let check_width = if state.cli.input { width } else { width / 4 };

                let mut size = font.text_width(&input, state.cli.font_size);
                if index + size >= check_width {
                    let mut truncated = String::new();

                    for c in input.chars() {
                        let next = format!("{}...", truncated.clone() + &c.to_string());
                        if font.text_width(&next, state.cli.font_size) + index >= check_width {
                            extra = true;
                            break;
                        }
                        truncated.push(c);
                    }

                    input = truncated;
                    size = font.text_width(&input, state.cli.font_size);
                }

                font.render_text(
                    &input,
                    state.cli.font_size,
                    index,
                    &state.cli.input_color,
                    buffer,
                    height,
                    width,
                );

                index += size;

                if extra {
                    font.render_text(
                        "...",
                        state.cli.font_size,
                        index,
                        &state.cli.extra_text_color,
                        buffer,
                        height,
                        width,
                    );

                    index += font.text_width("...", state.cli.font_size);
                }
            }

            if !state.cli.input {
                if index + state.cli.bin_start_margin < state.cli.default_bin_start {
                    index = state.cli.default_bin_start;
                } else {
                    index += state.cli.bin_start_margin;
                }

                {
                    // Start Arrow
                    let arrow = if state.inp.selected_index() == 0 {
                        &state.cli.start_arrow
                    } else {
                        &state.cli.start_arrow_more
                    };

                    let size = font.text_width(arrow, state.cli.font_size) + state.cli.arrow_margin;

                    font.render_text(
                        arrow,
                        state.cli.font_size,
                        index,
                        &state.cli.arrow_color,
                        buffer,
                        height,
                        width,
                    );
                    index += size;
                }

                // Packages
                let mut all_bins_shown = true;

                let end_arrow_size = font.text_width(&state.cli.end_arrow, state.cli.font_size)
                    + state.cli.end_margin;

                let end_arrow_size_more = font
                    .text_width(&state.cli.end_arrow_more, state.cli.font_size)
                    + state.cli.end_margin;

                for (i, bin) in state
                    .inp
                    .filtered_inputs()
                    .iter()
                    .enumerate()
                    .skip(state.inp.selected_index() as usize)
                {
                    let last = i == state.inp.filtered_inputs().len() - 1;

                    let size = font.text_width(bin.display(), state.cli.font_size)
                        + if last {
                            state.cli.arrow_margin
                        } else {
                            state.cli.text_margin
                        };

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
                        bin.display(),
                        state.cli.font_size,
                        index,
                        if i == state.inp.selected_index() as usize {
                            &state.cli.selected_color
                        } else {
                            &state.cli.item_color
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
                        &state.cli.end_arrow
                    } else {
                        &state.cli.end_arrow_more
                    };

                    let size = font.text_width(arrow, state.cli.font_size) + state.cli.end_margin;

                    font.render_text(
                        arrow,
                        state.cli.font_size,
                        if all_bins_shown { index } else { width - size },
                        &state.cli.arrow_color,
                        buffer,
                        height,
                        width,
                    );
                }
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

        // This call will not block, but may return [`None`] if the inner queue of the backend needs to be dispatched.
        let prep_read = match conn.prepare_read() {
            Some(v) => v,
            None => {
                tracing::warn!("prep_read returned smth");
                event_queue.dispatch_pending(&mut state)?;
                continue;
            }
        };

        let _ = prep_read.read().ok();

        event_queue.dispatch_pending(&mut state)?;
    }
}

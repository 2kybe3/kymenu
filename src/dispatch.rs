use std::time::Instant;

use memmap2::Mmap;
use wayland_client::{
    Connection, Dispatch, WEnum,
    protocol::{
        wl_buffer, wl_callback, wl_compositor, wl_display, wl_keyboard, wl_registry, wl_seat,
        wl_shm, wl_shm_pool, wl_surface,
    },
};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};
use xkbcommon::xkb;
use xkeysym::KeyCode;

use crate::{
    AppData,
    appdata::{RepeatConfig, output::Output},
};

impl Dispatch<wl_registry::WlRegistry, ()> for AppData {
    fn event(
        state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        event: <wl_registry::WlRegistry as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            state.wayland_globals.set(&interface, name, version);
        }
    }
}

impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, ()> for AppData {
    fn event(
        state: &mut Self,
        proxy: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: <zwlr_layer_surface_v1::ZwlrLayerSurfaceV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        if let zwlr_layer_surface_v1::Event::Configure {
            serial,
            width,
            height,
        } = event
        {
            proxy.ack_configure(serial);

            state.configured = true;

            let output = Output::new(width, height);

            state.output = Some(output.clone());

            if let Some(buffer) = &mut state.buffer {
                buffer.set_pending_resize(output);
            }
        }
    }
}

impl wayland_client::Dispatch<wl_buffer::WlBuffer, ()> for AppData {
    fn event(
        state: &mut Self,
        proxy: &wl_buffer::WlBuffer,
        event: <wl_buffer::WlBuffer as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        if let wl_buffer::Event::Release = event
            && let (Some(buffer), Some(output), Some(registries)) =
                (&mut state.buffer, &state.output, &state.registries)
        {
            buffer.buffer_released(proxy);

            if buffer.has_pending_resize()
                && buffer.all_buffer_free()
                && let Err(e) = buffer.apply_pending_resize(output, &registries.shm, qhandle)
            {
                tracing::error!("resizing buffer failed: {e}");
            }
        }
    }
}

impl Dispatch<wl_callback::WlCallback, ()> for AppData {
    fn event(
        state: &mut Self,
        _proxy: &wl_callback::WlCallback,
        event: <wl_callback::WlCallback as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        if let wl_callback::Event::Done { .. } = event {
            state.callback_done = true;
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for AppData {
    fn event(
        crate_state: &mut Self,
        _proxy: &wl_keyboard::WlKeyboard,
        event: <wl_keyboard::WlKeyboard as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        match event {
            wl_keyboard::Event::Keymap { format, fd, size } => {
                // No Keymap is a fatal error so we prefer to exit here
                let format = match format {
                    WEnum::Value(v) => v,
                    WEnum::Unknown(e) => {
                        tracing::error!("Unsupported keymap format '{format:?}': {e}");
                        std::process::exit(1);
                    }
                };
                if format != wl_keyboard::KeymapFormat::XkbV1 {
                    tracing::error!("Unsupported keymap format '{format:?}'");
                    std::process::exit(1);
                }

                let mmap = match unsafe { Mmap::map(&fd) } {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::error!(
                            "failed to open keymap file given from the compositor: {e}"
                        );
                        std::process::exit(1);
                    }
                };

                let keymap_string = match std::str::from_utf8(&mmap[..size as usize]) {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::error!(
                            "keymap provided from the compositor is not valid utf-8: {e}"
                        );
                        std::process::exit(1);
                    }
                };

                let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);

                let keymap = match xkb::Keymap::new_from_string(
                    &context,
                    keymap_string.to_owned(),
                    xkb::KEYMAP_FORMAT_TEXT_V1,
                    xkb::COMPILE_NO_FLAGS,
                ) {
                    Some(v) => v,
                    None => {
                        tracing::error!("failed to create xkb Keymap");
                        std::process::exit(1);
                    }
                };

                let state = xkb::State::new(&keymap);

                crate_state.xkb = Some(crate::appdata::Xkb(state))
            }
            wl_keyboard::Event::Key { key, state, .. } => {
                let state = match state {
                    WEnum::Value(v) => v,
                    WEnum::Unknown(e) => {
                        tracing::error!("Unsupported key state '{state:?}': {e}");
                        return;
                    }
                };

                if state != wl_keyboard::KeyState::Pressed {
                    crate_state.repeat_state = None;
                    return;
                }

                let key = KeyCode::from(key + 8);

                crate_state.repeat_state = Some(crate::appdata::RepeatState {
                    key,
                    started_at: Instant::now(),
                    last_repeat: Instant::now(),
                });

                crate_state.handle_key(key);
            }
            wl_keyboard::Event::Modifiers {
                mods_depressed,
                mods_latched,
                mods_locked,
                group,
                ..
            } => {
                if let Some(xkb) = &mut crate_state.xkb {
                    xkb.0
                        .update_mask(mods_depressed, mods_latched, mods_locked, 0, 0, group);
                }
            }
            wl_keyboard::Event::RepeatInfo { rate, delay } => {
                crate_state.repeat_config = Some(RepeatConfig {
                    rate: rate as u32,
                    delay: delay as u32,
                });
            }
            _ => {}
        }
    }
}

macro_rules! impl_dispatch {
    ($($proxy:ty),+ $(,)?) => {
        $(
            impl wayland_client::Dispatch<$proxy, ()> for AppData {
                fn event(
                    _state: &mut Self,
                    _proxy: &$proxy,
                    _event: <$proxy as wayland_client::Proxy>::Event,
                    _data: &(),
                    _conn: &Connection,
                    _qhandle: &wayland_client::QueueHandle<Self>,
                ) {
                }
            }
        )+
    };
}

impl_dispatch!(
    zwlr_layer_shell_v1::ZwlrLayerShellV1,
    wl_compositor::WlCompositor,
    wl_shm_pool::WlShmPool,
    wl_display::WlDisplay,
    wl_surface::WlSurface,
    wl_seat::WlSeat,
    wl_shm::WlShm
);

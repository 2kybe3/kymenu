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

use crate::AppData;

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
            match interface.as_str() {
                "wl_compositor" => {
                    state.wayland_globals.compositor_name = Some(name);
                    state.wayland_globals.compositor_version = Some(version);
                }
                "zwlr_layer_shell_v1" => {
                    state.wayland_globals.layer_shell_name = Some(name);
                    state.wayland_globals.layer_shell_version = Some(version);
                }
                "wl_shm" => {
                    state.wayland_globals.shm_name = Some(name);
                    state.wayland_globals.shm_version = Some(version);
                }
                "wl_seat" => {
                    state.wayland_globals.wl_seat_name = Some(name);
                    state.wayland_globals.wl_seat_version = Some(version);
                }
                _ => {}
            }
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

            state.output = Some(crate::appdata::Output { width, height })
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
            state.redraw_needed = true;
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
                let format = match format {
                    WEnum::Value(v) => v,
                    WEnum::Unknown(_) => panic!("Unsupported keymap format"),
                };
                if format != wl_keyboard::KeymapFormat::XkbV1 {
                    panic!("Unsupported keymap format")
                }

                let mmap = unsafe { Mmap::map(&fd).unwrap() };

                let keymap_string =
                    std::str::from_utf8(&mmap[..size as usize]).expect("keymap is not valid utf-8");

                let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);

                let keymap = xkb::Keymap::new_from_string(
                    &context,
                    keymap_string.to_owned(),
                    xkb::KEYMAP_FORMAT_TEXT_V1,
                    xkb::COMPILE_NO_FLAGS,
                )
                .expect("failed to create keymap");

                let state = xkb::State::new(&keymap);

                crate_state.xkb = Some(crate::appdata::Xkb(state))
            }
            wl_keyboard::Event::Key { key, state, .. } => {
                let state = match state {
                    WEnum::Value(v) => v,
                    WEnum::Unknown(_) => unreachable!(),
                };

                if state != wl_keyboard::KeyState::Pressed {
                    let repeat_state = crate_state.repeat.as_mut().unwrap();
                    repeat_state.key = None;
                    return;
                }

                let key = KeyCode::from(key + 8);

                let repeat_state = crate_state.repeat.as_mut().unwrap();

                repeat_state.key = Some(key);
                repeat_state.started_at = Instant::now();
                repeat_state.last_repeat = Instant::now();

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
                crate_state.repeat = Some(crate::appdata::RepeatState {
                    key: None,
                    started_at: Instant::now(),
                    last_repeat: Instant::now(),
                    rate,
                    delay,
                })
            }
            _ => {}
        }
    }
}

macro_rules! impl_dispatch {
    ($proxy:ty) => {
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
    };
}

impl_dispatch!(zwlr_layer_shell_v1::ZwlrLayerShellV1);
impl_dispatch!(wl_compositor::WlCompositor);
impl_dispatch!(wl_shm_pool::WlShmPool);
impl_dispatch!(wl_display::WlDisplay);
impl_dispatch!(wl_surface::WlSurface);
impl_dispatch!(wl_buffer::WlBuffer);
impl_dispatch!(wl_seat::WlSeat);
impl_dispatch!(wl_shm::WlShm);

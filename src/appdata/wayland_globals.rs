use wayland_client::{
    QueueHandle,
    protocol::{wl_compositor, wl_registry::WlRegistry, wl_seat, wl_shm},
};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1;

use crate::appdata::AppData;

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
    pub shm: Option<WaylandGlobal>,
    pub seat: Option<WaylandGlobal>,
}

#[derive(Debug, Clone)]
pub struct Registries {
    pub compositor: wl_compositor::WlCompositor,
    pub layer_shell: zwlr_layer_shell_v1::ZwlrLayerShellV1,
    pub seat: wl_seat::WlSeat,
    pub shm: wl_shm::WlShm,
}

impl WaylandGlobals {
    pub fn bind(&self, registry: &WlRegistry, qh: &QueueHandle<AppData>) -> Registries {
        macro_rules! bind {
            ($opt:expr, $registry:expr, $qh:expr, $iface:ty, $msg:literal) => {
                if let Some(obj) = &$opt {
                    $registry.bind::<$iface, _, _>(obj.name, obj.version, $qh, ())
                } else {
                    panic!($msg)
                }
            };
        }

        let compositor = bind!(
            self.compositor,
            registry,
            qh,
            wl_compositor::WlCompositor,
            "No compositor"
        );
        let layer_shell = bind!(
            self.layer_shell,
            registry,
            qh,
            zwlr_layer_shell_v1::ZwlrLayerShellV1,
            "No layer shell"
        );
        let seat = bind!(self.seat, registry, qh, wl_seat::WlSeat, "No Seat");
        let shm = bind!(self.shm, registry, qh, wl_shm::WlShm, "No shared memory");

        Registries {
            shm,
            seat,
            compositor,
            layer_shell,
        }
    }

    pub fn set(&mut self, interface: &str, name: u32, version: u32) {
        match interface {
            "wl_compositor" => self.compositor = Some(WaylandGlobal::new(name, version)),
            "zwlr_layer_shell_v1" => self.layer_shell = Some(WaylandGlobal::new(name, version)),
            "wl_seat" => self.seat = Some(WaylandGlobal::new(name, version)),
            "wl_shm" => self.shm = Some(WaylandGlobal::new(name, version)),
            _ => {}
        }
    }
}

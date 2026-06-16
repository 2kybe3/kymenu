use std::os::fd::{AsRawFd, BorrowedFd};

use memfd::Memfd;
use memmap2::{Mmap, MmapMut};
use wayland_client::{
    QueueHandle,
    protocol::{wl_buffer::WlBuffer, wl_shm::WlShm, wl_shm_pool::WlShmPool},
};

use crate::appdata::{AppData, Output};

#[derive(Debug)]
pub struct Buffer {
    memfd: Memfd,
    mmap: MmapMut,
    pool: WlShmPool,
    current_size: usize,
    pending_resize: Option<Output>,
}

impl Buffer {
    pub fn new(output: &Output, shm: &WlShm, qh: &QueueHandle<AppData>) -> anyhow::Result<Self> {
        let size = output.get_buffer_size() * 2;

        let memfd = memfd::MemfdOptions::default().create("kymenu_pool")?;
        memfd.as_file().set_len(size as u64)?;

        let pool = shm.create_pool(
            unsafe { BorrowedFd::borrow_raw(memfd.as_raw_fd()) },
            size as i32,
            qh,
            (),
        );

        let mmap = unsafe { Mmap::map(memfd.as_raw_fd())?.make_mut()? };

        Ok(Self {
            memfd,
            mmap,
            pool,
            current_size: output.get_buffer_size() as usize,
            pending_resize: None,
        })
    }

    pub fn apply_pending_resize(
        &mut self,
        shm: &WlShm,
        qh: &QueueHandle<AppData>,
    ) -> anyhow::Result<()> {
        let Some(pending_resize) = &self.pending_resize.take() else {
            return Ok(());
        };

        let size = pending_resize.get_buffer_size() * 2;
        let pool_size = size * 2;

        self.memfd.as_file().set_len(pool_size as u64)?;
        self.mmap = unsafe { Mmap::map(self.memfd.as_raw_fd())?.make_mut()? };

        self.pool.destroy();
        self.pool = shm.create_pool(
            unsafe { BorrowedFd::borrow_raw(self.memfd.as_raw_fd()) },
            pool_size as i32,
            qh,
            (),
        );

        self.current_size = size as usize;
        Ok(())
    }

    pub fn has_pending_resize(&self) -> bool {
        self.pending_resize.is_some()
    }

    pub fn set_pending_resize(&mut self, new_output: Output) {
        if new_output.get_buffer_size() as usize != self.current_size {
            self.pending_resize = Some(new_output);
        }
    }

    pub fn get_buffer(&mut self, frame: usize) -> &mut [u8] {
        let offset = (frame % 2) * self.current_size;
        &mut self.mmap[offset..offset + self.current_size]
    }

    pub fn get_wl_buffer(
        &mut self,
        frame: usize,
        output: &Output,
        qh: &QueueHandle<AppData>,
    ) -> WlBuffer {
        let offset = (frame % 2) * self.current_size;

        self.pool.create_buffer(
            offset as i32,
            output.width() as i32,
            output.height() as i32,
            (output.width() * crate::COLOR_SIZE) as i32,
            crate::COLOR_FORMAT,
            qh,
            (),
        )
    }
}

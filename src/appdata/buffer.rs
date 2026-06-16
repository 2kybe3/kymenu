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
    main_buffer: BufferObject,
    secondary_buffer: BufferObject,
}

#[derive(Debug)]
pub struct BufferObject {
    buffer: WlBuffer,
    used: bool,
}
impl BufferObject {
    pub fn new(buffer: WlBuffer) -> Self {
        Self {
            buffer,
            used: false,
        }
    }
}

impl Buffer {
    pub fn new(output: &Output, shm: &WlShm, qh: &QueueHandle<AppData>) -> anyhow::Result<Self> {
        let size = output.get_buffer_size();
        let pool_size = size * 2;

        let memfd = memfd::MemfdOptions::default().create("kymenu_pool")?;
        memfd.as_file().set_len(pool_size as u64)?;

        let pool = shm.create_pool(
            unsafe { BorrowedFd::borrow_raw(memfd.as_raw_fd()) },
            pool_size as i32,
            qh,
            (),
        );

        let main_buffer = BufferObject::new(pool.create_buffer(
            0,
            output.width() as i32,
            output.height() as i32,
            (output.width() * crate::COLOR_SIZE) as i32,
            crate::COLOR_FORMAT,
            qh,
            (),
        ));

        let secondary_buffer = BufferObject::new(pool.create_buffer(
            size as i32,
            output.width() as i32,
            output.height() as i32,
            (output.width() * crate::COLOR_SIZE) as i32,
            crate::COLOR_FORMAT,
            qh,
            (),
        ));

        let mmap = unsafe { Mmap::map(memfd.as_raw_fd())?.make_mut()? };

        Ok(Self {
            memfd,
            mmap,
            pool,
            current_size: output.get_buffer_size() as usize,
            pending_resize: None,
            main_buffer,
            secondary_buffer,
        })
    }

    pub fn apply_pending_resize(
        &mut self,
        output: &Output,
        shm: &WlShm,
        qh: &QueueHandle<AppData>,
    ) -> anyhow::Result<()> {
        let Some(pending_resize) = &self.pending_resize.take() else {
            return Ok(());
        };

        let size = pending_resize.get_buffer_size();
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

        self.main_buffer = BufferObject::new(self.pool.create_buffer(
            0,
            output.width() as i32,
            output.height() as i32,
            (output.width() * crate::COLOR_SIZE) as i32,
            crate::COLOR_FORMAT,
            qh,
            (),
        ));

        self.secondary_buffer = BufferObject::new(self.pool.create_buffer(
            size as i32,
            output.width() as i32,
            output.height() as i32,
            (output.width() * crate::COLOR_SIZE) as i32,
            crate::COLOR_FORMAT,
            qh,
            (),
        ));

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

    pub fn all_buffer_free(&self) -> bool {
        !self.main_buffer.used && !self.secondary_buffer.used
    }

    pub fn acquire_buffer(&mut self) -> Option<(&WlBuffer, &mut [u8])> {
        if !self.main_buffer.used {
            self.main_buffer.used = true;
            return Some((
                &self.main_buffer.buffer,
                &mut self.mmap[..self.current_size],
            ));
        }

        if !self.secondary_buffer.used {
            self.secondary_buffer.used = true;
            return Some((
                &self.secondary_buffer.buffer,
                &mut self.mmap[self.current_size..self.current_size * 2],
            ));
        }

        None
    }

    pub fn buffer_released(&mut self, buffer: &WlBuffer) {
        if self.main_buffer.buffer == *buffer {
            self.main_buffer.used = false;
        }

        if self.secondary_buffer.buffer == *buffer {
            self.secondary_buffer.used = false;
        }
    }
}

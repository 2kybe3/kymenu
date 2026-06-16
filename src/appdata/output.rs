#[derive(Default, Debug, Clone)]
pub struct Output {
    width: u32,
    height: u32,
}

impl Output {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn get_buffer_size(&self) -> u32 {
        self.width() * self.height() * crate::COLOR_SIZE
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}

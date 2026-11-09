use crate::color::Color;

pub struct Framebuffer {
    width: usize,
    height: usize,
    pixels: Vec<u32>,
    depth: Vec<f32>,
    cached_background: Vec<u32>,
    background_key: Option<usize>,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width * height],
            depth: vec![f32::INFINITY; width * height],
            cached_background: vec![0; width * height],
            background_key: None,
        }
    }

    pub const fn width(&self) -> usize {
        self.width
    }
    pub const fn height(&self) -> usize {
        self.height
    }
    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }

    pub(crate) fn render_buffers_mut(&mut self) -> (&mut [u32], &mut [f32]) {
        (&mut self.pixels, &mut self.depth)
    }

    pub(crate) fn restore_background(&mut self, key: usize) -> bool {
        if self.background_key != Some(key) {
            return false;
        }
        self.pixels.copy_from_slice(&self.cached_background);
        self.depth.fill(f32::INFINITY);
        true
    }

    pub(crate) fn cache_background(&mut self, key: usize) {
        self.cached_background.copy_from_slice(&self.pixels);
        self.background_key = Some(key);
    }

    pub fn clear_gradient(&mut self, top: Color, bottom: Color) {
        let denominator = self.height.saturating_sub(1).max(1) as f32;
        for y in 0..self.height {
            let color = top.lerp(bottom, y as f32 / denominator).to_u32();
            let row = y * self.width;
            self.pixels[row..row + self.width].fill(color);
        }
        self.depth.fill(f32::INFINITY);
    }

    pub fn set_pixel(&mut self, x: i32, y: i32, color: Color) {
        if x >= 0 && y >= 0 && x < self.width as i32 && y < self.height as i32 {
            self.pixels[y as usize * self.width + x as usize] = color.to_u32();
        }
    }

    pub fn set_pixel_depth(&mut self, x: i32, y: i32, depth: f32, color: Color) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let index = y as usize * self.width + x as usize;
        if depth < self.depth[index] {
            self.depth[index] = depth;
            self.pixels[index] = color.to_u32();
        }
    }

    /// Comprueba el z-buffer antes de ejecutar un fragment shader costoso.
    pub fn depth_is_closer(&self, x: i32, y: i32, depth: f32) -> bool {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return false;
        }
        depth < self.depth[y as usize * self.width + x as usize]
    }
}

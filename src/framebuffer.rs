//! Own framebuffer and BMP writer (software-rendering course stages 01-06).

use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

#[derive(Clone)]
pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub color: Vec<u32>,
    pub depth: Vec<f32>,
    /// Reusable source copy for screen-space post-processing.
    pub postprocess_source: Vec<u32>,
    /// Opaque mesh pixels are excluded when sampling the gravitational lens.
    /// Stars and the transparent accretion gas intentionally remain sampleable.
    pub lensing_excluded: Vec<u8>,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            color: vec![0; width * height],
            depth: vec![f32::INFINITY; width * height],
            postprocess_source: vec![0; width * height],
            lensing_excluded: vec![0; width * height],
        }
    }

    pub fn clear(&mut self, color: u32) {
        self.color.fill(color & 0x00FF_FFFF);
        self.depth.fill(f32::INFINITY);
        self.lensing_excluded.fill(0);
    }

    pub fn clear_vertical_gradient(&mut self, top: u32, bottom: u32) {
        let top_rgb = unpack_rgb(top);
        let bottom_rgb = unpack_rgb(bottom);
        for y in 0..self.height {
            let t = if self.height > 1 {
                y as f32 / (self.height - 1) as f32
            } else {
                0.0
            };
            let color = pack_rgb(
                mix_byte(top_rgb.0, bottom_rgb.0, t),
                mix_byte(top_rgb.1, bottom_rgb.1, t),
                mix_byte(top_rgb.2, bottom_rgb.2, t),
            );
            let start = y * self.width;
            self.color[start..start + self.width].fill(color);
        }
        self.depth.fill(f32::INFINITY);
        self.lensing_excluded.fill(0);
    }

    #[inline]
    pub fn set_pixel(&mut self, x: i32, y: i32, color: u32) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let index = y as usize * self.width + x as usize;
        self.color[index] = color & 0x00FF_FFFF;
    }

    /// Blend a screen-space effect pixel over the existing framebuffer color.
    /// Alpha is an 8-bit opacity value: 0 is transparent and 255 is opaque.
    #[inline]
    pub fn blend_pixel(&mut self, x: i32, y: i32, color: u32, alpha: u8) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 || alpha == 0 {
            return;
        }
        let index = y as usize * self.width + x as usize;
        let destination = self.color[index];
        let inverse = 255_u32 - alpha as u32;
        let a = alpha as u32;
        let blend = |shift: u32| {
            let source_channel = (color >> shift) & 0xFF;
            let destination_channel = (destination >> shift) & 0xFF;
            (source_channel * a + destination_channel * inverse) / 255
        };
        self.color[index] = (blend(16) << 16) | (blend(8) << 8) | blend(0);
    }

    /// Alpha-blended Bresenham line, used for subtle motion streaks and warp effects.
    pub fn draw_line_blended(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: u32, alpha: u8) {
        let mut x = x0;
        let mut y = y0;
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut error = dx + dy;

        loop {
            self.blend_pixel(x, y, color, alpha);
            if x == x1 && y == y1 {
                break;
            }
            let twice_error = 2 * error;
            if twice_error >= dy {
                error += dy;
                x += sx;
            }
            if twice_error <= dx {
                error += dx;
                y += sy;
            }
        }
    }

    /// Write a pixel only when it is closer than the value already in the Z-buffer.
    #[inline]
    pub fn set_pixel_depth(&mut self, x: i32, y: i32, depth: f32, color: u32) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        if !(0.0..=1.0).contains(&depth) {
            return;
        }
        let index = y as usize * self.width + x as usize;
        if depth < self.depth[index] {
            self.depth[index] = depth;
            self.color[index] = color & 0x00FF_FFFF;
        }
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: u32) {
        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + width).min(self.width as i32);
        let y1 = (y + height).min(self.height as i32);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        for row in y0..y1 {
            let start = row as usize * self.width + x0 as usize;
            let end = row as usize * self.width + x1 as usize;
            self.color[start..end].fill(color & 0x00FF_FFFF);
        }
    }

    pub fn draw_rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: u32) {
        self.draw_line(x, y, x + width, y, color);
        self.draw_line(x, y, x, y + height, color);
        self.draw_line(x + width, y, x + width, y + height, color);
        self.draw_line(x, y + height, x + width, y + height, color);
    }

    /// Bresenham line algorithm. This is also useful for debugging mesh edges.
    pub fn draw_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
        let mut x = x0;
        let mut y = y0;
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut error = dx + dy;

        loop {
            self.set_pixel(x, y, color);
            if x == x1 && y == y1 {
                break;
            }
            let twice_error = 2 * error;
            if twice_error >= dy {
                error += dy;
                x += sx;
            }
            if twice_error <= dx {
                error += dx;
                y += sy;
            }
        }
    }

    pub fn save_bmp<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let mut file = File::create(path)?;
        let row_bytes = self.width * 3;
        let padding = (4 - (row_bytes % 4)) % 4;
        let pixel_bytes = (row_bytes + padding) * self.height;
        let file_size = 54 + pixel_bytes;

        // BITMAPFILEHEADER
        file.write_all(b"BM")?;
        file.write_all(&(file_size as u32).to_le_bytes())?;
        file.write_all(&[0; 4])?;
        file.write_all(&54u32.to_le_bytes())?;

        // BITMAPINFOHEADER: 24-bit, uncompressed, bottom-up rows.
        file.write_all(&40u32.to_le_bytes())?;
        file.write_all(&(self.width as i32).to_le_bytes())?;
        file.write_all(&(self.height as i32).to_le_bytes())?;
        file.write_all(&1u16.to_le_bytes())?;
        file.write_all(&24u16.to_le_bytes())?;
        file.write_all(&0u32.to_le_bytes())?;
        file.write_all(&(pixel_bytes as u32).to_le_bytes())?;
        file.write_all(&2835i32.to_le_bytes())?;
        file.write_all(&2835i32.to_le_bytes())?;
        file.write_all(&0u32.to_le_bytes())?;
        file.write_all(&0u32.to_le_bytes())?;

        let zero_padding = [0u8; 3];
        for y in (0..self.height).rev() {
            for x in 0..self.width {
                let color = self.color[y * self.width + x];
                let red = ((color >> 16) & 0xFF) as u8;
                let green = ((color >> 8) & 0xFF) as u8;
                let blue = (color & 0xFF) as u8;
                file.write_all(&[blue, green, red])?;
            }
            file.write_all(&zero_padding[..padding])?;
        }
        file.flush()
    }
}

fn unpack_rgb(color: u32) -> (u8, u8, u8) {
    (
        ((color >> 16) & 0xFF) as u8,
        ((color >> 8) & 0xFF) as u8,
        (color & 0xFF) as u8,
    )
}

fn mix_byte(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round() as u8
}

pub fn pack_rgb(red: u8, green: u8, blue: u8) -> u32 {
    ((red as u32) << 16) | ((green as u32) << 8) | blue as u32
}

pub fn color_from_f32(red: f32, green: f32, blue: f32) -> u32 {
    pack_rgb(
        (red.clamp(0.0, 1.0) * 255.0) as u8,
        (green.clamp(0.0, 1.0) * 255.0) as u8,
        (blue.clamp(0.0, 1.0) * 255.0) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpha_blend_respects_opacity() {
        let mut fb = Framebuffer::new(1, 1);
        fb.clear(0x000000);
        fb.blend_pixel(0, 0, 0xFFFFFF, 128);
        let color = fb.color[0];
        assert!(((color >> 16) & 0xFF) >= 127 && ((color >> 16) & 0xFF) <= 128);
        assert!(((color >> 8) & 0xFF) >= 127 && ((color >> 8) & 0xFF) <= 128);
        assert!((color & 0xFF) >= 127 && (color & 0xFF) <= 128);
    }

    #[test]
    fn blended_line_draws_without_overwriting_full_opacity() {
        let mut fb = Framebuffer::new(4, 1);
        fb.clear(0x000000);
        fb.draw_line_blended(0, 0, 3, 0, 0xFFFFFF, 64);
        assert!(fb.color.iter().all(|pixel| *pixel > 0 && *pixel < 0xFFFFFF));
    }

    #[test]
    fn bmp_header_has_expected_signature() {
        let mut fb = Framebuffer::new(2, 2);
        fb.clear(0x0012_3456);
        let path = std::env::temp_dir().join("interstellar_framebuffer_test.bmp");
        fb.save_bmp(&path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[0..2], b"BM");
        assert_eq!(u32::from_le_bytes(bytes[18..22].try_into().unwrap()), 2);
        let _ = std::fs::remove_file(path);
    }
}

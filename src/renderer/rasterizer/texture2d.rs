use image::RgbaImage;
use nalgebra::Vector4;

pub struct Texture2D {
    pixels: Vec<Vector4<u8>>,
    width: usize,
    height: usize,
}

impl Texture2D {
    pub fn sample(&self, u: f32, v: f32) -> Vector4<f32> {
        let u = (u * (self.width - 1) as f32) as usize;
        let v = self.height - (v * (self.height - 1) as f32) as usize - 1;

        let u = u.min(self.width - 1);
        let v = v.min(self.height - 1);

        let u8_pixel = self.pixels[v * self.width + u];
        Vector4::new(u8_pixel.x as f32, u8_pixel.y as f32, u8_pixel.z as f32, u8_pixel.w as f32) / 255.0
    }
}

impl From<RgbaImage> for Texture2D {
    fn from(value: RgbaImage) -> Self {
        Self {
            pixels: value.pixels().map(|p| Vector4::new(p[0], p[1], p[2], p[3])).collect(),
            width: value.width() as usize,
            height: value.height() as usize,
        }
    }
}
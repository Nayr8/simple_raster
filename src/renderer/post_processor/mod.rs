use rayon::prelude::*;

pub struct PostProcessorOptions {
    pub fxaa: bool,
}

pub struct PostProcessor {
    options: PostProcessorOptions,
    width: usize,
    height: usize,
    buffer: Vec<u32>,
}

impl PostProcessor {
    pub fn new(width: usize, height: usize, options: PostProcessorOptions) -> Self {
        Self {
            width,
            height,
            options,
            buffer: vec![0; width * height],
        }
    }
    
    pub fn process(&mut self, buffer: &mut [u32]) {
        if self.options.fxaa {
            self.run_fxaa(buffer);
        }
    }
    
    fn run_fxaa(&mut self, buffer: &mut [u32]) {
        let width = self.width;
        let height = self.height;
        
        self.buffer.par_chunks_mut(width)
            .enumerate()
            .for_each(|(y, row)| {
                if y == 0 || y == height - 1 {
                    row.copy_from_slice(&buffer[y * width..(y + 1) * width]);
                    return;
                }
                
                for x in 0..width {
                    if x == 0 || x == width - 1 {
                        row[x] = buffer[y * width + x];
                        continue;
                    }
                    
                    Self::run_fxaa_for_pixel(buffer, row, x, y, width);
                }
            });
        
        buffer.copy_from_slice(&self.buffer);
    }
    
    fn run_fxaa_for_pixel(buffer: &[u32], row: &mut [u32], x: usize, y: usize, width: usize) {
        let index = y * width + x;
        
        let left_luma = Self::luminance(buffer[index - 1]);
        let right_luma = Self::luminance(buffer[index + 1]);
        let top_luma = Self::luminance(buffer[index - width]);
        let bottom_luma = Self::luminance(buffer[index + width]);
        
        let luma_diff = (left_luma - right_luma).abs() + (top_luma - bottom_luma).abs();
        let luma_diff_threshold = 0.1;
        
        if luma_diff > luma_diff_threshold {
            let mut r_sum = 0;
            let mut g_sum = 0;
            let mut b_sum = 0;
            
            for offset_y in (y - 1)..=(y + 1) {
                for offset_x in (x - 1)..=(x + 1) {
                    let index = offset_y * width + offset_x;
                    let pixel = buffer[index];
                    r_sum += (pixel >> 16) & 0xff;
                    g_sum += (pixel >> 8) & 0xff;
                    b_sum += pixel & 0xff;
                }
            }
            
            let r_avg = r_sum / 9;
            let g_avg = g_sum / 9;
            let b_avg = b_sum / 9;

            row[x] = (r_avg << 16) | (g_avg << 8) | b_avg;
        } else {
            row[x] = buffer[index];
        }
    }
    
    fn luminance(pixel: u32) -> f32 {
        let r = (pixel >> 16) & 0xff;
        let g = (pixel >> 8) & 0xff;
        let b = pixel & 0xff;
        (0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32) / 255.0
    }
}
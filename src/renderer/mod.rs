use crate::renderer::post_processor::{PostProcessor, PostProcessorOptions};
use crate::renderer::rasterizer::{RasterOptions, Rasterizer};

pub mod rasterizer;
pub mod post_processor;




pub struct RendererOptions {
    pub raster_options: RasterOptions,
    pub post_processor_options: PostProcessorOptions,
}

pub struct Renderer {
    pub rasterizer: Rasterizer,
    post_processor: PostProcessor,
}

impl Renderer {
    pub fn new(width: usize, height: usize, options: RendererOptions) -> Self {
        Self {
            rasterizer: Rasterizer::new(width, height, options.raster_options),
            post_processor: PostProcessor::new(width, height, options.post_processor_options),       
        }
    }
    
    pub fn render(&mut self, buffer: &mut [u32]) {
        let now = std::time::Instant::now();
        self.rasterizer.render_to_buffer(buffer);
        println!("Rasterization took {} ns", now.elapsed().as_nanos());
        let now = std::time::Instant::now();
        self.post_processor.process(buffer);
        println!("Post processing took {} ns", now.elapsed().as_nanos());
    }
}
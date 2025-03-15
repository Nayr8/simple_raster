use crate::mesh::{Face, Mesh};
use crate::shader::{FragmentShaderInputVariables, Shader, VertexShaderInputVariables, VertexShaderOutputVariables};
use nalgebra::{Matrix4, Vector2, Vector3, Vector4};
use rayon::prelude::*;
use crate::renderer::rasterizer::alpha_buffer::{Fragment, RenderBufferPixel};
use crate::renderer::rasterizer::bounding_box::BoundingBox;
use crate::renderer::rasterizer::storage::Storage;

pub mod texture2d;
mod bounding_box;
pub mod storage;
mod alpha_buffer;

pub struct RasterOptions {
    pub cull_backfaces: bool,
    pub background_colour: Vector3<f32>,
}

pub struct Rasterizer {
    width: usize,
    height: usize,
    storage: Storage,
    viewport: Matrix4<f32>,
    options: RasterOptions,
    render_buffer: Vec<RenderBufferPixel>
}

impl Rasterizer {
    pub fn new(width: usize, height: usize, options: RasterOptions) -> Self {
        let viewport = Self::build_viewport_matrix((0.0, 0.0), width as f32, height as f32);
        
        let mut alpha_buffer = Vec::with_capacity(width * height);
        for _ in 0..width * height {
            alpha_buffer.push(RenderBufferPixel::new(options.background_colour));
        }

        Self {
            width,
            storage: Storage::default(),
            height,
            viewport,
            options,
            render_buffer: alpha_buffer,
        }
    }

    fn build_viewport_matrix(margin: (f32, f32), width: f32, height: f32) -> Matrix4<f32> {
        Matrix4::new(
            width / 2.0, 0.0,           0.0, margin.0 + width / 2.0,
            0.0,       -height / 2.0, 0.0, margin.1 + height / 2.0,
            0.0,       0.0 ,         1.0, 0.0,
            0.0 ,      0.0,          0.0, 1.0
        )
    }
    
    fn calculate_barycentric_coordinates(
        vertex_positions: [Vector2<f32>; 3],
        pixel: Vector2<f32>,
    ) -> Vector3<f32> {
        let [a, b, c] = vertex_positions;

        // Calculate the area of the full triangle using cross product
        let area = 0.5 * (
            (b.x - a.x) * (c.y - a.y) -
                (c.x - a.x) * (b.y - a.y)
        );

        // Calculate barycentric coordinates using areas of sub-triangles
        let alpha = 0.5 * (
            (b.x - pixel.x) * (c.y - pixel.y) -
                (c.x - pixel.x) * (b.y - pixel.y)
        ) / area;

        let beta = 0.5 * (
            (c.x - pixel.x) * (a.y - pixel.y) -
                (a.x - pixel.x) * (c.y - pixel.y)
        ) / area;

        let gamma = 1.0 - alpha - beta;

        Vector3::new(alpha, beta, gamma)
    }

    fn cull_triangle(vertex_positions: &[Vector4<f32>; 3], options: &RasterOptions) -> bool {
        Self::triangle_outside_screen(vertex_positions)
            || (options.cull_backfaces && Self::is_backface(vertex_positions))
    }

    pub fn draw_mesh(&mut self, mesh: &Mesh, shader: &impl Shader) {
        let faces = mesh.faces.iter().map(|face| {
            let vertex_outputs = self.run_vertex_shader(&face, shader);
            let vertex_positions = [
                vertex_outputs[0].position,
                vertex_outputs[1].position,
                vertex_outputs[2].position,
            ];

            (vertex_positions, vertex_outputs)
        }).collect::<Vec<_>>();

        let num_threads = rayon::current_num_threads();

        let rows_per_thread = self.height / num_threads;
        
        self.render_buffer.par_chunks_mut(self.width * rows_per_thread)
            .enumerate()
            .for_each(|(row_num, alpha_buffer_row)| {
                let start = row_num * rows_per_thread;
                let end = start + rows_per_thread;
                
                let bounding_box = BoundingBox::new(Vector2::new(0, start), Vector2::new(self.width - 1, end));

                for (vertex_positions, vertex_outputs) in &faces {
                    Self::draw_triangle(vertex_positions, &self.options, &self.viewport, bounding_box, self.width, start, alpha_buffer_row, &self.storage, vertex_outputs, shader);
                }
        });
    }
    
    fn draw_triangle(
        vertex_positions: &[Vector4<f32>; 3],
        options: &RasterOptions,
        viewport: &Matrix4<f32>,
        bounding_box: BoundingBox,
        width: usize,
        start: usize,
        alpha_buffer_row: &mut [RenderBufferPixel],
        storage: &Storage,
        vertex_outputs: &[VertexShaderOutputVariables; 3],
        shader: &impl Shader,
    ) {
        if Self::cull_triangle(vertex_positions, options) { return }

        let screen_coords_pre_perspective = [
            viewport * vertex_positions[0],
            viewport * vertex_positions[1],
            viewport * vertex_positions[2],
        ];

        let screen_coords_2d = [
            screen_coords_pre_perspective[0].xy() / screen_coords_pre_perspective[0].w,
            screen_coords_pre_perspective[1].xy() / screen_coords_pre_perspective[1].w,
            screen_coords_pre_perspective[2].xy() / screen_coords_pre_perspective[2].w,
        ];

        let triangle_bounding_box = BoundingBox::from_triangle(screen_coords_2d, bounding_box);

        for x in triangle_bounding_box.x_iter() {
            for y in triangle_bounding_box.y_iter() {
                let bary_coords = Self::calculate_barycentric_coordinates(screen_coords_2d, Vector2::new(x as f32, y as f32));
                if (bary_coords.x < 0.0) || (bary_coords.y < 0.0) || (bary_coords.z < 0.0) { continue; }

                let bary_clip = Vector3::new(
                    bary_coords.x / screen_coords_pre_perspective[0].w,
                    bary_coords.y / screen_coords_pre_perspective[1].w,
                    bary_coords.z / screen_coords_pre_perspective[2].w,
                );
                let bary_clip = bary_clip / (bary_clip.x + bary_clip.y + bary_clip.z);

                let frag_depth = Self::get_frag_depth(vertex_positions, bary_clip);

                let index = x + y * width;
                let alpha_buffer_row_index = index - start * width;

                Self::draw_pixel(alpha_buffer_row_index, frag_depth, alpha_buffer_row, storage, bary_clip, vertex_outputs, shader);
            }
        }
    }
    
    fn draw_pixel(
        alpha_buffer_row_index: usize,
        frag_depth: f32,
        alpha_buffer_row: &mut [RenderBufferPixel],
        storage: &Storage,
        bary_clip: Vector3<f32>,
        vertex_outputs: &[VertexShaderOutputVariables; 3],
        shader: &impl Shader,
    ) {
        if frag_depth >= alpha_buffer_row[alpha_buffer_row_index].get_background().depth { return }

        let Some(colour) = Self::run_fragment_shader(storage, bary_clip, vertex_outputs, shader) else { return };

        let alpha = colour.w;

        if alpha <= 0.0001 { return }

        alpha_buffer_row[alpha_buffer_row_index].add(Fragment {
            colour,
            depth: frag_depth,
        });
    }

    fn triangle_outside_screen(vertex_positions: &[Vector4<f32>; 3]) -> bool {
        for vertex in vertex_positions {
            if (vertex.x < -vertex.w || vertex.x > vertex.w) &&
                (vertex.y < -vertex.w || vertex.y > vertex.w) &&
                (vertex.z < -vertex.w || vertex.z > vertex.w) {
                return true;
            }
        }
        false
    }

    fn is_backface(vertex_positions: &[Vector4<f32>; 3]) -> bool {
        let edge1 = vertex_positions[1] - vertex_positions[0];
        let edge2 = vertex_positions[2] - vertex_positions[0];

        let normal = Vector3::new(
            edge1.y * edge2.z - edge1.z * edge2.y,
            edge1.z * edge2.x - edge1.x * edge2.z,
            edge1.x * edge2.y - edge1.y * edge2.x,
        );

        let view_direction = Vector3::new(0.0, 0.0, 1.0);

        normal.dot(&view_direction) <= 0.0
    }

    fn get_frag_depth(vertex_positions: &[Vector4<f32>; 3], bary_clip: Vector3<f32>) -> f32 {
        bary_clip.dot(&Vector3::new(
            vertex_positions[0].z,
            vertex_positions[1].z,
            vertex_positions[2].z
        ))
    }
    
    pub fn render_to_buffer(&mut self, buffer: &mut [u32]) {
        if buffer.len() != self.width * self.height {
            panic!("Buffer length does not match image size");
        }
        
        for index in 0..self.width * self.height {
            let colour = self.render_buffer[index].resolve(self.options.background_colour);
            
            buffer[index] = Self::convert_colour_to_u32(colour);
        }
    }

    fn convert_colour_to_u32(colour: Vector3<f32>) -> u32 {
        let r = (colour.x * 255.0) as u8 as u32;
        let g = (colour.y * 255.0) as u8 as u32;
        let b = (colour.z * 255.0) as u8 as u32;
        (r << 16) | (g << 8) | b
    }

    fn run_vertex_shader(&self, face: &Face, shader: &impl Shader) -> Box<[VertexShaderOutputVariables; 3]> {
        let mut vertex_outputs = Vec::with_capacity(3);
        for vertex in &face.vertices {
            let input_vars = VertexShaderInputVariables {
                position: vertex.position,
                texture_coords: vertex.texture_coords,
                normal: vertex.normals,
                storage: &self.storage,
            };
            let output_vars = shader.vertex(input_vars);
            vertex_outputs.push(output_vars);
        }
        match vertex_outputs.try_into().map(Box::new) {
            Ok(value) => value,
            Err(_) => {
                panic!("Vertex shader output array too large");
            }
        }
    }

    fn run_fragment_shader(storage: &Storage, bary_coords: Vector3<f32>, vertex_outputs: &[VertexShaderOutputVariables; 3], shader: &impl Shader) -> Option<Vector4<f32>> {
        let input_vars = FragmentShaderInputVariables::new(vertex_outputs, bary_coords, storage);
        shader.fragment(input_vars)
    }

    pub fn storage_mut(&mut self) -> &mut Storage {
        &mut self.storage
    }
}



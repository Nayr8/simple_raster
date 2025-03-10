use nalgebra::{RealField, Vector2, Vector3, Vector4};
use crate::mesh::{Face, Mesh};
use crate::rasterizer::bounding_box::BoundingBox;
use crate::rasterizer::storage::Storage;
use crate::shader::{FragmentShaderInputVariables, Shader, VertexShaderInputVariables, VertexShaderOutputVariables};

pub mod texture2d;
mod bounding_box;
pub mod storage;

pub struct Rasterizer<'a> {
    buffer: &'a mut [u32],
    width: usize,
    height: usize,
    z_buffer: Vec<f32>,
    alpha_buffer: Vec<f32>,
    storage: Storage,
}

impl<'a> Rasterizer<'a> {
    pub fn new(buffer: &'a mut [u32], width: usize, height: usize) -> Self {
        Self {
            z_buffer: vec![f32::min_value().unwrap(); width * height],
            alpha_buffer: vec![0.0; width * height],
            buffer,
            width,
            storage: Storage::default(),
            height,
        }
    }

    pub fn clear(&mut self) {
        self.buffer.fill(0);
        self.alpha_buffer.fill(0.0);
        self.z_buffer.fill(f32::MIN);
    }

    fn calculate_barycentric_coordinates(vertex_positions: [Vector4<f32>; 3], pixel: Vector2<f32>) -> Vector3<f32> {
        let ux = Vector3::new(
            vertex_positions[2].x - vertex_positions[0].x,
            vertex_positions[1].x - vertex_positions[0].x,
            vertex_positions[0].x - pixel.x,
        );

        let uy = Vector3::new(
            vertex_positions[2].y - vertex_positions[0].y,
            vertex_positions[1].y - vertex_positions[0].y,
            vertex_positions[0].y - pixel.y,
        );

        let u = ux.cross(&uy);

        if u.z.abs() < 1e-5 { return Vector3::<f32>::new(-1.0, 1.0, 1.0) };

        let inv_z = 1.0 / u.z;
        Vector3::new(
            1.0 - (u.x + u.y) * inv_z,
            u.y * inv_z,
            u.x * inv_z
        )
    }

    fn draw_triangle(&mut self, mut vertex_positions: [Vector4<f32>; 3], vertex_outputs: &[VertexShaderOutputVariables; 3], shader: &impl Shader) {
        if Self::triangle_outside_screen(&vertex_positions)
            || Self::is_backface(&vertex_positions) { return }

        self.convert_vertices_to_screen_space(&mut vertex_positions);

        let bounding_box = BoundingBox::calculate(vertex_positions, self.width, self.height);

        for x in bounding_box.x_iter() {
            for y in bounding_box.y_iter() {
                self.draw_pixel(vertex_positions, x, y, &vertex_outputs, shader);
            }
        }
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

    fn convert_vertices_to_screen_space(&self, vertex_positions: &mut [Vector4<f32>; 3]) {
        for i in 0..3 {
            let v = vertex_positions[i];
            vertex_positions[i] = Vector4::new(
                (v.x + 1.0) * 0.5 * self.width as f32,
                (1.0 - v.y) * 0.5 * self.height as f32,
                v.z,
                v.w
            );
        }
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

    fn get_frag_depth(vertex_positions: [Vector4<f32>; 3], bary_coords: Vector3<f32>) -> f32 {
        let z =
            vertex_positions[0].z * bary_coords[0] +
            vertex_positions[1].z * bary_coords[1] +
            vertex_positions[2].z * bary_coords[2];
        let w =
            vertex_positions[0].w * bary_coords[0] +
            vertex_positions[1].w * bary_coords[1] +
            vertex_positions[2].w * bary_coords[2];
        z / w
    }

    #[inline(always)]
    fn draw_pixel(&mut self, vertex_positions: [Vector4<f32>; 3], x: usize, y: usize, vertex_outputs: &[VertexShaderOutputVariables; 3], shader: &impl Shader) {
        let bary_coords = Self::calculate_barycentric_coordinates(vertex_positions, Vector2::new(x as f32, y as f32));
        if (bary_coords.x < 0.0) || (bary_coords.y < 0.0) || (bary_coords.z < 0.0) { return; }

        let frag_depth = Self::get_frag_depth(vertex_positions, bary_coords);

        if frag_depth < 0.0 || frag_depth > 1.0 { return }

        let index = x + y * self.width;
        if self.z_buffer[index] >= frag_depth { return }

        let Some(colour) = self.run_fragment_shader(bary_coords, vertex_outputs, shader) else { return };

        let fragment_alpha = colour.w;

        let base_colour = Self::convert_u32_to_colour(self.get_pixel(x, y));
        let accumulated_alpha = self.alpha_buffer[index];

        let alpha_contribution = fragment_alpha - accumulated_alpha;
        let blended_colour = {
            let foreground = colour.xyz() * alpha_contribution;
            let background = base_colour.xyz() * (1.0 - alpha_contribution);
            foreground + background
        };

        let final_alpha = fragment_alpha + accumulated_alpha * (1.0 - fragment_alpha);
        self.alpha_buffer[index] = final_alpha;

        self.set_pixel(x, y, Self::convert_colour_to_u32(blended_colour));

        if fragment_alpha == 1.0 {
            self.z_buffer[index] = frag_depth;
        }
    }

    fn convert_colour_to_u32(colour: Vector3<f32>) -> u32 {
        let r = (colour.x * 255.0) as u8 as u32;
        let g = (colour.y * 255.0) as u8 as u32;
        let b = (colour.z * 255.0) as u8 as u32;
        (r << 16) | (g << 8) | b
    }

    fn convert_u32_to_colour(colour: u32) -> Vector3<f32> {
        let r = (colour >> 16 & 0xFF) as f32 / 255.0;
        let g = (colour >> 8 & 0xFF) as f32 / 255.0;
        let b = (colour & 0xFF) as f32 / 255.0;
        Vector3::new(r, g, b)
    }

    pub fn draw_mesh(&mut self, mesh: &Mesh, shader: &impl Shader) {
        let faces = mesh.faces.iter().map(|face| {
            let vertex_outputs = self.run_vertex_shader(&face, shader);
            let mut vertex_positions = [Vector4::default(); 3];
            for (i, vertex_output) in vertex_outputs.iter().enumerate() {
                vertex_positions[i] = Vector4::new(
                    vertex_output.position.x / vertex_output.position.w,
                    vertex_output.position.y / vertex_output.position.w,
                    vertex_output.position.z / vertex_output.position.w,
                    vertex_output.position.w,
                );

            }

            (vertex_positions, vertex_outputs)
        }).collect::<Vec<_>>();

        for (vertex_position, vertex_outputs) in faces {
            self.draw_triangle(vertex_position, &vertex_outputs, shader);
        }
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

    fn run_fragment_shader(&self, bary_coords: Vector3<f32>, vertex_outputs: &[VertexShaderOutputVariables; 3], shader: &impl Shader) -> Option<Vector4<f32>> {
        let input_vars = FragmentShaderInputVariables::new(vertex_outputs, bary_coords, &self.storage);
        shader.fragment(input_vars)
    }

    pub fn storage_mut(&mut self) -> &mut Storage {
        &mut self.storage
    }

    fn set_pixel(&mut self, x: usize, y: usize, colour: u32) {
        if (x >= self.width) || (y >= self.height) {
            return;
        }
        let index = y * self.width + x;
        self.buffer[index] = colour;
    }

    fn get_pixel(&self, x: usize, y: usize) -> u32 {
        if (x >= self.width) || (y >= self.height) {
            return 0;
        }
        let index = y * self.width + x;
        self.buffer[index]
    }

    pub fn buffer(&self) -> &[u32] {
        self.buffer
    }
}



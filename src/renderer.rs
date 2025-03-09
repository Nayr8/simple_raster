use image::RgbaImage;
use nalgebra::{Matrix4, RealField, Vector2, Vector3, Vector4};
use crate::Image;
use crate::mesh::{Face, Mesh};
use crate::shader::{FragmentShaderInputVariables, Shader, VertexShaderInputVariables, VertexShaderOutputVariables};

pub struct Renderer<'a> {
    pub(crate) image: &'a mut Image,
    z_buffer: Vec<f32>,
    storage: Storage,
}

impl<'a> Renderer<'a> {
    pub fn new(image: &'a mut Image) -> Self {
        Self {
            z_buffer: vec![f32::min_value().unwrap(); image.width * image.height],
            image,
            storage: Storage::default(),
        }
    }

    pub fn clear(&mut self) {
        self.image.clear();
        self.z_buffer.iter_mut().for_each(|p| *p = f32::min_value().unwrap());
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

    pub fn draw_triangle(&mut self, mut vertex_positions: [Vector4<f32>; 3], vertex_outputs: &[VertexShaderOutputVariables; 3], shader: &impl Shader) {
        for vertex in &vertex_positions {
            if (vertex.x < -vertex.w || vertex.x > vertex.w) &&
                (vertex.y < -vertex.w || vertex.y > vertex.w) &&
                (vertex.z < -vertex.w || vertex.z > vertex.w) {
                return;
            }
        }

        for i in 0..3 {
            let v = vertex_positions[i];
            vertex_positions[i] = Vector4::new(
                (v.x + 1.0) * 0.5 * self.image.width as f32,
                (1.0 - v.y) * 0.5 * self.image.height as f32,
                v.z,
                v.w
            );
        }

        let bounding_box = BoundingBox::calculate2(vertex_positions, self.image.width, self.image.height);

        for x in bounding_box.min.x..=bounding_box.max.x {
            for y in bounding_box.min.y..=bounding_box.max.y {
                self.draw_pixel(vertex_positions, x, y, &vertex_outputs, shader);
            }
        }
    }

    #[inline(always)]
    fn draw_pixel(&mut self, vertex_positions: [Vector4<f32>; 3], x: usize, y: usize, vertex_outputs: &[VertexShaderOutputVariables; 3], shader: &impl Shader) {
        let bary_coords = Self::calculate_barycentric_coordinates(vertex_positions, Vector2::new(x as f32, y as f32));
        if (bary_coords.x < 0.0) || (bary_coords.y < 0.0) || (bary_coords.z < 0.0) { return; }

        let z =
            vertex_positions[0].z * bary_coords[0] +
            vertex_positions[1].z * bary_coords[1] +
            vertex_positions[2].z * bary_coords[2];
        let w =
            vertex_positions[0].w * bary_coords[0] +
                vertex_positions[1].w * bary_coords[1] +
                vertex_positions[2].w * bary_coords[2];
        let frag_depth = z / w;

        if frag_depth < 0.0 || frag_depth > 1.0 { return }

        let index = x + y * self.image.width;
        if self.z_buffer[index] < frag_depth {
            self.z_buffer[index] = frag_depth;

            match self.run_fragment_shader(bary_coords, vertex_outputs, shader) {
                Some(colour) => {
                    let colour_u32 = colour.map(|c| (c * 255.0) as u8 as u32);
                    let pixel_value = (colour_u32.x << 16) | (colour_u32.y << 8) | colour_u32.z;

                    self.image.set_pixel(x, y, pixel_value);
                },
                None => {} // Discard
            }
        }
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
                );//vertex_output.position.xyz() / vertex_output.position.w;

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

    fn run_fragment_shader(&self, bary_coords: Vector3<f32>, vertex_outputs: &[VertexShaderOutputVariables; 3], shader: &impl Shader) -> Option<Vector3<f32>> {
        let input_vars = FragmentShaderInputVariables::new(vertex_outputs, bary_coords, &self.storage);
        shader.fragment(input_vars)
    }

    pub fn storage_mut(&mut self) -> &mut Storage {
        &mut self.storage
    }
}


#[derive(Default)]
pub struct Storage {
    textures2d: Vec<Texture2D>,
    f32s: Vec<f32>,
    mat4s: Vec<nalgebra::Matrix4<f32>>,
}

impl Storage {
    pub fn set_texture2ds(&mut self, textures: Vec<Texture2D>) {
        self.textures2d = textures;
    }

    pub fn get_texture2d(&self, index: usize) -> &Texture2D {
        &self.textures2d[index]
    }
    pub fn set_f32s(&mut self, f32s: Vec<f32>) {
        self.f32s = f32s;
    }

    pub fn get_f32(&self, index: usize) -> f32 {
        self.f32s[index]
    }

    pub fn set_mat4s(&mut self, mat4s: Vec<Matrix4<f32>>) {
        self.mat4s = mat4s;
    }

    pub fn get_mat4(&self, index: usize) -> &Matrix4<f32> {
        &self.mat4s[index]
    }
}

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

struct BoundingBox {
    min: Vector2<usize>,
    max: Vector2<usize>,
}

impl BoundingBox {
    fn calculate(face: &Face, width: usize, height: usize) -> Self {
        let clamp = Vector2::new(width as f32 - 1.0, height as f32 - 1.0);
        let mut bounding_box_min = clamp;
        let mut bounding_box_max = Vector2::new(0.0_f32, 0.0);

        for vertex in &face.vertices {
            bounding_box_min.x = bounding_box_min.x.min(vertex.position.x);
            bounding_box_min.y = bounding_box_min.y.min(vertex.position.y);

            bounding_box_max.x = bounding_box_max.x.max(vertex.position.x).min(clamp.x);
            bounding_box_max.y = bounding_box_max.y.max(vertex.position.y).min(clamp.y);
        }

        Self {
            min: Vector2::new(bounding_box_min.x as usize, bounding_box_min.y as usize),
            max: Vector2::new(bounding_box_max.x as usize, bounding_box_max.y as usize),
        }
    }
    fn calculate2(vertex_positions: [Vector4<f32>; 3], width: usize, height: usize) -> Self {
        let clamp = Vector2::new(width as f32 - 1.0, height as f32 - 1.0);
        let mut bounding_box_min = clamp;
        let mut bounding_box_max = Vector2::new(0.0_f32, 0.0);

        for vertex in &vertex_positions {
            bounding_box_min.x = bounding_box_min.x.min(vertex.x);
            bounding_box_min.y = bounding_box_min.y.min(vertex.y);

            bounding_box_max.x = bounding_box_max.x.max(vertex.x).min(clamp.x);
            bounding_box_max.y = bounding_box_max.y.max(vertex.y).min(clamp.y);
        }

        Self {
            min: Vector2::new(bounding_box_min.x as usize, bounding_box_min.y as usize),
            max: Vector2::new(bounding_box_max.x as usize, bounding_box_max.y as usize),
        }
    }
}
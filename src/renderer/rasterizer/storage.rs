use nalgebra::Matrix4;
use crate::renderer::rasterizer::texture2d::Texture2D;

#[derive(Default)]
pub struct Storage {
    textures2d: Vec<Texture2D>,
    textures2d_indices: Vec<usize>,
    f32s: Vec<f32>,
    mat4s: Vec<Matrix4<f32>>,
}

impl Storage {
    pub fn set_texture2ds(&mut self, textures: Vec<Texture2D>) {
        self.textures2d = textures;
    }

    pub fn set_texture2d_indices(&mut self, indices: Vec<usize>) {
        self.textures2d_indices = indices;
    }

    pub fn get_texture2d(&self, index: usize) -> &Texture2D {
        let index = self.textures2d_indices[index];
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


use crate::rasterizer::storage::Storage;
use nalgebra::{Vector2, Vector3, Vector4};

pub trait Shader : Send + Sync {
    fn vertex(&self, input_vars: VertexShaderInputVariables) -> VertexShaderOutputVariables;
    fn fragment(&self, input_vars: FragmentShaderInputVariables) -> Option<Vector3<f32>>;
}


pub struct BasicShader;

impl Shader for BasicShader {
    fn vertex(&self, input_vars: VertexShaderInputVariables) -> VertexShaderOutputVariables {
        let view_projection = input_vars.storage.get_mat4(0);

        let position = view_projection * input_vars.position;

        VertexShaderOutputVariables {
            position,
            vec2: vec![input_vars.texture_coords.xy()],
            ..Default::default()
        }

    }

    fn fragment(&self, input_vars: FragmentShaderInputVariables) -> Option<Vector3<f32>> {
        let uvs = input_vars.get_input_vec2(0);

        let texture = input_vars.storage.get_texture2d(0);
        let base_colour = texture.sample(uvs.x, uvs.y);


        Some(base_colour.xyz())
    }
}

pub struct VertexShaderInputVariables<'a> {
    pub position: Vector4<f32>,
    pub texture_coords: Vector3<f32>,
    pub normal: Vector3<f32>,

    pub storage: &'a Storage,
}

#[derive(Default)]
pub struct VertexShaderOutputVariables {
    pub position: Vector4<f32>,

    pub vec2: Vec<Vector2<f32>>,
    pub vec3: Vec<Vector3<f32>>,
    pub vec4: Vec<Vector4<f32>>,
}

pub struct FragmentShaderInputVariables<'a> {
    vertex_shader_output_variables: &'a [VertexShaderOutputVariables; 3],
    bary_coords: Vector3<f32>,

    pub storage: &'a Storage,
}

impl<'a> FragmentShaderInputVariables<'a> {
    pub fn new(vertex_shader_output_variables: &'a [VertexShaderOutputVariables; 3], bary_coords: Vector3<f32>, storage: &'a Storage,) -> Self {
        Self {
            vertex_shader_output_variables,
            bary_coords,
            storage,
        }
    }

    pub fn get_position(&self) -> Vector4<f32> {
        self.vertex_shader_output_variables[0].position * self.bary_coords.x +
        self.vertex_shader_output_variables[1].position * self.bary_coords.y +
        self.vertex_shader_output_variables[2].position * self.bary_coords.z
    }

    pub fn get_input_vec2(&self, index: usize) -> Vector2<f32> {
        self.vertex_shader_output_variables[0].vec2[index] * self.bary_coords.x +
        self.vertex_shader_output_variables[1].vec2[index] * self.bary_coords.y +
        self.vertex_shader_output_variables[2].vec2[index] * self.bary_coords.z
    }

    pub fn get_input_vec3(&self, index: usize) -> Vector3<f32> {
        self.vertex_shader_output_variables[0].vec3[index] * self.bary_coords.x +
        self.vertex_shader_output_variables[1].vec3[index] * self.bary_coords.y +
        self.vertex_shader_output_variables[2].vec3[index] * self.bary_coords.z
    }

    pub fn get_input_vec4(&self, index: usize) -> Vector4<f32> {
        self.vertex_shader_output_variables[0].vec4[index] * self.bary_coords.x +
        self.vertex_shader_output_variables[1].vec4[index] * self.bary_coords.y +
        self.vertex_shader_output_variables[2].vec4[index] * self.bary_coords.z
    }
}
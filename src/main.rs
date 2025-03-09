use std::fs::File;
use std::io::BufReader;
use minifb::Key;
use std::path::Path;
use std::time::Instant;
use nalgebra::{Matrix4, Perspective3, Point3, Projective3, Rotation3, Translation3, Vector3};
use crate::mesh::ObjLoader;
use crate::renderer::Renderer;
use crate::shader::BasicShader;

mod mesh;
mod shader;
mod renderer;

struct Image {
    width: usize,
    height: usize,
    buffer: Box<[u32]>,
}

impl Image {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            buffer: vec![0; width * height].into_boxed_slice(),
        }
    }

    pub fn clear(&mut self) {
        self.buffer.iter_mut().for_each(|p| *p = 0);
    }


    fn run(&mut self) {
        let mut window = minifb::Window::new("Simple Raster", self.width, self.height, minifb::WindowOptions::default()).unwrap();

        while window.is_open() && !window.is_key_down(Key::Escape) {
            window.update_with_buffer(&self.buffer, self.width, self.height).unwrap();
        }
    }

    fn set_pixel(&mut self, x: usize, y: usize, colour: u32) {
        if (x >= self.width) || (y >= self.height) {
            return;
        }
        let index = y * self.width + x;
        self.buffer[index] = colour;
    }
}

fn load_texture(path: impl AsRef<Path>) -> Option<image::RgbaImage> {
    let img = image::open(path).ok()?;
    Some(img.to_rgba8())
}

struct PerspectiveCamera {
    position: Point3<f32>,
    rotation: Vector3<f32>,
    view: Matrix4<f32>,
    projection: Matrix4<f32>,
    view_projection: Matrix4<f32>,
}

impl PerspectiveCamera {
    fn new(position: Point3<f32>, rotation: Vector3<f32>, fov: f32, aspect: f32, z_near: f32, z_far: f32) -> Self {
        let mut camera = Self {
            position,
            rotation,
            view: Matrix4::identity(),
            projection: Self::perspective_projection(fov, aspect, z_near, z_far),
            view_projection: Matrix4::identity(),
        };
        camera.update_view();
        camera
    }

    fn perspective_projection(fovy: f32, aspect: f32, z_near: f32, z_far: f32) -> Matrix4<f32> {
        let m11 = 1.0 / (aspect * (fovy/2.0).tan());
        let m22 = 1.0 / (fovy/2.0).tan();
        let m33 = -(z_far + z_near) / (z_far - z_near);
        let m34 = -(2.0 * z_far * z_near) / (z_far - z_near);

        Matrix4::new(
            m11, 0.0, 0.0, 0.0,
            0.0, m22, 0.0, 0.0,
            0.0, 0.0, m33, m34,
            0.0, 0.0, -1.0, 0.0,
        )
    }

    fn update_view(&mut self) {
        let roll = Rotation3::from_axis_angle(&Vector3::z_axis(), self.rotation.z);
        let pitch = Rotation3::from_axis_angle(&Vector3::x_axis(), self.rotation.x);
        let yaw = Rotation3::from_axis_angle(&Vector3::y_axis(), self.rotation.y);


        let rotate = roll * pitch * yaw;

        let translate = Translation3::from(-self.position);

        self.view = Matrix4::from(rotate) * Matrix4::from(translate);
        self.view_projection = self.projection * self.view
    }
}

fn main() {
    const WIDTH: usize = 1280;
    const HEIGHT: usize = 720;

    let mut image = Image::new(WIDTH, HEIGHT);


    let mut mesh_loader = ObjLoader::new();
    let file = File::open("african_head.obj").unwrap();
    let mut meshes = mesh_loader.parse(BufReader::new(file));
    let mesh = &meshes[0];

    let texture = load_texture("african_head_diffuse.tga").unwrap();

    let mut renderer = Renderer::new(&mut image);

    renderer.storage_mut().set_texture2ds(vec![texture.into()]);

    let fovy = 60.0 * (std::f32::consts::PI / 180.0); // 60 degrees fov y
    let aspect_ratio = WIDTH as f32 / HEIGHT as f32;
    let near = 0.1;
    let far = 100.0;

    let mut camera = PerspectiveCamera::new(
        Point3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 0.0),
        fovy,
        aspect_ratio,
        near,
        far,
    );
    renderer.storage_mut().set_mat4s(vec![
        camera.view_projection,
    ]);


    let shader = BasicShader;

    renderer.draw_mesh(mesh, &shader);


    let mut window = minifb::Window::new("Simple Raster", renderer.image.width, renderer.image.height, minifb::WindowOptions::default()).unwrap();
    window.set_target_fps(100);
    let mut now = Instant::now();
    while window.is_open() && !window.is_key_down(Key::Escape) {
        //camera.rotation.y += 0.005;
        //if (camera.rotation.y > 2.0 * std::f32::consts::PI) {
        //    camera.rotation.y -= 2.0 * std::f32::consts::PI;
        //}
        camera.position.z += 0.005;
        camera.update_view();
        renderer.storage_mut().set_mat4s(vec![
            camera.view_projection,
        ]);

        renderer.draw_mesh(mesh, &shader);
        window.update_with_buffer(&renderer.image.buffer, renderer.image.width, renderer.image.height).unwrap();
        renderer.clear();
        println!("{:?} fps", 1.0 / now.elapsed().as_secs_f64());
        now = Instant::now();
    }
    //image.run();
}

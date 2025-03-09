use crate::mesh::{Face, Mesh, ObjLoader, Vertex};
use crate::rasterizer::Rasterizer;
use crate::shader::BasicShader;
use minifb::Key;
use nalgebra::{Matrix4, Point3, Rotation3, Translation3, Vector3, Vector4};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Instant;

mod mesh;
mod shader;
mod rasterizer;

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

    let mut mesh_loader = ObjLoader::new();
    let file = File::open("african_head.obj").unwrap();
    let meshes = mesh_loader.parse(BufReader::new(file));
    let mesh = &meshes[0];

    let mesh2 = Mesh::new(None, vec![
        Face::new([
            Vertex::from_pos_tex(Vector4::new(0.0, 0.0, 0.0, 1.0), Vector3::new(0.0, 0.0, 0.0)),  // Bottom-left, tex(0.0, 0.0)
            Vertex::from_pos_tex(Vector4::new(1.0, 0.0, 0.0, 1.0), Vector3::new(1.0, 0.0, 0.0)),  // Bottom-right, tex(1.0, 0.0)
            Vertex::from_pos_tex(Vector4::new(1.0, 1.0, 0.0, 1.0), Vector3::new(1.0, 1.0, 0.0)),  // Top-right, tex(1.0, 1.0)
        ]),
        Face::new([
            Vertex::from_pos_tex(Vector4::new(0.0, 0.0, 0.0, 1.0), Vector3::new(0.0, 0.0, 0.0)),  // Bottom-left, tex(0.0, 0.0)
            Vertex::from_pos_tex(Vector4::new(1.0, 1.0, 0.0, 1.0), Vector3::new(1.0, 1.0, 0.0)),  // Top-right, tex(1.0, 1.0)
            Vertex::from_pos_tex(Vector4::new(0.0, 1.0, 0.0, 1.0), Vector3::new(0.0, 1.0, 0.0)),  // Top-left, tex(0.0, 1.0)
        ]),
    ]);

    let texture = load_texture("african_head_diffuse.tga").unwrap();
    let texture2 = load_texture("blending_transparent_window.png").unwrap();

    let mut buffer = vec![0; WIDTH * HEIGHT];
    let mut renderer = Rasterizer::new(&mut buffer, WIDTH, HEIGHT);


    let fovy = 60.0 * (std::f32::consts::PI / 180.0); // 60 degrees fov y
    let aspect_ratio = WIDTH as f32 / HEIGHT as f32;
    let near = 0.1;
    let far = 100.0;

    let mut camera = PerspectiveCamera::new(
        Point3::new(0.0, 0.0, 4.0),
        Vector3::new(0.0, 0.0, 0.0),
        fovy,
        aspect_ratio,
        near,
        far,
    );

    let mut model_rotation_angle = 0.0;
    let model_rotation_speed = 0.01;

    let shader = BasicShader;

    let window_transform = Translation3::from(Vector3::new(0.0, 0.0, 1.0)).to_homogeneous();
    renderer.storage_mut().set_mat4s(vec![
        camera.view_projection,
        window_transform,
    ]);
    renderer.storage_mut().set_texture2ds(vec![texture2.clone().into()]);
    renderer.draw_mesh(&mesh2, &shader);

    let mut model_transform = Matrix4::identity();
    renderer.storage_mut().set_mat4s(vec![
        camera.view_projection,
        model_transform,
    ]);
    renderer.storage_mut().set_texture2ds(vec![texture.clone().into()]);
    let now = Instant::now();
    renderer.draw_mesh(mesh, &shader);

    println!("{:?} fps", 1.0 / now.elapsed().as_secs_f64());

    let window_options = minifb::WindowOptions {
        resize: true,
        scale_mode: minifb::ScaleMode::Stretch,
        ..Default::default()
    };

    let mut window = minifb::Window::new("Simple Raster", WIDTH, HEIGHT, window_options).unwrap();
    window.update_with_buffer(renderer.buffer(), WIDTH, HEIGHT).unwrap();
    window.set_target_fps(100);
    let mut now = Instant::now();
    while window.is_open() && !window.is_key_down(Key::Escape) {
        //continue;

        let movement_speed = 0.05;
        let rotation_speed = 0.02;

        let yaw = camera.rotation.y;
        if window.is_key_down(Key::W) {
            camera.position.x += movement_speed * yaw.sin();
            camera.position.z -= movement_speed * yaw.cos();
        }
        if window.is_key_down(Key::S) {
            camera.position.x -= movement_speed * yaw.sin();
            camera.position.z += movement_speed * yaw.cos();
        }
        if window.is_key_down(Key::A) {
            camera.position.x -= movement_speed * yaw.cos();
            camera.position.z -= movement_speed * yaw.sin();
        }
        if window.is_key_down(Key::D) {
            camera.position.x += movement_speed * yaw.cos();
            camera.position.z += movement_speed * yaw.sin();
        }

        if window.is_key_down(Key::Left) {
            camera.rotation.y -= rotation_speed;
        }
        if window.is_key_down(Key::Right) {
            camera.rotation.y += rotation_speed;
        }

        camera.update_view();

        //model_rotation_angle += model_rotation_speed;
        let model_rotation = Rotation3::from_axis_angle(&Vector3::y_axis(), model_rotation_angle).to_homogeneous();
        model_transform = model_rotation;

        renderer.storage_mut().set_mat4s(vec![
            camera.view_projection,
            window_transform,
        ]);
        renderer.storage_mut().set_texture2ds(vec![texture2.clone().into()]);
        renderer.draw_mesh(&mesh2, &shader);

        renderer.storage_mut().set_mat4s(vec![
            camera.view_projection,
            model_transform,
        ]);
        renderer.storage_mut().set_texture2ds(vec![texture.clone().into()]);
        renderer.draw_mesh(mesh, &shader);

        window.update_with_buffer(renderer.buffer(), WIDTH, HEIGHT).unwrap();
        renderer.clear();
        println!("{:?} fps", 1.0 / now.elapsed().as_secs_f64());
        now = Instant::now();
    }
}


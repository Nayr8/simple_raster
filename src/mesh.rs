use std::io::BufRead;
use std::str::SplitWhitespace;
use nalgebra::{Vector3, Vector4};

pub struct Mesh {
    pub name: Option<String>,
    pub faces: Vec<Face>,
}

impl Mesh {
    pub fn new(name: Option<String>, faces: Vec<Face>) -> Self {
        Self {
            name,
            faces,
        }
    }
}

#[derive(Default, Copy, Clone)]
pub struct Face {
    pub vertices: [Vertex; 3],
}

impl Face {
    pub fn new(vertices: [Vertex; 3]) -> Self {
        Self {
            vertices,
        }
    }
}

#[derive(Default, Copy, Clone)]
pub struct Vertex {
    pub position: Vector4<f32>,
    pub texture_coords: Vector3<f32>,
    pub normals: Vector3<f32>,
}

impl Vertex {
    pub fn from_pos_tex(position: Vector4<f32>, texture_coords: Vector3<f32>) -> Self {
        Self {
            position,
            texture_coords,
            normals: Vector3::new(0.0, 0.0, 1.0),
        }
    }

    pub fn from_pos(position: Vector4<f32>) -> Self {
        Self {
            position,
            texture_coords: Vector3::new(0.0, 0.0, 1.0),
            normals: Vector3::new(0.0, 0.0, 1.0),
        }
    }
}



pub struct ObjLoader {
    positions: Vec<Vector4<f32>>,
    texture_coords: Vec<Vector3<f32>>,
    normals: Vec<Vector3<f32>>,

    meshes: Vec<ObjMesh>,

    // Warnings
    mtllib_is_not_supported: bool,
    mtl_is_not_supported: bool,
    groups_are_not_supported: bool,
}

impl ObjLoader {
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            texture_coords: Vec::new(),
            normals: Vec::new(),
            meshes: Vec::new(),

            mtllib_is_not_supported: false,
            mtl_is_not_supported: false,
            groups_are_not_supported: false,
        }
    }

    pub fn parse(&mut self, reader: impl BufRead) -> Vec<Mesh> {
        self.positions.clear();
        self.texture_coords.clear();
        self.normals.clear();
        self.meshes.clear();
        self.mtllib_is_not_supported = false;
        self.mtl_is_not_supported = false;
        self.groups_are_not_supported = false;


        for line in reader.lines() {
            let Ok(line) = line else { panic!("Failed to read line: {line:?}") };

            self.parse_line(&line);
        }

        if self.texture_coords.is_empty() {
            self.texture_coords.push(Vector3::new(0.0, 0.0, 1.0))
        }

        if self.normals.is_empty() {
            self.normals.push(Vector3::new(0.0, 0.0, 1.0))
        }

        let mut meshes = Vec::with_capacity(self.meshes.len());

        for mut mesh in self.meshes.drain(..) {
            let faces = mesh.faces.drain(..).map(|face| {
                let mut mesh_face = Face::default();
                for i in 0..3 {
                    let vert = face.vertex_indices[i];

                    let position = self.positions[vert.position_index as usize - 1];
                    let texture_coords = self.texture_coords[vert.texcoords_index as usize - 1];
                    let normals = self.normals[vert.normal_index as usize - 1];

                    mesh_face.vertices[i] = Vertex {
                        position,
                        texture_coords,
                        normals,
                    };
                }
                mesh_face
            }).collect::<Vec<_>>();

            meshes.push(Mesh {
                name: mesh.name,
                faces,
            })
        }

        meshes
    }

    fn parse_line(&mut self, line: &str) {
        let mut words = line.split_whitespace();

        let Some(line_prefix) = words.next() else {
            // If invalid we just skip the line
            return;
        };

        match line_prefix {
            "#" => return,
            "v" => self.parse_position(words),
            "vt" => self.parse_texture_coords(words),
            "vn" => self.parse_normal(words),
            "f" => self.parse_face(words),
            "o" => self.parse_object(line.trim_start_matches("o ")),
            "mtllib" => self.mtllib_is_not_supported = true,
            "usemtl" => self.mtl_is_not_supported = true,
            "g" => self.groups_are_not_supported = true,
            _ => {
                // If invalid we just skip the line
                return;
            },
        }
    }

    fn parse_position(&mut self, mut word: SplitWhitespace) {
        let Some(x) = word.next() else { return };
        let Ok(x) = x.parse::<f32>() else { return };

        let Some(y) = word.next() else { return };
        let Ok(y) = y.parse::<f32>() else { return };

        let Some(z) = word.next() else { return };
        let Ok(z) = z.parse::<f32>() else { return };

        let w = word.next().and_then(|w| w.parse::<f32>().ok()).unwrap_or(1.0);

        self.positions.push(Vector4::new(x, y, z, w));
    }

    fn parse_texture_coords(&mut self, mut word: SplitWhitespace) {
        let Some(u) = word.next() else { return };
        let Ok(u) = u.parse::<f32>() else { return };

        let Some(v) = word.next() else { return };
        let Ok(v) = v.parse::<f32>() else { return };

        let w = word.next().and_then(|w| w.parse::<f32>().ok()).unwrap_or(1.0);

        self.texture_coords.push(Vector3::new(u, v, w));
    }

    fn parse_normal(&mut self, mut word: SplitWhitespace) {
        let Some(x) = word.next() else { return };
        let Ok(x) = x.parse::<f32>() else { return };

        let Some(y) = word.next() else { return };
        let Ok(y) = y.parse::<f32>() else { return };

        let Some(z) = word.next() else { return };
        let Ok(z) = z.parse::<f32>() else { return };

        self.normals.push(Vector3::new(x, y, z));
    }

    fn parse_face(&mut self, mut word: SplitWhitespace) {
        if self.meshes.is_empty() {
            self.meshes.push(ObjMesh {
                name: None,
                faces: Vec::new(),
            });
        }

        let mut face = ObjFace::default();
        for i in 0..3 {
            let Some(index) = word.next() else { return };
            let Some(index) = self.parse_face_indices(index) else { return };

            face.vertex_indices[i] = index;
        }

        self.meshes
            .last_mut().unwrap()
            .faces.push(face)
    }

    fn parse_face_indices(&mut self, word: &str) -> Option<ObjFaceIndex> {
        let mut vertex_indices = word.split('/');

        let Some(position_index) = vertex_indices.next() else { return None };
        let Ok(position_index) = position_index.parse::<i32>() else { return None };

        let texcoords_index = vertex_indices.next().and_then(|i| i.parse::<i32>().ok()).unwrap_or(1);
        let normal_index = vertex_indices.next().and_then(|i| i.parse::<i32>().ok()).unwrap_or(1);

        Some(ObjFaceIndex {
            position_index,
            texcoords_index,
            normal_index,
        })
    }

    fn parse_object(&mut self, name: &str) {
        self.meshes.push(ObjMesh {
            name: Some(name.to_string()),
            faces: Vec::new(),
        });
    }
}

struct ObjMesh {
    name: Option<String>,
    faces: Vec<ObjFace>,
}

#[derive(Default, Copy, Clone)]
pub struct ObjFace {
    vertex_indices: [ObjFaceIndex; 3],
}

#[derive(Default, Copy, Clone)]
pub struct ObjFaceIndex {
    position_index: i32,
    texcoords_index: i32,
    normal_index: i32,
}
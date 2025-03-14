use std::collections::LinkedList;
use nalgebra::{Vector3, Vector4};

#[derive(Copy, Clone)]
pub struct Fragment {
    pub colour: Vector4<f32>,
    pub depth: f32,
}


pub struct AlphaBuffer {
    width: usize,
    height: usize,
    fragments: Vec<LinkedList<Fragment>>,
    background: Vec<Fragment>,
    background_colour: Vector4<f32>,
}

impl AlphaBuffer {
    pub fn new(width: usize, height: usize, background_colour: Vector3<f32>) -> Self {
        let mut fragments = Vec::with_capacity(width * height);
        for _ in 0..width * height {
            fragments.push(LinkedList::new());
        }
        
        let background_colour = background_colour.push(1.0);

        AlphaBuffer {
            width,
            height,
            fragments,
            background: vec![Fragment {
                colour: background_colour,
                depth: f32::MAX,
            }; width * height],
            background_colour,
        }
    }

    pub fn add_fragment(&mut self, index: usize, fragment: Fragment) {
        if fragment.colour.w >= 0.9999 {
            self.background[index] = fragment;
        } else {
            self.fragments[index].push_back(fragment);
        }
    }

    pub fn get_background(&self, index: usize) -> &Fragment {
        &self.background[index]
    }

    pub fn resolve_fragment(&mut self, index: usize) -> Vector3<f32> {
        let fragments_list = &mut self.fragments[index];
        let mut fragments = fragments_list.iter().collect::<Vec<_>>();
        fragments.sort_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap());

        let mut result_colour = self.background[index].colour.xyz();
        let background_depth = self.background[index].depth;

        for fragment in fragments {
            if fragment.depth > background_depth { continue }
            
            let alpha = fragment.colour.w;
            
            result_colour = fragment.colour.xyz() * alpha + result_colour * (1.0 - alpha);
        }

        fragments_list.clear();
        self.background[index] = Fragment {
            colour: self.background_colour,
            depth: f32::MAX,
        };

        result_colour
    }
}
use std::collections::LinkedList;
use nalgebra::{Vector3, Vector4};

#[derive(Copy, Clone)]
pub struct Fragment {
    pub colour: Vector4<f32>,
    pub depth: f32,
}

pub struct RenderBufferPixel {
    fragments: LinkedList<Fragment>,
    background: Fragment,
}

impl RenderBufferPixel {
    pub fn new(background_colour: Vector3<f32>) -> RenderBufferPixel {
        RenderBufferPixel {
            fragments: LinkedList::new(),
            background: Fragment {
                colour: background_colour.push(1.0),
                depth: f32::MAX,
            },
        }
    }
    
    pub fn add(&mut self, fragment: Fragment) {
        if fragment.colour.w >= 0.9999 {
            self.background = fragment;
        } else {
            self.fragments.push_back(fragment);
        }
    }
    
    pub fn resolve(&mut self, background_colour: Vector3<f32>) -> Vector3<f32> {
        let mut fragments = self.fragments.iter().collect::<Vec<_>>();
        fragments.sort_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap());

        let mut result_colour = self.background.colour.xyz();
        let background_depth = self.background.depth;

        for fragment in fragments {
            if fragment.depth > background_depth { continue }

            let alpha = fragment.colour.w;

            result_colour = fragment.colour.xyz() * alpha + result_colour * (1.0 - alpha);
        }

        self.fragments.clear();
        self.background = Fragment {
            colour: background_colour.push(1.0),
            depth: f32::MAX,
        };

        result_colour
    }
    
    pub fn get_background(&self) -> &Fragment {
        &self.background
    }
}
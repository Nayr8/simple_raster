use nalgebra::Vector2;
use std::ops::RangeInclusive;

#[derive(Copy, Clone)]
pub struct BoundingBox {
    min: Vector2<usize>,
    max: Vector2<usize>,
}

impl BoundingBox {
    pub fn new(min: Vector2<usize>, max: Vector2<usize>) -> Self {
        Self { min, max }
    }
    
    pub fn from_triangle(vertex_positions: [Vector2<f32>; 3], bounding_box: BoundingBox) -> Self {
        let upper_clamp = Vector2::new(bounding_box.max.x as f32 - 1.0, bounding_box.max.y as f32 - 1.0);
        let lower_clamp = Vector2::new(bounding_box.min.x as f32, bounding_box.min.y as f32);
        let mut bounding_box_min = upper_clamp;
        let mut bounding_box_max = Vector2::new(0.0_f32, 0.0);

        for vertex in &vertex_positions {
            bounding_box_min.x = bounding_box_min.x.min(vertex.x).max(lower_clamp.x);
            bounding_box_min.y = bounding_box_min.y.min(vertex.y).max(lower_clamp.y);

            bounding_box_max.x = bounding_box_max.x.max(vertex.x).min(upper_clamp.x);
            bounding_box_max.y = bounding_box_max.y.max(vertex.y).min(upper_clamp.y);
        }

        Self {
            min: Vector2::new(bounding_box_min.x as usize, bounding_box_min.y as usize),
            max: Vector2::new(bounding_box_max.x as usize, bounding_box_max.y as usize),
        }
    }

    pub fn x_iter(&self) -> RangeInclusive<usize> {
        self.min.x..=self.max.x
    }

    pub fn y_iter(&self) -> RangeInclusive<usize> {
        self.min.y..=self.max.y
    }
    
    pub fn min(&self) -> Vector2<usize> {
        self.min
    }
    
    pub fn max(&self) -> Vector2<usize> {
        self.max
    }
}
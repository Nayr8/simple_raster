use nalgebra::Vector2;
use std::ops::RangeInclusive;

pub struct BoundingBox {
    min: Vector2<usize>,
    max: Vector2<usize>,
}

impl BoundingBox {
    pub fn calculate(vertex_positions: [Vector2<f32>; 3], width: usize, height: usize) -> Self {
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

    pub fn x_iter(&self) -> RangeInclusive<usize> {
        self.min.x..=self.max.x
    }

    pub fn y_iter(&self) -> RangeInclusive<usize> {
        self.min.y..=self.max.y
    }
}
use crate::{Color, Pos};

use super::types;
use super::types::Canvas;
use derive_builder::Builder;
use femtovg::{ImageId, Paint, Path};

#[derive(Clone, Copy, Default, Debug, PartialEq, Builder)]
pub struct Instance {
    pub origin: Pos,
    pub radius: f32,
    #[builder(default = "Color::default()")]
    pub color: Color,
    #[builder(default = "None")]
    pub bg_image: Option<ImageId>,
}

#[derive(Debug, PartialEq)]
pub struct Circle {
    pub instance_data: Instance,
}

impl Circle {
    pub fn new(origin: Pos, radius: f32, color: Color) -> Self {
        Self {
            instance_data: Instance {
                origin,
                radius,
                color,
                bg_image: None,
            },
        }
    }

    pub fn from_instance_data(instance_data: Instance) -> Self {
        Self { instance_data }
    }

    pub fn render(&self, canvas: &mut Canvas) {
        let Instance {
            origin,
            radius,
            color,
            bg_image,
        } = self.instance_data;
        let mut path = Path::new();
        path.circle(origin.x, origin.y, radius);
        //Add background image
        let background = match bg_image {
            Some(image_id) => Paint::image(image_id, origin.x, origin.y, radius, radius, 0.0, 1.0),
            None => Paint::color(color.into()),
        };

        canvas.fill_path(&path, &background);
    }
}

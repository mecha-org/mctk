use crate::{Color, Pos};

use super::types;
use super::types::Canvas;
use derive_builder::Builder;
use femtovg::{ImageId, Paint, Path};

#[derive(Clone, Default, Debug, PartialEq, Builder)]
pub struct Instance {
    pub origin: Pos,
    pub radius: (f32, f32),
    pub colors: Vec<(f32, Color)>,
}

#[derive(Debug, PartialEq)]
pub struct RadialGradient {
    pub instance_data: Instance,
}

impl RadialGradient {
    pub fn new(origin: Pos, radius: (f32, f32), colors: Vec<(f32, Color)>) -> Self {
        Self {
            instance_data: Instance {
                origin,
                radius,
                colors,
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
            colors,
        } = &self.instance_data;
        let bg = Paint::radial_gradient_stops(
            origin.x,
            origin.y,
            radius.0,
            radius.1,
            colors.clone().into_iter().map(|(k, c)| (k, c.into())),
        );

        let mut path = Path::new();
        path.circle(origin.x, origin.y, radius.1);
        canvas.fill_path(&path, &bg);
        // canvas.stroke_path(&path, &paint);
    }
}

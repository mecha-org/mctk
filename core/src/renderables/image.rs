use std::collections::HashMap;

use crate::{Pos, Scale};

use super::types;
use super::types::Canvas;
use derive_builder::Builder;
use femtovg::{CompositeOperation, ImageId, Paint, Path};

type Point = types::Point<f32>;
type Size = types::Size<f32>;

#[derive(Clone, Debug, PartialEq, Builder)]
pub struct Instance {
    pub name: String,
    pub pos: Pos,
    pub scale: Scale,
    #[builder(default = "CompositeOperation::SourceOver")]
    pub composite_operation: CompositeOperation,
    #[builder(default = "0.0")]
    pub radius: f32,
}

#[derive(Debug, PartialEq)]
pub struct Image {
    pub instance_data: Instance,
}

impl Image {
    pub fn new<S: Into<String>>(pos: Pos, scale: Scale, name: S) -> Self {
        Self {
            instance_data: Instance {
                pos,
                scale,
                name: name.into(),
                composite_operation: CompositeOperation::SourceOver,
                radius: Default::default(),
            },
        }
    }

    pub fn composite_operation(mut self, co: CompositeOperation) -> Self {
        self.instance_data.composite_operation = co;
        self
    }

    pub fn render(&self, canvas: &mut Canvas, assets: &HashMap<String, ImageId>) {
        let Instance {
            pos,
            scale,
            composite_operation,
            radius,
            ..
        } = self.instance_data;

        canvas.global_composite_operation(composite_operation);

        let image_id: &ImageId = match assets.get(&self.instance_data.name) {
            Some(v) => v,
            None => panic!("asset not found {:?}", &self.instance_data.name),
        };
        let Pos { x, y, z } = pos;
        let Scale { width, height } = scale;

        let paint = Paint::image(*image_id, x, y, width, height, 0.0, 1.0);
        let mut path = Path::new();
        path.rounded_rect(x, y, width, height, radius);
        canvas.fill_path(&path, &paint);

        canvas.global_composite_operation(CompositeOperation::SourceOver);
    }

    pub fn from_instance_data(instance_data: Instance) -> Self {
        Self { instance_data }
    }
}

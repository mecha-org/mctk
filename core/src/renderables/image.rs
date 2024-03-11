use std::collections::HashMap;

use crate::{Pos, Scale};

use super::types;
use super::types::Canvas;
use derive_builder::Builder;
use femtovg::{ImageId, Paint, Path};

type Point = types::Point<f32>;
type Size = types::Size<f32>;

#[derive(Clone, Debug, PartialEq, Builder)]
pub struct Instance {
    pub name: String,
    pub pos: Pos,
    pub scale: Scale,
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
            },
        }
    }

    pub fn render(&self, canvas: &mut Canvas, assets: &HashMap<String, ImageId>) {
        let Instance { pos, scale, .. } = self.instance_data;

        let image_id: &ImageId = assets.get(&self.instance_data.name).unwrap();
        let Pos { x, y, z } = pos;
        let Scale { width, height } = scale;

        let paint = Paint::image(*image_id, x, y, width, height, 0.0, 1.0);
        let mut path = Path::new();
        path.rect(x, y, width, height);
        canvas.fill_path(&path, &paint);
    }
}

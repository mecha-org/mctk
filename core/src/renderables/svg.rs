use std::collections::HashMap;

use crate::{renderer::canvas::SvgData, Pos, Scale};

use super::types;
use super::types::Canvas;
use derive_builder::Builder;
use femtovg::{Color, ImageId, Paint, Path};

type Point = types::Point<f32>;
type Size = types::Size<f32>;

#[derive(Clone, Debug, PartialEq, Builder)]
pub struct Instance {
    pub name: String,
    pub pos: Pos,
    pub scale: Scale,
}

#[derive(Debug, PartialEq)]
pub struct Svg {
    pub instance_data: Instance,
}

impl Svg {
    pub fn new<S: Into<String>>(pos: Pos, scale: Scale, name: S) -> Self {
        Self {
            instance_data: Instance {
                pos,
                scale,
                name: name.into(),
            },
        }
    }

    pub fn render(&self, canvas: &mut Canvas, svgs: &HashMap<String, SvgData>) {
        let Instance { pos, scale, .. } = self.instance_data;

        let svg_data = svgs.get(&self.instance_data.name).unwrap();

        let Pos { x, y, z } = pos;
        let Scale { width, height } = scale;

        canvas.save();
        canvas.translate(x, y);

        // println!(
        //     "svg width {} height {} bounds width {} height {}",
        //     width, height, svg_data.scale.width, svg_data.scale.height
        // );
        canvas.scale(width / svg_data.scale.width, height / svg_data.scale.height);

        for (path, fill, stroke) in &svg_data.paths {
            if let Some(fill) = fill {
                canvas.fill_path(&path, &fill);
            }

            if let Some(stroke) = stroke {
                canvas.stroke_path(&path, &stroke);
            }
        }

        canvas.restore();
    }
}

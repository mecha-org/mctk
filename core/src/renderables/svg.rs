use super::types::Canvas;
use crate::{renderer::svg::SvgData, Pos, Scale};
use derive_builder::Builder;
use femtovg::Transform2D;
use std::collections::HashMap;

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

    pub fn render(&self, canvas: &mut Canvas, svgs: &mut HashMap<String, SvgData>) {
        let Instance { pos, scale, .. } = self.instance_data;

        let svg_data = svgs.get_mut(&self.instance_data.name).unwrap();

        let Pos { x, y, .. } = pos;
        let Scale { width, height } = scale;

        canvas.save();
        canvas.translate(x, y);

        // println!(
        //     "svg width {} height {} bounds width {} height {}",
        //     width, height, svg_data.scale.width, svg_data.scale.height
        // );
        canvas.scale(width / svg_data.scale.width, height / svg_data.scale.height);

        for (path, fill, stroke, transform) in &mut svg_data.paths {
            canvas.save();
            // canvas.set_transform(
            //     transform.a as f32,
            //     transform.b as f32,
            //     transform.c as f32,
            //     transform.d as f32,
            //     transform.e as f32,
            //     transform.f as f32,
            // );
            let canvas_transform: Transform2D = Transform2D([
                transform.sx,
                transform.kx,
                transform.ky,
                transform.sy,
                transform.tx,
                transform.ty,
            ]);
            canvas.set_transform(&canvas_transform);

            if let Some(fill) = fill {
                fill.set_anti_alias(true);
                canvas.fill_path(&path, &fill);
            }

            if let Some(stroke) = stroke {
                stroke.set_anti_alias(true);
                canvas.stroke_path(&path, &stroke);
            }

            canvas.restore();
        }

        canvas.restore();
    }
}

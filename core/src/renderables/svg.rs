use super::types::Canvas;
use crate::{
    renderer::svg::{load_svg_path, SvgData},
    Pos, Scale,
};
use derive_builder::Builder;
use femtovg::Transform2D;
use std::collections::HashMap;
use usvg::fontdb::Database;

#[derive(Clone, Debug, PartialEq, Builder)]
pub struct Instance {
    pub name: String,
    pub pos: Pos,
    pub scale: Scale,
    pub dynamic_load_from: Option<String>,
}

#[derive(Debug, PartialEq, Clone)]
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
                dynamic_load_from: None,
            },
        }
    }

    pub fn render(&self, canvas: &mut Canvas, svgs: &mut HashMap<String, SvgData>) {
        let Instance {
            pos,
            scale,
            dynamic_load_from,
            ..
        } = self.instance_data.clone();

        if svgs.get_mut(&self.instance_data.name).is_none() && dynamic_load_from.is_some() {
            let svg_data = load_svg_path(dynamic_load_from.unwrap(), &Database::default());
            svgs.insert(self.instance_data.name.clone(), svg_data);
        }

        if svgs.get_mut(&self.instance_data.name).is_none() {
            println!("error: svg not found {:?}", self.instance_data.name);
            return;
        }

        let svg_data = svgs.get_mut(&self.instance_data.name).unwrap();

        let Pos { x, y, .. } = pos;
        let Scale { width, height } = scale;

        canvas.save();
        canvas.translate(x, y);

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

    pub fn from_instance_data(instance_data: Instance) -> Self {
        Self { instance_data }
    }
}

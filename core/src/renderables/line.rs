use crate::{Color, Pos};

use super::types;
use super::types::Canvas;
use derive_builder::Builder;
use femtovg::{LineCap, LineJoin, Paint, Path};

#[derive(Clone, Copy, Default, Debug, PartialEq, Builder)]
pub struct Instance {
    pub from: Pos,
    pub to: Pos,
    #[builder(default = "Color::default()")]
    pub color: Color,
    #[builder(default = "2.0")]
    pub width: f32,
}

#[derive(Debug, PartialEq)]
pub struct Line {
    pub instance_data: Instance,
}

impl Line {
    pub fn new(from: Pos, to: Pos, color: Color) -> Self {
        Self {
            instance_data: Instance {
                from,
                to,
                color,
                width: 10.0,
            },
        }
    }

    pub fn from_instance_data(instance_data: Instance) -> Self {
        Self { instance_data }
    }

    pub fn render(&self, canvas: &mut Canvas) {
        let Instance {
            from,
            to,
            color,
            width,
        } = self.instance_data;
        let mut path = Path::new();
        path.move_to(from.x, from.y);
        path.line_to(to.x, to.y);

        let mut paint = Paint::default();
        paint.set_color(color.into());
        paint.set_line_cap(LineCap::Round);
        paint.set_line_join(LineJoin::Miter);
        paint.set_line_width(width);
        canvas.stroke_path(&path, &paint);
    }
}

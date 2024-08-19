use crate::{Color, Point, Pos};

use super::types;
use super::types::Canvas;
use derive_builder::Builder;
use femtovg::{LineCap, LineJoin, Paint, Path};

#[derive(Clone, Default, Debug, PartialEq, Builder)]
pub struct Instance {
    pub anchors: Vec<Point>,
    #[builder(default = "Color::default()")]
    pub color: Color,
    #[builder(default = "2.0")]
    pub width: f32,
    #[builder(default = "2.0")]
    pub anchor_width: f32,
    #[builder(default = "Color::default()")]
    pub anchor_color: Color,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Curve {
    pub instance_data: Instance,
}

impl Curve {
    pub fn new(anchors: Vec<Point>) -> Self {
        Self {
            instance_data: Instance {
                anchors,
                color: Color::BLACK,
                anchor_color: Color::BLUE,
                width: 2.,
                anchor_width: 4.,
            },
        }
    }

    pub fn from_instance_data(instance_data: Instance) -> Self {
        Self { instance_data }
    }

    pub fn render(&self, canvas: &mut Canvas) {
        let anchors = self.instance_data.anchors.clone();
        let Instance {
            color,
            width,
            anchor_width,
            anchor_color,
            ..
        } = self.instance_data;

        if anchors.len() <= 1 {
            return;
        }

        //draw anchors
        for anchor in anchors.clone() {
            let mut path = Path::new();
            path.circle(anchor.x, anchor.y, anchor_width);
            canvas.fill_path(&path, &Paint::color(anchor_color.into()));
        }

        //draw curve
        let mut path = Path::new();
        path.move_to(anchors[0].x, anchors[0].y);
        let mut line = Paint::color(color.into());
        for i in 1..anchors.len() {
            line.set_line_width(width);
            path.bezier_to(
                anchors[i].x,
                anchors[i].y,
                anchors[i].x,
                anchors[i].y,
                anchors[i].x,
                anchors[i].y,
            );
        }
        canvas.stroke_path(&path, &line);
    }
}

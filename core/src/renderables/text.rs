use std::collections::HashMap;

use super::types;
use super::types::Canvas;
use crate::types::{Color, Point, Pos, Scale, AABB};
use derive_builder::Builder;
use femtovg::{Align, Baseline, Color as fem_color, FontId, Paint, Path};
use resource::resource;

#[derive(Clone, Debug, PartialEq, Builder)]
pub struct Instance {
    pub pos: Pos,
    pub font: String,

    #[builder(default = "Default::default()")]
    pub color: Color,
    #[builder(default = "16.0")]
    pub font_size: f32,
    #[builder(default = "1.0")]
    pub line_width: f32,
    #[builder(default = "Align::Left")]
    pub align: Align,
    #[builder(default = "String::new()")]
    pub text: String,
}

#[derive(Debug, PartialEq)]
pub struct Text {
    pub instance_data: Instance,
}

impl Text {
    pub fn new(pos: Pos, text: impl Into<String>, font: impl Into<String>) -> Self {
        Self {
            instance_data: Instance {
                pos,
                color: Color::BLACK,
                font_size: 16.0,
                font: font.into(),
                line_width: 1.0,
                align: Align::Left,
                text: text.into(),
            },
        }
    }

    pub fn from_instance_data(instance_data: Instance) -> Self {
        Self { instance_data }
    }

    pub fn render(&self, canvas: &mut Canvas, fonts: &HashMap<String, FontId>) {
        let Instance {
            pos,
            color,
            align,
            font,
            font_size,
            line_width,
            text,
        } = self.instance_data.clone();
        let mut paint = Paint::color(color.into());
        paint.set_text_align(align);

        let font_id: &FontId = fonts.get(&font).unwrap();
        paint.set_font(&[*font_id]);
        paint.set_font_size(font_size);
        paint.set_line_width(line_width);
        paint.set_text_baseline(Baseline::Bottom);

        let _ = canvas.fill_text(pos.x, pos.y + font_size * 1.5, text, &paint);
    }
}

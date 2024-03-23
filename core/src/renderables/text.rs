use super::types::Canvas;
use crate::{renderer::text::TextRenderer, style::FontWeight, types::{Color, Pos}, Scale};
use cosmic_text::FontSystem;
use derive_builder::Builder;
use femtovg::{Align, Paint };

#[derive(Clone, Debug, PartialEq, Builder)]
pub struct Instance {
    pub pos: Pos,
    pub scale: Scale,
    #[builder(default = "None")]
    pub font: Option<String>,
    #[builder(default = "FontWeight::Normal")]
    pub weight: FontWeight,
    #[builder(default = "Default::default()")]
    pub color: Color,
    #[builder(default = "12.0")]
    pub font_size: f32,
    #[builder(default = "18.0")]
    pub line_height: f32,
    #[builder(default = "Align::Left")]
    pub align: Align,
    #[builder(default = "String::new()")]
    pub text: String,
}

#[derive(Debug, PartialEq)]
pub struct Text {
    pub instance_data: Instance,
}

lazy_static::lazy_static! {
    static ref FONT_SYSTEM: FontSystem = FontSystem::new();
}

impl Text {
    pub fn new(pos: Pos, scale: Scale, text: impl Into<String>) -> Self {
        Self {
            instance_data: Instance {
                pos,
                scale,
                color: Color::BLACK,
                font_size: 12.0,
                font: None,
                weight: FontWeight::Normal,
                line_height: 18.0,
                align: Align::Left,
                text: text.into(),
            },
        }
    }

    pub fn from_instance_data(instance_data: Instance) -> Self {
        Self { instance_data }
    }

    pub fn render(&self, canvas: &mut Canvas, text_renderer: &mut TextRenderer) {
        let Instance {
            color,
            ..
        } = self.instance_data;

        if let Ok(draw_commands) = text_renderer.draw_text(canvas, self.instance_data.clone()) {
            for (_, cmds) in draw_commands.into_iter() {
                let temp_paint = Paint::color(color.into());
                canvas.draw_glyph_commands(cmds, &temp_paint, 1.0);
            }
        }
    }
}

use std::hash::Hash;

use crate::component::{Component, ComponentHasher, RenderContext};
use crate::font_cache::{FontCache, TextSegment};
use crate::renderables::text::InstanceBuilder;
use crate::renderables::{text, Renderable};
use crate::style::{FontWeight, HorizontalPosition, Styled};
use crate::types::*;
use cosmic_text::LayoutGlyph;
use femtovg::Align;
use mctk_macros::{component, state_component_impl};

#[derive(Debug, Default)]
struct BoundsCache {
    width: Option<f32>,
    height: Option<f32>,
    max_width: Option<f32>,
    max_height: Option<f32>,
    output: Option<(Option<f32>, Option<f32>)>,
}

#[derive(Debug, Default)]
pub struct TextState {
    bounds_cache: BoundsCache,
}

#[component(State = "TextState", Styled, Internal)]
#[derive(Debug)]
pub struct Text {
    pub text: Vec<TextSegment>,
}

impl Text {
    pub fn new(text: Vec<TextSegment>) -> Self {
        Self {
            text,
            class: Default::default(),
            style_overrides: Default::default(),
            state: Some(TextState::default()),
            dirty: false,
        }
    }
}

#[state_component_impl(TextState)]
impl Component for Text {
    fn new_props(&mut self) {
        self.state = Some(TextState::default());
    }

    fn props_hash(&self, hasher: &mut ComponentHasher) {
        self.text.hash(hasher);
    }

    fn render_hash(&self, hasher: &mut ComponentHasher) {
        self.text.hash(hasher);
        (self.style_val("size").unwrap().f32() as u32).hash(hasher);
        (self.style_val("color").unwrap().color()).hash(hasher);
        (self.style_val("font").map(|p| p.str().to_string())).hash(hasher);
        (self.style_val("h_alignment").map(|v| v.horizontal_position())).hash(hasher);
    }

    fn fill_bounds(
        &mut self,
        width: Option<f32>,
        height: Option<f32>,
        max_width: Option<f32>,
        max_height: Option<f32>,
        font_cache: &mut FontCache,
        scale_factor: f32,
    ) -> (Option<f32>, Option<f32>) {
        // Temporary hack
        // (width, Some(self.style_val("size").unwrap().f32() * 1.75))

        let c = &self.state_ref().bounds_cache;
        if c.output.is_some()
            && c.width == width
            && c.height == height
            && c.max_width == max_width
            && c.max_height == max_height
        {
            let output = c.output.unwrap();
            (output.0, output.1);
        }

        let text = self.text.get(0).unwrap().text.clone();
        let size: f32 = self.style_val("size").unwrap().f32();
        let font = self.style_val("font").map(|p| p.str().to_string());
        let mut line_height = size * 1.3; // line height as 1.3 of font_size
        if self.style_val("line_height").is_some() {
            line_height = self.style_val("line_height").unwrap().f32();
        }

        let (t_w, t_h, ..) = font_cache.measure_text(
            text.clone(),
            font,
            size,
            scale_factor,
            line_height,
            HorizontalPosition::Left,
            (
                width.or(max_width).unwrap_or(std::f32::MAX) * scale_factor,
                height.or(max_height).unwrap_or(std::f32::MAX) * scale_factor,
            ),
        );

        let output = (t_w, t_h);
        self.state_mut().bounds_cache = BoundsCache {
            width,
            height,
            max_width,
            max_height,
            output: Some(output),
        };
        output
    }

    fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
        let h_alignment: HorizontalPosition =
            if let Some(h_alignment) = self.style_val("h_alignment") {
                h_alignment.horizontal_position()
            } else {
                HorizontalPosition::Left
            };
        let font = self.style_val("font").map(|p| p.str().to_string());
        let color: Color = self.style_val("color").into();
        let scale = context.aabb.size();
        let size: f32 = if let Some(size) = self.style_val("size") {
            size.f32()
        } else {
            16.
        };
        let AABB { pos, .. } = context.aabb;
        let font_weight = if let Some(font_weight) = self.style_val("font_weight") {
            font_weight.font_weight()
        } else {
            FontWeight::Normal
        };
        // line height as 1.3 of font_size
        let line_height = if let Some(line_height) = self.style_val("line_height") {
            line_height.f32()
        } else {
            size * 1.3
        };

        // let font = Some(String::from("SpaceGrotesk-Bold"));

        // let screen_position = (
        //     match h_alignment {
        //         HorizontalPosition::Left => 0.0,
        //         HorizontalPosition::Center => scale.width / 2.0,
        //         HorizontalPosition::Right => scale.width,
        //     },
        //     0.0,
        // );

        // let pos = context.aabb.pos
        //     + Pos {
        //         x: screen_position.0,
        //         y: screen_position.1,
        //         z: 0.,
        //     };

        let text_instance = InstanceBuilder::default()
            .align(match h_alignment {
                HorizontalPosition::Left => Align::Left,
                HorizontalPosition::Center => Align::Center,
                HorizontalPosition::Right => Align::Right,
            })
            .pos(pos)
            .scale(scale)
            .text(self.text.get(0).unwrap().text.clone())
            .color(color)
            .font(font)
            .weight(font_weight)
            .line_height(line_height)
            .font_size(size)
            .build()
            .unwrap();

        Some(vec![Renderable::Text(text::Text::from_instance_data(
            text_instance,
        ))])
    }
}

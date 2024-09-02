use crate::component::{Component, ComponentHasher, RenderContext};

use crate::renderables::rect::InstanceBuilder;
use crate::renderables::types::{Point, Size};
use crate::renderables::{Rect, Renderable};
use crate::types::*;
use std::hash::Hash;

#[derive(Debug)]
pub struct RoundedRect {
    pub background_color: Color,
    pub border_color: Color,
    pub border_width: f32,
    pub radius: (f32, f32, f32, f32),
    pub scissor: Option<bool>,
}

impl Default for RoundedRect {
    fn default() -> Self {
        Self {
            background_color: Color::WHITE,
            border_color: Color::BLACK,
            border_width: 0.0,
            radius: (3.0, 3.0, 3.0, 3.0),
            scissor: None,
        }
    }
}

impl RoundedRect {
    pub fn new<C: Into<Color>>(bg: C, radius: f32) -> Self {
        Self {
            background_color: bg.into(),
            border_color: Color::BLACK,
            border_width: 0.0,
            radius: (radius, radius, radius, radius),
            scissor: None,
        }
    }

    pub fn radius(mut self, r: f32) -> Self {
        self.radius = (r, r, r, r);
        self
    }
}

impl Component for RoundedRect {
    fn render_hash(&self, hasher: &mut ComponentHasher) {
        self.background_color.hash(hasher);
        self.border_color.hash(hasher);
        (self.border_width as u32).hash(hasher);
        (self.radius.0 as i32).hash(hasher);
        (self.radius.1 as i32).hash(hasher);
        (self.radius.2 as i32).hash(hasher);
        (self.radius.3 as i32).hash(hasher);
    }

    fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
        // println!("Rounded rect render {:?}", self.scissor);
        let width = context.aabb.width();
        let height = context.aabb.height();
        let AABB { pos, .. } = context.aabb;

        let instance_data = InstanceBuilder::default()
            .pos(pos)
            .scale(Scale { width, height })
            .color(self.background_color)
            .border_color(self.border_color)
            .border_size(self.border_width)
            .scissor(self.scissor)
            .radius(self.radius)
            .build()
            .unwrap();

        Some(vec![Renderable::Rect(Rect::from_instance_data(
            instance_data,
        ))])
    }
}

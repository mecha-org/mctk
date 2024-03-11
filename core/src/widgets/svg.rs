use std::hash::Hash;

use crate::component::{Component, ComponentHasher, RenderContext};

use crate::renderables::types::{Point, Size};
use crate::renderables::{self, Rect, Renderable};
use crate::types::*;

#[derive(Debug)]
pub struct Svg {
    pub name: String,
}

impl Default for Svg {
    fn default() -> Self {
        Self {
            name: "".to_string(),
        }
    }
}

impl Svg {
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self { name: name.into() }
    }
}

impl Component for Svg {
    fn render_hash(&self, hasher: &mut ComponentHasher) {
        self.name.hash(hasher);
    }

    fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
        let width = context.aabb.width();
        let height = context.aabb.height();
        let Pos { x, y, .. } = context.aabb.pos;

        Some(vec![Renderable::Svg(renderables::Svg::new(
            [x, y].into(),
            [width, height].into(),
            self.name.clone(),
        ))])
    }
}

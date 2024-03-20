use std::hash::Hash;

use mctk_macros::component;

use crate::component::{Component, ComponentHasher, RenderContext};

use crate::renderables::image::InstanceBuilder as ImageInstanceBuilder;
use crate::renderables::types::{Point, Size};
use crate::renderables::{self, Rect, Renderable};
use crate::style::{self, Styled};
use crate::types::*;

#[component(Styled)]
#[derive(Debug)]
pub struct Image {
    pub name: String,
}

impl Default for Image {
    fn default() -> Self {
        Self {
            name: "".to_string(),
            class: Default::default(),
            style_overrides: Default::default(),
        }
    }
}

impl Image {
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self {
            name: name.into(),
            class: Default::default(),
            style_overrides: Default::default(),
        }
    }
}

impl Component for Image {
    fn render_hash(&self, hasher: &mut ComponentHasher) {
        self.name.hash(hasher);
    }

    fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
        let width = context.aabb.width();
        let height = context.aabb.height();
        let AABB { pos, .. } = context.aabb;
        let radius = self.style_val("radius").unwrap().f32();

        let instance = ImageInstanceBuilder::default()
            .pos(pos)
            .scale(Scale { width, height })
            .name(self.name.clone())
            .radius(radius)
            .build()
            .unwrap();

        Some(vec![Renderable::Image(
            renderables::Image::from_instance_data(instance),
        )])
    }
}

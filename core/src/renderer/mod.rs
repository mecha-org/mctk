pub mod canvas;
pub mod gl;
pub mod svg;
pub mod text;

use crate::{font_cache::FontCache, window::Window, Node, PixelSize};
use std::{
    fmt,
    sync::{Arc, RwLock},
};

// /// The type returned by [`Component#render`][crate::Component#method.render], which contains the data required to render a Component (along with the [`Caches`][super::Caches]).
// #[derive(Debug, PartialEq)]
// pub enum Renderable {
//     Rect,
//     Shape,
//     Text,
//     Raster,
// }

/// The caches used by the Renderer. Passed to [`Component#render`][crate::Component#method.render] in a [`RenderContext`][crate::RenderContext].
#[derive(Clone)]
pub struct Caches {
    /// Font cache
    pub font: Arc<RwLock<FontCache>>,
}

pub(crate) trait Renderer: fmt::Debug + std::marker::Sized + Send + Sync {
    fn new<W: Window>(window: Arc<RwLock<W>>) -> Self;
    fn configure<W: crate::window::Window>(&mut self, window: Arc<RwLock<W>>) {}
    fn render(&mut self, _node: &Node, _physical_size: PixelSize) {}
    fn resize(&mut self, width: u32, height: u32) {}
    fn caches(&self) -> Caches;
}

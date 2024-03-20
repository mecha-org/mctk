use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use std::{
    fmt,
    sync::{Arc, RwLock},
};

use crate::{font_cache::FontCache, window::Window, Node, PixelSize};

pub mod canvas;

// /// The type returned by [`Component#render`][crate::Component#method.render], which contains the data required to render a Component (along with the [`Caches`][super::Caches]).
// #[derive(Debug, PartialEq)]
// pub enum Renderable {
//     Rect,
//     Shape,
//     Text,
//     Raster,
// }

/// The caches used by the Renderer. Passed to [`Component#render`][crate::Component#method.render] in a [`RenderContext`][crate::RenderContext].
#[derive(Clone, Default)]
pub struct Caches {
    /// Font cache
    pub font: Arc<RwLock<FontCache>>,
}

pub(crate) trait Renderer: fmt::Debug + std::marker::Sized + Send + Sync {
    fn new<W: Window>(window: Arc<RwLock<W>>) -> Self;
    fn configure<W: crate::window::Window>(&mut self, window: Arc<RwLock<W>>) {}
    fn render(&mut self, _node: &Node, _physical_size: PixelSize) {}
    /// This default is provided for tests, it should be overridden
    fn caches(&self) -> Caches {
        Default::default()
        // Caches {
        //     shape_buffer: Arc::new(RwLock::new(BufferCache::new())),
        //     text_buffer: Arc::new(RwLock::new(BufferCache::new())),
        //     image_buffer: Arc::new(RwLock::new(BufferCache::new())),
        //     raster: Arc::new(RwLock::new(RasterCache::new())),
        //     font: Default
        // }
    }
}

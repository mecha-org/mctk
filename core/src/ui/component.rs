use euclid::{self};
use femtovg::{self, renderer::OpenGl};

pub type Canvas = femtovg::Canvas<OpenGl>;
pub type Point = euclid::default::Point2D<f32>;
pub type Vector = euclid::default::Vector2D<f32>;
pub type Size = euclid::default::Size2D<f32>;
pub type Rect = euclid::default::Rect<f32>;

#[derive(Debug, Copy, Clone)]
pub enum Dimension {
    Abs(f32),
    Auto, // Percent(f32),
}

pub enum AlignItems {
    Start,
    End,
    Center,
    Stretch,
}
#[derive(Debug, Default, Copy, Clone)]
pub struct Margin {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

pub enum JustifyContent {
    Start,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

pub trait CanvasComponent {
    fn render(&self, canvas: &mut Canvas, rect: Rect);
    fn height(&self) -> Dimension;
    fn width(&self) -> Dimension;
    fn margin(&self) -> Margin;
}

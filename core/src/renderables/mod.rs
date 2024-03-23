pub mod circle;
pub mod image;
pub mod line;
pub mod radial_gradient;
pub mod rect;
pub mod svg;
pub mod text;
pub mod types;

pub use circle::Circle;
pub use image::Image;
pub use line::Line;
pub use radial_gradient::RadialGradient;
pub use rect::Rect;
pub use svg::Svg;
pub use text::Text;

use crate::renderer::canvas::SvgData;

use self::types::Canvas;

use femtovg::{FontId, ImageId, Paint, Path};
use std::collections::HashMap;

#[derive(Debug)]
pub enum Renderable {
    Rect(Rect),
    Line(Line),
    Circle(Circle),
    Image(Image),
    Text(Text),
    Svg(Svg),
    RadialGradient(RadialGradient),
}

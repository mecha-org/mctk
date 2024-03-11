pub mod circle;
pub mod image;
pub mod line;
pub mod rect;
pub mod svg;
pub mod text;
pub mod types;

pub use circle::Circle;
pub use image::Image;
pub use line::Line;
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
}

pub fn render_renderables(
    renderables: Vec<Renderable>,
    canvas: &mut Canvas,
    fonts: &HashMap<String, FontId>,
    assets: &HashMap<String, ImageId>,
    svgs: &HashMap<String, SvgData>,
) {
    for renderable in renderables {
        match renderable {
            Renderable::Rect(rect) => {
                rect.render(canvas);
            }
            Renderable::Line(line) => {
                line.render(canvas);
            }
            Renderable::Circle(circle) => {
                circle.render(canvas);
            }
            Renderable::Image(image) => {
                image.render(canvas, assets);
            }
            Renderable::Svg(svg) => {
                svg.render(canvas, svgs);
            }
            Renderable::Text(text) => {
                text.render(canvas, fonts);
            }
        }
    }
}

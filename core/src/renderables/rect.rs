use super::types::Canvas;
use super::types::{self, Corners, Edges};
use crate::types::{Color, Point, Pos, Scale, AABB};
use bytemuck::{Pod, Zeroable};
use derive_builder::Builder;
use femtovg::{Color as fem_color, CompositeOperation, ImageId, Paint, Path};

#[derive(Debug, Clone)]
pub enum Gradient {
    Linear {
        start: Point,
        end: Point,
        stops: Vec<(f32, Color)>,
    },
    Radial {
        center: Point,
        radius: (f32, f32),
        stops: Vec<(f32, Color)>,
    },
}

#[derive(Clone, Debug, Builder)]
pub struct Instance {
    pub pos: Pos,
    pub scale: Scale,
    #[builder(default = "Color::TRANSPARENT")]
    pub color: Color,
    #[builder(default = "(0., 0., 0., 0.)")]
    pub radius: (f32, f32, f32, f32),
    #[builder(default = "Color::TRANSPARENT")]
    pub border_color: Color,
    #[builder(default = "0.0")]
    pub border_size: f32,
    #[builder(default = "None")]
    pub bg_image: Option<ImageId>,
    #[builder(default = "None")]
    pub gradient: Option<Gradient>,
    #[builder(default = "CompositeOperation::SourceOver")]
    pub composite_operation: CompositeOperation,
}

#[derive(Debug)]
pub struct Rect {
    pub instance_data: Instance,
}

impl Rect {
    pub fn new(pos: Pos, scale: Scale, color: Color) -> Self {
        Self {
            instance_data: Instance {
                pos,
                scale,
                color,
                radius: (0., 0., 0., 0.),
                bg_image: None,
                border_color: Color::TRANSPARENT,
                border_size: 0.0,
                gradient: None,
                composite_operation: CompositeOperation::SourceOver,
            },
        }
    }

    pub fn from_instance_data(instance_data: Instance) -> Self {
        Self { instance_data }
    }

    pub fn render(&self, canvas: &mut Canvas) {
        let Instance {
            pos,
            scale,
            color,
            radius,
            bg_image,
            border_color,
            border_size,
            gradient,
            composite_operation,
        } = self.instance_data.clone();
        let origin = pos;
        let size = scale;

        canvas.global_composite_operation(composite_operation);
        let mut path = Path::new();
        path.rounded_rect_varying(
            origin.x,
            origin.y,
            size.width,
            size.height,
            radius.0,
            radius.1,
            radius.2,
            radius.3,
        );

        //Add background image if any
        let background = match bg_image {
            Some(image_id) => Paint::image(
                image_id,
                origin.x,
                origin.y,
                size.width,
                size.height,
                0.0,
                1.0,
            ),
            None => Paint::color(color.into()),
        };
        canvas.fill_path(&path, &background);

        let mut paint = Paint::color(border_color.into());
        paint.set_line_width(border_size);
        canvas.stroke_path(&path, &paint);
        canvas.global_composite_operation(CompositeOperation::SourceOver);

        //Add gradient
        // match gradient {
        //     Some(gradient_type) => match gradient_type {
        //         Gradient::Linear { start, end, stops } => {
        //             let paint = Paint::linear_gradient_stops(start.x, start.y, end.x, end.y, stops);
        //             canvas.fill_path(&path, &paint);
        //         }
        //         Gradient::Radial {
        //             center,
        //             radius,
        //             stops,
        //         } => {
        //             let paint =
        //                 Paint::radial_gradient_stops(center.x, center.y, radius.0, radius.1, stops);
        //             canvas.fill_path(&path, &paint);
        //         }
        //     },
        //     None => (),
        // }

        // //Add borders
        // //border top
        // let mut path = Path::new();
        // path.move_to(origin.x, origin.y);
        // path.line_to(origin.x + size.width, origin.y);
        // let mut paint = Paint::color(border_color.into());
        // paint.set_line_width(border_size);
        // canvas.stroke_path(&path, &paint);

        // //border right
        // let mut path = Path::new();
        // path.move_to(origin.x + size.width, origin.y);
        // path.line_to(origin.x + size.width, origin.y + size.height);
        // let mut paint = Paint::color(border_color.into());
        // paint.set_line_width(border_size);
        // canvas.stroke_path(&path, &paint);

        // //border bottom
        // let mut path = Path::new();
        // path.move_to(origin.x, origin.y + size.height);
        // path.line_to(origin.x + size.width, origin.y + size.height);
        // let mut paint = Paint::color(border_color.into());
        // paint.set_line_width(border_size);
        // canvas.stroke_path(&path, &paint);

        // //border left
        // let mut path = Path::new();
        // path.move_to(origin.x, origin.y);
        // path.line_to(origin.x, origin.y + size.height);
        // let mut paint = Paint::color(border_color.into());
        // paint.set_line_width(border_size);
        // canvas.stroke_path(&path, &paint);
    }
}

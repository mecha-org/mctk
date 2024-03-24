use crate::Scale;
use femtovg::{Color, Paint, Path};
use std::{borrow::Borrow, collections::HashMap};
use usvg::{fontdb::Database, tiny_skia_path::PathSegment, Transform};

#[derive(Debug)]
pub struct SvgData {
    pub paths: Vec<(Path, Option<Paint>, Option<Paint>, Transform)>,
    pub scale: Scale,
}

fn render_nodes_to_paths(
    nodes: &[usvg::Node],
) -> Vec<(Path, Option<Paint>, Option<Paint>, Transform)> {
    let mut paths = Vec::new();

    for node in nodes {
        let mut path = Path::new();

        match &*node.borrow() {
            usvg::Node::Group(child_group) => {
                let mut child_paths = render_nodes_to_paths(child_group.children());
                paths.append(&mut child_paths);
            }
            usvg::Node::Path(svg_path) => {
                for command in svg_path.data().segments() {
                    match command {
                        PathSegment::MoveTo(p) => path.move_to(p.x as f32, p.y as f32),
                        PathSegment::LineTo(p) => path.line_to(p.x as f32, p.y as f32),
                        PathSegment::QuadTo(p1, p2) => path.quad_to(p1.x, p1.y, p2.x, p2.y),
                        PathSegment::CubicTo(p1, p2, p3) => {
                            path.bezier_to(p1.x, p1.y, p2.x, p2.y, p3.x, p3.y)
                        }
                        PathSegment::Close => path.close(),
                    }
                }

                let to_femto_color = |usvg_paint: &usvg::Paint| match usvg_paint {
                    usvg::Paint::Color(usvg::Color { red, green, blue }) => {
                        Some(Color::rgb(*red, *green, *blue))
                    }
                    _ => None,
                };

                let fill = svg_path
                    .fill()
                    .as_ref()
                    .and_then(|fill| to_femto_color(&fill.paint()))
                    .map(|col| Paint::color(col).with_anti_alias(true));

                let stroke = svg_path.stroke().and_then(|stroke| {
                    to_femto_color(&stroke.paint()).map(|paint| {
                        let mut stroke_paint = Paint::color(paint);
                        stroke_paint.set_line_width(stroke.width().get() as f32);
                        stroke_paint.set_anti_alias(true);
                        stroke_paint.set_line_cap(match &stroke.linecap() {
                            usvg::LineCap::Butt => femtovg::LineCap::Butt,
                            usvg::LineCap::Round => femtovg::LineCap::Round,
                            usvg::LineCap::Square => femtovg::LineCap::Square,
                        });
                        stroke_paint.set_line_join(match &stroke.linejoin() {
                            usvg::LineJoin::Miter => femtovg::LineJoin::Miter,
                            usvg::LineJoin::Round => femtovg::LineJoin::Round,
                            usvg::LineJoin::Bevel => femtovg::LineJoin::Bevel,
                            usvg::LineJoin::MiterClip => femtovg::LineJoin::Miter,
                        });
                        stroke_paint.set_miter_limit(stroke.miterlimit().get() as f32);
                        stroke_paint
                    })
                });

                let transform = svg_path.abs_transform();

                paths.push((path, fill, stroke, transform))
            }
            usvg::Node::Image(_) => {}
            usvg::Node::Text(_) => {}
        }
    }

    paths
}

pub fn load_svg_paths(svgs: HashMap<String, String>, fonts: Database) -> HashMap<String, SvgData> {
    let mut loaded_svgs = HashMap::new();

    for (name, path) in svgs.into_iter() {
        let svg_data = match std::fs::read(&path) {
            Ok(file) => file,
            Err(e) => {
                println!("error {:?} path {:?}", e, path);
                panic!("{:?}", e);
            }
        };

        let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default(), &fonts).unwrap();
        let width = tree.size().width() as f32;
        let height = tree.size().height() as f32;

        let paths: Vec<(Path, Option<Paint>, Option<Paint>, Transform)> =
            render_nodes_to_paths(tree.root().children());
        loaded_svgs.insert(
            name,
            SvgData {
                paths,
                scale: Scale { width, height },
            },
        );
    }

    loaded_svgs
}

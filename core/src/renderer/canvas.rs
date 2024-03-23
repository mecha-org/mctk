use cosmic_text::{Buffer, FontSystem};
use femtovg::{Color, FontId, ImageFlags, ImageId, Paint, Path};
use glutin::api::egl;
use glutin::api::egl::context::PossiblyCurrentContext;
use glutin::api::egl::surface::Surface;
use glutin::context::PossiblyCurrentContextGlSurfaceAccessor;
use glutin::surface::{GlSurface, WindowSurface};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::fmt;
use usvg::TreeParsing;
use crate::font_cache::FontCache;
use crate::renderables::Renderable;
use crate::Scale;
use crate::{node::Node, types::PixelSize};
use super::text::TextRenderer;
use super::gl::{init_gl, init_gl_canvas};
use super::Caches;

#[derive(Debug)]
pub struct SvgData {
    pub paths: Vec<(Path, Option<Paint>, Option<Paint>)>,
    pub scale: Scale,
}


fn init_canvas_renderer(
    raw_display_handle: RawDisplayHandle,
    raw_window_handle: RawWindowHandle,
    logical_size: PixelSize,
    scale_factor: f32,
    assets: HashMap<String, String>,
) -> (
    CanvasContext,
    HashMap<String, ImageId>,
) {
    let size = logical_size;
    let width = size.width;
    let height = size.height;

    let (gl_display, gl_surface, gl_context) =
        init_gl(raw_display_handle, raw_window_handle, (width, height));
    let mut gl_canvas = init_gl_canvas(&gl_display, (width, height), scale_factor);

    let mut loaded_assets = HashMap::new();

    for (name, path_) in assets.into_iter() {
        match gl_canvas.load_image_file(path_.as_str(), ImageFlags::empty()) {
            Ok(font_id) => {
                loaded_assets.insert(name, font_id);
            }
            Err(e) => {
                println!("error while loading png {:?} error: {:?}", name, e);
            }
        }
    }

    let canvas_context = CanvasContext {
        gl_canvas,
        gl_context,
        gl_surface,
    };

    (canvas_context, loaded_assets)
}

pub struct CanvasContext {
    // egl context, surface
    pub gl_context: PossiblyCurrentContext,
    pub gl_surface: egl::surface::Surface<WindowSurface>,
    // femto canvas
    pub gl_canvas: femtovg::Canvas<femtovg::renderer::OpenGl>,
}

pub struct CanvasRenderer {
    fonts: cosmic_text::fontdb::Database,
    scale_factor: f32,
    context: CanvasContext,
    text_renderer: TextRenderer,
    assets: HashMap<String, ImageId>,
    svgs: HashMap<String, SvgData>,
}

impl CanvasRenderer {}

unsafe impl Send for CanvasRenderer {}
unsafe impl Sync for CanvasRenderer {}

impl fmt::Debug for CanvasRenderer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "CanvasRenderer")?;
        Ok(())
    }
}

impl super::Renderer for CanvasRenderer {
    fn new<W: crate::window::Window>(w: Arc<RwLock<W>>) -> Self {
        let window = w.read().unwrap();
        let fonts = window.fonts();
        let scale_factor = window.scale_factor();
        let (canvas_context, assets) = init_canvas_renderer(
            window.raw_display_handle(),
            window.raw_window_handle(),
            window.logical_size(),
            scale_factor,
            window.assets(),
        );
        let text_renderer = TextRenderer::new(fonts.clone());
        let svgs = window.svgs();
        let mut loaded_svgs = HashMap::new();

        for (name, path) in svgs.into_iter() {
            let svg_data = match std::fs::read(&path) {
                Ok(file) => file,
                Err(e) => {
                    println!("error {:?} path {:?}", e, path);
                    panic!("{:?}", e);
                }
            };
            let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default()).unwrap();
            let width = tree.size.width() as f32;
            let height = tree.size.height() as f32;

            let paths: Vec<(Path, Option<Paint>, Option<Paint>)> = render_svg(tree);
            loaded_svgs.insert(
                name,
                SvgData {
                    paths,
                    scale: Scale { width, height },
                },
            );
        }

        Self {
            fonts: fonts.clone(),
            scale_factor,
            context: canvas_context,
            text_renderer,
            assets,
            svgs: loaded_svgs,
        }
    }

    fn configure<W: crate::window::Window>(&mut self, w: Arc<RwLock<W>>) {
        // re-initialize
        let window = w.read().unwrap();
        let scale_factor = window.scale_factor();

        let (canvas_context, assets) = init_canvas_renderer(
            window.raw_display_handle(),
            window.raw_window_handle(),
            window.logical_size(),
            scale_factor,
            window.assets(),
        );

        self.scale_factor = scale_factor;
        self.context = canvas_context;
        self.assets = assets;
    }

    fn render(&mut self, node: &Node, _physical_size: PixelSize) {
        let canvas = &mut self.context.gl_canvas;
        let surface: &Surface<WindowSurface> = &mut self.context.gl_surface;
        let text_renderer = &mut self.text_renderer;

        let gl_context = &mut self.context.gl_context;

        let _ = gl_context
            .make_current(surface)
            .expect("Failed to make newly created OpenGL context current");

        canvas.clear_rect(
            0,
            0,
            canvas.width(),
            canvas.width(),
            Color::rgba(0, 0, 0, 0),
        );

        for (renderable, aabb, frame) in node.iter_renderables() {
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
                    image.render(canvas, &self.assets);
                }
                Renderable::Svg(svg) => {
                    svg.render(canvas, &self.svgs);
                }
                Renderable::Text(text) => {
                    text.render(canvas, text_renderer);
                }
                Renderable::RadialGradient(rg) => {
                    rg.render(canvas);
                }
            }
        }

        // Tell renderer to execute all drawing commands
        canvas.flush();

        // Display what we've just rendered
        surface
            .swap_buffers(&gl_context)
            .expect("Could not swap buffers");
    }

    /// This default is provided for tests, it should be overridden
    fn caches(&self) -> Caches {
        Caches {
            font: Arc::new(RwLock::new(FontCache::new(self.fonts.clone())))
        }
    }
}

fn render_svg(svg: usvg::Tree) -> Vec<(Path, Option<Paint>, Option<Paint>)> {
    use usvg::NodeKind;
    use usvg::PathSegment;

    let mut paths = Vec::new();

    for node in svg.root.descendants() {
        if let NodeKind::Path(svg_path) = &*node.borrow() {
            let mut path = Path::new();

            for command in svg_path.data.segments() {
                match command {
                    PathSegment::MoveTo { x, y } => path.move_to(x as f32, y as f32),
                    PathSegment::LineTo { x, y } => path.line_to(x as f32, y as f32),
                    PathSegment::CurveTo {
                        x1,
                        y1,
                        x2,
                        y2,
                        x,
                        y,
                    } => path.bezier_to(
                        x1 as f32, y1 as f32, x2 as f32, y2 as f32, x as f32, y as f32,
                    ),
                    PathSegment::ClosePath => path.close(),
                }
            }

            let to_femto_color = |usvg_paint: &usvg::Paint| match usvg_paint {
                usvg::Paint::Color(usvg::Color { red, green, blue }) => {
                    Some(Color::rgb(*red, *green, *blue))
                }
                _ => None,
            };

            let fill = svg_path
                .fill
                .as_ref()
                .and_then(|fill| to_femto_color(&fill.paint))
                .map(|col| Paint::color(col).with_anti_alias(true));

            let stroke = svg_path.stroke.as_ref().and_then(|stroke| {
                to_femto_color(&stroke.paint).map(|paint| {
                    let mut stroke_paint = Paint::color(paint);
                    stroke_paint.set_line_width(stroke.width.get() as f32);
                    stroke_paint.set_anti_alias(true);
                    stroke_paint
                })
            });

            paths.push((path, fill, stroke))
        }
    }

    paths
}

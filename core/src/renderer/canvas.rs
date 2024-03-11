use femtovg::renderer::OpenGl;
use femtovg::{Canvas, Color, FontId, ImageFlags, ImageId, Paint, Path};
use glutin::api::egl;
use glutin::api::egl::context::{NotCurrentContext, PossiblyCurrentContext};
use glutin::api::egl::surface::Surface;
use glutin::config::GlConfig;
use glutin::context::{
    ContextAttributesBuilder, NotCurrentGlContextSurfaceAccessor,
    PossiblyCurrentContextGlSurfaceAccessor, PossiblyCurrentGlContext,
};
use glutin::display::GlDisplay;
use glutin::surface::{GlSurface, SurfaceAttributesBuilder, WindowSurface};
use glutin::{api::egl::display::Display, config::ConfigTemplateBuilder};
use resource::resource;
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::{Arc, RwLock};
use std::{fmt, path};
use usvg::fontdb::Database;
use usvg::TreeParsing;

use crate::renderables::Renderable;
use crate::Scale;
use crate::{node::Node, types::PixelSize};

use super::Caches;

#[derive(Debug)]
pub struct SvgData {
    pub paths: Vec<(Path, Option<Paint>, Option<Paint>)>,
    pub scale: Scale,
}

pub fn init_gl<W: crate::window::Window>(
    window: &W,
    (width, height): (u32, u32),
) -> (Display, Surface<WindowSurface>, PossiblyCurrentContext) {
    let template = ConfigTemplateBuilder::new().with_alpha_size(8).build();
    let gl_display =
        unsafe { Display::new(window.raw_display_handle()).expect("Failed to create EGL Display") };

    let config = unsafe { gl_display.find_configs(template) }
        .unwrap()
        .reduce(|config, acc| {
            if config.num_samples() > acc.num_samples() {
                config
            } else {
                acc
            }
        })
        .expect("No available configs");

    let context_attributes = ContextAttributesBuilder::new().build(None);
    let not_current = unsafe {
        gl_display
            .create_context(&config, &context_attributes)
            .expect("Failed to create OpenGL context")
    };

    let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
        window.raw_window_handle(),
        NonZeroU32::new(width).unwrap(),
        NonZeroU32::new(height).unwrap(),
    );

    let gl_surface = unsafe {
        gl_display
            .create_window_surface(&config, &attrs)
            .expect("Failed to create OpenGl surface")
    };

    let gl_context = not_current
        .make_current(&gl_surface)
        .expect("Failed to make newly created OpenGL context current");

    (gl_display, gl_surface, gl_context)
}

pub fn init_gl_canvas(
    gl_display: &Display,
    (width, height): (u32, u32),
    scale_factor: f32,
) -> Canvas<OpenGl> {
    let renderer =
        unsafe { OpenGl::new_from_function_cstr(|s| gl_display.get_proc_address(s) as *const _) }
            .expect("cannot create opengl renderer");

    // create femtovg canvas
    let mut canvas = Canvas::new(renderer).expect("Cannot create canvas");
    canvas.set_size(width, height, scale_factor);

    canvas
}
pub struct CanvasContext {
    // egl context, surface
    pub gl_context: PossiblyCurrentContext,
    pub gl_surface: egl::surface::Surface<WindowSurface>,
    // femto canvas
    pub gl_canvas: femtovg::Canvas<femtovg::renderer::OpenGl>,
}

pub struct CanvasRenderer {
    context: CanvasContext,
    fonts: HashMap<String, FontId>,
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
    fn new<W: crate::window::Window>(window: &W) -> Self {
        let size = window.logical_size();
        let width = size.width;
        let height = size.height;
        let scale_factor = window.scale_factor();

        let (gl_display, gl_surface, gl_context) = init_gl(window, (width, height));
        let mut gl_canvas = init_gl_canvas(&gl_display, (width, height), scale_factor);

        println!("create renderer {} {} {}", width, height, scale_factor);

        let fonts = window.fonts();
        let mut loaded_fonts = HashMap::new();

        for (name, path_) in fonts.into_iter() {
            match gl_canvas.add_font(path_.as_str()) {
                Ok(font_id) => {
                    loaded_fonts.insert(name, font_id);
                }
                Err(e) => {
                    println!("error while loading font {:?} error: {:?}", name, e);
                }
            }
        }

        let assets = window.assets();
        let mut loaded_assets = HashMap::new();

        for (name, path_) in assets.into_iter() {
            match gl_canvas.load_image_file(path_.as_str(), ImageFlags::empty()) {
                Ok(font_id) => {
                    loaded_assets.insert(name, font_id);
                }
                Err(e) => {
                    println!("error while loading font {:?} error: {:?}", name, e);
                }
            }
        }

        let svgs = window.svgs();
        let mut loaded_svgs = HashMap::new();

        for (name, path) in svgs.into_iter() {
            let svg_data = std::fs::read(path).unwrap();
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
            context: CanvasContext {
                gl_context,
                gl_surface,
                gl_canvas,
            },
            fonts: loaded_fonts,
            assets: loaded_assets,
            svgs: loaded_svgs,
        }
    }

    fn render(&mut self, node: &Node, _physical_size: PixelSize) {
        let canvas = &mut self.context.gl_canvas;
        let surface: &Surface<WindowSurface> = &mut self.context.gl_surface;

        let gl_context = &mut self.context.gl_context;

        // let _ = gl_context
        //     .make_current(surface)
        //     .expect("Failed to make newly created OpenGL context current");

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
                    text.render(canvas, &self.fonts);
                }
            }
        }

        // Make smol red rectangle
        // canvas.clear_rect(0, 0, 30, 30, Color::rgbf(0., 1., 0.));

        // Tell renderer to execute all drawing commands
        canvas.flush();

        // Display what we've just rendered
        surface
            .swap_buffers(&gl_context)
            .expect("Could not swap buffers");
    }

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

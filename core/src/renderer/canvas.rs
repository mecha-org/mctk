use super::gl::{init_gl, init_gl_canvas};
use super::svg::{load_svg_paths, SvgData};
use super::text::TextRenderer;
use super::Caches;
use crate::font_cache::FontCache;
use crate::renderables::Renderable;
use crate::{node::Node, types::PixelSize};
use femtovg::renderer::OpenGl;
use femtovg::{Canvas, Color, ImageFlags, ImageId};
use glutin::api::egl;
use glutin::api::egl::context::PossiblyCurrentContext;
use glutin::api::egl::surface::Surface;
use glutin::context::PossiblyCurrentContextGlSurfaceAccessor;
use glutin::surface::{GlSurface, WindowSurface};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, RwLock};

fn load_assets(
    gl_canvas: &mut Canvas<OpenGl>,
    assets: HashMap<String, String>,
) -> HashMap<String, ImageId> {
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
    loaded_assets
}

fn init_canvas_renderer(
    raw_display_handle: RawDisplayHandle,
    raw_window_handle: RawWindowHandle,
    logical_size: PixelSize,
    scale_factor: f32,
    assets: HashMap<String, String>,
) -> (CanvasContext, HashMap<String, ImageId>) {
    let size = logical_size;
    let width = size.width;
    let height = size.height;

    let (gl_display, gl_surface, gl_context) =
        init_gl(raw_display_handle, raw_window_handle, (width, height));
    let mut gl_canvas = init_gl_canvas(&gl_display, (width, height), scale_factor);

    let loaded_assets = load_assets(&mut gl_canvas, assets);

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
    pub gl_canvas: Canvas<OpenGl>,
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
        let mut loaded_svgs = load_svg_paths(svgs, fonts.clone());

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
                    svg.render(canvas, &mut self.svgs);
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
            font: Arc::new(RwLock::new(FontCache::new(self.fonts.clone()))),
        }
    }
}

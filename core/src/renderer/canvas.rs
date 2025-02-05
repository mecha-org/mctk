use super::gl::{init_gl, init_gl_canvas};
use super::svg::{load_svg_paths, SvgData};
use super::text::TextRenderer;
use super::{Caches, RendererContext};
use crate::font_cache::FontCache;
use crate::renderables::Renderable;
use crate::{node::Node, types::PixelSize};
use crate::{AssetParams, ImgFilter};
use femtovg::renderer::OpenGl;
use femtovg::{Canvas, Color, ImageFlags, ImageId, ImageSource};
use glutin::api::egl;
use glutin::api::egl::context::PossiblyCurrentContext;
use glutin::api::egl::surface::Surface;
use glutin::context::{PossiblyCurrentContextGlSurfaceAccessor, PossiblyCurrentGlContext};
use glutin::surface::{GlSurface, WindowSurface};
use image::DynamicImage;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::num::NonZeroU32;
use std::sync::{Arc, RwLock};

pub struct GlCanvasContext {
    // egl context, surface
    pub gl_context: PossiblyCurrentContext,
    pub gl_surface: egl::surface::Surface<WindowSurface>,
    // femto canvas
    pub gl_canvas: Canvas<OpenGl>,
    // canvas images
    pub images: HashMap<String, ImageId>,
}

impl RendererContext for GlCanvasContext {}

pub fn load_assets_to_canvas(
    gl_canvas: &mut Canvas<OpenGl>,
    assets: HashMap<String, AssetParams>,
) -> HashMap<String, ImageId> {
    let mut loaded_assets = HashMap::new();

    for (name, params) in assets.into_iter() {
        let AssetParams { path, filter, blur } = params;
        let image_r = image::open(path);

        if let Err(e) = image_r {
            println!("Error while opening image {:?} error: {:?}", name, e);
            continue;
        }

        let mut image = image_r.unwrap();

        if let Some(sigma) = blur {
            image = image.blur(sigma);
        }

        let buffer;
        let img_src_r = match filter {
            ImgFilter::RGB => ImageSource::try_from(&image),
            ImgFilter::GRAY => {
                //Temporary patch as gray scale image was not rendering
                let gray_scale = image.grayscale().into_rgb8();
                buffer = DynamicImage::ImageRgb8(gray_scale);
                ImageSource::try_from(&buffer)
            }
        };

        if let Err(e) = img_src_r {
            println!("Error while creating image src {:?} error: {:?}", name, e);
            continue;
        }

        let img_src = img_src_r.unwrap();

        let img_create_res = gl_canvas.create_image(img_src, ImageFlags::empty());

        if let Err(img_create_res) = img_create_res {
            println!(
                "Error while creating image {:?} error: {:?}",
                name, img_create_res
            );
            continue;
        }

        let image_id = img_create_res.unwrap();
        let x = gl_canvas.get_image(image_id).unwrap();

        loaded_assets.insert(name, image_id);
    }
    loaded_assets
}

pub struct CanvasRenderer {
    fonts_cache: FontCache,
    text_renderer: TextRenderer,
    assets: HashMap<String, ImageId>,
    svgs: HashMap<String, SvgData>,
}

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
        // let (canvas_context, assets) = init_canvas_context(
        //     window.raw_display_handle(),
        //     window.raw_window_handle(),
        //     window.logical_size(),
        //     scale_factor,
        //     window.assets(),
        // );
        let text_renderer = TextRenderer::new(fonts.clone());
        let svgs = window.svgs();
        let loaded_svgs = load_svg_paths(svgs, fonts.clone());

        Self {
            fonts_cache: FontCache::new(fonts.clone()),
            text_renderer,
            assets: HashMap::new(),
            svgs: loaded_svgs,
        }
    }

    fn resize(&mut self, _: u32, _: u32) {
        // clear any saved canvas linked caches,
        // for e.g the text renderer stored glyphs
        // that were rendered by the canvas
        // but during resizing canvas is re-initialized
        self.text_renderer.clear();
    }

    fn render(&mut self, node: &Node, _physical_size: PixelSize, ctx: &mut (dyn Any + 'static)) {
        let context = &mut ctx.downcast_mut::<GlCanvasContext>().unwrap();
        let canvas = &mut context.gl_canvas;
        let surface: &Surface<WindowSurface> = &context.gl_surface;
        let text_renderer = &mut self.text_renderer;

        let gl_context = &context.gl_context;

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

        for (renderable, _, _) in node.iter_renderables() {
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
                    image.render(canvas, &mut context.images);
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
                Renderable::Curve(curve) => {
                    curve.render(canvas);
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
    fn caches(&mut self) -> Caches {
        // println!("caches()");
        Caches {
            font: Arc::new(RwLock::new(&mut self.fonts_cache)),
        }
    }
}

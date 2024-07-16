use femtovg::renderer::OpenGl;
use femtovg::Canvas;
use glutin::api::egl::context::PossiblyCurrentContext;
use glutin::api::egl::surface::Surface;
use glutin::config::GlConfig;
use glutin::context::{ContextApi, ContextAttributesBuilder, NotCurrentGlContextSurfaceAccessor};
use glutin::display::GlDisplay;
use glutin::surface::{SurfaceAttributesBuilder, WindowSurface};
use glutin::{api::egl::display::Display, config::ConfigTemplateBuilder};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use std::num::NonZeroU32;

pub fn init_gl_surface_context(
    raw_window_handle: RawWindowHandle,
    (width, height): (u32, u32),
    gl_display: &Display,
) -> (Surface<WindowSurface>, PossiblyCurrentContext) {
    let template = ConfigTemplateBuilder::new().with_alpha_size(8).build();

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

    let context_attributes = ContextAttributesBuilder::new().build(Some(raw_window_handle));

    // Since glutin by default tries to create OpenGL core context, which may not be
    // present we should try gles.
    let fallback_context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::Gles(None))
        .build(Some(raw_window_handle));
    let mut not_current_gl_context = Some(unsafe {
        gl_display
            .create_context(&config, &context_attributes)
            .unwrap_or_else(|_| {
                gl_display
                    .create_context(&config, &fallback_context_attributes)
                    .expect("failed to create context")
            })
    });

    let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
        raw_window_handle,
        NonZeroU32::new(width).unwrap(),
        NonZeroU32::new(height).unwrap(),
    );

    let gl_surface = unsafe {
        gl_display
            .create_window_surface(&config, &attrs)
            .expect("Failed to create OpenGl surface")
    };

    let gl_context = not_current_gl_context
        .take()
        .unwrap()
        .make_current(&gl_surface)
        .expect("Failed to make newly created OpenGL context current");

    (gl_surface, gl_context)
}

pub fn init_gl(
    raw_display_handle: RawDisplayHandle,
    raw_window_handle: RawWindowHandle,
    (width, height): (u32, u32),
) -> (Display, Surface<WindowSurface>, PossiblyCurrentContext) {
    let gl_display =
        unsafe { Display::new(raw_display_handle).expect("Failed to create EGL Display") };

    let (gl_surface, gl_context) =
        init_gl_surface_context(raw_window_handle, (width, height), &gl_display);

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

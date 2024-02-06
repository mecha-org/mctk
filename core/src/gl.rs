use std::num::NonZeroU32;
use glutin::api::egl::context::PossiblyCurrentContext;
use glutin::api::egl::surface::Surface;
use glutin::config::GlConfig;
use glutin::context::{ContextAttributesBuilder, NotCurrentGlContextSurfaceAccessor};
use glutin::surface::{SurfaceAttributesBuilder, WindowSurface};
use glutin::{api::egl::display::Display, config::ConfigTemplateBuilder};
use glutin::display::GlDisplay;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

use super::raw_handle::RawWaylandHandle;

pub fn init_gl(wayland_handle: RawWaylandHandle, (width, height): (u32, u32))
    -> (Display, Surface<WindowSurface>, PossiblyCurrentContext) {
    let template = ConfigTemplateBuilder::new().with_alpha_size(8).build();
    let gl_display =
        unsafe { Display::new(wayland_handle.raw_display_handle()).expect("Failed to create EGL Display") };

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
        wayland_handle.raw_window_handle(),
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

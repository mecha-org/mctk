use femtovg::{renderer::OpenGl, Canvas, Color, Renderer};
use glutin::{api::egl::display::Display, display::GlDisplay};

pub fn init_gl_canvas(gl_display: &Display, (width, height): (u32, u32), scale_factor: f32) -> Canvas<OpenGl>  {
    let renderer = unsafe { OpenGl::new_from_function_cstr(|s| gl_display.get_proc_address(s) as *const _) }
        .expect("cannot create opengl renderer");
    
    // create femtovg canvas
    let mut canvas = Canvas::new(renderer).expect("Cannot create canvas");
    canvas.set_size(width, height, 1.0);
    
    canvas
}


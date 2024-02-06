use femtovg::{renderer::OpenGl, Canvas};
use glutin::{api::egl::{context::PossiblyCurrentContext, surface::Surface}, surface::WindowSurface};
use super::input::pointer::{Cursor, MouseEvent};

#[derive(Debug, Clone)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
    pub scale_factor: f32,
}

#[derive(Debug, Clone)]
pub enum Event {
    Mouse(MouseEvent),
}

pub trait CanvasApplication {
    fn dispatch(
        &mut self,
        gl_context: &PossiblyCurrentContext,
        gl_surface: &Surface<WindowSurface>,
        canvas: &mut Canvas<OpenGl>,
        viewport: &Viewport,
        cursor: Cursor
    );
    fn push_event(&mut self, event: Event);
    fn render(
        &self,
        gl_context: &PossiblyCurrentContext,
        gl_surface: &Surface<WindowSurface>,
        canvas: &mut Canvas<OpenGl>,
        viewport: &Viewport
    );
}

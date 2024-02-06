use femtovg::{renderer::OpenGl, Canvas, Color};
use glutin::{
    api::egl::{context::PossiblyCurrentContext, surface::Surface},
    surface::{GlSurface, WindowSurface},
};
use mctk_core::{
    app::{CanvasApplication, Event, Viewport},
    input::pointer::{Cursor, MouseEvent},
};

pub enum Message {
    Trigger,
}

pub struct DemoApp {
    color: Color,
    event_queue: Vec<Event>,
    message_queue: Vec<Message>,
}

impl DemoApp {
    pub fn new() -> Self {
        Self {
            color: Color::rgbaf(0., 1., 0., 0.5),
            event_queue: vec![],
            message_queue: vec![],
        }
    }

    pub fn push_message(&mut self, message: Message) {
        let _ = &self.message_queue.push(message);
    }
}

impl CanvasApplication for DemoApp {
    fn push_event(&mut self, event: Event) {
        // println!("event received event={:?}", event);
        let _ = &self.event_queue.push(event);
    }

    fn dispatch(
        &mut self,
        gl_context: &PossiblyCurrentContext,
        gl_surface: &Surface<WindowSurface>,
        canvas: &mut Canvas<OpenGl>,
        viewport: &Viewport,
        _: Cursor,
    ) {
        // println!("dispatch received viewport={:?}, cursor={:?}", viewport, cursor);
        if self.event_queue.is_empty() && self.message_queue.is_empty() {
            return ();
        }

        // check if any click event
        let iter = self.event_queue.iter();
        for event in iter {
            let _ = match event {
                Event::Mouse(m) => match m {
                    MouseEvent::ButtonPressed { .. } => {
                        println!("button pressed");
                        self.color = Color::rgbaf(1., 0., 0., 0.5);
                    }
                    _ => {}
                },
                _ => {}
            };
        }

        // check if any trigger
        let iter = self.message_queue.iter();
        for message in iter {
            let _ = match message {
                Message::Trigger => {
                    self.color = Color::rgbaf(0., 0., 1., 1.0);
                }
                _ => {}
            };
        }

        let _ = &self.render(gl_context, gl_surface, canvas, viewport);
        self.event_queue.clear();
        self.message_queue.clear();
    }

    fn render(
        &self,
        gl_context: &PossiblyCurrentContext,
        gl_surface: &Surface<WindowSurface>,
        canvas: &mut Canvas<OpenGl>,
        viewport: &Viewport,
    ) {
        let Viewport { width, height, .. } = viewport;

        canvas.clear_rect(0, 0, *width, *height, Color::rgbaf(0., 0., 0., 0.5));

        // Make smol red rectangle
        canvas.clear_rect(30, 30, 90, 90, self.color);

        // Tell renderer to execute all drawing commands
        canvas.flush();
        let _ = gl_surface
            .swap_buffers(gl_context)
            .expect("Could not swap buffers");
    }
}

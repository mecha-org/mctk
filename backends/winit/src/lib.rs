use std::collections::HashMap;

use mctk_core::component::{Component, RootComponent};
use mctk_core::input::{Button, Input, Motion, MouseButton};
use mctk_core::reexports::cosmic_text;
use mctk_core::reexports::smithay_client_toolkit::reexports::calloop::channel::Sender;
use mctk_core::types::AssetParams;
use mctk_core::types::PixelSize;
use mctk_core::ui::UI;
use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub struct Window {
    winit_window: winit::window::Window,
    fonts: cosmic_text::fontdb::Database,
    assets: HashMap<String, AssetParams>,
    svgs: HashMap<String, String>,
}
unsafe impl Send for Window {}
unsafe impl Sync for Window {}

impl Window {
    pub fn open_blocking<A, B>(
        title: &str,
        width: u32,
        height: u32,
        fonts: cosmic_text::fontdb::Database,
        assets: HashMap<String, AssetParams>,
        svgs: HashMap<String, String>,
        app_channel: Option<Sender<B>>,
    ) where
        A: 'static + RootComponent<B> + Component + Default + Send + Sync,
        B: 'static,
    {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_title(title)
            .with_inner_size(LogicalSize::new(width as f32, height as f32))
            .build(&event_loop)
            .unwrap();
        let mut ui: UI<Window, A, B> = UI::new(
            Window {
                winit_window: window,
                fonts,
                assets,
                svgs,
            },
            app_channel,
        );

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::MainEventsCleared => {
                    println!("event::maineventscleared");
                    println!("ui::draw");
                    ui.draw();
                }
                Event::RedrawRequested(_) => {
                    println!("event::redrawrequested");
                    println!("ui::render");
                    ui.render();
                }
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::CursorMoved { position, .. } => {
                        let scale_factor = ui.window.read().unwrap().winit_window.scale_factor();
                        // println!("{:?}", position);
                        ui.handle_input(&Input::Motion(Motion::Mouse {
                            x: position.x as f32 / scale_factor as f32,
                            y: position.y as f32 / scale_factor as f32,
                        }));
                    }
                    WindowEvent::MouseInput {
                        button: _,
                        state: winit::event::ElementState::Pressed,
                        ..
                    } => {
                        ui.handle_input(&Input::Press(Button::Mouse(MouseButton::Left)));
                    }
                    WindowEvent::MouseInput {
                        button: _,
                        state: winit::event::ElementState::Released,
                        ..
                    } => {
                        ui.handle_input(&Input::Release(Button::Mouse(MouseButton::Left)));
                    }
                    WindowEvent::MouseWheel { delta, .. } => {
                        // println!("scroll delta{:?}", delta);
                        let scroll = match delta {
                            winit::event::MouseScrollDelta::LineDelta(x, y) => Motion::Scroll {
                                x: x * -10.0,
                                y: y * -10.0,
                            },
                            winit::event::MouseScrollDelta::PixelDelta(
                                winit::dpi::PhysicalPosition { x, y },
                            ) => Motion::Scroll {
                                x: -x as f32,
                                y: -y as f32,
                            },
                        };
                        ui.handle_input(&Input::Motion(scroll));
                    }
                    _ => (),
                },
                _ => (),
            };

            // inst_end();
        });
    }
}

impl mctk_core::window::Window for Window {
    // TODO: This isn't good

    fn logical_size(&self) -> PixelSize {
        let size = self.winit_window.inner_size();
        PixelSize {
            width: size.width,
            height: size.width,
        }
    }

    fn physical_size(&self) -> PixelSize {
        // let size = self.winit_window.inner_size();
        self.logical_size() // This should transform to device size
    }

    fn scale_factor(&self) -> f32 {
        winit::window::Window::scale_factor(&self.winit_window) as f32
    }

    fn redraw(&self) {
        self.winit_window.request_redraw();
    }

    fn exit(&mut self) {}

    fn fonts(&self) -> cosmic_text::fontdb::Database {
        self.fonts.clone()
    }

    fn assets(&self) -> HashMap<String, AssetParams> {
        self.assets.clone()
    }

    fn svgs(&self) -> HashMap<String, String> {
        self.svgs.clone()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

unsafe impl HasRawWindowHandle for Window {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.winit_window.raw_window_handle()
    }
}

unsafe impl HasRawDisplayHandle for Window {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        self.winit_window.raw_display_handle()
    }
}

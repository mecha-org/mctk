mod gl;
pub mod layer;
mod pointer;
use std::collections::HashMap;
use std::thread;

use layer::{LayerApp, LayerOptions};
use mctk_core::component::{self, Component};
use mctk_core::input::{Button, Input, Motion, MouseButton};
use mctk_core::raw_handle::RawWaylandHandle;
use mctk_core::reexports::femtovg::Color;
use mctk_core::reexports::glutin::api::egl::surface::Surface;
use mctk_core::reexports::glutin::surface::{GlSurface, WindowSurface};
use mctk_core::renderer::canvas::CanvasContext;
use mctk_core::types::PixelSize;
use mctk_core::ui::UI;
use pointer::{MouseEvent, ScrollDelta};
use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use smithay_client_toolkit::reexports::calloop::channel::{Event, Sender};
use smithay_client_toolkit::reexports::calloop::{self, EventLoop};
use smithay_client_toolkit::shell::wlr_layer;
// use winit::{
//     dpi::LogicalSize,
//     event::{Event, WindowEvent},
//     event_loop::{ControlFlow, EventLoop},
//     window::WindowBuilder,
// };

// pub enum WindowType {
//     LayerShell(LayerApp)
// }

pub struct PhysicalPosition<P> {
    pub x: P,
    pub y: P,
}

pub struct WindowOptions {
    pub height: u32,
    pub width: u32,
    pub scale_factor: f32,
}
pub struct Window {
    width: u32,
    height: u32,
    scale_factor: f32,
    handle: RawWaylandHandle,
    window_tx: Sender<WindowMessage>,
    fonts: HashMap<String, String>,
    assets: HashMap<String, String>,
    svgs: HashMap<String, String>,
}
unsafe impl Send for Window {}
unsafe impl Sync for Window {}

#[derive(Debug)]
pub enum WindowMessage {
    MainEventsCleared,
    RedrawRequested,
    Send { message: component::Message },
    WindowEvent { event: WindowEvent },
}
#[derive(Debug, Copy, Clone)]
pub enum WindowEvent {
    CloseRequested,
    Mouse(MouseEvent),
    KeyboardInput,
}

pub struct OpenBlockingParams {
    pub title: String,
    pub namespace: String,
    pub window_opts: WindowOptions,
    pub fonts: HashMap<String, String>,
    pub assets: HashMap<String, String>,
    pub svgs: HashMap<String, String>,
    pub layer_shell_opts: LayerOptions,
}

impl Window {
    pub fn open_blocking<A>(
        params: OpenBlockingParams,
    ) -> (
        LayerApp,
        EventLoop<'static, LayerApp>,
        Sender<WindowMessage>,
    )
    where
        A: 'static + Component + Default + Send + Sync,
    {
        let OpenBlockingParams {
            title,
            namespace,
            window_opts,
            fonts,
            assets,
            svgs,
            layer_shell_opts,
        } = params;

        let (window_tx, window_rx) = calloop::channel::channel();

        let (app_window, event_loop) =
            LayerApp::new(window_tx.clone(), window_opts, layer_shell_opts)
                .expect("failed to create application");

        let mut ui: UI<Window, A> = UI::new(Window {
            width: app_window.width,
            height: app_window.height,
            handle: app_window.wayland_handle,
            scale_factor: app_window.scale_factor,
            window_tx: window_tx.clone(),
            fonts,
            assets,
            svgs,
        });

        // let sig = event_loop.get_signal();

        // insert handle
        let handle = event_loop.handle();
        let _ = handle.insert_source(
            window_rx,
            move |ev: Event<WindowMessage>, &mut _, app_window| {
                let _ = match ev {
                    calloop::channel::Event::Msg(event) => {
                        match event {
                            WindowMessage::Send { message } => {
                                ui.update(message);
                            }
                            WindowMessage::MainEventsCleared => {
                                println!("event::main_events_cleared");
                                ui.draw();
                                ui.render();
                            }
                            WindowMessage::RedrawRequested => {
                                ui.draw();
                                ui.render();
                            }
                            WindowMessage::WindowEvent { event: w_ev } => {
                                // println!("window_event::{:?}", w_ev);
                                match w_ev {
                                    WindowEvent::CloseRequested => {
                                        // signal loop to stop
                                        // sig.stop();
                                    }
                                    WindowEvent::Mouse(m_event) => match m_event {
                                        MouseEvent::CursorEntered => {}
                                        MouseEvent::CursorLeft => {}
                                        MouseEvent::CursorMoved {
                                            position,
                                            scale_factor,
                                        } => {
                                            ui.handle_input(&Input::Motion(Motion::Mouse {
                                                x: position.x as f32 / scale_factor as f32,
                                                y: position.y as f32 / scale_factor as f32,
                                            }));
                                        }
                                        MouseEvent::ButtonPressed { button } => match button {
                                            pointer::Button::Left => ui.handle_input(
                                                &Input::Press(Button::Mouse(MouseButton::Left)),
                                            ),
                                            pointer::Button::Right => ui.handle_input(
                                                &Input::Press(Button::Mouse(MouseButton::Right)),
                                            ),
                                            pointer::Button::Middle => ui.handle_input(
                                                &Input::Press(Button::Mouse(MouseButton::Middle)),
                                            ),
                                        },
                                        MouseEvent::ButtonReleased { button } => match button {
                                            pointer::Button::Left => ui.handle_input(
                                                &Input::Release(Button::Mouse(MouseButton::Left)),
                                            ),
                                            pointer::Button::Right => ui.handle_input(
                                                &Input::Release(Button::Mouse(MouseButton::Right)),
                                            ),
                                            pointer::Button::Middle => ui.handle_input(
                                                &Input::Release(Button::Mouse(MouseButton::Middle)),
                                            ),
                                        },
                                        MouseEvent::WheelScrolled { delta } => {
                                            let scroll = match delta {
                                                ScrollDelta::Lines { x, y } => Motion::Scroll {
                                                    x: x * -10.0,
                                                    y: y * -10.0,
                                                },
                                                ScrollDelta::Pixels { x, y } => Motion::Scroll {
                                                    x: -x as f32,
                                                    y: -y as f32,
                                                },
                                            };
                                            ui.handle_input(&Input::Motion(scroll));
                                        }
                                    },
                                    WindowEvent::KeyboardInput => todo!(),
                                    // _ => {},
                                }
                            }
                        }
                    }
                    calloop::channel::Event::Closed => {}
                };
            },
        );

        (app_window, event_loop, window_tx.clone())

        // event_loop
        //     .run(None, &mut window, |_| {})
        //     .expect("Error during event loop!");

        // for (name, data) in fonts.drain(..) {
        //     ui.add_font(name, data);
        // }

        // event_loop.run(move |event, _, control_flow| {
        //     *control_flow = ControlFlow::Wait;
        //     // inst(&format!("event_handler <{:?}>", &event));

        //     match event {
        //         Event::MainEventsCleared => {
        //             ui.draw();
        //         }
        //         Event::RedrawRequested(_) => ui.render(),
        //         Event::WindowEvent { event, .. } => match event {
        //             WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
        //             WindowEvent::CursorMoved { position, .. } => {
        //                 let scale_factor = ui.window.read().unwrap().inner_window.scale_factor();
        //                 // println!("{:?}", position);
        //                 ui.handle_input(&Input::Motion(Motion::Mouse {
        //                     x: position.x as f32 / scale_factor as f32,
        //                     y: position.y as f32 / scale_factor as f32,
        //                 }));
        //             }
        //             WindowEvent::MouseInput {
        //                 button: _,
        //                 state: winit::event::ElementState::Pressed,
        //                 ..
        //             } => {
        //                 ui.handle_input(&Input::Press(Button::Mouse(MouseButton::Left)));
        //             }
        //             WindowEvent::MouseInput {
        //                 button: _,
        //                 state: winit::event::ElementState::Released,
        //                 ..
        //             } => {
        //                 ui.handle_input(&Input::Release(Button::Mouse(MouseButton::Left)));
        //             }
        //             WindowEvent::MouseWheel { delta, .. } => {
        //                 // println!("scroll delta{:?}", delta);
        //                 let scroll = match delta {
        //                     winit::event::MouseScrollDelta::LineDelta(x, y) => Motion::Scroll {
        //                         x: x * -10.0,
        //                         y: y * -10.0,
        //                     },
        //                     winit::event::MouseScrollDelta::PixelDelta(
        //                         winit::dpi::PhysicalPosition { x, y },
        //                     ) => Motion::Scroll {
        //                         x: -x as f32,
        //                         y: -y as f32,
        //                     },
        //                 };
        //                 ui.handle_input(&Input::Motion(scroll));
        //             }
        //             _ => (),
        //         },
        //         _ => (),
        //     };

        //     // inst_end();
        // });
    }
}

impl mctk_core::window::Window for Window {
    // TODO: This isn't good

    fn logical_size(&self) -> PixelSize {
        PixelSize {
            width: self.width,
            height: self.height,
        }
    }

    fn physical_size(&self) -> PixelSize {
        // let size = self.inner_window.inner_size();
        self.logical_size() // This should transform to device size
    }

    fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    fn redraw(&self) {
        println!("window::redraw()");
        let _ = self.window_tx.send(WindowMessage::RedrawRequested);
    }

    fn fonts(&self) -> HashMap<String, String> {
        self.fonts.clone()
    }

    fn assets(&self) -> HashMap<String, String> {
        self.assets.clone()
    }

    fn svgs(&self) -> HashMap<String, String> {
        self.svgs.clone()
    }
}

unsafe impl HasRawWindowHandle for Window {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.handle.raw_window_handle()
    }
}

unsafe impl HasRawDisplayHandle for Window {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        self.handle.raw_display_handle()
    }
}

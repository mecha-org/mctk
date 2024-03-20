use layer::{LayerOptions, LayerShellSctkWindow};
use mctk_core::component::{self, Component, RootComponent};
use mctk_core::input::{Button, Input, Motion, MouseButton};
use mctk_core::raw_handle::RawWaylandHandle;
use mctk_core::types::PixelSize;
use mctk_core::ui::UI;
use pointer::{MouseEvent, ScrollDelta};
use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use smithay_client_toolkit::reexports::calloop::channel::{Event, Sender};
use smithay_client_toolkit::reexports::calloop::{self, EventLoop};
use std::collections::HashMap;
use std::ptr::null;

use crate::{layer, pointer, WindowEvent, WindowMessage, WindowOptions};

pub struct LayerWindow {
    width: u32,
    height: u32,
    scale_factor: f32,
    handle: Option<RawWaylandHandle>,
    window_tx: Sender<WindowMessage>,
    fonts: HashMap<String, String>,
    assets: HashMap<String, String>,
    svgs: HashMap<String, String>,
}
unsafe impl Send for LayerWindow {}
unsafe impl Sync for LayerWindow {}

pub struct LayerWindowParams {
    pub title: String,
    pub namespace: String,
    pub window_opts: WindowOptions,
    pub fonts: HashMap<String, String>,
    pub assets: HashMap<String, String>,
    pub svgs: HashMap<String, String>,
    pub layer_shell_opts: LayerOptions,
}

impl LayerWindow {
    pub fn open_blocking<A>(
        params: LayerWindowParams,
    ) -> (
        LayerShellSctkWindow,
        EventLoop<'static, LayerShellSctkWindow>,
        Sender<WindowMessage>,
    )
    where
        A: 'static + RootComponent + Component + Default + Send + Sync,
    {
        let LayerWindowParams {
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
            LayerShellSctkWindow::new(window_tx.clone(), window_opts, layer_shell_opts)
                .expect("failed to create application");

        // let (app_window, event_loop) =
        //     SessionLockWindow::new(window_tx.clone(), window_opts)
        //         .expect("failed to create application");

        let mut ui: UI<LayerWindow, A> = UI::new(LayerWindow {
            width: app_window.width,
            height: app_window.height,
            handle: None,
            scale_factor: app_window.scale_factor,
            window_tx: window_tx.clone(),
            fonts,
            assets,
            svgs,
        });

        let window_tx_loop = window_tx.clone();

        // let sig = event_loop.get_signal();

        // insert handle
        let handle = event_loop.handle();
        let _ = handle.insert_source(
            window_rx,
            move |ev: Event<WindowMessage>, &mut _, app_window| {
                let _ = match ev {
                    calloop::channel::Event::Msg(event) => {
                        match event {
                            WindowMessage::Configure {
                                width,
                                height,
                                wayland_handle,
                            } => {
                                ui.configure(width, height, wayland_handle);
                                ui.draw();
                                ui.render();
                            }
                            WindowMessage::Send { message } => {
                                ui.update(message);
                            }
                            WindowMessage::MainEventsCleared => {
                                ui.draw();
                                ui.render();
                            }
                            WindowMessage::RedrawRequested => {
                                ui.handle_input(&Input::Timer);
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
    }
}

impl mctk_core::window::Window for LayerWindow {
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

    fn set_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    fn set_wayland_handle(&mut self, wayland_handle: RawWaylandHandle) {
        self.handle = Some(wayland_handle);
    }

    fn has_handle(&self) -> bool {
        self.handle.is_some()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

unsafe impl HasRawWindowHandle for LayerWindow {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.handle.unwrap().raw_window_handle()
    }
}

unsafe impl HasRawDisplayHandle for LayerWindow {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        self.handle.unwrap().raw_display_handle()
    }
}

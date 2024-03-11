use std::sync::{Arc, RwLock};

use crate::{
    gl::{init_gl, init_gl_canvas},
    pointer::{convert_button, MouseEvent, Point, ScrollDelta},
    PhysicalPosition, WindowEvent, WindowMessage, WindowOptions,
};
use anyhow::Context;
use glutin::api::egl::{self, context::PossiblyCurrentContext};
use mctk_core::{
    raw_handle::RawWaylandHandle,
    reexports::{
        femtovg::{self, Color},
        glutin::{
            self,
            surface::{GlSurface, WindowSurface},
        },
    },
    renderer::canvas::CanvasContext,
};
use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat,
    output::{OutputHandler, OutputState},
    reexports::{
        calloop::{channel::Sender, EventLoop},
        calloop_wayland_source::WaylandSource,
        client::{
            globals::registry_queue_init,
            protocol::{
                wl_keyboard::{self, WlKeyboard},
                wl_output::{self, WlOutput},
                wl_pointer::{self, AxisSource, WlPointer},
                wl_seat::WlSeat,
                wl_surface::WlSurface,
            },
            Connection, Proxy, QueueHandle,
        },
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers},
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    shell::{
        wlr_layer::{self, LayerShell, LayerShellHandler, LayerSurface},
        WaylandSurface,
    },
};

pub struct LayerApp {
    window_tx: Sender<WindowMessage>,
    pub registry_state: RegistryState,
    pub seat_state: SeatState,
    pub output_state: OutputState,
    pub layer: LayerSurface,
    pub width: u32,
    pub height: u32,
    pub keyboard: Option<wl_keyboard::WlKeyboard>,
    pub keyboard_focus: bool,
    pub keyboard_modifiers: Modifiers,
    pub pointer: Option<wl_pointer::WlPointer>,
    pub initial_configure_sent: bool,
    pub wayland_handle: RawWaylandHandle,
    pub scale_factor: f32,
    pub exit: bool,
    // pub gl_context: PossiblyCurrentContext,
    // pub gl_surface: egl::surface::Surface<WindowSurface>,

    // femto canvas
    // pub gl_canvas: femtovg::Canvas<femtovg::renderer::OpenGl>,
}

pub struct LayerOptions {
    pub anchor: wlr_layer::Anchor,
    pub layer: wlr_layer::Layer,
    pub keyboard_interactivity: wlr_layer::KeyboardInteractivity,
    pub namespace: Option<String>,
    pub zone: i32,
}

impl LayerApp {
    pub fn new(
        window_tx: Sender<WindowMessage>,
        window_opts: WindowOptions,
        layer_opts: LayerOptions,
    ) -> anyhow::Result<(Self, EventLoop<'static, Self>)> {
        let conn = Connection::connect_to_env().expect("failed to connect to wayland");
        let event_loop = EventLoop::<Self>::try_new()?;
        let WindowOptions {
            height,
            width,
            scale_factor,
        } = window_opts;
        let LayerOptions {
            anchor,
            layer,
            keyboard_interactivity,
            namespace,
            zone,
        } = layer_opts;

        let (globals, event_queue) =
            registry_queue_init::<Self>(&conn).context("failed to init registry queue")?;

        let queue_handle = event_queue.handle();

        let loop_handle = event_loop.handle();
        WaylandSource::new(conn.clone(), event_queue)
            .insert(loop_handle.clone())
            .expect("failed to insert wayland source into event loop");

        let compositor = CompositorState::bind(&globals, &queue_handle)
            .context("wl_compositor not availible")?;

        let layer_shell =
            LayerShell::bind(&globals, &queue_handle).context("layer shell not availible")?;

        let surface = compositor.create_surface(&queue_handle);
        let layer =
            layer_shell.create_layer_surface(&queue_handle, surface, layer, namespace, None);

        // set layer shell props
        layer.set_keyboard_interactivity(keyboard_interactivity);
        layer.set_size(width, height);
        layer.set_anchor(anchor);
        layer.set_exclusive_zone(zone);

        layer.commit();

        // create wayland handle
        let wayland_handle = {
            let mut handle = WaylandDisplayHandle::empty();
            handle.display = conn.backend().display_ptr() as *mut _;
            let display_handle = RawDisplayHandle::Wayland(handle);

            let mut handle = WaylandWindowHandle::empty();
            handle.surface = layer.wl_surface().id().as_ptr() as *mut _;
            let window_handle = RawWindowHandle::Wayland(handle);

            RawWaylandHandle(display_handle, window_handle)
        };

        let (gl_display, gl_surface, gl_context) = init_gl(wayland_handle, (width, height));
        let gl_canvas = init_gl_canvas(&gl_display, (width, height), scale_factor);

        let canvas_context = CanvasContext {
            gl_context,
            gl_surface,
            gl_canvas,
        };

        let state = LayerApp {
            // app,
            window_tx,
            registry_state: RegistryState::new(&globals),
            seat_state: SeatState::new(&globals, &queue_handle),
            output_state: OutputState::new(&globals, &queue_handle),
            layer,
            width,
            height,
            keyboard: None,
            keyboard_focus: false,
            keyboard_modifiers: Modifiers::default(),
            pointer: None,
            initial_configure_sent: false,
            wayland_handle,
            scale_factor,
            exit: false,
            // gl_context,
            // gl_surface,
            // gl_canvas,
        };

        Ok((state, event_loop))
    }

    // pub fn render(&mut self) {
    //     let canvas = &mut self.gl_canvas;
    //     let surface = &self.gl_surface;

    //     // Make smol red rectangle
    //     canvas.clear_rect(30, 30, 30, 30, Color::rgbf(1., 0., 0.));

    //     // Tell renderer to execute all drawing commands
    //     canvas.flush();

    //     // Display what we've just rendered
    //     surface.swap_buffers(&self.gl_context).expect("Could not swap buffers");
    // }

    pub fn draw(&mut self, queue_handle: &QueueHandle<Self>, surface: &WlSurface) {
        if self.layer.wl_surface() != surface {
            return;
        }

        // for continous rendering
        self.send_redraw_requested();

        self.layer
            .wl_surface()
            .damage_buffer(0, 0, self.width as i32, self.height as i32);

        self.layer
            .wl_surface()
            .frame(queue_handle, self.layer.wl_surface().clone());

        self.layer.commit();
    }

    pub fn send_main_events_cleared(&mut self) {
        let _ = &self.window_tx.send(WindowMessage::MainEventsCleared);
    }

    pub fn send_close_requested(&mut self) {
        let _ = &self.window_tx.send(WindowMessage::WindowEvent {
            event: WindowEvent::CloseRequested,
        });
    }

    pub fn send_redraw_requested(&mut self) {
        let _ = &self.window_tx.send(WindowMessage::RedrawRequested);
    }

    pub fn send_window_event(&mut self, event: WindowEvent) {
        let _ = &self.window_tx.send(WindowMessage::WindowEvent { event });
    }
}

impl CompositorHandler for LayerApp {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        new_scale_factor: i32,
    ) {
        self.scale_factor = new_scale_factor as f32;
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        surface: &WlSurface,
        _time: u32,
    ) {
        self.draw(qh, surface);
    }

    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlSurface,
        _: wl_output::Transform,
    ) {
        // TODO handle transform change
    }
}

impl OutputHandler for LayerApp {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {}

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {}

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {}
}

impl LayerShellHandler for LayerApp {
    fn closed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _layer: &wlr_layer::LayerSurface,
    ) {
        let _ = &self.send_close_requested();
        self.exit = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        layer: &wlr_layer::LayerSurface,
        _: wlr_layer::LayerSurfaceConfigure,
        _serial: u32,
    ) {
        // TODO: handle resize?
        self.send_main_events_cleared();
        self.draw(qh, layer.wl_surface());
        self.initial_configure_sent = true;
    }
}

impl SeatHandler for LayerApp {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            let keyboard = self.seat_state.get_keyboard(qh, &seat, None).unwrap();
            self.keyboard = Some(keyboard);
        }
        if capability == Capability::Pointer && self.pointer.is_none() {
            let pointer = self.seat_state.get_pointer(qh, &seat).unwrap();
            self.pointer = Some(pointer);
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard {
            if let Some(keyboard) = self.keyboard.take() {
                keyboard.release();
            }
        }
        if capability == Capability::Pointer {
            if let Some(pointer) = self.pointer.take() {
                pointer.release();
            }
        }
    }

    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: WlSeat) {}
}

impl KeyboardHandler for LayerApp {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        surface: &WlSurface,
        _serial: u32,
        _raw: &[u32],
        _keysyms: &[Keysym],
    ) {
        if self.layer.wl_surface() != surface {
            return;
        }

        self.keyboard_focus = true;
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        surface: &WlSurface,
        _serial: u32,
    ) {
        if self.layer.wl_surface() != surface {
            return;
        }

        self.keyboard_focus = false;
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        if !self.keyboard_focus {
            return;
        }

        // let Some(keycode) = crate::layer_shell::keyboard::keysym_to_keycode(event.keysym) else {
        //     return;
        // };

        // let mut modifiers = keyboard::Modifiers::default();

        // let Modifiers {
        //     ctrl,
        //     alt,
        //     shift,
        //     caps_lock: _,
        //     logo,
        //     num_lock: _,
        // } = &self.keyboard_modifiers;

        // if *ctrl {
        //     modifiers |= keyboard::Modifiers::CTRL;
        // }
        // if *alt {
        //     modifiers |= keyboard::Modifiers::ALT;
        // }
        // if *shift {
        //     modifiers |= keyboard::Modifiers::SHIFT;
        // }
        // if *logo {
        //     modifiers |= keyboard::Modifiers::LOGO;
        // }

        // let event = Event::Keyboard(keyboard::KeyboardEvent::KeyPressed {
        //     key_code: keycode,
        //     modifiers,
        // });

        // let _ = &self.app.push_event(event);
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        if !self.keyboard_focus {
            return;
        }

        // let Some(keycode) = crate::layer_shell::keyboard::keysym_to_keycode(event.keysym) else {
        //     return;
        // };

        // let mut modifiers = keyboard::Modifiers::default();

        // let Modifiers {
        //     ctrl,
        //     alt,
        //     shift,
        //     caps_lock: _,
        //     logo,
        //     num_lock: _,
        // } = &self.keyboard_modifiers;

        // if *ctrl {
        //     modifiers |= keyboard::Modifiers::CTRL;
        // }
        // if *alt {
        //     modifiers |= keyboard::Modifiers::ALT;
        // }
        // if *shift {
        //     modifiers |= keyboard::Modifiers::SHIFT;
        // }
        // if *logo {
        //     modifiers |= keyboard::Modifiers::LOGO;
        // }

        // let event = Event::Keyboard(keyboard::KeyboardEvent::KeyReleased {
        //     key_code: keycode,
        //     modifiers,
        // });

        // let _ = &self.app.push_event(event);
    }

    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _serial: u32,
        modifiers: Modifiers,
    ) {
        self.keyboard_modifiers = modifiers;
    }
}

impl PointerHandler for LayerApp {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            if &event.surface != self.layer.wl_surface() {
                continue;
            }

            let window_event = match event.kind {
                PointerEventKind::Enter { .. } => WindowEvent::Mouse(MouseEvent::CursorEntered),
                PointerEventKind::Leave { .. } => WindowEvent::Mouse(MouseEvent::CursorLeft),
                PointerEventKind::Motion { .. } => WindowEvent::Mouse(MouseEvent::CursorMoved {
                    position: Point {
                        x: event.position.0 as f32,
                        y: event.position.1 as f32,
                    },
                    scale_factor: self.scale_factor,
                }),
                PointerEventKind::Press { button, .. } => {
                    if let Some(button) = convert_button(button) {
                        WindowEvent::Mouse(MouseEvent::ButtonPressed { button })
                    } else {
                        continue;
                    }
                }
                PointerEventKind::Release { button, .. } => {
                    if let Some(button) = convert_button(button) {
                        WindowEvent::Mouse(MouseEvent::ButtonReleased { button })
                    } else {
                        continue;
                    }
                }
                PointerEventKind::Axis {
                    horizontal,
                    vertical,
                    source,
                    time: _,
                } => {
                    let delta = match source.unwrap() {
                        AxisSource::Wheel => ScrollDelta::Lines {
                            x: horizontal.discrete as f32,
                            y: vertical.discrete as f32,
                        },
                        AxisSource::Finger => ScrollDelta::Pixels {
                            x: horizontal.absolute as f32,
                            y: vertical.absolute as f32,
                        },
                        AxisSource::Continuous => ScrollDelta::Pixels {
                            x: horizontal.absolute as f32,
                            y: vertical.absolute as f32,
                        },
                        AxisSource::WheelTilt => ScrollDelta::Lines {
                            x: horizontal.discrete as f32,
                            y: vertical.discrete as f32,
                        },
                        _ => continue,
                    };
                    WindowEvent::Mouse(MouseEvent::WheelScrolled { delta })
                }
            };

            let _ = &self.send_window_event(window_event);
        }
    }
}

impl ProvidesRegistryState for LayerApp {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers!(OutputState, SeatState);
}

delegate_compositor!(LayerApp);
delegate_output!(LayerApp);
delegate_seat!(LayerApp);
delegate_keyboard!(LayerApp);
delegate_pointer!(LayerApp);
delegate_layer!(LayerApp);
delegate_registry!(LayerApp);

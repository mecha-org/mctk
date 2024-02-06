use super::{
    app::{CanvasApplication, Event, Viewport},
    canvas::init_gl_canvas,
    gl::init_gl,
    input::pointer::{self, Cursor, MouseEvent, Point, ScrollDelta},
};
use crate::raw_handle::RawWaylandHandle;
use anyhow::Context;
use glutin::{
    api::egl::{self, context::PossiblyCurrentContext},
    surface::WindowSurface,
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
        calloop::EventLoop,
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

pub struct LayerShellApplication<T: CanvasApplication + 'static> {
    pub app: T,
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
    pub pointer_location: (f64, f64),

    pub initial_configure_sent: bool,

    // gl
    gl_context: PossiblyCurrentContext,
    gl_surface: egl::surface::Surface<WindowSurface>,

    // femto canvas
    gl_canvas: femtovg::Canvas<femtovg::renderer::OpenGl>,

    viewport: Viewport,
}

pub struct WindowOptions {
    pub height: u32,
    pub width: u32,
    pub scale_factor: f32,
}

pub struct LayerShellOptions {
    pub anchor: wlr_layer::Anchor,
    pub layer: wlr_layer::Layer,
    pub keyboard_interactivity: wlr_layer::KeyboardInteractivity,
    pub namespace: Option<String>,
}

impl<T: CanvasApplication + 'static> LayerShellApplication<T> {
    pub fn new(
        app: T,
        window_opts: WindowOptions,
        layer_opts: LayerShellOptions,
    ) -> anyhow::Result<(Self, EventLoop<'static, Self>)> {
        let conn = Connection::connect_to_env().expect("failed to connect to wayland");
        let event_loop = EventLoop::<Self>::try_new()?;
        let WindowOptions {
            height,
            width,
            scale_factor,
        } = window_opts;
        let LayerShellOptions {
            anchor,
            layer,
            keyboard_interactivity,
            namespace,
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

        // create gl_display, gl_surface
        let (gl_display, gl_surface, gl_context) = init_gl(wayland_handle, (width, height));

        let gl_canvas = init_gl_canvas(&gl_display, (width, height), scale_factor);

        let viewport = Viewport {
            height,
            width,
            scale_factor,
        };

        let state = LayerShellApplication {
            app,
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
            pointer_location: (0.0, 0.0),
            initial_configure_sent: false,
            gl_context,
            gl_surface,
            gl_canvas,
            viewport,
        };

        Ok((state, event_loop))
    }

    pub fn draw(&mut self, queue_handle: &QueueHandle<Self>, surface: &WlSurface) {
        if self.layer.wl_surface() != surface {
            return;
        }

        // update and render
        if !self.initial_configure_sent {
            self.render();
        } else {
            self.dispatch();
        }

        self.layer
            .wl_surface()
            .damage_buffer(0, 0, self.width as i32, self.height as i32);

        self.layer
            .wl_surface()
            .frame(queue_handle, self.layer.wl_surface().clone());

        self.layer.commit();
    }

    pub fn render(&mut self) {
        let _ = &self.app.render(
            &self.gl_context,
            &self.gl_surface,
            &mut self.gl_canvas,
            &self.viewport,
        );
    }

    pub fn dispatch(&mut self) {
        let _ = &self.app.dispatch(
            &self.gl_context,
            &self.gl_surface,
            &mut self.gl_canvas,
            &self.viewport,
            Cursor::Available {
                position: Point {
                    x: self.pointer_location.0 as f32,
                    y: self.pointer_location.1 as f32,
                },
            },
        );
    }
}

impl<T: CanvasApplication + 'static> CompositorHandler for LayerShellApplication<T> {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _: i32,
    ) {
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
    }
}

impl<T: CanvasApplication + 'static> OutputHandler for LayerShellApplication<T> {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {}

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {}

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {}
}

impl<T: CanvasApplication + 'static> LayerShellHandler for LayerShellApplication<T> {
    fn closed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _layer: &wlr_layer::LayerSurface,
    ) {
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        layer: &wlr_layer::LayerSurface,
        _: wlr_layer::LayerSurfaceConfigure,
        _serial: u32,
    ) {
        self.dispatch();

        if !self.initial_configure_sent {
            self.draw(qh, layer.wl_surface());
            self.initial_configure_sent = true;
        }
    }
}

impl<T: CanvasApplication + 'static> SeatHandler for LayerShellApplication<T> {
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

impl<T: CanvasApplication + 'static> KeyboardHandler for LayerShellApplication<T> {
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
        _: KeyEvent,
    ) {
        if !self.keyboard_focus {
            return;
        }
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _serial: u32,
        _: KeyEvent,
    ) {
        if !self.keyboard_focus {
            return;
        }
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

impl<T: CanvasApplication + 'static> PointerHandler for LayerShellApplication<T> {
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

            let iced_event = match event.kind {
                PointerEventKind::Enter { .. } => Event::Mouse(MouseEvent::CursorEntered),
                PointerEventKind::Leave { .. } => Event::Mouse(MouseEvent::CursorLeft),
                PointerEventKind::Motion { .. } => {
                    self.pointer_location = event.position;
                    Event::Mouse(MouseEvent::CursorMoved {
                        position: Point {
                            x: event.position.0 as f32,
                            y: event.position.1 as f32,
                        },
                    })
                }
                PointerEventKind::Press { button, .. } => {
                    if let Some(button) = pointer::convert_button(button) {
                        Event::Mouse(MouseEvent::ButtonPressed { button })
                    } else {
                        continue;
                    }
                }
                PointerEventKind::Release { button, .. } => {
                    if let Some(button) = pointer::convert_button(button) {
                        Event::Mouse(MouseEvent::ButtonReleased { button })
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
                    Event::Mouse(MouseEvent::WheelScrolled { delta })
                }
            };

            let _ = &self.app.push_event(iced_event);
        }
    }
}

impl<T: CanvasApplication + 'static> ProvidesRegistryState for LayerShellApplication<T> {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers!(OutputState, SeatState);
}

delegate_compositor!(@<T: CanvasApplication + 'static> LayerShellApplication<T>);
delegate_output!(@<T: CanvasApplication + 'static> LayerShellApplication<T>);
delegate_seat!(@<T: CanvasApplication + 'static> LayerShellApplication<T>);
delegate_keyboard!(@<T: CanvasApplication + 'static> LayerShellApplication<T>);
delegate_pointer!(@<T: CanvasApplication + 'static> LayerShellApplication<T>);
delegate_layer!(@<T: CanvasApplication + 'static> LayerShellApplication<T>);
delegate_registry!(@<T: CanvasApplication + 'static> LayerShellApplication<T>);

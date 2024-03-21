use crate::{keyboard::KeyboardEvent, new_raw_wayland_handle, pointer::{convert_button, MouseEvent, Point, ScrollDelta}, touch::{Position, TouchEvent, TouchPoint}, PhysicalPosition, WindowEvent, WindowMessage, WindowOptions
};
use ahash::AHashMap;
use anyhow::Context;
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState}, delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer, delegate_registry, delegate_seat, delegate_touch, output::{OutputHandler, OutputState}, reexports::{
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
            Connection, QueueHandle,
        },
    }, registry::{ProvidesRegistryState, RegistryState}, registry_handlers, seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers}, pointer::{PointerEvent, PointerEventKind, PointerHandler}, touch::TouchHandler, Capability, SeatHandler, SeatState
    }, shell::{
        wlr_layer::{self, LayerShell, LayerShellHandler, LayerSurface},
        WaylandSurface,
    }
};
use wayland_client::protocol::{wl_display::WlDisplay, wl_touch::{self, WlTouch}};

pub struct LayerShellSctkWindow {
    window_tx: Sender<WindowMessage>,
    wl_display: WlDisplay,
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    layer: LayerSurface,
    pub width: u32,
    pub height: u32,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    keyboard_focus: bool,
    keyboard_modifiers: Modifiers,
    pointer: Option<wl_pointer::WlPointer>,
    touch: Option<wl_touch::WlTouch>,
    touch_map: AHashMap<i32, TouchPoint>,
    initial_configure_sent: bool,
    pub scale_factor: f32,
    exit: bool,
}

pub struct LayerOptions {
    pub anchor: wlr_layer::Anchor,
    pub layer: wlr_layer::Layer,
    pub keyboard_interactivity: wlr_layer::KeyboardInteractivity,
    pub namespace: Option<String>,
    pub zone: i32,
}

impl LayerShellSctkWindow {
    pub fn new(
        window_tx: Sender<WindowMessage>,
        window_opts: WindowOptions,
        layer_opts: LayerOptions,
    ) -> anyhow::Result<(Self, EventLoop<'static, Self>)> {
        let conn = Connection::connect_to_env().expect("failed to connect to wayland");
        let wl_display = conn.display();
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

        let state = LayerShellSctkWindow {
            // app,
            window_tx,
            wl_display,
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
            touch: None,
            touch_map: AHashMap::new(),
            initial_configure_sent: false,
            scale_factor,
            exit: false,
            // gl_context,
            // gl_surface,
            // gl_canvas,
        };

        Ok((state, event_loop))
    }

    // pub fn draw(&mut self, queue_handle: &QueueHandle<Self>, surface: &WlSurface) {
    //     if self.layer.wl_surface() != surface {
    //         return;
    //     }

    //     // for continous rendering
    //     self.send_redraw_requested();

    //     self.layer
    //         .wl_surface()
    //         .damage_buffer(0, 0, self.width as i32, self.height as i32);

    //     self.layer
    //         .wl_surface()
    //         .frame(queue_handle, self.layer.wl_surface().clone());

    //     self.layer.commit();
    // }

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

    pub fn send_configure_event(&mut self, width: u32, height: u32) {
        let wayland_handle = new_raw_wayland_handle(&self.wl_display, &self.layer.wl_surface());
        let _ = &self.window_tx.send(WindowMessage::Configure {
            width,
            height,
            wayland_handle,
        });
    }
}

impl CompositorHandler for LayerShellSctkWindow {
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
        if self.layer.wl_surface() != surface {
            return;
        }
        let _ = self.send_redraw_requested();

        self.layer
            .wl_surface()
            .damage_buffer(0, 0, self.width as i32, self.height as i32);
        self.layer
            .wl_surface()
            .frame(qh, self.layer.wl_surface().clone());
        self.layer.commit();
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

impl OutputHandler for LayerShellSctkWindow {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {}

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {}

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {}
}

impl LayerShellHandler for LayerShellSctkWindow {
    fn closed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _layer: &wlr_layer::LayerSurface,
    ) {
        let _ = self.send_close_requested();
        self.exit = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _: &wlr_layer::LayerSurface,
        _: wlr_layer::LayerSurfaceConfigure,
        _serial: u32,
    ) {
        // TODO: handle resize?
        if !self.initial_configure_sent {
            self.send_configure_event(self.width, self.height);
            self.initial_configure_sent = true;

            // request next frame
            self.layer
                .wl_surface()
                .frame(qh, self.layer.wl_surface().clone());
        }
    }
}

impl SeatHandler for LayerShellSctkWindow {
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
        if capability == Capability::Touch && self.touch.is_none() {
            let touch = self.seat_state.get_touch(qh, &seat).unwrap();
            self.touch = Some(touch);
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

impl KeyboardHandler for LayerShellSctkWindow {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        surface: &WlSurface,
        _serial: u32,
        _raw: &[u32],
        _: &[Keysym],
    ) {
        if self.layer.wl_surface() != surface {
            return;
        }

        self.keyboard_focus = true;
        self.send_window_event(WindowEvent::Focused);
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
        self.send_window_event(WindowEvent::Unfocused);
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
        let key = event.keysym;
        self.send_window_event(WindowEvent::Keyboard(KeyboardEvent::KeyPressed { key }))
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

        let key = event.keysym;
        self.send_window_event(WindowEvent::Keyboard(KeyboardEvent::KeyReleased { key }))
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

impl PointerHandler for LayerShellSctkWindow {
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

            let _ = self.send_window_event(window_event);
        }
    }
}

impl TouchHandler for LayerShellSctkWindow {
    fn down(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlTouch,
        _: u32,
        time: u32,
        surface: WlSurface,
        id: i32,
        position: (f64, f64),
    ) {
        if self.layer.wl_surface() != &surface {
            return;
        }
        let scale_factor = self.scale_factor;

        // insert the touch point
        self.touch_map.insert(id, TouchPoint { surface, position: Position { x: position.0 as f32, y: position.1 as f32 } });

        self.send_window_event(WindowEvent::Touch(TouchEvent::Down {
            id,
            time,
            position: Position { x: position.0 as f32, y: position.1 as f32 },
            scale_factor,
        }));
    }

    fn up(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlTouch,
        _: u32,
        time: u32,
        id: i32,
    ) {
        let scale_factor = self.scale_factor;
        let touch_point = match self.touch_map.remove(&id) {
            Some(touch_point) => touch_point,
            None => return,
        };

        self.send_window_event(WindowEvent::Touch(TouchEvent::Up {
            id,
            time,
            position: Position { x: touch_point.position.x, y: touch_point.position.y },
            scale_factor,
        }));
    }

    fn motion(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlTouch,
        time: u32,
        id: i32,
        position: (f64, f64),
    ) {
        let scale_factor = self.scale_factor;
        let touch_point = match self.touch_map.get_mut(&id) {
            Some(touch_point) => touch_point,
            None => return,
        };

        touch_point.position = Position { x: position.0 as f32, y: position.1 as f32 };
        self.send_window_event(WindowEvent::Touch(TouchEvent::Motion {
            id,
            time,
            position: Position { x: position.0 as f32, y: position.0 as f32 },
            scale_factor,
        }));
    }

    fn shape(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlTouch,
        _: i32,
        _: f64,
        _: f64,
    ) {
        // blank
    }

    fn orientation(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlTouch,
        _: i32,
        _: f64,
    ) {
        // blank
    }

    fn cancel(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlTouch
    ) {
        let scale_factor = self.scale_factor;
        for (id, tp) in self.touch_map.clone().into_iter() {
            let touch_point = tp.clone();
            self.send_window_event(WindowEvent::Touch(TouchEvent::Cancel { 
                id,
                position: Position { x: touch_point.position.x, y: touch_point.position.y },
                scale_factor,
            }));
        }

        self.touch_map.drain();
    }
}

impl ProvidesRegistryState for LayerShellSctkWindow {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers!(OutputState, SeatState);
}

delegate_compositor!(LayerShellSctkWindow);
delegate_output!(LayerShellSctkWindow);
delegate_seat!(LayerShellSctkWindow);
delegate_keyboard!(LayerShellSctkWindow);
delegate_pointer!(LayerShellSctkWindow);
delegate_touch!(LayerShellSctkWindow);
delegate_layer!(LayerShellSctkWindow);
delegate_registry!(LayerShellSctkWindow);

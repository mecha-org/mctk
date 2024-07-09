use crate::{
    input::keyboard::KeyboardEvent,
    input::pointer::{convert_button, MouseEvent, Point, ScrollDelta},
    input::touch::{Position, TouchEvent, TouchPoint},
    new_raw_wayland_handle,
    session_lock::lock_window::SessionLockMessage,
    WindowEvent, WindowMessage, WindowOptions,
};
use ahash::AHashMap;
use anyhow::Context;
use mctk_core::raw_handle::RawWaylandHandle;
use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_output, delegate_pointer, delegate_registry,
    delegate_seat, delegate_touch,
    output::{OutputHandler, OutputState},
    reexports::{
        calloop::{
            self,
            channel::{Channel, Sender},
            EventLoop,
        },
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
        protocols::ext::session_lock::v1::client::{
            ext_session_lock_manager_v1::ExtSessionLockManagerV1,
            ext_session_lock_surface_v1::{self, ExtSessionLockSurfaceV1},
            ext_session_lock_v1::{self, ExtSessionLockV1},
        },
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers},
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        touch::TouchHandler,
        Capability, SeatHandler, SeatState,
    },
};
use wayland_client::{
    protocol::{
        wl_display::WlDisplay,
        wl_touch::{self, WlTouch},
    },
    Dispatch,
};

pub struct SessionLockSctkWindow {
    conn: Connection,
    window_tx: Sender<WindowMessage>,
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    pub width: u32,
    pub height: u32,
    pub is_exited: bool,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    keyboard_focus: bool,
    keyboard_modifiers: Modifiers,
    pointer: Option<wl_pointer::WlPointer>,
    initial_configure_sent: bool,
    pub wayland_handle: RawWaylandHandle,
    pub scale_factor: f32,
    wl_display: WlDisplay,
    wl_surface: WlSurface,
    touch: Option<wl_touch::WlTouch>,
    touch_map: AHashMap<i32, TouchPoint>,
    // session lock surface
    // session_lock_manager: ExtSessionLockManagerV1,
    pub session_lock: ExtSessionLockV1,
    // session_lock_surface: ExtSessionLockSurfaceV1,
}

impl SessionLockSctkWindow {
    pub fn new(
        window_tx: Sender<WindowMessage>,
        window_opts: WindowOptions,
        session_lock_rx: Channel<SessionLockMessage>,
    ) -> anyhow::Result<(Self, EventLoop<'static, Self>)> {
        let conn = Connection::connect_to_env().expect("failed to connect to wayland");
        let wl_display = conn.display();
        let event_loop = EventLoop::<Self>::try_new()?;
        let WindowOptions {
            height,
            width,
            scale_factor,
        } = window_opts;

        let (globals, event_queue) =
            registry_queue_init::<Self>(&conn).context("failed to init registry queue")?;

        let queue_handle = event_queue.handle();

        let loop_handle = event_loop.handle();
        let _ = WaylandSource::new(conn.clone(), event_queue)
            .insert(loop_handle.clone())
            .expect("failed to insert wayland source into event loop");

        let output_state = OutputState::new(&globals, &queue_handle);
        let compositor = CompositorState::bind(&globals, &queue_handle)
            .context("wl_compositor not availible")?;
        let session_lock_manager = globals
            .bind::<ExtSessionLockManagerV1, _, _>(
                &queue_handle,
                core::ops::RangeInclusive::new(1, 1),
                (),
            )
            .map_err(|_| "compositor does not implement ext session lock manager (v1).")
            .unwrap();

        // create the surfacce
        let wl_surface = compositor.create_surface(&queue_handle);

        let session_lock = session_lock_manager.lock(&queue_handle, ());
        let output = output_state.outputs().next().unwrap();
        // set surface role as session lock surface
        let _ = session_lock.get_lock_surface(&wl_surface, &output, &queue_handle, ());

        // create wayland handle
        let wayland_handle = {
            let mut handle = WaylandDisplayHandle::empty();
            handle.display = conn.backend().display_ptr() as *mut _;
            let display_handle = RawDisplayHandle::Wayland(handle);

            let mut handle = WaylandWindowHandle::empty();
            handle.surface = wl_surface.id().as_ptr() as *mut _;
            let window_handle = RawWindowHandle::Wayland(handle);

            RawWaylandHandle(display_handle, window_handle)
        };

        // insert source for session_lock_rx messages
        let _ = loop_handle.insert_source(session_lock_rx, move |event, _, state| {
            let _ = match event {
                // calloop::channel::Event::Msg(msg) => app.app.push_message(msg),
                calloop::channel::Event::Msg(msg) => {
                    match msg {
                        SessionLockMessage::Unlock => {
                            state.unlock_and_destroy();
                        }
                    };
                }
                calloop::channel::Event::Closed => {}
            };
        });

        let state = SessionLockSctkWindow {
            // app,
            conn,
            window_tx,
            registry_state: RegistryState::new(&globals),
            seat_state: SeatState::new(&globals, &queue_handle),
            output_state,
            wl_display,
            wl_surface,
            width,
            height,
            is_exited: false,
            keyboard: None,
            keyboard_focus: false,
            keyboard_modifiers: Modifiers::default(),
            pointer: None,
            touch: None,
            touch_map: AHashMap::new(),
            initial_configure_sent: false,
            wayland_handle,
            scale_factor,
            // session_lock_manager: session_lock_manager,
            session_lock: session_lock,
            // session_lock_surface: session_lock_surface,
        };

        Ok((state, event_loop))
    }

    pub fn send_main_events_cleared(&mut self) {
        let _ = &self.window_tx.send(WindowMessage::MainEventsCleared);
    }

    pub fn send_close_requested(&self) {
        let _ = &self.window_tx.send(WindowMessage::WindowEvent {
            event: WindowEvent::CloseRequested,
        });
    }

    pub fn close(&mut self) {
        self.is_exited = true;
    }

    pub fn send_redraw_requested(&mut self) {
        let _ = &self.window_tx.send(WindowMessage::RedrawRequested);
    }

    pub fn send_compositor_frame(&mut self) {
        let _ = &self.window_tx.send(WindowMessage::CompositorFrame);
    }

    pub fn send_window_event(&mut self, event: WindowEvent) {
        let _ = &self.window_tx.send(WindowMessage::WindowEvent { event });
    }

    pub fn send_configure_event(&mut self, width: u32, height: u32) {
        let wayland_handle = new_raw_wayland_handle(&self.wl_display, &self.wl_surface);
        let _ = &self.window_tx.send(WindowMessage::Configure {
            width,
            height,
            wayland_handle,
        });
    }

    pub fn unlock_and_destroy(&mut self) {
        self.session_lock.unlock_and_destroy();

        // send roundtrip to wait for reply from server
        // TODO handle faild
        let _ = self.conn.roundtrip();

        // close the client
        self.close();
    }
}

impl CompositorHandler for SessionLockSctkWindow {
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
        if &self.wl_surface != surface {
            return;
        }
        let _ = self.send_compositor_frame();

        self.wl_surface
            .damage_buffer(0, 0, self.width as i32, self.height as i32);
        self.wl_surface.frame(qh, self.wl_surface.clone());
        self.wl_surface.commit();
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

impl OutputHandler for SessionLockSctkWindow {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {}

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {}

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {}
}

impl SeatHandler for SessionLockSctkWindow {
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

impl KeyboardHandler for SessionLockSctkWindow {
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
        if &self.wl_surface != surface {
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
        if &self.wl_surface != surface {
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

impl PointerHandler for SessionLockSctkWindow {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            if &event.surface != &self.wl_surface {
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

impl TouchHandler for SessionLockSctkWindow {
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
        if &self.wl_surface != &surface {
            return;
        }
        let scale_factor = self.scale_factor;

        // insert the touch point
        self.touch_map.insert(
            id,
            TouchPoint {
                surface,
                position: Position {
                    x: position.0 as f32,
                    y: position.1 as f32,
                },
            },
        );

        self.send_window_event(WindowEvent::Touch(TouchEvent::Down {
            id,
            time,
            position: Position {
                x: position.0 as f32,
                y: position.1 as f32,
            },
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
            position: Position {
                x: touch_point.position.x,
                y: touch_point.position.y,
            },
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

        touch_point.position = Position {
            x: position.0 as f32,
            y: position.1 as f32,
        };
        self.send_window_event(WindowEvent::Touch(TouchEvent::Motion {
            id,
            time,
            position: Position {
                x: position.0 as f32,
                y: position.1 as f32,
            },
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

    fn orientation(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlTouch, _: i32, _: f64) {
        // blank
    }

    fn cancel(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlTouch) {
        let scale_factor = self.scale_factor;
        for (id, tp) in self.touch_map.clone().into_iter() {
            let touch_point = tp.clone();
            self.send_window_event(WindowEvent::Touch(TouchEvent::Cancel {
                id,
                position: Position {
                    x: touch_point.position.x,
                    y: touch_point.position.y,
                },
                scale_factor,
            }));
        }

        self.touch_map.drain();
    }
}

impl ProvidesRegistryState for SessionLockSctkWindow {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers!(OutputState, SeatState);
}

delegate_compositor!(SessionLockSctkWindow);
delegate_output!(SessionLockSctkWindow);
delegate_seat!(SessionLockSctkWindow);
delegate_keyboard!(SessionLockSctkWindow);
delegate_pointer!(SessionLockSctkWindow);
delegate_touch!(SessionLockSctkWindow);
delegate_registry!(SessionLockSctkWindow);

/* Session Lock binds */
impl Dispatch<ExtSessionLockManagerV1, ()> for SessionLockSctkWindow {
    fn event(
        _: &mut Self,
        _: &ExtSessionLockManagerV1,
        event: <ExtSessionLockManagerV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtSessionLockV1, ()> for SessionLockSctkWindow {
    fn event(
        _: &mut Self,
        _: &ExtSessionLockV1,
        event: <ExtSessionLockV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_session_lock_v1::Event::Locked => {}
            ext_session_lock_v1::Event::Finished => {}
            _ => {}
        }
    }
}

impl Dispatch<ExtSessionLockSurfaceV1, ()> for SessionLockSctkWindow {
    fn event(
        state: &mut Self,
        surface: &ExtSessionLockSurfaceV1,
        event: <ExtSessionLockSurfaceV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            ext_session_lock_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                if !state.initial_configure_sent {
                    state.send_configure_event(width, height);
                    state.initial_configure_sent = true;
                    surface.ack_configure(serial);

                    // request next frame
                    state.wl_surface.frame(qh, state.wl_surface.clone());
                }
            }
            _ => todo!(),
        }
    }
}

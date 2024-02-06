use std::{
    cell::{OnceCell, RefCell},
    rc::Rc, time::Duration,
};
use anyhow::Context;
use iced::{
    mouse::ScrollDelta,
    Color, Size, Theme,
};
use iced_runtime::Debug;
use iced_wgpu::{
    graphics::{Renderer, Viewport},
    wgpu::{self, Backends},
};
use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_output, delegate_pointer,
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
        Capability, SeatHandler, SeatState, touch::TouchHandler,
    },
    shell::{
        WaylandSurface,
        xdg::{XdgShell, window::{WindowDecorations, Window, WindowHandler}, XdgSurface},
    }, delegate_xdg_window, delegate_xdg_shell, delegate_touch,
};
use tracing::{debug, trace};

use crate::{
    input::{
        clipboard::WaylandClipboard,
        keyboard,
        pointer,
    },
    counter::Counter, sctk::raw_handle::RawWaylandHandle,
};

pub struct XdgWindowApp {
    container: Option<(XdgWindowContainer, EventLoop<'static, XdgWindowContainer>)>,
    conn: Connection,
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: Rc<wgpu::Device>,
    queue: Rc<wgpu::Queue>,
    renderer: OnceCell<Rc<RefCell<iced_wgpu::Renderer<Theme>>>>,
    // loop_handle: LoopHandle<'static, Self>,
}

impl XdgWindowApp {
    pub fn new() -> anyhow::Result<(Self, EventLoop<'static, Self>)> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::GL | wgpu::Backends::VULKAN,
            ..Default::default()
        });

        let adapter = futures::executor::block_on(async {
            wgpu::util::initialize_adapter_from_env_or_default(
                &instance,
                Backends::GL | Backends::VULKAN,
                None,
            )
            .await
            .unwrap()
        });

        let (device, queue) = futures::executor::block_on(async {
            let adapter_features = adapter.features();
            let needed_limits = wgpu::Limits::default();
            adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: None,
                        features: adapter_features & wgpu::Features::default(),
                        limits: needed_limits,
                    },
                    None,
                )
                .await
                .expect("Request device")
        });

        let event_loop = EventLoop::<XdgWindowApp>::try_new()?;

        Ok((
            XdgWindowApp {
                container: None,
                conn: Connection::connect_to_env()?,
                instance,
                adapter,
                device: Rc::new(device),
                queue: Rc::new(queue),
                renderer: OnceCell::new(),
                // loop_handle: event_loop.handle(),
            },
            event_loop,
        ))
    }

    pub fn run(
        &mut self,
        (width, height): (u32, u32)
    ) {
        if let Ok(container) = XdgWindowContainer::new(
            &self.conn,
            &self.instance,
            &self.adapter,
            self.device.clone(),
            self.queue.clone(),
            &self.renderer,
            (width, height)
        ) {
            self.container = Some(container);
        }
    }

    pub fn configure_wgpu_surfaces(&self) {
        let (container, _) = self.container.as_ref().unwrap();
        container.configure_wgpu_surface(&self.device);
    }

    pub fn dispatch_loops(&mut self) -> anyhow::Result<()> {
        let (container, event_loop) = self.container.as_mut().unwrap();
        event_loop.dispatch(Duration::ZERO, container)?;
        Ok(())
    }
}

pub struct XdgWindowContainer {
    pub registry_state: RegistryState,
    pub seat_state: SeatState,
    pub output_state: OutputState,

    // surface must be dropped before layer
    pub surface: wgpu::Surface,
    pub dirty: bool,

    pub iced_program: iced_runtime::program::State<Counter>,
    pub window: Window,
    pub width: u32,
    pub height: u32,
    pub viewport: Viewport,
    pub capabilities: wgpu::SurfaceCapabilities,

    pub clipboard: WaylandClipboard,

    pub keyboard: Option<wl_keyboard::WlKeyboard>,
    pub keyboard_focus: bool,
    pub keyboard_modifiers: Modifiers,

    pub pointer: Option<wl_pointer::WlPointer>,
    pub pointer_location: (f64, f64),

    pub initial_configure_sent: bool,

    pub device: Rc<wgpu::Device>,
    pub queue: Rc<wgpu::Queue>,
    pub renderer: Rc<RefCell<iced_wgpu::Renderer<Theme>>>,
}

impl XdgWindowContainer {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        conn: &Connection,
        instance: &wgpu::Instance,
        adapter: &wgpu::Adapter,
        device: Rc<wgpu::Device>,
        queue: Rc<wgpu::Queue>,
        renderer: &OnceCell<Rc<RefCell<iced_wgpu::Renderer<Theme>>>>,
        (width, height): (u32, u32),
    ) -> anyhow::Result<(Self, EventLoop<'static, Self>)> {
        debug!("top of State::new");
        debug!("init registry");
        let (globals, event_queue) =
            registry_queue_init::<Self>(conn).context("failed to init registry queue")?;

        let queue_handle = event_queue.handle();

        debug!("create loop");
        let event_loop = EventLoop::<Self>::try_new()?;
        let loop_handle = event_loop.handle();
        debug!("create wayland source");
        WaylandSource::new(conn.clone(), event_queue)
            .insert(loop_handle.clone())
            .expect("failed to insert wayland source into event loop");

        debug!("bind globals");
        let compositor = CompositorState::bind(&globals, &queue_handle)
            .context("wl_compositor not availible")?;
        let xdg_shell =
            XdgShell::bind(&globals, &queue_handle).context("layer shell not availible")?;

        debug!("create layer surface");
        let surface = compositor.create_surface(&queue_handle);

        let window = xdg_shell.create_window(
            surface,
            WindowDecorations::RequestClient,
            &queue_handle,
        );

        window.set_window_geometry(0, 0, width, height);
        window.commit();

        debug!("create wayland handle");
        let wayland_handle = {
            let mut handle = WaylandDisplayHandle::empty();
            handle.display = conn.backend().display_ptr() as *mut _;
            let display_handle = RawDisplayHandle::Wayland(handle);

            let mut handle = WaylandWindowHandle::empty();
            handle.surface = window.wl_surface().id().as_ptr() as *mut _;
            let window_handle = RawWindowHandle::Wayland(handle);

            RawWaylandHandle(display_handle, window_handle)
        };

        debug!("create wgpu surface");
        let wgpu_surface = unsafe { instance.create_surface(&wayland_handle).unwrap() };

        debug!("get capabilities"); // PERF: SLOW
        let capabilities = wgpu_surface.get_capabilities(adapter);
        let renderer = renderer.get_or_init(|| {
            debug!("get texture format");
            let format = capabilities
                .formats
                .iter()
                .copied()
                .find(wgpu::TextureFormat::is_srgb)
                .or_else(|| capabilities.formats.first().copied())
                .expect("Get preferred format");

            tracing::debug!("---------FORMAT IS {format:?}");

            // TODO: speed up
            debug!("create iced backend"); // PERF: SLOW
            let backend = iced_wgpu::Backend::new(
                &device,
                &queue,
                iced_wgpu::Settings {
                    present_mode: wgpu::PresentMode::Mailbox,
                    internal_backend: Backends::GL | Backends::VULKAN,
                    ..Default::default()
                },
                format,
            );

            debug!("create iced renderer");
            let renderer: Renderer<iced_wgpu::Backend, Theme> = Renderer::new(backend);
            Rc::new(RefCell::new(renderer))
        });


        let state = {
            let mut ren = renderer.borrow_mut();

            iced_runtime::program::State::new(
                Counter {
                    value: 0,
                },
                Size::new(width as f32, height as f32),
                &mut ren,
                &mut Debug::new(), // TODO:
            )
        };

        debug!("create state");
        let state = XdgWindowContainer {
            registry_state: RegistryState::new(&globals),
            seat_state: SeatState::new(&globals, &queue_handle),
            output_state: OutputState::new(&globals, &queue_handle),
            iced_program: state,
            window,
            width,
            height,
            viewport: Viewport::with_physical_size(Size::new(width, height), 1.0),
            capabilities,

            dirty: true,

            surface: wgpu_surface,

            clipboard: unsafe { WaylandClipboard::new(conn.backend().display_ptr() as *mut _) },

            keyboard: None,
            keyboard_focus: false,
            keyboard_modifiers: Modifiers::default(),

            pointer: None,
            pointer_location: (0.0, 0.0),

            initial_configure_sent: false,

            device: device.clone(),
            queue,
            renderer: renderer.clone(),
        };

        state.configure_wgpu_surface(&device);

        Ok((state, event_loop))
    }

    pub fn configure_wgpu_surface(&self, device: &wgpu::Device) {
        let capabilities = &self.capabilities;
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: capabilities.formats[0],
            width: self.width,
            height: self.height,
            present_mode: wgpu::PresentMode::Mailbox,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![capabilities.formats[0]],
        };

        self.surface.configure(device, &surface_config);
    }
}

impl CompositorHandler for XdgWindowContainer {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _new_factor: i32,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        surface: &WlSurface,
        _time: u32,
    ) {
        tracing::trace!("CompositorHandler::frame");
        self.update_iced_program();
        self.draw(qh, surface);
    }

    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlSurface,
        _: wl_output::Transform,
    ) {
        todo!()
    }
}

impl OutputHandler for XdgWindowContainer {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {}

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {}

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {}
}

impl WindowHandler for XdgWindowContainer {
    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &smithay_client_toolkit::shell::xdg::window::Window) {
        println!("request_close");
    }

    fn configure(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        window: &smithay_client_toolkit::shell::xdg::window::Window,
        _: smithay_client_toolkit::shell::xdg::window::WindowConfigure,
        _: u32,
    ) {
        debug!("update_iced_program");
        self.update_iced_program();

        if !self.initial_configure_sent {
            self.initial_configure_sent = true;
            self.draw(qh, window.wl_surface());
        }
    }
}


impl SeatHandler for XdgWindowContainer {
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

impl KeyboardHandler for XdgWindowContainer {
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
        if self.window.wl_surface() != surface {
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
        if self.window.wl_surface() != surface {
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
        debug!("start of press_key");
        if !self.keyboard_focus {
            return;
        }

        let Some(keycode) = keyboard::keysym_to_keycode(event.keysym) else {
            return;
        };

        let mut modifiers = iced_runtime::keyboard::Modifiers::default();

        let Modifiers {
            ctrl,
            alt,
            shift,
            caps_lock: _,
            logo,
            num_lock: _,
        } = &self.keyboard_modifiers;

        if *ctrl {
            modifiers |= iced_runtime::keyboard::Modifiers::CTRL;
        }
        if *alt {
            modifiers |= iced_runtime::keyboard::Modifiers::ALT;
        }
        if *shift {
            modifiers |= iced_runtime::keyboard::Modifiers::SHIFT;
        }
        if *logo {
            modifiers |= iced_runtime::keyboard::Modifiers::LOGO;
        }

        let event = iced::Event::Keyboard(iced_runtime::keyboard::Event::KeyPressed {
            key_code: keycode,
            modifiers,
        });

        self.iced_program.queue_event(event);
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

        let Some(keycode) = keyboard::keysym_to_keycode(event.keysym) else {
            return;
        };

        let mut modifiers = iced_runtime::keyboard::Modifiers::default();

        let Modifiers {
            ctrl,
            alt,
            shift,
            caps_lock: _,
            logo,
            num_lock: _,
        } = &self.keyboard_modifiers;

        if *ctrl {
            modifiers |= iced_runtime::keyboard::Modifiers::CTRL;
        }
        if *alt {
            modifiers |= iced_runtime::keyboard::Modifiers::ALT;
        }
        if *shift {
            modifiers |= iced_runtime::keyboard::Modifiers::SHIFT;
        }
        if *logo {
            modifiers |= iced_runtime::keyboard::Modifiers::LOGO;
        }

        let event = iced::Event::Keyboard(iced_runtime::keyboard::Event::KeyReleased {
            key_code: keycode,
            modifiers,
        });

        self.iced_program.queue_event(event);
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

impl PointerHandler for XdgWindowContainer {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &WlPointer,
        events: &[PointerEvent],
    ) {
        trace!("pointer_frame");
        for event in events {
            if &event.surface != self.window.wl_surface() {
                continue;
            }

            let iced_event = match event.kind {
                PointerEventKind::Enter { .. } => {
                    iced::Event::Mouse(iced::mouse::Event::CursorEntered)
                }
                PointerEventKind::Leave { .. } => {
                    iced::Event::Mouse(iced::mouse::Event::CursorLeft)
                }
                PointerEventKind::Motion { .. } => {
                    self.pointer_location = event.position;
                    iced::Event::Mouse(iced::mouse::Event::CursorMoved {
                        position: iced::Point {
                            x: event.position.0 as f32,
                            y: event.position.1 as f32,
                        },
                    })
                }
                PointerEventKind::Press { button, .. } => {
                    if let Some(button) = pointer::button_to_iced_button(button) {
                        iced::Event::Mouse(iced::mouse::Event::ButtonPressed(button))
                    } else {
                        continue;
                    }
                }
                PointerEventKind::Release { button, .. } => {
                    if let Some(button) = pointer::button_to_iced_button(button) {
                        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(button))
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
                    iced::Event::Mouse(iced::mouse::Event::WheelScrolled { delta })
                }
            };

            self.iced_program.queue_event(iced_event);
        }
    }
}

impl TouchHandler for XdgWindowContainer {
    fn down(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        touch: &smithay_client_toolkit::reexports::client::protocol::wl_touch::WlTouch,
        serial: u32,
        time: u32,
        surface: WlSurface,
        id: i32,
        position: (f64, f64),
    ) {
        todo!()
    }

    fn up(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        touch: &smithay_client_toolkit::reexports::client::protocol::wl_touch::WlTouch,
        serial: u32,
        time: u32,
        id: i32,
    ) {
        todo!()
    }

    fn motion(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        touch: &smithay_client_toolkit::reexports::client::protocol::wl_touch::WlTouch,
        time: u32,
        id: i32,
        position: (f64, f64),
    ) {
        todo!()
    }

    fn shape(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        touch: &smithay_client_toolkit::reexports::client::protocol::wl_touch::WlTouch,
        id: i32,
        major: f64,
        minor: f64,
    ) {
    }

    fn orientation(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        touch: &smithay_client_toolkit::reexports::client::protocol::wl_touch::WlTouch,
        id: i32,
        orientation: f64,
    ) {
        todo!()
    }

    fn cancel(&mut self, conn: &Connection, qh: &QueueHandle<Self>, touch: &smithay_client_toolkit::reexports::client::protocol::wl_touch::WlTouch) {
        todo!()
    }
}

delegate_compositor!(XdgWindowContainer);
delegate_output!(XdgWindowContainer);
delegate_seat!(XdgWindowContainer);
delegate_keyboard!(XdgWindowContainer);
delegate_touch!(XdgWindowContainer);
delegate_pointer!(XdgWindowContainer);
delegate_xdg_shell!(XdgWindowContainer);
delegate_xdg_window!(XdgWindowContainer);
delegate_registry!(XdgWindowContainer);

impl ProvidesRegistryState for XdgWindowContainer {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers!(OutputState, SeatState);
}

impl XdgWindowContainer {
    pub fn draw(&mut self, queue_handle: &QueueHandle<Self>, surface: &WlSurface) {
        tracing::trace!("State::draw");
        if self.window.wl_surface() != surface {
            return;
        }
        // if !self.dirty {
        //     return;
        // }
        match self.surface.get_current_texture() {
            Ok(frame) => {
                let mut encoder = self
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                {
                    let mut renderer = self.renderer.borrow_mut();
                    renderer.with_primitives(|backend, primitives| {
                        backend.present::<String>(
                            &self.device,
                            &self.queue,
                            &mut encoder,
                            Some(iced::Color::new(0.6, 0.6, 0.6, 1.0)),
                            &view,
                            primitives,
                            &self.viewport,
                            &[],
                        );
                    });
                }

                self.queue.submit(Some(encoder.finish()));
                frame.present();

                self.window
                    .wl_surface()
                    .damage_buffer(0, 0, self.width as i32, self.height as i32);

                self.window
                    .wl_surface()
                    .frame(queue_handle, self.window.wl_surface().clone());

                self.window.commit();
            }
            Err(_) => todo!(),
        }
        self.dirty = false;
    }

    pub fn update_iced_program(&mut self) {
        tracing::trace!("State::update_iced_program");
        let mut renderer = self.renderer.borrow_mut();
        let _ = self.iced_program.update(
            self.viewport.logical_size(),
            iced::mouse::Cursor::Available(iced::Point {
                x: self.pointer_location.0 as f32,
                y: self.pointer_location.1 as f32,
            }),
            &mut renderer,
            &Theme::Dark,
            &iced_wgpu::core::renderer::Style {
                text_color: Color::WHITE,
            },
            &mut self.clipboard,
            &mut Debug::new(),
        );
    }
}

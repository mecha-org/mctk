use mctk_core::animations::{easing_functions::*, Animation};
use mctk_core::context::Model;
use mctk_core::layout::Alignment;
use mctk_core::prelude::*;
use mctk_core::reexports::smithay_client_toolkit::{
    reexports::calloop::{self, channel::Event},
    shell::wlr_layer,
};
use mctk_core::widgets::Text;
use mctk_smithay::layer_shell::layer_surface::LayerOptions;
use mctk_smithay::layer_shell::layer_window::{LayerWindow, LayerWindowParams};
use mctk_smithay::xdg_shell::xdg_window::{self, XdgWindowMessage, XdgWindowParams};
use mctk_smithay::{WindowInfo, WindowMessage, WindowOptions};
use smithay_client_toolkit::reexports::calloop::channel::Sender;
use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use tracing_subscriber::fmt::format;

#[derive(Debug)]
pub enum AppMessage {
    Exit,
}

#[derive(Clone)]
pub struct AppParams {
    app_channel: Option<calloop::channel::Sender<AppMessage>>,
}

#[derive(Default)]
pub struct AppState {
    window_sender: Option<Sender<XdgWindowMessage>>,
    app_channel: Option<Sender<AppMessage>>,
    linear_animation: Animation<Linear>,
    ease_in_out_animation: Animation<EaseInOutQuadratic>,
    ease_in_animation: Animation<EaseInQuadratic>,
    ease_out_animation: Animation<EaseOutQuadratic>,
}

impl Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState").finish()
    }
}

#[component(State = "AppState")]
#[derive(Debug, Default)]
pub struct App {}

#[state_component_impl(AppState)]
impl Component for App {
    fn init(&mut self) {
        self.state = Some(AppState {
            window_sender: None,
            app_channel: None,
            linear_animation: Animation::new(
                std::time::Duration::from_secs(1),
                animations::AnimationRepeat::PingPong,
            ),
            ease_in_out_animation: Animation::new(
                std::time::Duration::from_secs(1),
                animations::AnimationRepeat::PingPong,
            ),
            ease_in_animation: Animation::new(
                std::time::Duration::from_secs(1),
                animations::AnimationRepeat::PingPong,
            ),
            ease_out_animation: Animation::new(
                std::time::Duration::from_secs(1),
                animations::AnimationRepeat::PingPong,
            ),
        });
    }

    fn render_hash(&self, hasher: &mut ComponentHasher) {
        self.state_ref().linear_animation.hash(hasher);
        self.state_ref().ease_in_animation.hash(hasher);
        self.state_ref().ease_out_animation.hash(hasher);
        self.state_ref().ease_in_out_animation.hash(hasher);
    }

    fn props_hash(&self, hasher: &mut ComponentHasher) {
        self.state_ref().linear_animation.hash(hasher);
        self.state_ref().ease_in_animation.hash(hasher);
        self.state_ref().ease_out_animation.hash(hasher);
        self.state_ref().ease_in_out_animation.hash(hasher);
    }

    fn on_tick(&mut self, _event: &mut mctk_core::event::Event<mctk_core::event::Tick>) {
        self.dirty = true;
    }

    fn view(&self) -> Option<Node> {
        let linear_animation = self.state_ref().linear_animation.get_value();
        let linear_label = node!(
            Text::new(txt!(format!("Liner Animation",)))
                .style("color", Color::WHITE)
                .style("size", 25.0),
            lay![size: size!(480.0, 50.0), margin: [10.0, 25.0, 0.0, 0.0],]
        );
        let linear_rectangle = node!(
            RoundedRect::new(Color::WHITE, 100.0),
            lay![
                size: size!(50.0, 50.0),
                margin: [25.0, 25.0 + 375.0 * linear_animation , 0.0, 0.0],
            ]
        )
        .key((linear_animation * 1000000.0) as u64);

        let ease_in_out_label = node!(
            Text::new(txt!(format!("Ease In Out",)))
                .style("color", Color::WHITE)
                .style("size", 25.0),
            lay![size: size!(480.0, 50.0), margin: [10.0, 25.0, 0.0, 0.0],]
        );
        let ease_in_out_animation = self.state_ref().ease_in_out_animation.get_value();
        let ease_in_out_rectangle = node!(
            RoundedRect::new(Color::WHITE, 100.0),
            lay![
                size: size!(50.0, 50.0),
                margin: [25.0, 25.0 + 375.0 * ease_in_out_animation , 0.0, 0.0],
            ]
        )
        .key((ease_in_out_animation * 1000000.0) as u64);

        let mut base = node!(
            Div::new().bg(Color::BLACK),
            lay![direction: Direction::Column, size: size!(480.0, 480.0)]
        );
        base = base.push(linear_rectangle);
        base = base.push(linear_label);

        base = base.push(ease_in_out_rectangle);
        base = base.push(ease_in_out_label);
        Some(base)
    }

    fn update(&mut self, message: Message) -> Vec<Message> {
        vec![message]
    }
}

// Layer Surface App
#[tokio::main]
async fn main() {
    let id = 1;
    let ui_t = std::thread::spawn(move || {
        let _ = launch_ui(id);
    });
    ui_t.join().unwrap();
}

impl RootComponent<AppParams> for App {
    fn root(&mut self, w: &dyn std::any::Any, app_params: &dyn Any) {
        let app_params = app_params.downcast_ref::<AppParams>().unwrap();
        self.state_mut().app_channel = app_params.app_channel.clone();
    }
}

fn launch_ui(id: i32) -> anyhow::Result<()> {
    let assets: HashMap<String, AssetParams> = HashMap::new();
    let mut svgs: HashMap<String, String> = HashMap::new();

    svgs.insert(
        "eye_icon".to_string(),
        "./src/assets/icons/eye.svg".to_string(),
    );

    let mut fonts = cosmic_text::fontdb::Database::new();
    fonts.load_system_fonts();

    fonts.load_font_data(include_bytes!("assets/fonts/SpaceGrotesk-Regular.ttf").into());

    let window_opts = WindowOptions {
        height: 480_u32,
        width: 480_u32,
        scale_factor: 1.0,
    };

    println!("id: {id:?}");
    let window_info = WindowInfo {
        id: format!("{:?}{:?}", "mctk.examples.animations".to_string(), id),
        title: format!("{:?}{:?}", "mctk.examples.animations".to_string(), id),
        namespace: format!("{:?}{:?}", "mctk.examples.animations".to_string(), id),
    };
    let layer_shell_opts = LayerOptions {
        anchor: wlr_layer::Anchor::LEFT | wlr_layer::Anchor::RIGHT | wlr_layer::Anchor::TOP,
        layer: wlr_layer::Layer::Top,
        keyboard_interactivity: wlr_layer::KeyboardInteractivity::Exclusive,
        namespace: Some(window_info.namespace.clone()),
        zone: 0_i32,
    };

    let (app_channel_tx, app_channel_rx) = calloop::channel::channel();
    let (mut app, mut event_loop, window_tx) = LayerWindow::open_blocking::<App, AppParams>(
        LayerWindowParams {
            window_info,
            window_opts,
            fonts,
            assets,
            layer_shell_opts,
            svgs,
            ..Default::default()
        },
        AppParams {
            app_channel: Some(app_channel_tx),
        },
    );
    let handle = event_loop.handle();
    let window_tx_2 = window_tx.clone();

    let window_tx_channel = window_tx.clone();

    let _ = handle.insert_source(app_channel_rx, move |event: Event<AppMessage>, _, app| {
        match event {
            calloop::channel::Event::Msg(msg) => match msg {
                AppMessage::Exit => {
                    println!("app channel message {:?}", AppMessage::Exit);
                    let _ = window_tx_2.send(WindowMessage::WindowEvent {
                        event: mctk_smithay::WindowEvent::CloseRequested,
                    });
                }
            },
            calloop::channel::Event::Closed => {
                println!("calloop::event::closed");
            }
        };
    });

    loop {
        let _ = event_loop.dispatch(None, &mut app);

        if app.is_exited {
            break;
        }
    }

    Ok(())
}

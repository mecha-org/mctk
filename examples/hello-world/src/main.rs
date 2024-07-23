use anyhow::Error;
use mctk_core::component::{Component, Message, RenderContext, RootComponent};
use mctk_core::layout::Direction;
use mctk_core::reexports::cosmic_text;
use mctk_core::reexports::smithay_client_toolkit::{
    reexports::calloop::{self, channel::Event},
    shell::wlr_layer,
};
use mctk_core::renderables::Renderable;
use mctk_core::style::Styled;
use mctk_core::widgets::{Button, Div, TextBox};
use mctk_core::{lay, msg, rect, size, size_pct, txt, AssetParams, Color};
use mctk_core::{node, node::Node};
use mctk_macros::{component, state_component_impl};
use mctk_smithay::layer_shell::layer_surface::LayerOptions;
use mctk_smithay::layer_shell::layer_window::{LayerWindow, LayerWindowParams};
use mctk_smithay::xdg_shell::xdg_window::{self, XdgWindowMessage, XdgWindowParams};
use mctk_smithay::{WindowInfo, WindowMessage, WindowOptions};
use smithay_client_toolkit::reexports::calloop::channel::Sender;
use std::any::Any;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time;

// App level channel
#[derive(Debug)]
pub enum AppMessage {
    Exit,
}

#[derive(Debug, Clone)]
pub struct AppParams {
    app_channel: Option<calloop::channel::Sender<AppMessage>>,
}

#[derive(Debug, Default)]
pub struct AppState {
    value: f32,
    btn_pressed: bool,
    window_sender: Option<Sender<XdgWindowMessage>>,
    app_channel: Option<Sender<AppMessage>>,
}

#[derive(Debug, Clone)]
enum HelloEvent {
    ButtonPressed {
        name: String,
    },
    TextBox {
        name: String,
        value: String,
        update_type: String,
    },
    Exit,
}

#[component(State = "AppState")]
#[derive(Debug, Default)]
pub struct App {}

#[state_component_impl(AppState)]
impl Component for App {
    fn init(&mut self) {
        self.state = Some(AppState {
            value: 30.,
            btn_pressed: false,
            window_sender: None,
            app_channel: None,
        })
    }

    fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
        None
    }

    // fn on_tick(&mut self, _event: &mut mctk_core::event::Event<mctk_core::event::Tick>) {
    //     let value = self.state_ref().value;
    //     self.state_mut().value = value + 1.;
    // }

    fn view(&self) -> Option<Node> {
        let btn_pressed = self.state_ref().btn_pressed;
        let value = self.state_ref().value;

        println!("value is {:?}", value);

        Some(
            node!(
                Div::new().bg(Color::rgb(255., 0., 0.)),
                lay![
                    size: size_pct!(100.0),
                    direction: Direction::Column
                ]
            )
            .push(node!(
                Button::new(txt!(""))
                    .on_click(Box::new(|| msg!(HelloEvent::Exit)))
                    .on_double_click(Box::new(|| msg!(HelloEvent::ButtonPressed {
                        name: "Double clicked".to_string()
                    })))
                    .style("color", Color::rgb(255., 0., 0.))
                    .style("background_color", Color::rgb(value % 255., 255., 255.))
                    .style("active_color", Color::rgb(200., 200., 200.))
                    .style("font_size", 24.0),
                lay![size: size!(180.0, 180.0), margin: [0., 0., 20., 0.]]
            )),
        )
    }

    fn update(&mut self, message: Message) -> Vec<Message> {
        println!("App has sent: {:?}", message.downcast_ref::<HelloEvent>());
        match message.downcast_ref::<HelloEvent>() {
            Some(HelloEvent::ButtonPressed { name }) => {
                println!("{}", name);
                self.state_mut().btn_pressed = true;
            }
            Some(HelloEvent::Exit) => {
                println!("button clicked");
            }
            _ => (),
        }
        vec![]
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
        println!("root initialized");
        let app_params = app_params.downcast_ref::<AppParams>().unwrap();
        self.state_mut().app_channel = app_params.app_channel.clone();
    }
}

fn launch_ui(id: i32) -> anyhow::Result<()> {
    // let env_filter = EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("debug"));
    // tracing_subscriber::fmt()
    //     .compact()
    //     .with_env_filter(env_filter)
    //     .init();

    // let mut fonts: Vec<String> = Vec::new();
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
        height: 300 as u32,
        width: 350 as u32,
        scale_factor: 1.0,
    };

    println!("id: {id:?}");
    let window_info = WindowInfo {
        id: format!("{:?}{:?}", "mctk.examples.hello-world".to_string(), id),
        title: format!("{:?}{:?}", "mctk.examples.hello-world".to_string(), id),
        namespace: format!("{:?}{:?}", "mctk.examples.hello-world".to_string(), id),
    };
    let layer_shell_opts = LayerOptions {
        anchor: wlr_layer::Anchor::LEFT | wlr_layer::Anchor::RIGHT | wlr_layer::Anchor::TOP,
        layer: wlr_layer::Layer::Top,
        keyboard_interactivity: wlr_layer::KeyboardInteractivity::Exclusive,
        namespace: Some(window_info.namespace.clone()),
        zone: 0 as i32,
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

    let _ = handle.insert_source(app_channel_rx, move |event: Event<AppMessage>, _, app| {
        let _ = match event {
            // calloop::channel::Event::Msg(msg) => app.app.push_message(msg),
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

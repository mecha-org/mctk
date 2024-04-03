use mctk_core::component::{Component, Message, RootComponent};
use mctk_core::layout::Alignment;
use mctk_core::reexports::cosmic_text;
use mctk_core::widgets::{self, Button, Image, Svg};
use mctk_core::{lay, msg, size, size_pct, txt, Color};
use mctk_core::{node, node::Node};
use mctk_macros::{component, state_component_impl};
use mctk_smithay::layer_surface::LayerOptions;
use mctk_smithay::layer_window::LayerWindowParams;
use mctk_smithay::WindowOptions;
use smithay_client_toolkit::shell::wlr_layer;
use std::collections::HashMap;
use tracing_subscriber::EnvFilter;

// App level channel
pub enum AppMessage {}

#[derive(Debug, Default)]
pub struct AppState {}

#[derive(Debug, Clone)]
enum HelloEvent {
    Button { name: String },
}

#[component(State = "AppState")]
#[derive(Debug, Default)]
pub struct App {}

#[state_component_impl(AppState)]
impl Component for App {
    fn init(&mut self) {
        self.state = Some(AppState {})
    }

    fn view(&self) -> Option<Node> {
        println!("view called");

        let svg_name = String::from("android");
        Some(
            node!(
                widgets::Div::new(),
                lay![size_pct: [100.0],
                     wrap: true,
                    //  padding: [10.0],
                     axis_alignment: Alignment::Center,
                     cross_alignment: Alignment::Center,
                ]
            )
            .push(
                node!(
                    widgets::Div::new().bg(Color::rgba(0., 0., 0., 0.)),
                    lay![size: size!(24.0, 24.0),
                         axis_alignment: Alignment::Start,
                         cross_alignment: Alignment::Center,
                    ]
                )
                .push(node!(
                    Svg::new(svg_name.clone()),
                    lay!(size: size!(16.0, 16.0), ),
                )),
            )
            .push(
                node!(
                    widgets::Div::new().bg(Color::rgba(0., 0., 0., 0.)),
                    lay![size: size!(36.0, 36.0),
                         axis_alignment: Alignment::Start,
                         cross_alignment: Alignment::Center,
                    ]
                )
                .push(node!(
                    Svg::new(svg_name.clone()),
                    lay!(size: size!(24.0, 24.0), ),
                )),
            )
            .push(
                node!(
                    widgets::Div::new().bg(Color::rgba(0., 0., 0., 0.)),
                    lay![size: size!(48.0, 48.0),
                         axis_alignment: Alignment::Start,
                         cross_alignment: Alignment::Center,
                    ]
                )
                .push(node!(
                    Svg::new(svg_name.clone()),
                    lay!(size: size!(32.0, 32.0), ),
                )),
            )
            .push(
                node!(
                    widgets::Div::new().bg(Color::rgba(0., 0., 0., 0.)),
                    lay![size: size!(84.0, 84.0),
                         axis_alignment: Alignment::Start,
                         cross_alignment: Alignment::Center,
                    ]
                )
                .push(node!(
                    Svg::new(svg_name.clone()),
                    lay!(size: size!(64.0, 64.0), ),
                )),
            )
            .push(
                node!(
                    widgets::Div::new().bg(Color::rgba(0., 0., 0., 0.)),
                    lay![size: size!(320.0, 320.0),
                         axis_alignment: Alignment::Start,
                         cross_alignment: Alignment::Center,
                    ]
                )
                .push(node!(
                    Svg::new(svg_name.clone()),
                    lay!(size: size!(300.0, 300.0), ),
                )),
            ),
        )
    }

    fn update(&mut self, message: Message) -> Vec<Message> {
        println!("App was sent: {:?}", message.downcast_ref::<HelloEvent>());
        vec![]
    }
}

// Layer Surface App
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("inside main");
    let env_filter = EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("debug"));
    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(env_filter)
        .init();

    let mut fonts = cosmic_text::fontdb::Database::new();
    fonts.load_system_fonts();

    fonts.load_font_data(include_bytes!("assets/fonts/SpaceGrotesk-Regular.ttf").into());

    let mut assets: HashMap<String, String> = HashMap::new();

    assets.insert("bg".to_string(), "src/assets/icons/bg.png".to_string());

    let mut svgs = HashMap::new();
    svgs.insert(
        "battery".to_string(),
        "src/assets/icons/battery.svg".to_string(),
    );
    svgs.insert(
        "android".to_string(),
        "src/assets/svgs/android.svg".to_string(),
    );

    let namespace = "mctk.layer_shell.demo".to_string();

    let layer_shell_opts = LayerOptions {
        anchor: wlr_layer::Anchor::TOP | wlr_layer::Anchor::LEFT | wlr_layer::Anchor::RIGHT,
        layer: wlr_layer::Layer::Overlay,
        keyboard_interactivity: wlr_layer::KeyboardInteractivity::Exclusive,
        namespace: Some(namespace.clone()),
        zone: 0,
    };

    let window_opts = WindowOptions {
        height: 480 as u32,
        width: 480 as u32,
        scale_factor: 1.0,
    };

    let (mut app, mut event_loop, ..) =
        mctk_smithay::layer_window::LayerWindow::open_blocking::<App, AppMessage>(
            LayerWindowParams {
                title: "Hello scroll!".to_string(),
                namespace,
                window_opts,
                fonts,
                assets,
                svgs,
                layer_shell_opts,
            },
            None,
        );

    // event_loop
    // .run(None, &mut app, |_| {
    //     // event_loop.d
    // })
    loop {
        event_loop.dispatch(None, &mut app).unwrap();
    }

    // let window_opts = WindowOptions {
    //     height: 480 as u32,
    //     width: 480 as u32,
    //     scale_factor: 1.0,
    // };

    // let layer_shell_opts = LayerShellOptions {
    //     anchor: wlr_layer::Anchor::TOP | wlr_layer::Anchor::LEFT | wlr_layer::Anchor::RIGHT,
    //     layer: wlr_layer::Layer::Overlay,
    //     keyboard_interactivity: wlr_layer::KeyboardInteractivity::Exclusive,
    //     namespace: Some(String::from("mechanix.layer_shell.demo")),
    // };

    // // let app_root = AppRoot {};
    // // let app = Counter::new();
    // // let app_renderer: Application<CounterMessage> = Application::new(Box::new(app));

    // let (mut state, mut event_loop) =
    //     LayerShellApplication::new(window_opts, layer_shell_opts)
    //         .expect("failed to create application");

    // let handle = event_loop.handle();

    // //subscribe to events channel
    // // let (channel_tx, channel_rx) = calloop::channel::channel();

    // // let _ = handle.insert_source(channel_rx, |event, _, app| {
    // //     let _ = match event {
    // //         // calloop::channel::Event::Msg(msg) => app.app.push_message(msg),
    // //         calloop::channel::Event::Msg(msg) => {}
    // //         calloop::channel::Event::Closed => {}
    // //     };
    // // });

    // // init_services(settings, channel_tx).await;

    // event_loop
    //     .run(std::time::Duration::from_millis(20), &mut state, |_| {})
    //     .expect("Error during event loop!");

    Ok(())
}

impl RootComponent<AppMessage> for App {}

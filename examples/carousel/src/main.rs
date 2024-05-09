use mctk_core::component::{Component, RootComponent};
use mctk_core::reexports::cosmic_text;
use mctk_core::renderables::types;
use mctk_core::widgets::{Carousel, Div};
use mctk_core::{lay, size, AssetParams};
use mctk_core::{node, node::Node};
use mctk_smithay::layer_surface::LayerOptions;
use mctk_smithay::layer_window::LayerWindowParams;
use mctk_smithay::WindowOptions;
use smithay_client_toolkit::reexports::calloop;
use smithay_client_toolkit::shell::wlr_layer;
use std::collections::HashMap;
use std::time::Duration;
use tracing_subscriber::EnvFilter;

type Point = types::Point<f32>;

// App level channel
pub enum AppMessage {}

#[derive(Debug, Clone)]
enum HelloEvent {
    Button { name: String },
}

#[derive(Debug, Default)]
pub struct App {}

impl Component for App {
    fn view(&self) -> Option<Node> {
        // println!("app view called");

        Some(
            node!(Carousel::new().scroll_x(), [
                size: [480, 60],
                direction: Row,
            ])
            .push(node!(
                Div::new().bg([255.0, 0.0, 0.0]),
                [ size: [40, 40]],
            ))
            .push(node!(
                Div::new().bg([255.0, 133.5, 0.0]),
                [ size: [40, 40]],
            ))
            .push(node!(
                Div::new().bg([255.0, 255.0, 0.0]),
                [ size: [40, 40]],
            ))
            .push(node!(
                Div::new().bg([0.0, 255.0, 0.0]),
                [ size: [40, 40]],
            ))
            .push(node!(
                Div::new().bg([255.0, 0.0, 0.0]),
                [ size: [40, 40]],
            ))
            .push(node!(
                Div::new().bg([255.0, 133.5, 0.0]),
                [ size: [40, 40]],
            ))
            .push(node!(
                Div::new().bg([255.0, 255.0, 0.0]),
                [ size: [40, 40]],
            ))
            .push(node!(
                Div::new().bg([255.0, 0.0, 0.0]),
                [ size: [40, 40]],
            ))
            .push(node!(
                Div::new().bg([255.0, 133.5, 0.0]),
                [ size: [40, 40]],
            ))
            .push(node!(
                Div::new().bg([255.0, 255.0, 0.0]),
                [ size: [40, 40]],
            ))
            .push(node!(
                Div::new().bg([0.0, 255.0, 0.0]),
                [ size: [40, 40]],
            ))
            .push(node!(
                Div::new().bg([255.0, 0.0, 0.0]),
                [ size: [40, 40]],
            ))
            .push(node!(
                Div::new().bg([255.0, 133.5, 0.0]),
                [ size: [40, 40]],
            ))
            .push(node!(
                Div::new().bg([255.0, 255.0, 0.0]),
                [ size: [40, 40]],
            )),
        )
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

    let mut assets: HashMap<String, AssetParams> = HashMap::new();

    assets.insert(
        "bg".to_string(),
        AssetParams::new("src/assets/icons/bg.png".to_string()),
    );

    let mut svgs = HashMap::new();
    svgs.insert(
        "battery".to_string(),
        "src/assets/icons/battery.svg".to_string(),
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

    let (layer_tx, layer_rx) = calloop::channel::channel();
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
                layer_tx,
                layer_rx,
            },
            None,
        );

    // event_loop
    // .run(None, &mut app, |_| {
    //     // event_loop.d
    // })
    loop {
        event_loop
            .dispatch(Duration::from_millis(16), &mut app)
            .unwrap();
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

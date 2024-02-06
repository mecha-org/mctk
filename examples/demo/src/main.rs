mod demo;
use demo::DemoApp;
use mctk_core::{
    layer_shell::{LayerShellApplication, LayerShellOptions, WindowOptions},
    reexports::smithay_client_toolkit::{
        reexports::calloop::timer::{TimeoutAction, Timer},
        shell::wlr_layer,
    },
};
use std::time::Duration;
use tracing_subscriber::EnvFilter;

use crate::demo::Message;

// Layer Surface App
fn main() -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("debug"));
    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(env_filter)
        .init();

    let app = DemoApp::new();

    let window_opts = WindowOptions {
        height: 480,
        width: 480,
        scale_factor: 2.0,
    };

    let layer_shell_opts = LayerShellOptions {
        anchor: wlr_layer::Anchor::TOP
            | wlr_layer::Anchor::BOTTOM
            | wlr_layer::Anchor::LEFT
            | wlr_layer::Anchor::RIGHT,
        layer: wlr_layer::Layer::Overlay,
        keyboard_interactivity: wlr_layer::KeyboardInteractivity::Exclusive,
        namespace: Some(String::from("mechanix.layer_shell.demo")),
    };

    let (mut state, mut event_loop) =
        LayerShellApplication::new(app, window_opts, layer_shell_opts)
            .expect("failed to create application");

    let handle = event_loop.handle();
    let source = Timer::from_duration(std::time::Duration::from_millis(5000));

    handle
        .insert_source(
            // a type which implements the EventSource trait
            source,
            // a callback that is invoked whenever this source generates an event
            |event, _metadata, state| {
                println!("timeout for {:?} expired!", event);
                state.app.push_message(Message::Trigger);
                TimeoutAction::ToDuration(Duration::from_millis(5000))
            },
        )
        .expect("failed to insert event source!");

    event_loop
        .run(std::time::Duration::from_millis(20), &mut state, |_| {})
        .expect("Error during event loop!");

    // loop {
    //     event_loop.dispatch(None, &mut state)?;

    //     if false {
    //         break;
    //     }
    // }

    Ok(())
}

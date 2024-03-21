use std::collections::HashMap;
use std::time::Duration;

// mod counter;
use mctk_core::component::{Component, Message, RenderContext, RootComponent};
use mctk_core::layout::Alignment;
// use counter::{Counter, CounterMessage};
// use mctk_core::app::Application;
use mctk_core::reexports::smithay_client_toolkit::{
    reexports::calloop::{
        self,
        channel::Sender,
        timer::{TimeoutAction, Timer},
    },
    shell::wlr_layer,
};
use mctk_core::renderables::types::Size;
use mctk_core::renderables::{types, Renderable};
use mctk_core::style::{HorizontalPosition, Styled};
use mctk_core::types::Pos;
use mctk_core::widgets::{self, Button, Image, Svg};
use mctk_core::{lay, layout, msg, rect, size, size_pct, txt, Color};
use mctk_core::{node, node::Node};
use mctk_macros::{component, state_component_impl};
use mctk_smithay::layer_surface::LayerOptions;
use mctk_smithay::layer_window::LayerWindowParams;
use mctk_smithay::WindowOptions;
use tracing::info;
use tracing_subscriber::EnvFilter;

type Point = types::Point<f32>;

#[derive(Debug, Default)]
pub struct AppState {
    value: f32,
    btn_pressed: bool,
}

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
        self.state = Some(AppState {
            value: 30.,
            btn_pressed: false,
        })
    }

    fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
        println!("render called");

        let value = self.state_ref().value;
        // Some(vec![Renderable::Rect(Rect::new(
        //     [0.0, 0.0].into(),
        //     [100., 100.0].into(),
        //     Color::rgb(0., 0., 1.),
        // ))])
        None
    }

    fn on_click(&mut self, _event: &mut mctk_core::event::Event<mctk_core::event::Click>) {
        let value = self.state_ref().value;
        println!("app:component:on_click() {}", value);
        self.state_mut().value = value + 1.;

        self.state_mut().btn_pressed = true;
    }

    fn view(&self) -> Option<Node> {
        println!("view called");

        let btn_pressed = self.state_ref().btn_pressed;

        // Some(
        //     node!(widgets::Div::new().bg(0.7),
        //            [size_pct: [100, Auto]])
        //     .push(node!(widgets::Text::new(
        //          txt!("Lorem")).style("font", "SpaceGrotesk-Bold"),
        //                  [size_pct: [100.0, Auto], margin: [10]])),
        // )

        // Some(
        //     node!(widgets::Div::new().bg(0.7),
        //            [size_pct: [100, Auto]])
        //     .push(node!(widgets::Text::new(
        //     txt!("Lorem"))
        //     .style("h_alignment", HorizontalPosition::Left)
        //     .style("font", "SpaceGrotesk-Bold")
        //     ,
        //             [
        //                 size_pct: [100, Auto],
        //                margin: [10],

        //            ])),
        // )

        Some(
            node!(
                widgets::Div::new(),
                lay![size_pct: [100.0],
                     wrap: true,
                    //  padding: [10.0],
                     axis_alignment: Alignment::Start,
                     cross_alignment: Alignment::Start,
                ]
            )
            .push(node!(
                Button::new(txt!("Click me!"))
                    .on_click(Box::new(|| msg!(HelloEvent::Button {
                        name: "It me, a button!".to_string()
                    })))
                    .style(
                        "background_color",
                        match btn_pressed {
                            true => Color::rgb(0., 1.0, 0.),
                            false => Color::rgb(0., 0.0, 1.),
                        }
                    )
                    .style("font_size", 16.0),
                lay!(size: size!(60.0, Auto)),
            ))
            .push(node!(
                widgets::Div::new().bg(Color::RED),
                lay![size: [100.0, 24.0],
                     wrap: true,
                    //  padding: [10.0],
                     axis_alignment: Alignment::Start,
                     cross_alignment: Alignment::Start,
                ]
            ))
            // .push(node!(
            //     Button::new(txt!("Click me!."))
            //         .on_click(Box::new(|| msg!(HelloEvent::Button {
            //             name: "It me, a button!".to_string()
            //         })))
            //         .style(
            //             "background_color",
            //             match btn_pressed {
            //                 true => Color::rgb(0., 1.0, 0.),
            //                 false => Color::rgb(1., 0.0, 0.),
            //             }
            //         ),
            //     lay!(size: size!(100.0, 60.0 ), margin: [10],),
            // ))
            .push(
                node!(
                    widgets::Div::new().bg(Color::rgb(0., 1.0, 1.)),
                    lay![size: size!(100.0, 100.0),
                         axis_alignment: Alignment::Center,
                         cross_alignment: Alignment::Center,
                    ]
                )
                .push(node!(Image::new("bg"), lay!(size: size!(40.0, 40.0), ),)),
            )
            .push(
                node!(
                    widgets::Div::new().bg(Color::rgb(1., 0., 1.)),
                    lay![size: size!(100.0, 100.0),
                         axis_alignment: Alignment::Center,
                         cross_alignment: Alignment::Center,
                    ]
                )
                .push(node!(Svg::new("battery"), lay!(size: size!(32.0, 32.0), ),)),
            ),
        )
    }

    fn update(&mut self, message: Message) -> Vec<Message> {
        println!("App was sent: {:?}", message.downcast_ref::<HelloEvent>());
        match message.downcast_ref::<HelloEvent>() {
            Some(HelloEvent::Button { name }) => {
                println!("{}", name);
                self.state_mut().btn_pressed = true;
            }
            _ => (),
        }
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

    let mut fonts: HashMap<String, String> = HashMap::new();

    fonts.insert(
        "SpaceGrotesk-Bold".to_string(),
        "src/assets/fonts/SpaceGrotesk-Bold.ttf".to_string(),
    );

    fonts.insert(
        "SpaceGrotesk-Light".to_string(),
        "src/assets/fonts/SpaceGrotesk-Light.ttf".to_string(),
    );

    fonts.insert(
        "SpaceGrotesk-Medium".to_string(),
        "src/assets/fonts/SpaceGrotesk-Medium.ttf".to_string(),
    );

    fonts.insert(
        "SpaceGrotesk-Regular".to_string(),
        "src/assets/fonts/SpaceGrotesk-Regular.ttf".to_string(),
    );

    fonts.insert(
        "SpaceGrotesk-SemiBold".to_string(),
        "src/assets/fonts/SpaceGrotesk-SemiBold.ttf".to_string(),
    );

    let mut assets: HashMap<String, String> = HashMap::new();

    assets.insert("bg".to_string(), "src/assets/icons/bg.png".to_string());

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

    let (mut app, mut event_loop, ..) =
        mctk_smithay::layer_window::LayerWindow::open_blocking::<App>(LayerWindowParams {
            title: "Hello scroll!".to_string(),
            namespace,
            window_opts,
            fonts,
            assets,
            svgs,
            layer_shell_opts,
        });

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

impl RootComponent for App {}

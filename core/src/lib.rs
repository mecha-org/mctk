pub mod component;
pub mod context;
pub mod event;
pub mod font_cache;
pub mod instrumenting;
pub mod pointer;
pub mod raw_handle;
pub mod renderables;
pub mod renderer;
pub mod style;
pub mod ui;
pub mod window;

pub mod reexports {
    pub use cosmic_text;
    pub use euclid;
    pub use femtovg;
    pub use glutin;
    pub use resource;
    pub use smithay_client_toolkit;
}

//
#[macro_use]
pub mod widgets;

#[macro_use]
pub mod animations;

pub mod types;
pub use types::*;

#[macro_use]
pub mod layout;

#[doc(hidden)]
pub use mctk_macros;

#[doc(inline)]
pub use mctk_macros::{component, state_component_impl};

#[macro_use]
pub mod node;
pub use node::*;

pub mod input;
pub use input::*;

pub mod prelude {
    pub use crate::component::*;
    pub use crate::layout::*;
    pub use crate::reexports::*;
    pub use crate::style::*;
    pub use crate::widgets::{
        Button, Carousel, Div, IconButton, IconType, Image, RoundedRect, Slider, Svg, TextBox,
        TextBoxAction, TextBoxVariant, TransitionPositions,
    };
    pub use crate::*;
}

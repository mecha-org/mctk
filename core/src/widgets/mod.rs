//! Built-in Components.

mod button;
pub use button::Button;

mod icon_button;
pub use icon_button::{IconButton, IconType};

mod rounded_rect;
pub use rounded_rect::RoundedRect;

mod text;
pub use text::Text;

mod div;
pub use div::Div;

mod image;
pub use image::Image;

mod svg;
pub use svg::Svg;

mod slider;
pub use slider::Slider;

mod carousel;
pub use carousel::{Carousel, TransitionPositions};

mod textbox;
pub use textbox::{TextBox, TextBoxAction, TextBoxVariant};

// mod slide_show;
// pub use slide_show::SlideShow;

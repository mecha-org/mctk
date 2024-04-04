use cosmic_text::fontdb::Database;
use cosmic_text::{Buffer, FontSystem, LayoutGlyph, Metrics};
use femtovg::Align;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

use crate::renderables::text::{self, InstanceBuilder};
use crate::renderer::text::TextRenderer;
use crate::style::HorizontalPosition;
use crate::{Pos, Scale};

/// Value by which fonts are scaled. 12 px fonts render at scale 18 px for some reason. Useful if you need to compute the line height: it will be `<font_size> * SIZE_SCALE` in logical size, and `<font_size> * SIZE_SCALE * <scale_factor>` in physical pixels.
pub const SIZE_SCALE: f32 = 1.5;
pub const DEFAULT_FONT_SIZE: f32 = 12.;
pub const DEFAULT_LINE_HEIGHT: f32 = 16.;
pub const GLYPH_PADDING: u32 = 0;
pub const GLYPH_MARGIN: u32 = 0;
pub const TEXTURE_SIZE: usize = 512;

pub struct FontCache {
    text_renderer: TextRenderer,
}

impl FontCache {
    pub fn new(fonts: Database) -> Self {
        let text_renderer = TextRenderer::new(fonts);

        Self { text_renderer }
    }

    pub fn measure_text(
        &mut self,
        text: String,
        font: Option<String>,
        size: f32,
        scale_factor: f32,
        line_height: f32,
        h_alignment: HorizontalPosition,
        bounds: (f32, f32),
    ) -> (Option<f32>, Option<f32>, Vec<LayoutGlyph>) {
        let font_size = size * scale_factor;
        let text_renderer = &mut self.text_renderer;

        let text_instance = InstanceBuilder::default()
            .align(match h_alignment {
                HorizontalPosition::Left => Align::Left,
                HorizontalPosition::Center => Align::Center,
                HorizontalPosition::Right => Align::Right,
            })
            .pos(Pos {
                x: 0.,
                y: 0.,
                z: 0.,
            })
            .scale(Scale {
                width: bounds.0,
                height: bounds.1,
            })
            .text(text.to_string())
            .font(font)
            .line_height(line_height)
            .font_size(font_size)
            .build()
            .unwrap();

        text_renderer.measure_text(text_instance)
    }
}

/// Used by [`FontCache#layout_text`][FontCache#method.layout_text] as an input. Accordingly, it is also commonly used as the input to Components that display text, e.g. [`widgets::Text`][crate::widgets::Text] and [`widgets::Button`][crate::widgets::Button].
///
/// [`txt`][crate::txt] is provided as a convenient constructor, but you can also use `into` from a `&str` or `String`, e.g. `"some text".into()`.
#[derive(Debug, Clone)]
pub struct TextSegment {
    /// The text to be laid out.
    pub text: String,
    /// An optional size. A default will be selected if `None`.
    pub size: Option<f32>,
    /// An optional font name. A default will be selected if `None`.
    pub font: Option<String>,
}

impl From<&str> for TextSegment {
    fn from(s: &str) -> TextSegment {
        s.to_string().into()
    }
}

impl From<String> for TextSegment {
    fn from(text: String) -> TextSegment {
        TextSegment {
            text,
            size: None,
            font: None,
        }
    }
}

#[cfg(feature = "open_iconic")]
impl From<crate::open_iconic::Icon> for TextSegment {
    fn from(icon: crate::open_iconic::Icon) -> TextSegment {
        String::from(icon).into()
    }
}

/// Convenience constructor for a `Vec` of [`TextSegment`]s.
///
/// `txt` accepts a variable number of arguments. Each argument can come in one of four forms:
/// - `"text"`: A value that is `Into<String>`.
/// - `("text", "font_name")`: A text string, and a font name, both must be `Into<String>`.
/// - `("text", "font_name", 12.0)`: A text string, a font name, and an `f32` font size.
/// - `("text", None, 12.0)`: A text string and a font size.
///
/// If no font name or size is given, defaults are assumed.
///
/// This lets you mix different text styles, e.g.:
/// ```
/// # use mctk_core::*;
/// let text = txt!("Hello", ("world", "Helvetica Bold", 22.0), "!");
/// ```

#[macro_export]
macro_rules! txt {
    // split_comma taken from: https://gist.github.com/kyleheadley/c2f64e24c14e45b1e39ee664059bd86f

    // give initial params to the function
    {@split_comma  ($($first:tt)*) <= $($item:tt)*} => {
        txt![@split_comma ($($first)*) () () <= $($item)*]

    };
    // give inital params and initial inner items in every group
    {@split_comma  ($($first:tt)*) ($($every:tt)*) <= $($item:tt)*} => {
        txt![@split_comma ($($first)*) ($($every)*) ($($every)*) <= $($item)*]

    };
    // KEYWORD line
    // on non-final seperator, stash the accumulator and restart it
    {@split_comma  ($($first:tt)*) ($($every:tt)*) ($($current:tt)*) <= , $($item:tt)+} => {
        txt![@split_comma ($($first)* ($($current)*)) ($($every)*) ($($every)*) <= $($item)*]

    };
    // KEYWORD line
    // ignore final seperator, run the function
    {@split_comma  ($($first:tt)*) ($($every:tt)*) ($($current:tt)+) <= , } => {
        txt![@txt_seg $($first)* ($($current)*)]

    };
    // on next item, add it to the accumulator
    {@split_comma  ($($first:tt)*) ($($every:tt)*) ($($current:tt)*) <= $next:tt $($item:tt)*} => {
        txt![@split_comma ($($first)*) ($($every)*) ($($current)* $next)  <= $($item)*]

    };
    // at end of items, run the function
    {@split_comma  ($($first:tt)*) ($($every:tt)*) ($($current:tt)+) <= } => {
        txt![@txt_seg $($first)* ($($current)*)]

    };
    // if there were no items and no default, run with only initial params, if any
    {@split_comma  ($($first:tt)*) () () <= } => {
        txt![@txt_seg $($first)*]

    };
    // End split_comma

    // Operation performed per comma-separated expr
    (@as_txt_seg  ($text:expr, None, $size:expr)) => { $crate::font_cache::TextSegment {
        text: $text.into(),
        size: Some($size),
        font: None,
    } };

    (@as_txt_seg  ($text:expr, $font:expr, $size:expr)) => { $crate::font_cache::TextSegment {
        text: $text.into(),
        size: Some($size),
        font: Some($font.into()),
    } };

    (@as_txt_seg  ($text:expr, $font:expr)) => { $crate::font_cache::TextSegment {
        text: $text.into(),
        size: None,
        font: Some($font.into()),
    } };

    (@as_txt_seg  $e:expr) => {
        $e.into()
    };

    // Operation called by split_comma with parenthesized groups
    (@txt_seg  $(($($item:tt)*))*) => { vec![$(txt!(@as_txt_seg $($item)*) , )*] };

    // Entry point
    ($($e:tt)*) => {
        txt![@split_comma () () () <= $($e)*]
    }
}

impl Hash for TextSegment {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.size.map(|s| (s * 100.0) as u32).hash(state);
        self.font.hash(state);
        self.text.hash(state);
    }
}

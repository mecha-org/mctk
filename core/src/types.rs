use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::mem;
use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Sub, SubAssign};
use std::path::PathBuf;

/// Data that can be shared between processes, e.g. by the Clipboard or Drag and Drop.
#[derive(Debug, Clone, PartialEq)]
pub enum Data {
    String(String),
    Filepath(PathBuf),
    // Custom(Vec<u8>),
}

impl From<&str> for Data {
    fn from(s: &str) -> Data {
        Data::String(s.to_string())
    }
}

/// An object that can be scaled by a scale factor. This is used to adjust the size of things to the scale factor used by the user's monitor.
pub trait Scalable {
    // Logical to physical coordinates
    fn scale(self, _scale_factor: f32) -> Self;

    // Physical to logical coordinates
    fn unscale(self, scale_factor: f32) -> Self
    where
        Self: Sized,
    {
        self.scale(1.0 / scale_factor)
    }
}

/// Clamp the input `x` between `min` and `max`.
pub(crate) fn clamp(x: f32, min: f32, max: f32) -> f32 {
    if min > max {
        panic!("`min` should not be greater than `max`");
    } else if x < min {
        min
    } else if x > max {
        max
    } else {
        x
    }
}

/// The size of something, in pixels.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub struct PixelSize {
    pub width: u32,
    pub height: u32,
}

impl PixelSize {
    /// Constructor
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Calculate the area in pixels.
    pub fn area(&self) -> u32 {
        self.width * self.height
    }
}

/// Two dimensional scale factor, used by [`renderables::Rect`][crate::renderables::Rect].
#[derive(Debug, Default, Copy, Clone, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct Scale {
    pub width: f32,
    pub height: f32,
}

impl Hash for Scale {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.width as u64).hash(state);
        (self.height as u64).hash(state);
    }
}

impl Scalable for Scale {
    fn scale(self, scale_factor: f32) -> Self {
        Self {
            width: self.width * scale_factor,
            height: self.height * scale_factor,
        }
    }
}

impl Scale {
    /// Constructor
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

impl Sub for Scale {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Scale {
            width: self.width - other.width,
            height: self.height - other.height,
        }
    }
}

impl Mul<f32> for Scale {
    type Output = Self;
    fn mul(self, factor: f32) -> Scale {
        Scale {
            width: self.width * factor,
            height: self.height * factor,
        }
    }
}

impl Add<f32> for Scale {
    type Output = Self;
    fn add(self, factor: f32) -> Scale {
        Scale {
            width: self.width + factor,
            height: self.height + factor,
        }
    }
}

impl From<[f32; 2]> for Scale {
    fn from(p: [f32; 2]) -> Self {
        unsafe { mem::transmute(p) }
    }
}

impl From<PixelSize> for Scale {
    fn from(s: PixelSize) -> Self {
        Self {
            width: s.width as f32,
            height: s.height as f32,
        }
    }
}

/// An `(x, y)` coordinate, in pixels.
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub struct PixelPoint {
    pub x: u32,
    pub y: u32,
}

impl PixelPoint {
    /// Constructor
    pub fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
}

impl From<Point> for PixelPoint {
    fn from(p: Point) -> Self {
        Self {
            x: p.x.round() as u32,
            y: p.y.round() as u32,
        }
    }
}

/// An `(x, y)` coordinate.
#[derive(Debug, Default, Copy, Clone, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    /// Constructor
    pub fn new(x: f32, y: f32) -> Self {
        Point { x, y }
    }

    /// Clamp the point to be within the bounds of the [`AABB`].
    pub fn clamp(self, aabb: AABB) -> Self {
        Self {
            x: clamp(self.x, aabb.pos.x, aabb.bottom_right.x),
            y: clamp(self.y, aabb.pos.y, aabb.bottom_right.y),
        }
    }

    /// The distance between two points.
    pub fn dist(self, p2: Point) -> f32 {
        ((self.x - p2.x).powf(2.0) + (self.y - p2.y).powf(2.0)).sqrt()
    }
}

impl Scalable for Point {
    fn scale(self, scale_factor: f32) -> Self {
        Self {
            x: self.x * scale_factor,
            y: self.y * scale_factor,
        }
    }
}

impl Hash for Point {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.x as i32).hash(state);
        (self.y as i32).hash(state);
    }
}

impl From<[f32; 2]> for Point {
    fn from(p: [f32; 2]) -> Self {
        unsafe { mem::transmute(p) }
    }
}

impl From<Pos> for Point {
    fn from(p: Pos) -> Self {
        Self { x: p.x, y: p.y }
    }
}

impl Add for Point {
    type Output = Point;

    fn add(self, other: Point) -> Point {
        Point {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Point {
    type Output = Point;

    fn sub(self, other: Point) -> Point {
        Point {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Div<f32> for Point {
    type Output = Self;
    fn div(self, f: f32) -> Self {
        Self {
            x: self.x / f,
            y: self.y / f,
        }
    }
}

impl Mul<f32> for Point {
    type Output = Self;
    fn mul(self, f: f32) -> Self {
        Self {
            x: self.x * f,
            y: self.y * f,
        }
    }
}

impl AddAssign for Point {
    fn add_assign(&mut self, other: Self) {
        *self = Self {
            x: self.x + other.x,
            y: self.y + other.y,
        };
    }
}

impl SubAssign for Point {
    fn sub_assign(&mut self, other: Self) {
        *self = Self {
            x: self.x - other.x,
            y: self.y - other.y,
        };
    }
}

/// A Position coordinate `(x, y, z)`. The `z` dimension refers to the [z-index](https://developer.mozilla.org/en-US/docs/Web/CSS/z-index).
#[derive(Debug, Copy, Clone, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct Pos {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Hash for Pos {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.x as i32).hash(state);
        (self.y as i32).hash(state);
        (self.z as i32).hash(state);
    }
}

impl From<[f32; 3]> for Pos {
    fn from(p: [f32; 3]) -> Self {
        unsafe { mem::transmute(p) }
    }
}

impl From<[f32; 2]> for Pos {
    fn from(p: [f32; 2]) -> Self {
        Self {
            x: p[0],
            y: p[1],
            z: 0.0,
        }
    }
}

impl From<Point> for Pos {
    fn from(p: Point) -> Self {
        Self {
            x: p.x,
            y: p.y,
            z: 0.0,
        }
    }
}

impl Default for Pos {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

impl Scalable for Pos {
    fn scale(self, scale_factor: f32) -> Self {
        Self {
            x: self.x * scale_factor,
            y: self.y * scale_factor,
            z: self.z,
        }
    }
}

impl Pos {
    /// Constructor
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Apply [`round`](std::f32#round) to all elements.
    pub fn round(&self) -> Self {
        Self {
            x: self.x.round(),
            y: self.y.round(),
            z: self.z.round(),
        }
    }
}

impl Add for Pos {
    type Output = Pos;

    fn add(self, other: Pos) -> Pos {
        Pos {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl Sub for Pos {
    type Output = Pos;

    fn sub(self, other: Pos) -> Pos {
        Pos {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl AddAssign for Pos {
    fn add_assign(&mut self, other: Self) {
        *self = Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        };
    }
}

impl SubAssign for Pos {
    fn sub_assign(&mut self, other: Self) {
        *self = Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        };
    }
}

/// An Axis-Aligned Bounding Box, in pixels.
#[derive(Debug, Copy, Clone, PartialEq, Default)]
#[repr(C)]
pub(crate) struct PixelAABB {
    pub pos: PixelPoint,
    pub bottom_right: PixelPoint,
}

impl PixelAABB {
    pub fn normalize(&self, scale: PixelSize) -> (Point, Point) {
        (
            Point {
                x: self.pos.x as f32 / scale.width as f32,
                y: self.pos.y as f32 / scale.height as f32,
            },
            Point {
                x: self.bottom_right.x as f32 / scale.width as f32,
                y: self.bottom_right.y as f32 / scale.height as f32,
            },
        )
    }

    pub fn width(&self) -> u32 {
        self.bottom_right.x - self.pos.x
    }

    pub fn height(&self) -> u32 {
        self.bottom_right.y - self.pos.y
    }

    pub fn size(&self) -> PixelSize {
        PixelSize {
            width: self.width(),
            height: self.height(),
        }
    }

    pub fn area(&self) -> u32 {
        self.size().area()
    }
}

/// An [Axis-Aligned Bounding Box](https://en.wikipedia.org/wiki/Minimum_bounding_box). Used by some of the advanced [`Component`](crate::Component) methods, including [`render`](crate::Component#render).
#[derive(Debug, Copy, Clone, PartialEq, Default)]
#[repr(C)]
pub struct AABB {
    /// Top left + z
    pub pos: Pos,
    /// Bottom right
    pub bottom_right: Point,
}

impl AABB {
    /// Construct from a [`Pos`] (top left + z) and [`Scale`].
    pub fn new(pos: Pos, size: Scale) -> Self {
        Self {
            pos,
            bottom_right: Point {
                x: pos.x + size.width,
                y: pos.y + size.height,
            },
        }
    }

    pub fn width(&self) -> f32 {
        self.bottom_right.x - self.pos.x
    }

    pub fn height(&self) -> f32 {
        self.bottom_right.y - self.pos.y
    }

    pub fn size(&self) -> Scale {
        Scale {
            width: self.width(),
            height: self.height(),
        }
    }

    /// Is the AABB under the given [`Point`]?
    pub fn is_under(&self, p: Point) -> bool {
        p.x >= self.pos.x
            && p.x <= self.bottom_right.x
            && p.y >= self.pos.y
            && p.y <= self.bottom_right.y
    }

    /// Mutate `self`, translating by `(x, y)`.
    pub fn translate_mut(&mut self, x: f32, y: f32) {
        self.pos.x += x;
        self.bottom_right.x += x;
        self.pos.y += y;
        self.bottom_right.y += y;
    }

    /// Mutate `self`, setting the top left to `(x, y)`.
    pub fn set_top_left_mut(&mut self, x: f32, y: f32) {
        let w = self.width();
        let h = self.height();
        self.pos.x = x;
        self.bottom_right.x = x + w;
        self.pos.y = y;
        self.bottom_right.y = y + h;
    }

    /// Mutate `self`, setting width and height to `(w, h)`, maintaining the top left position.
    pub fn set_scale_mut(&mut self, w: f32, h: f32) {
        self.bottom_right.x = self.pos.x + w;
        self.bottom_right.y = self.pos.y + h;
    }

    /// Mutate `self`, applying [`round`](std::f32#round) to all `(x, y)` elements.
    pub fn round_mut(&mut self) {
        self.pos.x = self.pos.x.round();
        self.pos.y = self.pos.y.round();
        self.bottom_right.x = self.bottom_right.x.round();
        self.bottom_right.y = self.bottom_right.y.round();
    }

    /// Translating by `(x, y)`.
    pub fn translate(self, x: f32, y: f32) -> Self {
        Self {
            pos: Pos::new(self.pos.x + x, self.pos.y + y, self.pos.z),
            bottom_right: Point::new(self.bottom_right.x + x, self.bottom_right.y + y),
        }
    }

    /// Set the top left to `(x, y)`.
    pub fn set_top_left(self, x: f32, y: f32) -> Self {
        Self {
            pos: Pos::new(x, y, self.pos.z),
            bottom_right: Point::new(x + self.width(), y + self.height()),
        }
    }

    /// Set the width and height to `(w, h)`, maintaining the top left position.
    pub fn set_scale(self, w: f32, h: f32) -> Self {
        Self {
            pos: self.pos,
            bottom_right: Point::new(self.pos.x + w, self.pos.y + h),
        }
    }

    /// Apply [`round`](std::f32#round) to all `(x, y)` elements.
    pub fn round(self) -> Self {
        Self {
            pos: Pos::new(self.pos.x.round(), self.pos.y.round(), self.pos.z),
            bottom_right: Point::new(self.bottom_right.x.round(), self.bottom_right.y.round()),
        }
    }

    /// Move the top left to `(x: 0.0, y: 0.0, z: 0.0)`, but maintain the width and height.
    pub fn to_origin(self) -> Self {
        Self {
            pos: Pos::default(),
            bottom_right: Point {
                x: self.width(),
                y: self.height(),
            },
        }
    }
}

impl Scalable for AABB {
    fn scale(self, scale_factor: f32) -> Self {
        Self {
            pos: self.pos.scale(scale_factor),
            bottom_right: self.bottom_right.scale(scale_factor),
        }
    }
}

impl MulAssign<f32> for AABB {
    fn mul_assign(&mut self, f: f32) {
        self.pos.x *= f;
        self.pos.y *= f;
        self.bottom_right.x *= f;
        self.bottom_right.y *= f;
    }
}

impl Mul<f32> for AABB {
    type Output = Self;
    fn mul(self, f: f32) -> Self {
        Self {
            pos: Pos {
                x: self.pos.x * f,
                y: self.pos.y * f,
                z: self.pos.z,
            },
            bottom_right: Point {
                x: self.bottom_right.x * f,
                y: self.bottom_right.y * f,
            },
        }
    }
}

impl Div<f32> for AABB {
    type Output = Self;
    fn div(self, f: f32) -> Self {
        Self {
            pos: Pos {
                x: self.pos.x / f,
                y: self.pos.y / f,
                z: self.pos.z,
            },
            bottom_right: Point {
                x: self.bottom_right.x / f,
                y: self.bottom_right.y / f,
            },
        }
    }
}

/// RGBA color struct, used for styling and rendering. Values are normalized (0.0--1.0) floating point.
#[derive(Debug, Copy, Clone, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
#[repr(C)]
pub struct Color {
    /// Red
    pub r: f32,
    /// Green
    pub g: f32,
    /// Blue
    pub b: f32,
    /// Alpha
    pub a: f32,
}

impl Default for Color {
    fn default() -> Self {
        Self {
            r: 255.0,
            g: 255.0,
            b: 255.0,
            a: 255.0,
        }
    }
}

impl Hash for Color {
    fn hash<H: Hasher>(&self, state: &mut H) {
        ((self.r * 100000.0) as i32).hash(state);
        ((self.g * 100000.0) as i32).hash(state);
        ((self.b * 100000.0) as i32).hash(state);
        ((self.a * 100000.0) as i32).hash(state);
    }
}

impl Color {
    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const WHITE: Self = Self {
        r: 255.0,
        g: 255.0,
        b: 255.0,
        a: 1.0,
    };
    pub const LIGHT_GREY: Self = Self {
        r: 229.0,
        g: 229.0,
        b: 229.0,
        a: 1.0,
    };
    pub const MID_GREY: Self = Self {
        r: 153.0,
        g: 153.0,
        b: 153.0,
        a: 1.0,
    };
    pub const DARK_GREY: Self = Self {
        r: 76.0,
        g: 76.0,
        b: 76.0,
        a: 1.0,
    };
    pub const RED: Self = Self {
        r: 255.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const GREEN: Self = Self {
        r: 0.0,
        g: 255.0,
        b: 0.0,
        a: 1.0,
    };
    pub const BLUE: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 255.0,
        a: 1.0,
    };
    pub const YELLOW: Self = Self {
        r: 255.0,
        g: 255.0,
        b: 0.0,
        a: 1.0,
    };
    pub const MAGENTA: Self = Self {
        r: 255.0,
        g: 0.0,
        b: 255.0,
        a: 1.0,
    };

    /// RGBA constructor.
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// RGB constructor, with `A = 1.0`.
    pub fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }
    /// RGBA constructor.
    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}

impl From<[f32; 4]> for Color {
    /// Converts an array of four floats `[R, G, B, A]` into a color with values `{r: R, g: G, b: B, a: A}`
    fn from(c: [f32; 4]) -> Self {
        unsafe { mem::transmute(c) }
    }
}

impl From<&[f32; 4]> for Color {
    /// Converts a slice of four floats `[R, G, B, A]` into a color with values `{r: R, g: G, b: B, a: A}`
    fn from(c: &[f32; 4]) -> Self {
        unsafe { mem::transmute(*c) }
    }
}

impl From<[f32; 3]> for Color {
    /// Converts an array of three floats `[R, G, B]` into a color with values `{r: R, g: G, b: B, a: 1.0}`
    fn from(c: [f32; 3]) -> Self {
        Self {
            r: c[0],
            g: c[1],
            b: c[2],
            a: 1.0,
        }
    }
}

impl From<&[f32; 3]> for Color {
    /// Converts a slice of three floats `[R, G, B]` into a color with values `{r: R, g: G, b: B, a: 1.0}`
    fn from(c: &[f32; 3]) -> Self {
        Self {
            r: c[0],
            g: c[1],
            b: c[2],
            a: 1.0,
        }
    }
}
impl From<[u8; 3]> for Color {
    /// 8bit color conversion, e.g. `[0xRR, 0xGG, 0xBB].into()`
    fn from(c: [u8; 3]) -> Self {
        let b = u8_to_norm(c[2]);
        let g = u8_to_norm(c[1]);
        let r = u8_to_norm(c[0]);
        Color::new(r, g, b, 1.0)
    }
}

impl From<f32> for Color {
    /// Converts a single float `C` into a color with values `{r: C, g: C, b: C, a: 1.0}`
    fn from(c: f32) -> Self {
        Color::rgb(c, c, c)
    }
}

impl From<u32> for Color {
    /// Treats an int as a packed 8-bit RGBA value, allowing you to write `0xRRGGBBAA.into()`
    fn from(c: u32) -> Self {
        let a = u8_to_norm(c as u8);
        let b = u8_to_norm((c >> 8) as u8);
        let g = u8_to_norm((c >> 16) as u8);
        let r = u8_to_norm((c >> 24) as u8);
        Color::new(r, g, b, a)
    }
}

impl From<Color> for [u8; 4] {
    /// Converts a Color into an array of 8bit ints: `[0xRR, 0xGG, 0xBB, 0xAA]`
    fn from(c: Color) -> Self {
        [
            norm_to_u8(c.r),
            norm_to_u8(c.g),
            norm_to_u8(c.b),
            norm_to_u8(c.a),
        ]
    }
}

impl From<Color> for [u8; 3] {
    /// 8bit color conversion, with alpha assumed to be `0xFF`. E.g. `[0xRR, 0xGG, 0xBB].into()`
    fn from(c: Color) -> Self {
        [norm_to_u8(c.r), norm_to_u8(c.g), norm_to_u8(c.b)]
    }
}

impl From<Color> for u32 {
    /// Converts a Color into a packed 8-bit RGBA value.
    fn from(c: Color) -> Self {
        let [r, g, b, a]: [u8; 4] = c.into();
        ((r as u32) << 24) + ((g as u32) << 16) + ((b as u32) << 8) + (a as u32)
    }
}

impl From<Color> for [f32; 4] {
    /// Converts a Color into an array of floats `[R, G, B, A]`.
    fn from(c: Color) -> Self {
        unsafe { mem::transmute(c) }
    }
}

impl From<Color> for femtovg::Color {
    fn from(value: Color) -> Self {
        let Color { r, g, b, a } = value;
        femtovg::Color::rgba((r) as u8, (g) as u8, (b) as u8, (a * 255.0) as u8)
    }
}

#[inline]
fn u8_to_norm(x: u8) -> f32 {
    x as f32 / 255.0
}

#[inline]
fn norm_to_u8(x: f32) -> u8 {
    (x * 255.0) as u8
}

/// 8 bit [`Color`] constructor. Useful when defining static colors.
///
/// Has two forms, (R, G, B) and (R, G, B, A), where the former assumes an alpha of `0xFF`.
///
/// E.g.:
/// ```
/// use lemna::{color, Color};
/// pub const DARK_GRAY: Color = color!(0x16, 0x16, 0x16);
/// ```
#[macro_export]
macro_rules! color {
    ($r:expr, $g:expr, $b:expr) => {
        $crate::Color {
            r: $r as f32,
            g: $g as f32,
            b: $b as f32,
            a: 255.0,
        }
    };
    ($r:expr, $g:expr, $b:expr, $a:expr) => {
        $crate::Color {
            r: $r as f32,
            g: $g as f32,
            b: $b as f32,
            a: $a as f32,
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pos_from() {
        assert_eq!(
            Pos::from([1.0, 2.0, 3.0]),
            Pos {
                x: 1.0,
                y: 2.0,
                z: 3.0
            }
        );
    }

    #[test]
    fn test_color_from() {
        // A float that is representable in 8 bits:
        let c: Color = (0.49803921568).into();
        assert_eq!(c, Into::<Color>::into(Into::<u32>::into(c)))
    }
}

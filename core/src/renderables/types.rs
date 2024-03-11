use std::{
    cmp::{self, PartialOrd},
    fmt::{self, Debug},
    ops::{Add, Div, Mul, MulAssign, Sub},
    sync::mpsc::Sender,
};
use euclid::{self};
use femtovg::{self, renderer::OpenGl};
use derive_more::{Add, AddAssign, Div, DivAssign, Mul, Neg, Sub, SubAssign};
use serde::{Deserialize, Serialize};


pub type Canvas = femtovg::Canvas<OpenGl>;
pub type Point<T> = euclid::default::Point2D<T>;
pub type Vector = euclid::default::Vector2D<f32>;
pub type Size<T> = euclid::default::Size2D<T>;
pub type Rect = euclid::default::Rect<f32>;

/// Represents the edges of a box in a 2D space, such as padding or margin.
///
/// Each field represents the size of the edge on one side of the box: `top`, `right`, `bottom`, and `left`.
///
/// # Examples
///
/// ```
/// # use mctk::Edges;
/// let edges = Edges {
///     top: 10.0,
///     right: 20.0,
///     bottom: 30.0,
///     left: 40.0,
/// };
///
/// assert_eq!(edges.top, 10.0);
/// assert_eq!(edges.right, 20.0);
/// assert_eq!(edges.bottom, 30.0);
/// assert_eq!(edges.left, 40.0);
/// ```
#[derive(Clone, Default, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct Edges<T: Clone + Default + Debug> {
    /// The size of the top edge.
    pub top: T,
    /// The size of the right edge.
    pub right: T,
    /// The size of the bottom edge.
    pub bottom: T,
    /// The size of the left edge.
    pub left: T,
}

impl<T> Mul for Edges<T>
where
    T: Mul<Output = T> + Clone + Default + Debug,
{
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            top: self.top.clone() * rhs.top,
            right: self.right.clone() * rhs.right,
            bottom: self.bottom.clone() * rhs.bottom,
            left: self.left.clone() * rhs.left,
        }
    }
}

impl<T, S> MulAssign<S> for Edges<T>
where
    T: Mul<S, Output = T> + Clone + Default + Debug,
    S: Clone,
{
    fn mul_assign(&mut self, rhs: S) {
        self.top = self.top.clone() * rhs.clone();
        self.right = self.right.clone() * rhs.clone();
        self.bottom = self.bottom.clone() * rhs.clone();
        self.left = self.left.clone() * rhs;
    }
}

impl<T: Clone + Default + Debug + Copy> Copy for Edges<T> {}

impl<T: Clone + Default + Debug> Edges<T> {
    /// Constructs `Edges` where all sides are set to the same specified value.
    ///
    /// This function creates an `Edges` instance with the `top`, `right`, `bottom`, and `left` fields all initialized
    /// to the same value provided as an argument. This is useful when you want to have uniform edges around a box,
    /// such as padding or margin with the same size on all sides.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to set for all four sides of the edges.
    ///
    /// # Returns
    ///
    /// An `Edges` instance with all sides set to the given value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use mctk::Edges;
    /// let uniform_edges = Edges::all(10.0);
    /// assert_eq!(uniform_edges.top, 10.0);
    /// assert_eq!(uniform_edges.right, 10.0);
    /// assert_eq!(uniform_edges.bottom, 10.0);
    /// assert_eq!(uniform_edges.left, 10.0);
    /// ```
    pub fn all(value: T) -> Self {
        Self {
            top: value.clone(),
            right: value.clone(),
            bottom: value.clone(),
            left: value,
        }
    }

    /// Applies a function to each field of the `Edges`, producing a new `Edges<U>`.
    ///
    /// This method allows for converting an `Edges<T>` to an `Edges<U>` by specifying a closure
    /// that defines how to convert between the two types. The closure is applied to each field
    /// (`top`, `right`, `bottom`, `left`), resulting in new edges of the desired type.
    ///
    /// # Arguments
    ///
    /// * `f` - A closure that takes a reference to a value of type `T` and returns a value of type `U`.
    ///
    /// # Returns
    ///
    /// Returns a new `Edges<U>` with each field mapped by the provided function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use mctk::Edges;
    /// let edges = Edges { top: 10, right: 20, bottom: 30, left: 40 };
    /// let edges_float = edges.map(|&value| value as f32 * 1.1);
    /// assert_eq!(edges_float, Edges { top: 11.0, right: 22.0, bottom: 33.0, left: 44.0 });
    /// ```
    pub fn map<U>(&self, f: impl Fn(&T) -> U) -> Edges<U>
    where
        U: Clone + Default + Debug,
    {
        Edges {
            top: f(&self.top),
            right: f(&self.right),
            bottom: f(&self.bottom),
            left: f(&self.left),
        }
    }

    /// Checks if any of the edges satisfy a given predicate.
    ///
    /// This method applies a predicate function to each field of the `Edges` and returns `true` if any field satisfies the predicate.
    ///
    /// # Arguments
    ///
    /// * `predicate` - A closure that takes a reference to a value of type `T` and returns a `bool`.
    ///
    /// # Returns
    ///
    /// Returns `true` if the predicate returns `true` for any of the edge values, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// # use mctk::Edges;
    /// let edges = Edges {
    ///     top: 10,
    ///     right: 0,
    ///     bottom: 5,
    ///     left: 0,
    /// };
    ///
    /// assert!(edges.any(|value| *value == 0));
    /// assert!(edges.any(|value| *value > 0));
    /// assert!(!edges.any(|value| *value > 10));
    /// ```
    pub fn any<F: Fn(&T) -> bool>(&self, predicate: F) -> bool {
        predicate(&self.top)
            || predicate(&self.right)
            || predicate(&self.bottom)
            || predicate(&self.left)
    }
}

impl Edges<Length> {
    /// Sets the edges of the `Edges` struct to `auto`, which is a special value that allows the layout engine to automatically determine the size of the edges.
    ///
    /// This is typically used in layout contexts where the exact size of the edges is not important, or when the size should be calculated based on the content or container.
    ///
    /// # Returns
    ///
    /// Returns an `Edges<Length>` with all edges set to `Length::Auto`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use mctk::Edges;
    /// let auto_edges = Edges::auto();
    /// assert_eq!(auto_edges.top, Length::Auto);
    /// assert_eq!(auto_edges.right, Length::Auto);
    /// assert_eq!(auto_edges.bottom, Length::Auto);
    /// assert_eq!(auto_edges.left, Length::Auto);
    /// ```
    pub fn auto() -> Self {
        Self {
            top: Length::Auto,
            right: Length::Auto,
            bottom: Length::Auto,
            left: Length::Auto,
        }
    }

    /// Sets the edges of the `Edges` struct to zero, which means no size or thickness.
    ///
    /// This is typically used when you want to specify that a box (like a padding or margin area)
    /// should have no edges, effectively making it non-existent or invisible in layout calculations.
    ///
    /// # Returns
    ///
    /// Returns an `Edges<Length>` with all edges set to zero length.
    ///
    /// # Examples
    ///
    /// ```
    /// # use mctk::Edges;
    /// let no_edges = Edges::zero();
    /// assert_eq!(no_edges.top, Length::Definite(DefiniteLength::from(Pixels(0.))));
    /// assert_eq!(no_edges.right, Length::Definite(DefiniteLength::from(Pixels(0.))));
    /// assert_eq!(no_edges.bottom, Length::Definite(DefiniteLength::from(Pixels(0.))));
    /// assert_eq!(no_edges.left, Length::Definite(DefiniteLength::from(Pixels(0.))));
    /// ```
    pub fn zero() -> Self {
        Self {
            top: px(0.).into(),
            right: px(0.).into(),
            bottom: px(0.).into(),
            left: px(0.).into(),
        }
    }
}

impl Edges<DefiniteLength> {
    /// Sets the edges of the `Edges` struct to zero, which means no size or thickness.
    ///
    /// This is typically used when you want to specify that a box (like a padding or margin area)
    /// should have no edges, effectively making it non-existent or invisible in layout calculations.
    ///
    /// # Returns
    ///
    /// Returns an `Edges<DefiniteLength>` with all edges set to zero length.
    ///
    /// # Examples
    ///
    /// ```
    /// # use mctk::Edges;
    /// let no_edges = Edges::zero();
    /// assert_eq!(no_edges.top, DefiniteLength::from(mctk::px(0.)));
    /// assert_eq!(no_edges.right, DefiniteLength::from(mctk::px(0.)));
    /// assert_eq!(no_edges.bottom, DefiniteLength::from(mctk::px(0.)));
    /// assert_eq!(no_edges.left, DefiniteLength::from(mctk::px(0.)));
    /// ```
    pub fn zero() -> Self {
        Self {
            top: px(0.).into(),
            right: px(0.).into(),
            bottom: px(0.).into(),
            left: px(0.).into(),
        }
    }

    /// Converts the `DefiniteLength` to `Pixels` based on the parent size and the REM size.
    ///
    /// This method allows for a `DefiniteLength` value to be converted into pixels, taking into account
    /// the size of the parent element (for percentage-based lengths) and the size of a rem unit (for rem-based lengths).
    ///
    /// # Arguments
    ///
    /// * `parent_size` - `Size<AbsoluteLength>` representing the size of the parent element.
    /// * `rem_size` - `Pixels` representing the size of one REM unit.
    ///
    /// # Returns
    ///
    /// Returns an `Edges<Pixels>` representing the edges with lengths converted to pixels.
    ///
    /// # Examples
    ///
    /// ```
    /// # use mctk::{Edges, DefiniteLength, px, AbsoluteLength, Size};
    /// let edges = Edges {
    ///     top: DefiniteLength::Absolute(AbsoluteLength::Pixels(px(10.0))),
    ///     right: DefiniteLength::Fraction(0.5),
    ///     bottom: DefiniteLength::Absolute(AbsoluteLength::Rems(rems(2.0))),
    ///     left: DefiniteLength::Fraction(0.25),
    /// };
    /// let parent_size = Size {
    ///     width: AbsoluteLength::Pixels(px(200.0)),
    ///     height: AbsoluteLength::Pixels(px(100.0)),
    /// };
    /// let rem_size = px(16.0);
    /// let edges_in_pixels = edges.to_pixels(parent_size, rem_size);
    ///
    /// assert_eq!(edges_in_pixels.top, px(10.0)); // Absolute length in pixels
    /// assert_eq!(edges_in_pixels.right, px(100.0)); // 50% of parent width
    /// assert_eq!(edges_in_pixels.bottom, px(32.0)); // 2 rems
    /// assert_eq!(edges_in_pixels.left, px(50.0)); // 25% of parent width
    /// ```
    pub fn to_pixels(&self, parent_size: Size<AbsoluteLength>, rem_size: Pixels) -> Edges<Pixels> {
        Edges {
            top: self.top.to_pixels(parent_size.height, rem_size),
            right: self.right.to_pixels(parent_size.width, rem_size),
            bottom: self.bottom.to_pixels(parent_size.height, rem_size),
            left: self.left.to_pixels(parent_size.width, rem_size),
        }
    }
}

impl Edges<AbsoluteLength> {
    /// Sets the edges of the `Edges` struct to zero, which means no size or thickness.
    ///
    /// This is typically used when you want to specify that a box (like a padding or margin area)
    /// should have no edges, effectively making it non-existent or invisible in layout calculations.
    ///
    /// # Returns
    ///
    /// Returns an `Edges<AbsoluteLength>` with all edges set to zero length.
    ///
    /// # Examples
    ///
    /// ```
    /// # use mctk::Edges;
    /// let no_edges = Edges::zero();
    /// assert_eq!(no_edges.top, AbsoluteLength::Pixels(Pixels(0.0)));
    /// assert_eq!(no_edges.right, AbsoluteLength::Pixels(Pixels(0.0)));
    /// assert_eq!(no_edges.bottom, AbsoluteLength::Pixels(Pixels(0.0)));
    /// assert_eq!(no_edges.left, AbsoluteLength::Pixels(Pixels(0.0)));
    /// ```
    pub fn zero() -> Self {
        Self {
            top: px(0.).into(),
            right: px(0.).into(),
            bottom: px(0.).into(),
            left: px(0.).into(),
        }
    }

    /// Converts the `AbsoluteLength` to `Pixels` based on the `rem_size`.
    ///
    /// If the `AbsoluteLength` is already in pixels, it simply returns the corresponding `Pixels` value.
    /// If the `AbsoluteLength` is in rems, it multiplies the number of rems by the `rem_size` to convert it to pixels.
    ///
    /// # Arguments
    ///
    /// * `rem_size` - The size of one rem unit in pixels.
    ///
    /// # Returns
    ///
    /// Returns an `Edges<Pixels>` representing the edges with lengths converted to pixels.
    ///
    /// # Examples
    ///
    /// ```
    /// # use mctk::{Edges, AbsoluteLength, Pixels, px};
    /// let edges = Edges {
    ///     top: AbsoluteLength::Pixels(px(10.0)),
    ///     right: AbsoluteLength::Rems(rems(1.0)),
    ///     bottom: AbsoluteLength::Pixels(px(20.0)),
    ///     left: AbsoluteLength::Rems(rems(2.0)),
    /// };
    /// let rem_size = px(16.0);
    /// let edges_in_pixels = edges.to_pixels(rem_size);
    ///
    /// assert_eq!(edges_in_pixels.top, px(10.0)); // Already in pixels
    /// assert_eq!(edges_in_pixels.right, px(16.0)); // 1 rem converted to pixels
    /// assert_eq!(edges_in_pixels.bottom, px(20.0)); // Already in pixels
    /// assert_eq!(edges_in_pixels.left, px(32.0)); // 2 rems converted to pixels
    /// ```
    pub fn to_pixels(&self, rem_size: Pixels) -> Edges<Pixels> {
        Edges {
            top: self.top.to_pixels(rem_size),
            right: self.right.to_pixels(rem_size),
            bottom: self.bottom.to_pixels(rem_size),
            left: self.left.to_pixels(rem_size),
        }
    }
}

impl Edges<Pixels> {
    /// Scales the `Edges<Pixels>` by a given factor, returning `Edges<ScaledPixels>`.
    ///
    /// This method is typically used for adjusting the edge sizes for different display densities or scaling factors.
    ///
    /// # Arguments
    ///
    /// * `factor` - The scaling factor to apply to each edge.
    ///
    /// # Returns
    ///
    /// Returns a new `Edges<ScaledPixels>` where each edge is the result of scaling the original edge by the given factor.
    ///
    /// # Examples
    ///
    /// ```
    /// # use mctk::{Edges, Pixels};
    /// let edges = Edges {
    ///     top: Pixels(10.0),
    ///     right: Pixels(20.0),
    ///     bottom: Pixels(30.0),
    ///     left: Pixels(40.0),
    /// };
    /// let scaled_edges = edges.scale(2.0);
    /// assert_eq!(scaled_edges.top, ScaledPixels(20.0));
    /// assert_eq!(scaled_edges.right, ScaledPixels(40.0));
    /// assert_eq!(scaled_edges.bottom, ScaledPixels(60.0));
    /// assert_eq!(scaled_edges.left, ScaledPixels(80.0));
    /// ```
    pub fn scale(&self, factor: f32) -> Edges<ScaledPixels> {
        Edges {
            top: self.top.scale(factor),
            right: self.right.scale(factor),
            bottom: self.bottom.scale(factor),
            left: self.left.scale(factor),
        }
    }

    /// Returns the maximum value of any edge.
    ///
    /// # Returns
    ///
    /// The maximum `Pixels` value among all four edges.
    pub fn max(&self) -> Pixels {
        self.top.max(self.right).max(self.bottom).max(self.left)
    }
}

impl From<f32> for Edges<Pixels> {
    fn from(val: f32) -> Self {
        Edges {
            top: val.into(),
            right: val.into(),
            bottom: val.into(),
            left: val.into(),
        }
    }
}

impl From<f32> for Edges<f32> {
    fn from(val: f32) -> Self {
        Edges {
            top: val,
            right: val,
            bottom: val,
            left: val,
        }
    }
}

// impl Default for Edges<f32> {
//     fn default() -> Self {
//         Self {
//             top: 0.0,
//             right: 0.0,
//             bottom: 0.0,
//             left: 0.0,
//         }
//     }
// }

/// Represents the corners of a box in a 2D space, such as border radius.
///
/// Each field represents the size of the corner on one side of the box: `top_left`, `top_right`, `bottom_right`, and `bottom_left`.
#[derive(Clone, Default, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct Corners<T: Clone + Default + Debug> {
    /// The value associated with the top left corner.
    pub top_left: T,
    /// The value associated with the top right corner.
    pub top_right: T,
    /// The value associated with the bottom right corner.
    pub bottom_right: T,
    /// The value associated with the bottom left corner.
    pub bottom_left: T,
}

impl<T> Corners<T>
where
    T: Clone + Default + Debug,
{
    /// Constructs `Corners` where all sides are set to the same specified value.
    ///
    /// This function creates a `Corners` instance with the `top_left`, `top_right`, `bottom_right`, and `bottom_left` fields all initialized
    /// to the same value provided as an argument. This is useful when you want to have uniform corners around a box,
    /// such as a uniform border radius on a rectangle.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to set for all four corners.
    ///
    /// # Returns
    ///
    /// An `Corners` instance with all corners set to the given value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use mctk::Corners;
    /// let uniform_corners = Corners::all(5.0);
    /// assert_eq!(uniform_corners.top_left, 5.0);
    /// assert_eq!(uniform_corners.top_right, 5.0);
    /// assert_eq!(uniform_corners.bottom_right, 5.0);
    /// assert_eq!(uniform_corners.bottom_left, 5.0);
    /// ```
    pub fn all(value: T) -> Self {
        Self {
            top_left: value.clone(),
            top_right: value.clone(),
            bottom_right: value.clone(),
            bottom_left: value,
        }
    }
}

impl Corners<AbsoluteLength> {
    /// Converts the `AbsoluteLength` to `Pixels` based on the provided size and rem size, ensuring the resulting
    /// `Pixels` do not exceed half of the maximum of the provided size's width and height.
    ///
    /// This method is particularly useful when dealing with corner radii, where the radius in pixels should not
    /// exceed half the size of the box it applies to, to avoid the corners overlapping.
    ///
    /// # Arguments
    ///
    /// * `size` - The `Size<Pixels>` against which the maximum allowable radius is determined.
    /// * `rem_size` - The size of one REM unit in pixels, used for conversion if the `AbsoluteLength` is in REMs.
    ///
    /// # Returns
    ///
    /// Returns a `Corners<Pixels>` instance with each corner's length converted to pixels and clamped to the
    /// maximum allowable radius based on the provided size.
    ///
    /// # Examples
    ///
    /// ```
    /// # use mctk::{Corners, AbsoluteLength, Pixels, Size};
    /// let corners = Corners {
    ///     top_left: AbsoluteLength::Pixels(Pixels(15.0)),
    ///     top_right: AbsoluteLength::Rems(Rems(1.0)),
    ///     bottom_right: AbsoluteLength::Pixels(Pixels(20.0)),
    ///     bottom_left: AbsoluteLength::Rems(Rems(2.0)),
    /// };
    /// let size = Size { width: Pixels(100.0), height: Pixels(50.0) };
    /// let rem_size = Pixels(16.0);
    /// let corners_in_pixels = corners.to_pixels(size, rem_size);
    ///
    /// // The resulting corners should not exceed half the size of the smallest dimension (50.0 / 2.0 = 25.0).
    /// assert_eq!(corners_in_pixels.top_left, Pixels(15.0));
    /// assert_eq!(corners_in_pixels.top_right, Pixels(16.0)); // 1 rem converted to pixels
    /// assert_eq!(corners_in_pixels.bottom_right, Pixels(20.0).min(Pixels(25.0))); // Clamped to 25.0
    /// assert_eq!(corners_in_pixels.bottom_left, Pixels(32.0).min(Pixels(25.0))); // 2 rems converted to pixels and clamped
    /// ```
    pub fn to_pixels(&self, size: Size<Pixels>, rem_size: Pixels) -> Corners<Pixels> {
        let max = size.width.max(size.height) / 2.;
        Corners {
            top_left: self.top_left.to_pixels(rem_size).min(max),
            top_right: self.top_right.to_pixels(rem_size).min(max),
            bottom_right: self.bottom_right.to_pixels(rem_size).min(max),
            bottom_left: self.bottom_left.to_pixels(rem_size).min(max),
        }
    }
}

impl Corners<Pixels> {
    /// Scales the `Corners<Pixels>` by a given factor, returning `Corners<ScaledPixels>`.
    ///
    /// This method is typically used for adjusting the corner sizes for different display densities or scaling factors.
    ///
    /// # Arguments
    ///
    /// * `factor` - The scaling factor to apply to each corner.
    ///
    /// # Returns
    ///
    /// Returns a new `Corners<ScaledPixels>` where each corner is the result of scaling the original corner by the given factor.
    ///
    /// # Examples
    ///
    /// ```
    /// # use mctk::{Corners, Pixels};
    /// let corners = Corners {
    ///     top_left: Pixels(10.0),
    ///     top_right: Pixels(20.0),
    ///     bottom_right: Pixels(30.0),
    ///     bottom_left: Pixels(40.0),
    /// };
    /// let scaled_corners = corners.scale(2.0);
    /// assert_eq!(scaled_corners.top_left, ScaledPixels(20.0));
    /// assert_eq!(scaled_corners.top_right, ScaledPixels(40.0));
    /// assert_eq!(scaled_corners.bottom_right, ScaledPixels(60.0));
    /// assert_eq!(scaled_corners.bottom_left, ScaledPixels(80.0));
    /// ```
    pub fn scale(&self, factor: f32) -> Corners<ScaledPixels> {
        Corners {
            top_left: self.top_left.scale(factor),
            top_right: self.top_right.scale(factor),
            bottom_right: self.bottom_right.scale(factor),
            bottom_left: self.bottom_left.scale(factor),
        }
    }

    /// Returns the maximum value of any corner.
    ///
    /// # Returns
    ///
    /// The maximum `Pixels` value among all four corners.
    pub fn max(&self) -> Pixels {
        self.top_left
            .max(self.top_right)
            .max(self.bottom_right)
            .max(self.bottom_left)
    }
}

impl<T: Clone + Default + Debug> Corners<T> {
    /// Applies a function to each field of the `Corners`, producing a new `Corners<U>`.
    ///
    /// This method allows for converting a `Corners<T>` to a `Corners<U>` by specifying a closure
    /// that defines how to convert between the two types. The closure is applied to each field
    /// (`top_left`, `top_right`, `bottom_right`, `bottom_left`), resulting in new corners of the desired type.
    ///
    /// # Arguments
    ///
    /// * `f` - A closure that takes a reference to a value of type `T` and returns a value of type `U`.
    ///
    /// # Returns
    ///
    /// Returns a new `Corners<U>` with each field mapped by the provided function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use mctk::{Corners, Pixels};
    /// let corners = Corners {
    ///     top_left: Pixels(10.0),
    ///     top_right: Pixels(20.0),
    ///     bottom_right: Pixels(30.0),
    ///     bottom_left: Pixels(40.0),
    /// };
    /// let corners_in_rems = corners.map(|&px| Rems(px.0 / 16.0));
    /// assert_eq!(corners_in_rems, Corners {
    ///     top_left: Rems(0.625),
    ///     top_right: Rems(1.25),
    ///     bottom_right: Rems(1.875),
    ///     bottom_left: Rems(2.5),
    /// });
    /// ```
    pub fn map<U>(&self, f: impl Fn(&T) -> U) -> Corners<U>
    where
        U: Clone + Default + Debug,
    {
        Corners {
            top_left: f(&self.top_left),
            top_right: f(&self.top_right),
            bottom_right: f(&self.bottom_right),
            bottom_left: f(&self.bottom_left),
        }
    }
}

impl<T> Mul for Corners<T>
where
    T: Mul<Output = T> + Clone + Default + Debug,
{
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            top_left: self.top_left.clone() * rhs.top_left,
            top_right: self.top_right.clone() * rhs.top_right,
            bottom_right: self.bottom_right.clone() * rhs.bottom_right,
            bottom_left: self.bottom_left.clone() * rhs.bottom_left,
        }
    }
}

impl<T, S> MulAssign<S> for Corners<T>
where
    T: Mul<S, Output = T> + Clone + Default + Debug,
    S: Clone,
{
    fn mul_assign(&mut self, rhs: S) {
        self.top_left = self.top_left.clone() * rhs.clone();
        self.top_right = self.top_right.clone() * rhs.clone();
        self.bottom_right = self.bottom_right.clone() * rhs.clone();
        self.bottom_left = self.bottom_left.clone() * rhs;
    }
}

impl<T> Copy for Corners<T> where T: Copy + Clone + Default + Debug {}

impl From<f32> for Corners<Pixels> {
    fn from(val: f32) -> Self {
        Corners {
            top_left: val.into(),
            top_right: val.into(),
            bottom_right: val.into(),
            bottom_left: val.into(),
        }
    }
}

impl From<Pixels> for Corners<Pixels> {
    fn from(val: Pixels) -> Self {
        Corners {
            top_left: val,
            top_right: val,
            bottom_right: val,
            bottom_left: val,
        }
    }
}

impl From<f32> for Corners<f32> {
    fn from(val: f32) -> Self {
        Corners {
            top_left: val,
            top_right: val,
            bottom_left: val,
            bottom_right: val,
        }
    }
}

/// Represents a length in pixels, the base unit of measurement in the UI framework.
///
/// `Pixels` is a value type that represents an absolute length in pixels, which is used
/// for specifying sizes, positions, and distances in the UI. It is the fundamental unit
/// of measurement for all visual elements and layout calculations.
///
/// The inner value is an `f32`, allowing for sub-pixel precision which can be useful for
/// anti-aliasing and animations. However, when applied to actual pixel grids, the value
/// is typically rounded to the nearest integer.
///
/// # Examples
///
/// ```
/// use mctk::Pixels;
///
/// // Define a length of 10 pixels
/// let length = Pixels(10.0);
///
/// // Define a length and scale it by a factor of 2
/// let scaled_length = length.scale(2.0);
/// assert_eq!(scaled_length, Pixels(20.0));
/// ```
#[derive(
    Clone,
    Copy,
    Default,
    Add,
    AddAssign,
    Sub,
    SubAssign,
    Neg,
    Div,
    DivAssign,
    PartialEq,
    Serialize,
    Deserialize,
)]
#[repr(transparent)]
pub struct Pixels(pub f32);

impl std::ops::Div for Pixels {
    type Output = f32;

    fn div(self, rhs: Self) -> Self::Output {
        self.0 / rhs.0
    }
}

impl std::ops::DivAssign for Pixels {
    fn div_assign(&mut self, rhs: Self) {
        *self = Self(self.0 / rhs.0);
    }
}

impl std::ops::RemAssign for Pixels {
    fn rem_assign(&mut self, rhs: Self) {
        self.0 %= rhs.0;
    }
}

impl std::ops::Rem for Pixels {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self {
        Self(self.0 % rhs.0)
    }
}

impl Mul<f32> for Pixels {
    type Output = Pixels;

    fn mul(self, other: f32) -> Pixels {
        Pixels(self.0 * other)
    }
}

impl Mul<usize> for Pixels {
    type Output = Pixels;

    fn mul(self, other: usize) -> Pixels {
        Pixels(self.0 * other as f32)
    }
}

impl Mul<Pixels> for f32 {
    type Output = Pixels;

    fn mul(self, rhs: Pixels) -> Self::Output {
        Pixels(self * rhs.0)
    }
}

impl MulAssign<f32> for Pixels {
    fn mul_assign(&mut self, other: f32) {
        self.0 *= other;
    }
}

impl Pixels {
    /// Represents zero pixels.
    pub const ZERO: Pixels = Pixels(0.0);
    /// The maximum value that can be represented by `Pixels`.
    pub const MAX: Pixels = Pixels(f32::MAX);

    /// Floors the `Pixels` value to the nearest whole number.
    ///
    /// # Returns
    ///
    /// Returns a new `Pixels` instance with the floored value.
    pub fn floor(&self) -> Self {
        Self(self.0.floor())
    }

    /// Rounds the `Pixels` value to the nearest whole number.
    ///
    /// # Returns
    ///
    /// Returns a new `Pixels` instance with the rounded value.
    pub fn round(&self) -> Self {
        Self(self.0.round())
    }

    /// Returns the ceiling of the `Pixels` value to the nearest whole number.
    ///
    /// # Returns
    ///
    /// Returns a new `Pixels` instance with the ceiling value.
    pub fn ceil(&self) -> Self {
        Self(self.0.ceil())
    }

    /// Scales the `Pixels` value by a given factor, producing `ScaledPixels`.
    ///
    /// This method is used when adjusting pixel values for display scaling factors,
    /// such as high DPI (dots per inch) or Retina displays, where the pixel density is higher and
    /// thus requires scaling to maintain visual consistency and readability.
    ///
    /// The resulting `ScaledPixels` represent the scaled value which can be used for rendering
    /// calculations where display scaling is considered.
    pub fn scale(&self, factor: f32) -> ScaledPixels {
        ScaledPixels(self.0 * factor)
    }

    /// Raises the `Pixels` value to a given power.
    ///
    /// # Arguments
    ///
    /// * `exponent` - The exponent to raise the `Pixels` value by.
    ///
    /// # Returns
    ///
    /// Returns a new `Pixels` instance with the value raised to the given exponent.
    pub fn pow(&self, exponent: f32) -> Self {
        Self(self.0.powf(exponent))
    }

    /// Returns the absolute value of the `Pixels`.
    ///
    /// # Returns
    ///
    /// A new `Pixels` instance with the absolute value of the original `Pixels`.
    pub fn abs(&self) -> Self {
        Self(self.0.abs())
    }
}

impl Mul<Pixels> for Pixels {
    type Output = Pixels;

    fn mul(self, rhs: Pixels) -> Self::Output {
        Pixels(self.0 * rhs.0)
    }
}

impl Eq for Pixels {}

impl PartialOrd for Pixels {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Pixels {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl std::hash::Hash for Pixels {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl From<f64> for Pixels {
    fn from(pixels: f64) -> Self {
        Pixels(pixels as f32)
    }
}

impl From<f32> for Pixels {
    fn from(pixels: f32) -> Self {
        Pixels(pixels)
    }
}

impl Debug for Pixels {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} px", self.0)
    }
}

impl From<Pixels> for f32 {
    fn from(pixels: Pixels) -> Self {
        pixels.0
    }
}

impl From<&Pixels> for f32 {
    fn from(pixels: &Pixels) -> Self {
        pixels.0
    }
}

impl From<Pixels> for f64 {
    fn from(pixels: Pixels) -> Self {
        pixels.0 as f64
    }
}

impl From<Pixels> for u32 {
    fn from(pixels: Pixels) -> Self {
        pixels.0 as u32
    }
}

impl From<u32> for Pixels {
    fn from(pixels: u32) -> Self {
        Pixels(pixels as f32)
    }
}

impl From<Pixels> for usize {
    fn from(pixels: Pixels) -> Self {
        pixels.0 as usize
    }
}

impl From<usize> for Pixels {
    fn from(pixels: usize) -> Self {
        Pixels(pixels as f32)
    }
}

/// Represents physical pixels on the display.
///
/// `DevicePixels` is a unit of measurement that refers to the actual pixels on a device's screen.
/// This type is used when precise pixel manipulation is required, such as rendering graphics or
/// interfacing with hardware that operates on the pixel level. Unlike logical pixels that may be
/// affected by the device's scale factor, `DevicePixels` always correspond to real pixels on the
/// display.
#[derive(
    Add, AddAssign, Clone, Copy, Default, Div, Eq, Hash, Ord, PartialEq, PartialOrd, Sub, SubAssign,
)]
#[repr(transparent)]
pub struct DevicePixels(pub(crate) i32);

impl DevicePixels {
    /// Converts the `DevicePixels` value to the number of bytes needed to represent it in memory.
    ///
    /// This function is useful when working with graphical data that needs to be stored in a buffer,
    /// such as images or framebuffers, where each pixel may be represented by a specific number of bytes.
    ///
    /// # Arguments
    ///
    /// * `bytes_per_pixel` - The number of bytes used to represent a single pixel.
    ///
    /// # Returns
    ///
    /// The number of bytes required to represent the `DevicePixels` value in memory.
    ///
    /// # Examples
    ///
    /// ```
    /// # use mctk::DevicePixels;
    /// let pixels = DevicePixels(10); // 10 device pixels
    /// let bytes_per_pixel = 4; // Assume each pixel is represented by 4 bytes (e.g., RGBA)
    /// let total_bytes = pixels.to_bytes(bytes_per_pixel);
    /// assert_eq!(total_bytes, 40); // 10 pixels * 4 bytes/pixel = 40 bytes
    /// ```
    pub fn to_bytes(&self, bytes_per_pixel: u8) -> u32 {
        self.0 as u32 * bytes_per_pixel as u32
    }
}

impl fmt::Debug for DevicePixels {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} px (device)", self.0)
    }
}

impl From<DevicePixels> for i32 {
    fn from(device_pixels: DevicePixels) -> Self {
        device_pixels.0
    }
}

impl From<i32> for DevicePixels {
    fn from(device_pixels: i32) -> Self {
        DevicePixels(device_pixels)
    }
}

impl From<u32> for DevicePixels {
    fn from(device_pixels: u32) -> Self {
        DevicePixels(device_pixels as i32)
    }
}

impl From<DevicePixels> for u32 {
    fn from(device_pixels: DevicePixels) -> Self {
        device_pixels.0 as u32
    }
}

impl From<DevicePixels> for u64 {
    fn from(device_pixels: DevicePixels) -> Self {
        device_pixels.0 as u64
    }
}

impl From<u64> for DevicePixels {
    fn from(device_pixels: u64) -> Self {
        DevicePixels(device_pixels as i32)
    }
}

impl From<DevicePixels> for usize {
    fn from(device_pixels: DevicePixels) -> Self {
        device_pixels.0 as usize
    }
}

impl From<usize> for DevicePixels {
    fn from(device_pixels: usize) -> Self {
        DevicePixels(device_pixels as i32)
    }
}

/// Represents scaled pixels that take into account the device's scale factor.
///
/// `ScaledPixels` are used to ensure that UI elements appear at the correct size on devices
/// with different pixel densities. When a device has a higher scale factor (such as Retina displays),
/// a single logical pixel may correspond to multiple physical pixels. By using `ScaledPixels`,
/// dimensions and positions can be specified in a way that scales appropriately across different
/// display resolutions.
#[derive(Clone, Copy, Default, Add, AddAssign, Sub, SubAssign, Div, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ScaledPixels(pub(crate) f32);

impl ScaledPixels {
    /// Floors the `ScaledPixels` value to the nearest whole number.
    ///
    /// # Returns
    ///
    /// Returns a new `ScaledPixels` instance with the floored value.
    pub fn floor(&self) -> Self {
        Self(self.0.floor())
    }

    /// Rounds the `ScaledPixels` value to the nearest whole number.
    ///
    /// # Returns
    ///
    /// Returns a new `ScaledPixels` instance with the rounded value.
    pub fn ceil(&self) -> Self {
        Self(self.0.ceil())
    }
}

impl Eq for ScaledPixels {}

impl Debug for ScaledPixels {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} px (scaled)", self.0)
    }
}

impl From<ScaledPixels> for DevicePixels {
    fn from(scaled: ScaledPixels) -> Self {
        DevicePixels(scaled.0.ceil() as i32)
    }
}

impl From<DevicePixels> for ScaledPixels {
    fn from(device: DevicePixels) -> Self {
        ScaledPixels(device.0 as f32)
    }
}

impl From<ScaledPixels> for f64 {
    fn from(scaled_pixels: ScaledPixels) -> Self {
        scaled_pixels.0 as f64
    }
}

/// Represents pixels in a global coordinate space, which can span across multiple displays.
///
/// `GlobalPixels` is used when dealing with a coordinate system that is not limited to a single
/// display's boundaries. This type is particularly useful in multi-monitor setups where
/// positioning and measurements need to be consistent and relative to a "global" origin point
/// rather than being relative to any individual display.
#[derive(Clone, Copy, Default, Add, AddAssign, Sub, SubAssign, Div, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct GlobalPixels(pub(crate) f32);

impl Debug for GlobalPixels {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} px (global coordinate space)", self.0)
    }
}

impl From<GlobalPixels> for f64 {
    fn from(global_pixels: GlobalPixels) -> Self {
        global_pixels.0 as f64
    }
}

impl From<f64> for GlobalPixels {
    fn from(global_pixels: f64) -> Self {
        GlobalPixels(global_pixels as f32)
    }
}

/// Represents a length in rems, a unit based on the font-size of the window, which can be assigned with [`WindowContext::set_rem_size`][set_rem_size].
///
/// Rems are used for defining lengths that are scalable and consistent across different UI elements.
/// The value of `1rem` is typically equal to the font-size of the root element (often the `<html>` element in browsers),
/// making it a flexible unit that adapts to the user's text size preferences. In this framework, `rems` serve a similar
/// purpose, allowing for scalable and accessible design that can adjust to different display settings or user preferences.
///
/// For example, if the root element's font-size is `16px`, then `1rem` equals `16px`. A length of `2rems` would then be `32px`.
///
/// [set_rem_size]: crate::WindowContext::set_rem_size
#[derive(Clone, Copy, Default, Add, Sub, Mul, Div, Neg, PartialEq)]
pub struct Rems(pub f32);

impl Mul<Pixels> for Rems {
    type Output = Pixels;

    fn mul(self, other: Pixels) -> Pixels {
        Pixels(self.0 * other.0)
    }
}

impl Debug for Rems {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} rem", self.0)
    }
}

/// Represents an absolute length in pixels or rems.
///
/// `AbsoluteLength` can be either a fixed number of pixels, which is an absolute measurement not
/// affected by the current font size, or a number of rems, which is relative to the font size of
/// the root element. It is used for specifying dimensions that are either independent of or
/// related to the typographic scale.

#[derive(Clone, Copy, Debug, Neg, PartialEq)]
pub enum AbsoluteLength {
    /// A length in pixels.
    Pixels(Pixels),
    /// A length in rems.
    Rems(Rems),
}

impl AbsoluteLength {
    /// Checks if the absolute length is zero.
    pub fn is_zero(&self) -> bool {
        match self {
            AbsoluteLength::Pixels(px) => px.0 == 0.0,
            AbsoluteLength::Rems(rems) => rems.0 == 0.0,
        }
    }
}

impl From<Pixels> for AbsoluteLength {
    fn from(pixels: Pixels) -> Self {
        AbsoluteLength::Pixels(pixels)
    }
}

impl From<Rems> for AbsoluteLength {
    fn from(rems: Rems) -> Self {
        AbsoluteLength::Rems(rems)
    }
}

impl AbsoluteLength {
    /// Converts an `AbsoluteLength` to `Pixels` based on a given `rem_size`.
    ///
    /// # Arguments
    ///
    /// * `rem_size` - The size of one rem in pixels.
    ///
    /// # Returns
    ///
    /// Returns the `AbsoluteLength` as `Pixels`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use mctk::{AbsoluteLength, Pixels};
    /// let length_in_pixels = AbsoluteLength::Pixels(Pixels(42.0));
    /// let length_in_rems = AbsoluteLength::Rems(Rems(2.0));
    /// let rem_size = Pixels(16.0);
    ///
    /// assert_eq!(length_in_pixels.to_pixels(rem_size), Pixels(42.0));
    /// assert_eq!(length_in_rems.to_pixels(rem_size), Pixels(32.0));
    /// ```
    pub fn to_pixels(&self, rem_size: Pixels) -> Pixels {
        match self {
            AbsoluteLength::Pixels(pixels) => *pixels,
            AbsoluteLength::Rems(rems) => *rems * rem_size,
        }
    }
}

impl Default for AbsoluteLength {
    fn default() -> Self {
        px(0.).into()
    }
}

/// A non-auto length that can be defined in pixels, rems, or percent of parent.
///
/// This enum represents lengths that have a specific value, as opposed to lengths that are automatically
/// determined by the context. It includes absolute lengths in pixels or rems, and relative lengths as a
/// fraction of the parent's size.
#[derive(Clone, Copy, Neg, PartialEq)]
pub enum DefiniteLength {
    /// An absolute length specified in pixels or rems.
    Absolute(AbsoluteLength),
    /// A relative length specified as a fraction of the parent's size, between 0 and 1.
    Fraction(f32),
}

impl DefiniteLength {
    /// Converts the `DefiniteLength` to `Pixels` based on a given `base_size` and `rem_size`.
    ///
    /// If the `DefiniteLength` is an absolute length, it will be directly converted to `Pixels`.
    /// If it is a fraction, the fraction will be multiplied by the `base_size` to get the length in pixels.
    ///
    /// # Arguments
    ///
    /// * `base_size` - The base size in `AbsoluteLength` to which the fraction will be applied.
    /// * `rem_size` - The size of one rem in pixels, used to convert rems to pixels.
    ///
    /// # Returns
    ///
    /// Returns the `DefiniteLength` as `Pixels`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use mctk::{DefiniteLength, AbsoluteLength, Pixels, px, rems};
    /// let length_in_pixels = DefiniteLength::Absolute(AbsoluteLength::Pixels(px(42.0)));
    /// let length_in_rems = DefiniteLength::Absolute(AbsoluteLength::Rems(rems(2.0)));
    /// let length_as_fraction = DefiniteLength::Fraction(0.5);
    /// let base_size = AbsoluteLength::Pixels(px(100.0));
    /// let rem_size = px(16.0);
    ///
    /// assert_eq!(length_in_pixels.to_pixels(base_size, rem_size), Pixels(42.0));
    /// assert_eq!(length_in_rems.to_pixels(base_size, rem_size), Pixels(32.0));
    /// assert_eq!(length_as_fraction.to_pixels(base_size, rem_size), Pixels(50.0));
    /// ```
    pub fn to_pixels(&self, base_size: AbsoluteLength, rem_size: Pixels) -> Pixels {
        match self {
            DefiniteLength::Absolute(size) => size.to_pixels(rem_size),
            DefiniteLength::Fraction(fraction) => match base_size {
                AbsoluteLength::Pixels(px) => px * *fraction,
                AbsoluteLength::Rems(rems) => rems * rem_size * *fraction,
            },
        }
    }

    pub fn to_definite_pixels(&self) -> Pixels {
        match self {
            DefiniteLength::Absolute(size) => size.to_pixels(16.0.into()),
            DefiniteLength::Fraction(fraction) => 0.0.into(),
        }
    }
}

impl Debug for DefiniteLength {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DefiniteLength::Absolute(length) => Debug::fmt(length, f),
            DefiniteLength::Fraction(fract) => write!(f, "{}%", (fract * 100.0) as i32),
        }
    }
}

impl From<Pixels> for DefiniteLength {
    fn from(pixels: Pixels) -> Self {
        Self::Absolute(pixels.into())
    }
}

impl From<Rems> for DefiniteLength {
    fn from(rems: Rems) -> Self {
        Self::Absolute(rems.into())
    }
}

impl From<AbsoluteLength> for DefiniteLength {
    fn from(length: AbsoluteLength) -> Self {
        Self::Absolute(length)
    }
}

impl Default for DefiniteLength {
    fn default() -> Self {
        Self::Absolute(AbsoluteLength::default())
    }
}

/// A length that can be defined in pixels, rems, percent of parent, or auto.
#[derive(Clone, Copy)]
pub enum Length {
    /// A definite length specified either in pixels, rems, or as a fraction of the parent's size.
    Definite(DefiniteLength),
    /// An automatic length that is determined by the context in which it is used.
    Auto,
}

impl Debug for Length {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Length::Definite(definite_length) => write!(f, "{:?}", definite_length),
            Length::Auto => write!(f, "auto"),
        }
    }
}

/// Constructs a `DefiniteLength` representing a relative fraction of a parent size.
///
/// This function creates a `DefiniteLength` that is a specified fraction of a parent's dimension.
/// The fraction should be a floating-point number between 0.0 and 1.0, where 1.0 represents 100% of the parent's size.
///
/// # Arguments
///
/// * `fraction` - The fraction of the parent's size, between 0.0 and 1.0.
///
/// # Returns
///
/// A `DefiniteLength` representing the relative length as a fraction of the parent's size.
pub fn relative(fraction: f32) -> DefiniteLength {
    DefiniteLength::Fraction(fraction)
}

/// Returns the Golden Ratio, i.e. `~(1.0 + sqrt(5.0)) / 2.0`.
pub fn phi() -> DefiniteLength {
    relative(1.618_034)
}

/// Constructs a `Rems` value representing a length in rems.
///
/// # Arguments
///
/// * `rems` - The number of rems for the length.
///
/// # Returns
///
/// A `Rems` representing the specified number of rems.
pub fn rems(rems: f32) -> Rems {
    Rems(rems)
}

/// Constructs a `Pixels` value representing a length in pixels.
///
/// # Arguments
///
/// * `pixels` - The number of pixels for the length.
///
/// # Returns
///
/// A `Pixels` representing the specified number of pixels.
pub const fn px(pixels: f32) -> Pixels {
    Pixels(pixels)
}

/// Returns a `Length` representing an automatic length.
///
/// The `auto` length is often used in layout calculations where the length should be determined
/// by the layout context itself rather than being explicitly set. This is commonly used in CSS
/// for properties like `width`, `height`, `margin`, `padding`, etc., where `auto` can be used
/// to instruct the layout engine to calculate the size based on other factors like the size of the
/// container or the intrinsic size of the content.
///
/// # Returns
///
/// A `Length` variant set to `Auto`.
pub fn auto() -> Length {
    Length::Auto
}

impl From<Pixels> for Length {
    fn from(pixels: Pixels) -> Self {
        Self::Definite(pixels.into())
    }
}

impl From<Rems> for Length {
    fn from(rems: Rems) -> Self {
        Self::Definite(rems.into())
    }
}

impl From<DefiniteLength> for Length {
    fn from(length: DefiniteLength) -> Self {
        Self::Definite(length)
    }
}

impl From<AbsoluteLength> for Length {
    fn from(length: AbsoluteLength) -> Self {
        Self::Definite(length.into())
    }
}

impl Default for Length {
    fn default() -> Self {
        Self::Definite(DefiniteLength::default())
    }
}

impl From<()> for Length {
    fn from(_: ()) -> Self {
        Self::Definite(DefiniteLength::default())
    }
}

/// Provides a trait for types that can calculate half of their value.
///
/// The `Half` trait is used for types that can be evenly divided, returning a new instance of the same type
/// representing half of the original value. This is commonly used for types that represent measurements or sizes,
/// such as lengths or pixels, where halving is a frequent operation during layout calculations or animations.
pub trait Half {
    /// Returns half of the current value.
    ///
    /// # Returns
    ///
    /// A new instance of the implementing type, representing half of the original value.
    fn half(&self) -> Self;
}

impl Half for f32 {
    fn half(&self) -> Self {
        self / 2.
    }
}

impl Half for DevicePixels {
    fn half(&self) -> Self {
        Self(self.0 / 2)
    }
}

impl Half for ScaledPixels {
    fn half(&self) -> Self {
        Self(self.0 / 2.)
    }
}

impl Half for Pixels {
    fn half(&self) -> Self {
        Self(self.0 / 2.)
    }
}

impl Half for Rems {
    fn half(&self) -> Self {
        Self(self.0 / 2.)
    }
}

impl Half for GlobalPixels {
    fn half(&self) -> Self {
        Self(self.0 / 2.)
    }
}

/// A trait for checking if a value is zero.
///
/// This trait provides a method to determine if a value is considered to be zero.
/// It is implemented for various numeric and length-related types where the concept
/// of zero is applicable. This can be useful for comparisons, optimizations, or
/// determining if an operation has a neutral effect.
pub trait IsZero {
    /// Determines if the value is zero.
    ///
    /// # Returns
    ///
    /// Returns `true` if the value is zero, `false` otherwise.
    fn is_zero(&self) -> bool;
}

impl IsZero for DevicePixels {
    fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

impl IsZero for ScaledPixels {
    fn is_zero(&self) -> bool {
        self.0 == 0.
    }
}

impl IsZero for Pixels {
    fn is_zero(&self) -> bool {
        self.0 == 0.
    }
}

impl IsZero for Rems {
    fn is_zero(&self) -> bool {
        self.0 == 0.
    }
}

impl IsZero for AbsoluteLength {
    fn is_zero(&self) -> bool {
        match self {
            AbsoluteLength::Pixels(pixels) => pixels.is_zero(),
            AbsoluteLength::Rems(rems) => rems.is_zero(),
        }
    }
}

impl IsZero for DefiniteLength {
    fn is_zero(&self) -> bool {
        match self {
            DefiniteLength::Absolute(length) => length.is_zero(),
            DefiniteLength::Fraction(fraction) => *fraction == 0.,
        }
    }
}

impl IsZero for Length {
    fn is_zero(&self) -> bool {
        match self {
            Length::Definite(length) => length.is_zero(),
            Length::Auto => false,
        }
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct Margin {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}
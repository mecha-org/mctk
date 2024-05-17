//! [Flexbox](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_flexible_box_layout/Basic_concepts_of_flexbox)-like layout resolution.
//! All [`Nodes`](crate::Node) have a [`Layout`] attached, and this module is responsible for assigning a [`LayoutResult`] -- an absolution position and size --
//! to the Node, during the draw phase. All [`Layout`] creation functionality -- and thus the entire user-facing interface -- is exposed through the less-verbose [`lay!`][crate::lay] macro.
//!
use std::ops::{Add, AddAssign, Div, DivAssign, Sub, SubAssign};
// use mctk_core::size;

#[derive(Clone, Copy, Debug, Default)]
pub struct ScrollPosition {
    pub x: Option<f32>,
    pub y: Option<f32>,
}

impl Div<f32> for ScrollPosition {
    type Output = Self;
    fn div(self, f: f32) -> Self {
        Self {
            x: self.x.map(|x| x / f),
            y: self.y.map(|y| y / f),
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum Dimension {
    Auto,
    Px(f64),
    Pct(f64),
}

impl std::fmt::Debug for Dimension {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Auto => write!(f, "Auto"),
            Self::Px(x) => write!(f, "{} px", x),
            Self::Pct(x) => write!(f, "{} %", x),
        }
    }
}

impl Default for Dimension {
    fn default() -> Self {
        Self::Auto
    }
}

impl Dimension {
    /// Between two dimensions, return the most specific value
    fn most_specific(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::Auto, _) => *other,
            (_, Self::Auto) => *self,
            (Self::Px(_), _) => *self,
            (_, Self::Px(_)) => *other,
            _ => *self,
        }
    }

    /// Between two dimensions, return the value of the second if the first is Auto, otherwise return the first value
    fn more_specific(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::Auto, _) => *other,
            _ => *self,
        }
    }

    pub fn resolved(&self) -> bool {
        matches!(self, Self::Px(_))
    }

    fn maybe_resolve(&self, relative_to: &Self) -> Self {
        match self {
            Dimension::Px(px) => Dimension::Px(*px),
            Dimension::Pct(pct) => {
                if let Dimension::Px(px) = relative_to {
                    Dimension::Px(px * pct / 100.0)
                } else {
                    Dimension::Pct(*pct)
                }
            }
            Dimension::Auto => Dimension::Auto,
        }
    }

    fn max(&self, other: Self) -> Self {
        match (self, other) {
            (Self::Px(a), Self::Px(b)) => Self::Px(a.max(b)),
            (Self::Px(a), _) => Self::Px(*a),
            (_, Self::Px(b)) => Self::Px(b),
            _ => Dimension::Auto,
        }
    }

    fn maybe_px(&self) -> Option<f32> {
        match self {
            Self::Px(x) => Some(*x as f32),
            _ => None,
        }
    }

    fn is_pct(&self) -> bool {
        matches!(self, Self::Pct(_))
    }
}

impl Sub for Dimension {
    type Output = Dimension;

    fn sub(self, other: Self) -> Self {
        match (self, other) {
            (Self::Px(a), Self::Px(b)) => Self::Px(a - b),
            (Self::Pct(a), Self::Pct(b)) => Self::Pct(a - b),
            (s, _) => s,
        }
    }
}

impl SubAssign for Dimension {
    fn sub_assign(&mut self, other: Self) {
        let val = match (*self, other) {
            (Self::Px(a), Self::Px(b)) => Self::Px(a - b),
            (Self::Pct(a), Self::Pct(b)) => Self::Pct(a - b),
            (s, _) => s,
        };
        *self = val;
    }
}

impl Add for Dimension {
    type Output = Dimension;

    fn add(self, other: Self) -> Self {
        match (self, other) {
            (Self::Px(a), Self::Px(b)) => Self::Px(a + b),
            (Self::Pct(a), Self::Pct(b)) => Self::Pct(a + b),
            (s, _) => s,
        }
    }
}

impl AddAssign for Dimension {
    fn add_assign(&mut self, other: Self) {
        let val = match (*self, other) {
            (Self::Px(a), Self::Px(b)) => Self::Px(a + b),
            (Self::Pct(a), Self::Pct(b)) => Self::Pct(a + b),
            (s, _) => s,
        };
        *self = val;
    }
}

impl DivAssign<f64> for Dimension {
    fn div_assign(&mut self, b: f64) {
        let val = match *self {
            Self::Px(a) => Self::Px(a / b),
            Self::Pct(a) => Self::Pct(a / b),
            s => s,
        };
        *self = val;
    }
}

impl From<Dimension> for f32 {
    fn from(d: Dimension) -> Self {
        match d {
            Dimension::Px(p) => p as f32,
            _ => 0.0,
        }
    }
}
impl From<Dimension> for f64 {
    fn from(d: Dimension) -> Self {
        match d {
            Dimension::Px(p) => p,
            _ => 0.0,
        }
    }
}

#[derive(Default, Copy, Clone, PartialEq)]
pub struct Size {
    pub width: Dimension,
    pub height: Dimension,
}

impl std::fmt::Debug for Size {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Size[{:?}, {:?}]", self.width, self.height)
    }
}

impl Size {
    fn resolved(&self) -> bool {
        self.width.resolved() && self.height.resolved()
    }

    fn most_specific(&self, other: &Self) -> Self {
        Self {
            width: self.width.most_specific(&other.width),
            height: self.height.most_specific(&other.height),
        }
    }

    fn more_specific(&self, other: &Self) -> Self {
        Self {
            width: self.width.more_specific(&other.width),
            height: self.height.more_specific(&other.height),
        }
    }

    fn main(&self, dir: Direction) -> Dimension {
        match dir {
            Direction::Row => self.width,
            Direction::Column => self.height,
        }
    }

    fn cross(&self, dir: Direction) -> Dimension {
        match dir {
            Direction::Row => self.height,
            Direction::Column => self.width,
        }
    }

    fn main_mut(&mut self, dir: Direction) -> &mut Dimension {
        match dir {
            Direction::Row => &mut self.width,
            Direction::Column => &mut self.height,
        }
    }

    fn cross_mut(&mut self, dir: Direction) -> &mut Dimension {
        match dir {
            Direction::Row => &mut self.height,
            Direction::Column => &mut self.width,
        }
    }

    fn maybe_resolve(&self, relative_to: &Self) -> Self {
        Self {
            width: self.width.maybe_resolve(&relative_to.width),
            height: self.height.maybe_resolve(&relative_to.height),
        }
    }

    fn minus_rect(&self, rect: &Rect) -> Self {
        Self {
            width: self.width - rect.left - rect.right,
            height: self.height - rect.top - rect.bottom,
        }
    }

    fn plus_rect(&self, rect: &Rect) -> Self {
        Self {
            width: self.width + rect.left + rect.right,
            height: self.height + rect.top + rect.bottom,
        }
    }
}

impl From<ScrollPosition> for Size {
    fn from(p: ScrollPosition) -> Self {
        Self {
            width: Dimension::Px(p.x.unwrap_or(0.0).into()),
            height: Dimension::Px(p.y.unwrap_or(0.0).into()),
        }
    }
}

#[derive(Default, Copy, Clone, PartialEq)]
pub struct Rect {
    pub left: Dimension,
    pub right: Dimension,
    pub top: Dimension,
    pub bottom: Dimension,
}

impl std::fmt::Debug for Rect {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Rect[l:{:?}, r:{:?}, t:{:?}, b:{:?}]",
            self.left, self.right, self.top, self.bottom
        )
    }
}

impl Rect {
    const ZERO: Self = Self {
        left: Dimension::Px(0.0),
        right: Dimension::Px(0.0),
        top: Dimension::Px(0.0),
        bottom: Dimension::Px(0.0),
    };

    fn maybe_resolve(&self, relative_to: &Size) -> Self {
        Self {
            left: self.left.maybe_resolve(&relative_to.width),
            right: self.right.maybe_resolve(&relative_to.width),
            top: self.top.maybe_resolve(&relative_to.height),
            bottom: self.bottom.maybe_resolve(&relative_to.height),
        }
    }

    fn main(&self, dir: Direction, align: Alignment) -> Dimension {
        match (dir, align) {
            (Direction::Row, Alignment::End) => self.right,
            (Direction::Row, _) => self.left,
            (Direction::Column, Alignment::End) => self.bottom,
            (Direction::Column, _) => self.top,
        }
    }

    fn main_mut(&mut self, dir: Direction, align: Alignment) -> &mut Dimension {
        match (dir, align) {
            (Direction::Row, Alignment::End) => &mut self.right,
            (Direction::Row, _) => &mut self.left,
            (Direction::Column, Alignment::End) => &mut self.bottom,
            (Direction::Column, _) => &mut self.top,
        }
    }

    fn main_reverse(&self, dir: Direction, align: Alignment) -> Dimension {
        match (dir, align) {
            (Direction::Row, Alignment::End) => self.left,
            (Direction::Row, _) => self.right,
            (Direction::Column, Alignment::End) => self.top,
            (Direction::Column, _) => self.bottom,
        }
    }

    fn cross(&self, dir: Direction, align: Alignment) -> Dimension {
        match (dir, align) {
            (Direction::Row, Alignment::End) => self.bottom,
            (Direction::Row, _) => self.top,
            (Direction::Column, Alignment::End) => self.right,
            (Direction::Column, _) => self.left,
        }
    }

    fn cross_mut(&mut self, dir: Direction, align: Alignment) -> &mut Dimension {
        match (dir, align) {
            (Direction::Row, Alignment::End) => &mut self.bottom,
            (Direction::Row, _) => &mut self.top,
            (Direction::Column, Alignment::End) => &mut self.right,
            (Direction::Column, _) => &mut self.left,
        }
    }

    fn cross_reverse(&self, dir: Direction, align: Alignment) -> Dimension {
        match (dir, align) {
            (Direction::Row, Alignment::End) => self.top,
            (Direction::Row, _) => self.bottom,
            (Direction::Column, Alignment::End) => self.left,
            (Direction::Column, _) => self.right,
        }
    }

    fn most_specific(&self, other: &Self) -> Self {
        let top = if self.top.resolved() {
            self.top
        } else if other.top.resolved() && !self.bottom.resolved() {
            other.top
        } else {
            self.top
        };
        let bottom = if self.bottom.resolved() {
            self.bottom
        } else if other.bottom.resolved() && !self.top.resolved() {
            other.bottom
        } else {
            self.bottom
        };
        let left = if self.left.resolved() {
            self.left
        } else if other.left.resolved() && !self.right.resolved() {
            other.left
        } else {
            self.left
        };
        let right = if self.right.resolved() {
            self.right
        } else if other.right.resolved() && !self.left.resolved() {
            other.right
        } else {
            self.right
        };
        Self {
            top,
            left,
            bottom,
            right,
        }
    }

    // fn minus_size(&self, size: Size) -> Self {
    //     let top = if self.top.resolved() && size.height.resolved() {
    //         Dimension::Px(f32::from(self.top) - f32::from(size.height))
    //     } else {
    //         self.top
    //     };
    //     let bottom = if self.bottom.resolved() && size.height.resolved() {
    //         Dimension::Px(f32::from(self.bottom) - f32::from(size.height))
    //     } else {
    //         self.bottom
    //     };
    //     let left = if self.left.resolved() && size.width.resolved() {
    //         Dimension::Px(f32::from(self.left) - f32::from(size.width))
    //     } else {
    //         self.left
    //     };
    //     let right = if self.right.resolved() && size.width.resolved() {
    //         Dimension::Px(f32::from(self.right) - f32::from(size.width))
    //     } else {
    //         self.right
    //     };
    //     Self {
    //         top,
    //         left,
    //         bottom,
    //         right,
    //     }
    // }

    // fn plus_size(&self, size: Size) -> Self {
    //     let top = if self.top.resolved() && size.height.resolved() {
    //         Dimension::Px(f32::from(self.top) + f32::from(size.height))
    //     } else {
    //         self.top
    //     };
    //     let bottom = if self.bottom.resolved() && size.height.resolved() {
    //         Dimension::Px(f32::from(self.bottom) + f32::from(size.height))
    //     } else {
    //         self.bottom
    //     };
    //     let left = if self.left.resolved() && size.width.resolved() {
    //         Dimension::Px(f32::from(self.left) + f32::from(size.width))
    //     } else {
    //         self.left
    //     };
    //     let right = if self.right.resolved() && size.width.resolved() {
    //         Dimension::Px(f32::from(self.right) + f32::from(size.width))
    //     } else {
    //         self.right
    //     };
    //     Self {
    //         top,
    //         left,
    //         bottom,
    //         right,
    //     }
    // }
}

impl From<crate::types::Point> for Rect {
    fn from(p: crate::types::Point) -> Self {
        Self {
            top: Dimension::Px(p.y.into()),
            left: Dimension::Px(p.x.into()),
            ..Default::default()
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Direction {
    Row,
    Column,
}

impl Default for Direction {
    fn default() -> Self {
        Self::Row
    }
}

impl Direction {
    fn size(&self, main: Dimension, cross: Dimension) -> Size {
        match self {
            Self::Row => Size {
                width: main,
                height: cross,
            },
            Self::Column => Size {
                width: cross,
                height: main,
            },
        }
    }

    fn rect(
        &self,
        main: Dimension,
        cross: Dimension,
        axis_alignment: Alignment,
        cross_alignment: Alignment,
    ) -> Rect {
        let mut rect = Rect::default();

        match (self, axis_alignment) {
            (Direction::Row, Alignment::End) => rect.right = main,
            (Direction::Row, _) => rect.left = main,
            (Direction::Column, Alignment::End) => rect.bottom = main,
            (Direction::Column, _) => rect.top = main,
        }

        match (self, cross_alignment) {
            (Direction::Row, Alignment::End) => rect.bottom = cross,
            (Direction::Row, _) => rect.top = cross,
            (Direction::Column, Alignment::End) => rect.right = cross,
            (Direction::Column, _) => rect.left = cross,
        }

        rect
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PositionType {
    Absolute,
    Relative,
}

impl Default for PositionType {
    fn default() -> Self {
        Self::Relative
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Alignment {
    Start,
    End,
    Center,
    Stretch,
}

impl Default for Alignment {
    fn default() -> Self {
        Self::Start
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub direction: Direction,
    pub wrap: bool,
    pub position: Rect,
    pub position_type: PositionType,
    pub axis_alignment: Alignment,
    pub cross_alignment: Alignment,
    pub margin: Rect,
    pub padding: Rect,
    pub size: Size,
    // TODO employ this more consistently
    pub max_size: Size,
    pub min_size: Size,
    pub z_index: Option<f64>,
    pub z_index_increment: f64,
    pub debug: Option<String>,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            direction: Default::default(),
            wrap: false,
            position: Default::default(),
            position_type: Default::default(),
            axis_alignment: Default::default(),
            cross_alignment: Default::default(),
            margin: Rect::ZERO,
            padding: Rect::ZERO,
            size: Default::default(),
            max_size: Default::default(),
            min_size: Size {
                width: Dimension::Px(10.0),
                height: Dimension::Px(10.0),
            },
            z_index: None,
            z_index_increment: 0.0,
            debug: None,
        }
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct LayoutResult {
    pub size: Size,
    pub position: Rect,
}

impl From<LayoutResult> for crate::types::AABB {
    fn from(p: LayoutResult) -> Self {
        Self::new(
            crate::types::Pos::new(p.position.left.into(), p.position.top.into(), 0.0),
            crate::types::Scale::new(p.size.width.into(), p.size.height.into()),
        )
    }
}

impl super::node::Node {
    fn resolve_child_sizes(
        &mut self,
        inner_size: Size,
        font_cache: &mut crate::font_cache::FontCache,
        scale_factor: f32,
        final_pass: bool,
    ) {
        let dir = self.layout.direction;
        let mut main_remaining = f64::from(inner_size.main(dir));
        let mut max_cross_size = 0.0;
        let mut unresolved = 0;
        // dbg!(&self.component, inner_size);

        for child in self.children.iter_mut() {
            // Stretch alignment
            if self.layout.cross_alignment == Alignment::Stretch {
                *child.layout_result.size.cross_mut(dir) = Dimension::Pct(100.0)
            }

            if cfg!(debug_assertions) && child.layout.debug.is_some() {
                println!(
                    "{} Resolving child position of {} - Basing off child.layout.size {:?}, child.layout_result.size {:?}, inner_size {:?})",
                    if final_pass {
                        "Final pass"
                    } else {
                        "First pass"
                    },
                    child.layout.debug.as_ref().unwrap(),
                    &child.layout.size,
                    &child.layout_result.size,
                    &inner_size,
                );
            }

            let child_margin = child.layout.margin.maybe_resolve(&inner_size);

            child.layout_result.size = child
                .layout
                .size
                .more_specific(&child.layout_result.size.plus_rect(&child_margin))
                .maybe_resolve(&inner_size)
                .minus_rect(&child_margin);

            if self.layout.axis_alignment == Alignment::Stretch
                && child.layout.size.main(dir) == Dimension::Auto
            {
                // We want to calculate this in the next for block
                *child.layout_result.size.main_mut(dir) = Dimension::Auto;
            }
            if !child.layout_result.size.resolved() {
                let inner_size =
                    inner_size.minus_rect(&child.layout.margin.maybe_resolve(&inner_size));
                let (w, h) = child.component.fill_bounds(
                    child.layout_result.size.width.maybe_px(),
                    child.layout_result.size.height.maybe_px(),
                    inner_size
                        .width
                        .maybe_px()
                        .or(self.layout.max_size.width.maybe_px()),
                    inner_size
                        .height
                        .maybe_px()
                        .or(self.layout.max_size.height.maybe_px()),
                    font_cache,
                    scale_factor,
                );
                if let Some(w) = w {
                    child.layout_result.size.width = Dimension::Px(w.into());
                }
                if let Some(h) = h {
                    child.layout_result.size.height = Dimension::Px(h.into());
                }
            }

            if f32::from(child.layout_result.size.cross(dir)) > max_cross_size {
                max_cross_size = child.layout_result.size.cross(dir).into();
            }

            if let Dimension::Px(x) = child.layout_result.size.main(dir) {
                main_remaining -= x;
            } else {
                unresolved += 1;
            }
        }
        main_remaining = main_remaining.max(0.0);

        for child in self.children.iter_mut() {
            if self.layout.axis_alignment == Alignment::Stretch
                && !child.layout_result.size.main(dir).resolved()
            {
                let margin = child.layout.margin.maybe_resolve(&inner_size);
                *child.layout_result.size.main_mut(dir) =
                    Dimension::Px(main_remaining / unresolved as f64)
                        - margin.main(dir, Alignment::Start)
                        - margin.main(dir, Alignment::End);
            }

            // size as a pct of max sibling
            if (child.layout.size.cross_mut(dir).is_pct()
                || child.layout_result.size.cross_mut(dir).is_pct())
                && !child.layout_result.size.cross(dir).resolved()
                && !self.layout.wrap
                && max_cross_size > 0.0
            {
                let mut max_cross = Size::default();
                *max_cross.cross_mut(dir) = Dimension::Px(max_cross_size.into());

                child.layout_result.size = child
                    .layout
                    .size
                    .most_specific(&child.layout_result.size)
                    .maybe_resolve(&max_cross)
                    .minus_rect(&child.layout.margin.maybe_resolve(&inner_size));
            }

            child.resolve_layout(inner_size, font_cache, scale_factor, final_pass);
        }
    }

    fn resolve_position(&mut self, bounds: Size) {
        let pos = self.layout_result.position;
        let size = self.layout_result.size;
        match (pos.top, pos.bottom) {
            (Dimension::Px(top), _) => {
                // Correct any discrepancy with bottom relative to top
                self.layout_result.position.bottom = Dimension::Px(top + f64::from(size.height));
            }
            (_, Dimension::Px(bottom)) => {
                self.layout_result.position.top =
                    Dimension::Px(f64::from(bounds.height) - bottom - f64::from(size.height));
                // Transform the bottom relative position into top relative
                self.layout_result.position.bottom =
                    Dimension::Px(f64::from(bounds.height) - bottom);
            }
            _ => self.layout_result.position.top = Dimension::Px(0.0),
        }
        match (pos.left, pos.right) {
            (Dimension::Px(left), _) => {
                // Correct any discrepancy with bottom relative to top
                self.layout_result.position.right = Dimension::Px(left + f64::from(size.width));
            }
            (_, Dimension::Px(right)) => {
                self.layout_result.position.left =
                    Dimension::Px(f64::from(bounds.width) - right - f64::from(size.width));
                // Transform the right relative position into left relative
                self.layout_result.position.right = Dimension::Px(f64::from(bounds.width) - right);
            }
            _ => self.layout_result.position.left = Dimension::Px(0.0),
        }
    }

    fn set_children_position(&mut self, size: Size) -> Size {
        let dir = self.layout.direction;
        let axis_align = self.layout.axis_alignment;
        let cross_align = self.layout.cross_alignment;
        let main_start_padding: f64 = self
            .layout
            .padding
            .main(dir, axis_align)
            .maybe_resolve(&size.main(dir))
            .into();
        let main_end_padding: f64 = self
            .layout
            .padding
            .main_reverse(dir, axis_align)
            .maybe_resolve(&size.main(dir))
            .into();
        let mut main_pos: f64 = main_start_padding;
        let mut cross_pos = self
            .layout
            .padding
            .cross(dir, cross_align)
            .maybe_resolve(&size.cross(dir))
            .into();
        let mut max_cross_size = 0.0;
        let mut row_lengths: Vec<(f64, usize)> = vec![];
        let mut row_elements_count: usize = 0;

        // Reverse the calculation when End axis_aligned
        let mut children: Vec<&mut Self> = if axis_align == Alignment::End {
            self.children.iter_mut().rev().collect()
        } else {
            self.children.iter_mut().collect()
        };

        for child in children.iter_mut() {
            let margin = child.layout.margin.maybe_resolve(&size);
            let child_outer_size = child.layout_result.size.plus_rect(&margin);

            // Perform a wrap?
            if self.layout.wrap
                && size.main(dir).resolved()
                && child.layout.position_type != PositionType::Absolute
                && (main_pos + main_end_padding + f64::from(child_outer_size.main(dir)))
                    > f64::from(size.main(dir))
                && main_pos > main_start_padding
            {
                row_lengths.push((main_pos + main_end_padding, row_elements_count));
                main_pos = main_start_padding;
                cross_pos += max_cross_size;
                max_cross_size = 0.0;
                row_elements_count = 0;
            }

            if child.layout.position_type == PositionType::Relative {
                child.layout_result.position = dir.rect(
                    Dimension::Px(main_pos),
                    Dimension::Px(cross_pos),
                    axis_align,
                    cross_align,
                );
                *child.layout_result.position.main_mut(dir, axis_align) +=
                    margin.main(dir, axis_align);
                *child.layout_result.position.cross_mut(dir, cross_align) +=
                    margin.cross(dir, cross_align);

                child.resolve_position(size);

                // Push bounds
                main_pos += f64::from(child_outer_size.main(dir));
                row_elements_count += 1;
                if f64::from(child_outer_size.cross(dir)) > max_cross_size {
                    max_cross_size = child_outer_size.cross(dir).into();
                }

                if cfg!(debug_assertions) && child.layout.debug.is_some() {
                    println!(
                        "Setting relative position of {} to {:#?} - Basing off ...",
                        child.layout.debug.as_ref().unwrap(),
                        &child.layout_result.position,
                    );
                }
            } else {
                child.layout_result.position = child.layout.position.most_specific(&dir.rect(
                    Dimension::Px(main_pos),
                    Dimension::Px(cross_pos),
                    axis_align,
                    cross_align,
                ));
                *child.layout_result.position.main_mut(dir, axis_align) +=
                    margin.main(dir, axis_align);
                *child.layout_result.position.cross_mut(dir, cross_align) +=
                    margin.cross(dir, cross_align);

                child.resolve_position(size);

                // TODO: More of these
                if cfg!(debug_assertions) && child.layout.debug.is_some() {
                    println!("Setting absolute position of {} to {:#?} - Basing off explicit position ({:#?}), parent size ({:#?}))", child.layout.debug.as_ref().unwrap(), &child.layout_result.position, &child.layout.position, &size);
                }
            }
        }

        row_lengths.push((main_pos + main_end_padding, row_elements_count));

        // Combined size of children
        let mut children_size = if self.children.is_empty() {
            Size::default()
        } else {
            // This won't be accurate for wrapped elements, but it doesn't really matter
            dir.size(
                Dimension::Px(main_pos),
                Dimension::Px(cross_pos + max_cross_size),
            )
        };
        *children_size.main_mut(dir) += self.layout.padding.main_reverse(dir, axis_align);
        *children_size.cross_mut(dir) += self.layout.padding.cross_reverse(dir, cross_align);

        // TODO Alignment::Stretch when not all space is filled

        if axis_align == Alignment::Center || cross_align == Alignment::Center {
            // Reposition center alignment
            let main_offset = if axis_align == Alignment::Center && size.main(dir).resolved() {
                // This is only accurate when for non-wrapped elements.
                // For wrapped elements, we compute within the loop
                (f64::from(size.main(dir)) - f64::from(children_size.main(dir))) / 2.0
            } else {
                0.0
            };
            let cross_size = {
                if size.cross(dir).resolved() {
                    f64::from(size.cross(dir))
                } else {
                    f64::from(children_size.cross(dir))
                }
            };

            let mut elements_positioned_in_row = 0;
            let mut current_row = 0;
            for child in self.children.iter_mut() {
                if child.layout.position_type == PositionType::Absolute {
                    continue;
                }
                let main_offset = if self.layout.wrap {
                    if elements_positioned_in_row >= row_lengths[current_row].1 {
                        elements_positioned_in_row = 0;
                        current_row += 1;
                    }
                    (f64::from(size.main(dir)) - row_lengths[current_row].0) / 2.0
                } else {
                    main_offset
                };
                *child.layout_result.position.main_mut(dir, axis_align) +=
                    Dimension::Px(main_offset);

                if cross_align == Alignment::Center {
                    if row_lengths.len() > 1 {
                        // TODO: Center within a row?
                        *child.layout_result.position.cross_mut(dir, cross_align) +=
                            Dimension::Px((cross_size - f64::from(children_size.cross(dir))) / 2.0);
                    } else {
                        *child.layout_result.position.cross_mut(dir, cross_align) = Dimension::Px(
                            (cross_size - f64::from(child.layout_result.size.cross(dir))) / 2.0,
                        );
                    };
                }

                child.resolve_position(size);
                elements_positioned_in_row += 1;

                if cfg!(debug_assertions) && child.layout.debug.is_some() {
                    println!(
                        "Resolved aligned position of {} to {:#?} - Basing off ...)",
                        child.layout.debug.as_ref().unwrap(),
                        &child.layout_result.position
                    );
                }
            }
        }

        if self.scrollable() {
            children_size.width += Dimension::Px(
                (self.component.spacing().width * (self.children.len() - 1) as f32).into(),
            );
            children_size.height += Dimension::Px(
                (self.component.spacing().height * (self.children.len() - 1) as f32).into(),
            );
        }

        children_size
    }

    /// Make sure the node has a size, either taken from its children or from itself
    fn resolve_size(&mut self, mut size: Size, children_size: Size) {
        let min_size = self.layout.min_size;
        if !size.width.resolved() || f64::from(size.width) < 0.0 {
            if self.scroll_x().is_none() && children_size.width.resolved() {
                size.width = children_size.width;
            } else if min_size.width.resolved() {
                size.width = min_size.width
            } else {
                size.width = Dimension::Px(10.0)
            }
        }
        if !size.height.resolved() || f64::from(size.height) < 0.0 {
            if self.scroll_y().is_none() && children_size.height.resolved() {
                size.height = children_size.height;
            } else if min_size.height.resolved() {
                size.height = min_size.height
            } else {
                size.height = Dimension::Px(10.0)
            }
        }

        self.layout_result.size = size;
    }

    fn set_inner_scale(&mut self, children_size: Size) {
        if self.scrollable() {
            let inner_width = if self.scroll_x().is_some() {
                children_size.width.max(self.layout_result.size.width)
            } else {
                self.layout_result.size.width
            };
            let inner_height = if self.scroll_y().is_some() {
                children_size.height.max(self.layout_result.size.height)
            } else {
                self.layout_result.size.height
            };
            self.inner_scale = Some(crate::types::Scale {
                width: inner_width.into(),
                height: inner_height.into(),
            });
        }
    }

    /// For each axis in a node, it either has a size (or margin, or padding) in pixels,
    /// or its parent does (at the time of resolution). If a size axis is Auto, then
    /// it gets its size from its children, who must all have a resolved size on that axis.
    /// If it's children can not resolve its size, then it falls back to the min_size
    ///
    /// Wrapping cannot be performed on an axis that isn't resolved.
    ///
    /// A node that it scrollable on an axis must have a resolved size on that axis.
    fn resolve_layout(
        &mut self,
        bounds_size: Size,
        font_cache: &mut crate::font_cache::FontCache,
        scale_factor: f32,
        final_pass: bool,
    ) {
        let size = self.layout.size.most_specific(&self.layout_result.size);

        let mut inner_size = size.minus_rect(&self.layout.padding.maybe_resolve(&bounds_size));
        if self.scroll_x().is_some() {
            inner_size.width = Dimension::Auto;
        };
        if self.scroll_y().is_some() {
            inner_size.height = Dimension::Auto;
        };
        if cfg!(debug_assertions) && self.layout.debug.is_some() {
            println!(
                "{} Laying out {} in bounds {:?} with a resulting inner size {:?}: {:#?}",
                if final_pass {
                    "Final pass"
                } else {
                    "First pass"
                },
                self.layout.debug.as_ref().unwrap(),
                &bounds_size,
                &inner_size,
                &self.layout,
            );
        }

        self.resolve_child_sizes(inner_size, font_cache, scale_factor, final_pass);
        let children_size = self.set_children_position(size);
        self.resolve_size(size, children_size);
        self.set_inner_scale(children_size);

        if cfg!(debug_assertions) && self.layout.debug.is_some() {
            println!(
                "{} Layout result of {}: {:?}",
                if final_pass {
                    "Final pass"
                } else {
                    "First pass"
                },
                self.layout.debug.as_ref().unwrap(),
                &self.layout_result
            );
        }
    }

    pub(crate) fn calculate_layout(
        &mut self,
        font_cache: &mut crate::font_cache::FontCache,
        scale_factor: f32,
    ) {
        self.layout_result.position = Rect {
            top: Dimension::Px(0.0),
            left: Dimension::Px(0.0),
            bottom: Dimension::Auto,
            right: Dimension::Auto,
        };
        self.resolve_layout(self.layout.size, font_cache, scale_factor, false);
        // Layout is resolved twice, the second time to resolve percentages that couldn't have been known without better knowledge of the children
        self.resolve_layout(self.layout.size, font_cache, scale_factor, true);
    }
}

#[macro_export]
macro_rules! lay {
    // Finish it
    ( @ { } -> ($($result:tt)*) ) => (
        $crate::layout::Layout {
            $($result)*
                ..Default::default()
        }
    );

    // margin
    ( @ { $(,)* margin : [$($vals:tt)+] $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                margin : rect!($($vals)*),
        ))
    );
    ( @ { $(,)* margin_pct : [$($vals:tt)+] $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                margin : rect_pct!($($vals)*),
        ))
    );

    // padding
    ( @ { $(,)* padding : [$($vals:tt)+] $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                padding : rect!($($vals)*),
        ))
    );
    ( @ { $(,)* padding_pct : [$($vals:tt)+] $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                padding : rect_pct!($($vals)*),
        ))
    );

    // position
    ( @ { $(,)* position : [$($vals:tt)+] $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                position : rect!($($vals)*),
        ))
    );
    ( @ { $(,)* position_pct : [$($vals:tt)+] $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                position : rect_pct!($($vals)*),
        ))
    );

    // size
    ( @ { $(,)* size : [$($vals:tt)+] $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                size : size!($($vals)*),
        ))
    );
    ( @ { $(,)* size_pct : [$($vals:tt)+] $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                size : size_pct!($($vals)*),
        ))
    );
    ( @ { $(,)* min_size : [$($vals:tt)+] $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                min_size : size!($($vals)*),
        ))
    );
    ( @ { $(,)* max_size : [$($vals:tt)+] $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                max_size : size!($($vals)*),
        ))
    );

    // Direction
    ( @ { $(,)* $param:ident : Row $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                $param : $crate::layout::Direction::Row,
        ))
    );
    ( @ { $(,)* $param:ident : Column $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                $param : $crate::layout::Direction::Column,
        ))
    );

    // PositionType
    ( @ { $(,)* $param:ident : Relative $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                $param : $crate::layout::PositionType::Relative,
        ))
    );
    ( @ { $(,)* $param:ident : Absolute $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                $param : $crate::layout::PositionType::Absolute,
        ))
    );


    // Alignment
    ( @ { $(,)* $param:ident : Start $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                $param : $crate::layout::Alignment::Start,
        ))
    );
    ( @ { $(,)* $param:ident : End $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                $param : $crate::layout::Alignment::End,
        ))
    );
    ( @ { $(,)* $param:ident : Center $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                $param : $crate::layout::Alignment::Center,
        ))
    );
    ( @ { $(,)* $param:ident : Stretch $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                $param : $crate::layout::Alignment::Stretch,
        ))
    );

    // z_index
    ( @ { $(,)* z_index : $z_index:expr, $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                z_index : Some($z_index .into()),
        ))
    );
    ( @ { $(,)* z_index : $z_index:expr} -> ($($result:tt)*) ) => (
        lay!(@ { } -> ( $($result)* z_index : Some($z_index .into()), ))
    );

    // Debug
    ( @ { $(,)* debug : $debug:expr, $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                debug : Some($debug .into()),
        ))
    );
    ( @ { $(,)* debug : $debug:expr} -> ($($result:tt)*) ) => (
        lay!(@ { } -> ( $($result)* debug : Some($debug .into()), ))
    );


    // Everything else
    ( @ { $(,)* $param:ident : $val:expr } -> ($($result:tt)*) ) => (
        lay!(@ { } -> (
            $($result)*
                $param : $val,
        ))
    );
    ( @ { $(,)* $param:ident : $val:expr, $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                $param : $val,
        ))
    );
    ( @ { $(,)* } -> ($($result:tt)*) ) => (
        lay!(@ {} -> (
            $($result)*
        ))
    );


    // Entry point
    ( $( $tt:tt )* ) => (
        lay!(@ { $($tt)* } -> ())
    );
}

#[macro_export]
macro_rules! px {
    ($val:expr) => {
        $crate::layout::Dimension::Px($val)
    };
}

#[macro_export]
macro_rules! pct {
    ($val:expr) => {
        $crate::layout::Dimension::Pct($val)
    };
}

#[macro_export]
macro_rules! size {
    ($width:expr, Auto) => {
        $crate::layout::Size {
            width: $crate::layout::Dimension::Px($width.into()),
            height: $crate::layout::Dimension::Auto,
        }
    };
    (Auto, $height:expr) => {
        $crate::layout::Size {
            width: $crate::layout::Dimension::Auto,
            height: $crate::layout::Dimension::Px($height.into()),
        }
    };
    ($width:expr, $height:expr) => {
        $crate::layout::Size {
            width: $crate::layout::Dimension::Px($width.into()),
            height: $crate::layout::Dimension::Px($height.into()),
        }
    };
    (Auto) => {
        $crate::layout::Size {
            width: $crate::layout::Dimension::Auto,
            height: $crate::layout::Dimension::Auto,
        }
    };
    ($x:expr) => {
        $crate::layout::Size {
            width: $crate::layout::Dimension::Px($x.into()),
            height: $crate::layout::Dimension::Px($x.into()),
        }
    };
}

#[macro_export]
macro_rules! size_pct {
    ($width:expr, Auto) => {
        $crate::layout::Size {
            width: $crate::layout::Dimension::Pct($width.into()),
            height: $crate::layout::Dimension::Auto,
        }
    };
    (Auto, $height:expr) => {
        $crate::layout::Size {
            width: $crate::layout::Dimension::Auto,
            height: $crate::layout::Dimension::Pct($height.into()),
        }
    };
    ($width:expr, $height:expr) => {
        $crate::layout::Size {
            width: $crate::layout::Dimension::Pct($width.into()),
            height: $crate::layout::Dimension::Pct($height.into()),
        }
    };
    ($x:expr) => {
        $crate::layout::Size {
            width: $crate::layout::Dimension::Pct($x.into()),
            height: $crate::layout::Dimension::Pct($x.into()),
        }
    };
}

#[macro_export]
macro_rules! rect {
    // One arg
    (Auto) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    ($all:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Px($all.into()),
            right: $crate::layout::Dimension::Px($all.into()),
            top: $crate::layout::Dimension::Px($all.into()),
            bottom: $crate::layout::Dimension::Px($all.into()),
        }
    };
    // Two args
    (Auto, $se:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Px($se.into()),
            right: $crate::layout::Dimension::Px($se.into()),
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    ($tb:expr, Auto) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Px($tb.into()),
            bottom: $crate::layout::Dimension::Px($tb.into()),
        }
    };
    ($tb:expr, $se:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Px($se.into()),
            right: $crate::layout::Dimension::Px($se.into()),
            top: $crate::layout::Dimension::Px($tb.into()),
            bottom: $crate::layout::Dimension::Px($tb.into()),
        }
    };
    // Three args
    ($t:expr, Auto, Auto) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Px($t),
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    (Auto, Auto, $b:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Px($b.into()),
        }
    };
    (Auto, $se:expr, $b:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Px($se.into()),
            right: $crate::layout::Dimension::Px($se.into()),
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Px($b.into()),
        }
    };
    ($t:expr, Auto, $b:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Px($t.into()),
            bottom: $crate::layout::Dimension::Px($b.into()),
        }
    };
    ($t:expr, $se:expr, Auto) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Px($se.into()),
            right: $crate::layout::Dimension::Px($se.into()),
            top: $crate::layout::Dimension::Px($t.into()),
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    ($t:expr, $se:expr, $b:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Px($se.into()),
            right: $crate::layout::Dimension::Px($se.into()),
            top: $crate::layout::Dimension::Px($t.into()),
            bottom: $crate::layout::Dimension::Px($b.into()),
        }
    };
    // Four args
    (Auto, $s:expr, Auto, Auto) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Px($s.into()),
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    (Auto, Auto, Auto, $e:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Px($e.into()),
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    (Auto, $s:expr, Auto, $e:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Px($s.into()),
            right: $crate::layout::Dimension::Px($e.into()),
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    (Auto, Auto, $b:expr, $e:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Px($e.into()),
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Px($b.into()),
        }
    };
    ($t:expr, $s:expr, Auto, Auto) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Px($s.into()),
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Px($t.into()),
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    ($t:expr, Auto, Auto, $e:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Px($e.into()),
            top: $crate::layout::Dimension::Px($t.into()),
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    (Auto, $s:expr, $b:expr, $e:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Px($s.into()),
            right: $crate::layout::Dimension::Px($e.into()),
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Px($b.into()),
        }
    };
    ($t:expr, Auto, $b:expr, $e:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Px($e.into()),
            top: $crate::layout::Dimension::Px($t.into()),
            bottom: $crate::layout::Dimension::Px($b.into()),
        }
    };
    ($t:expr, $s:expr, Auto, $e:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Px($s.into()),
            right: $crate::layout::Dimension::Px($e.into()),
            top: $crate::layout::Dimension::Px($t.into()),
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    ($t:expr, $s:expr, $b:expr, Auto) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Px($s.into()),
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Px($t.into()),
            bottom: $crate::layout::Dimension::Px($b.into()),
        }
    };
    ($t:expr, $s:expr, $b:expr, $e:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Px($s.into()),
            right: $crate::layout::Dimension::Px($e.into()),
            top: $crate::layout::Dimension::Px($t.into()),
            bottom: $crate::layout::Dimension::Px($b.into()),
        }
    };
}

#[macro_export]
macro_rules! rect_pct {
    // One arg
    (Auto) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    ($all:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Pct($all.into()),
            right: $crate::layout::Dimension::Pct($all.into()),
            top: $crate::layout::Dimension::Pct($all.into()),
            bottom: $crate::layout::Dimension::Pct($all.into()),
        }
    };
    // Two args
    (Auto, $se:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Pct($se.into()),
            right: $crate::layout::Dimension::Pct($se.into()),
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    ($tb:expr, Auto) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Pct($tb.into()),
            bottom: $crate::layout::Dimension::Pct($tb.into()),
        }
    };
    ($tb:expr, $se:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Pct($se.into()),
            right: $crate::layout::Dimension::Pct($se.into()),
            top: $crate::layout::Dimension::Pct($tb.into()),
            bottom: $crate::layout::Dimension::Pct($tb.into()),
        }
    };
    // Three args
    ($t:expr, Auto, Auto) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Pct($t.into()),
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    (Auto, Auto, $b:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Pct($b.into()),
        }
    };
    (Auto, $se:expr, $b:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Pct($se.into()),
            right: $crate::layout::Dimension::Pct($se.into()),
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Pct($b.into()),
        }
    };
    ($t:expr, Auto, $b:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Pct($t.into()),
            bottom: $crate::layout::Dimension::Pct($b.into()),
        }
    };
    ($t:expr, $se:expr, Auto) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Pct($se.into()),
            right: $crate::layout::Dimension::Pct($se.into()),
            top: $crate::layout::Dimension::Pct($t.into()),
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    ($t:expr, $se:expr, $b:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Pct($se.into()),
            right: $crate::layout::Dimension::Pct($se.into()),
            top: $crate::layout::Dimension::Pct($t.into()),
            bottom: $crate::layout::Dimension::Pct($b.into()),
        }
    };
    // Four args
    (Auto, $s:expr, Auto, Auto) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Pct($s.into()),
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    (Auto, Auto, Auto, $e:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Pct($e.into()),
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    (Auto, $s:expr, Auto, $e:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Pct($s.into()),
            right: $crate::layout::Dimension::Pct($e.into()),
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    (Auto, Auto, $b:expr, $e:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Pct($e.into()),
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Pct($b.into()),
        }
    };
    ($t:expr, $s:expr, Auto, Auto) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Pct($s.into()),
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Pct($t.into()),
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    ($t:expr, Auto, Auto, $e:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Pct($e.into()),
            top: $crate::layout::Dimension::Pct($t.into()),
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    (Auto, $s:expr, $b:expr, $e:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Pct($s.into()),
            right: $crate::layout::Dimension::Pct($e.into()),
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Pct($b.into()),
        }
    };
    ($t:expr, Auto, $b:expr, $e:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Pct($e.into()),
            top: $crate::layout::Dimension::Pct($t.into()),
            bottom: $crate::layout::Dimension::Pct($b.into()),
        }
    };
    ($t:expr, $s:expr, Auto, $e:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Pct($s.into()),
            right: $crate::layout::Dimension::Pct($e.into()),
            top: $crate::layout::Dimension::Pct($t.into()),
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    ($t:expr, $s:expr, $b:expr, Auto) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Pct($s.into()),
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Pct($t.into()),
            bottom: $crate::layout::Dimension::Pct($b.into()),
        }
    };
    ($t:expr, $s:expr, $b:expr, $e:expr) => {
        $crate::layout::Rect {
            left: $crate::layout::Dimension::Pct($s.into()),
            right: $crate::layout::Dimension::Pct($e.into()),
            top: $crate::layout::Dimension::Pct($t.into()),
            bottom: $crate::layout::Dimension::Pct($b.into()),
        }
    };
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::node;
//     use crate::widgets::Div;

//     #[test]
//     fn test_empty() {
//         let mut nodes = node!(Div::new(), lay!(size: size!(300.0)));
//         nodes.calculate_layout(&crate::font_cache::FontCache::default(), 1.0);
//         assert_eq!(nodes.layout_result.size, size!(300.0));
//         assert_eq!(nodes.layout_result.position.top, px!(0.0));
//         assert_eq!(nodes.layout_result.position.left, px!(0.0));
//     }

//     #[test]
//     fn test_wrap() {
//         let mut nodes = node!(
//             Div::new(),
//             lay!(size: size!(300.0), direction: Direction::Row, wrap: true)
//         )
//         .push(node!(Div::new(), lay!(size: size!(150.0))))
//         .push(node!(Div::new(), lay!(size: size!(100.0))))
//         .push(node!(Div::new(), lay!(size: size!(200.0))));
//         nodes.calculate_layout(&crate::font_cache::FontCache::default(), 1.0);
//         assert_eq!(nodes.layout_result.size, size!(300.0));
//         assert_eq!(nodes.children[0].layout_result.position.left, px!(0.0));
//         assert_eq!(nodes.children[0].layout_result.position.top, px!(0.0));
//         assert_eq!(nodes.children[1].layout_result.position.left, px!(150.0));
//         assert_eq!(nodes.children[1].layout_result.position.top, px!(0.0));
//         assert_eq!(nodes.children[2].layout_result.position.left, px!(0.0));
//         assert_eq!(nodes.children[2].layout_result.position.top, px!(150.0));
//     }

//     #[test]
//     fn test_wrap_margins_and_padding() {
//         let mut nodes = node!(
//             Div::new(),
//             lay!(size: size!(300.0), direction: Direction::Row, wrap: true, padding: rect_pct!(1.0))
//         )
//         .push(node!(
//             Div::new(),
//             lay!(size: size!(150.0), margin: rect_pct!(1.0))
//         ))
//         .push(node!(
//             Div::new(),
//             lay!(size: size!(100.0), margin: rect_pct!(1.0))
//         ))
//         .push(node!(
//             Div::new(),
//             lay!(size: size!(200.0), margin: rect_pct!(1.0))
//         ));
//         nodes.calculate_layout(&crate::font_cache::FontCache::default(), 1.0);
//         assert_eq!(nodes.layout_result.size, size!(300.0));
//         assert_eq!(
//             nodes.children[0].layout_result.position.left,
//             px!(3.0 + 3.0)
//         );
//         assert_eq!(nodes.children[0].layout_result.position.top, px!(3.0 + 3.0));
//         assert_eq!(
//             nodes.children[1].layout_result.position.left,
//             px!((3.0 * 4.0) + 150.0)
//         );
//         assert_eq!(nodes.children[1].layout_result.position.top, px!(3.0 + 3.0));
//         // Wrapped
//         assert_eq!(
//             nodes.children[2].layout_result.position.left,
//             px!(3.0 + 3.0)
//         );
//         assert_eq!(
//             nodes.children[2].layout_result.position.top,
//             px!((3.0 * 4.0) + 150.0)
//         );
//     }

//     #[test]
//     fn test_pct() {
//         let mut nodes = node!(Div::new(), lay!(size: size!(300.0))).push(
//             node!(Div::new(), lay!(size: size_pct!(50.0, 100.0)))
//                 .push(node!(Div::new(), lay!(size: size_pct!(50.0, 100.0)))),
//         );
//         nodes.calculate_layout(&crate::font_cache::FontCache::default(), 1.0);
//         assert_eq!(nodes.layout_result.size, size!(300.0));
//         assert_eq!(nodes.children[0].layout_result.size, size!(150.0, 300.0));
//         assert_eq!(
//             nodes.children[0].children[0].layout_result.size,
//             size!(75.0, 300.0)
//         );
//     }

//     #[test]
//     fn test_pct_from_sibling() {
//         let mut nodes = node!(
//             Div::new(),
//             lay!(size: size!(Auto), direction: Direction::Column)
//         )
//         .push(node!(Div::new(), lay!(size: size!(50.0, 100.0))))
//         .push(node!(
//             Div::new(),
//             lay!(size: Size {width: Dimension::Pct(100.0), height: Dimension::Px(50.0)})
//         ));
//         nodes.calculate_layout(&crate::font_cache::FontCache::default(), 1.0);
//         assert_eq!(nodes.layout_result.size, size!(50.0, 150.0));
//         assert_eq!(nodes.children[0].layout_result.size, size!(50.0, 100.0));
//         assert_eq!(nodes.children[1].layout_result.size, size!(50.0, 50.0));
//     }

//     #[test]
//     fn test_stretch() {
//         let mut nodes = node!(
//             Div::new(),
//             lay!(
//                 size: size!(300.0),
//                 direction: Direction::Row,
//                 axis_alignment: Alignment::Stretch,
//                 cross_alignment: Alignment::Stretch,
//             )
//         )
//         .push(node!(Div::new()))
//         .push(node!(Div::new()));
//         nodes.calculate_layout(&crate::font_cache::FontCache::default(), 1.0);
//         assert_eq!(nodes.layout_result.size, size!(300.0));
//         assert_eq!(nodes.children[0].layout_result.size, size!(150.0, 300.0));
//         assert_eq!(nodes.children[0].layout_result.position.left, px!(0.0));
//         assert_eq!(nodes.children[0].layout_result.position.top, px!(0.0));
//         assert_eq!(nodes.children[1].layout_result.size, size!(150.0, 300.0));
//         assert_eq!(nodes.children[1].layout_result.position.left, px!(150.0));
//         assert_eq!(nodes.children[1].layout_result.position.top, px!(0.0));
//     }

//     #[test]
//     fn test_stretch_with_resolved_nodes() {
//         let mut nodes = node!(
//             Div::new(),
//             lay!(
//                 size: size!(300.0),
//                 direction: Direction::Row,
//                 axis_alignment: Alignment::Stretch,
//                 cross_alignment: Alignment::Stretch,
//             )
//         )
//         .push(node!(Div::new()))
//         .push(node!(Div::new(), lay!(size: size!(100.0))));
//         nodes.calculate_layout(&crate::font_cache::FontCache::default(), 1.0);

//         assert_eq!(nodes.layout_result.size, size!(300.0));
//         assert_eq!(nodes.children[0].layout_result.size, size!(200.0, 300.0));
//         assert_eq!(nodes.children[0].layout_result.position.left, px!(0.0));
//         assert_eq!(nodes.children[0].layout_result.position.top, px!(0.0));
//         assert_eq!(nodes.children[1].layout_result.size, size!(100.0, 100.0));
//         assert_eq!(nodes.children[1].layout_result.position.left, px!(200.0));
//         assert_eq!(nodes.children[1].layout_result.position.top, px!(0.0));
//     }

//     #[test]
//     fn test_padding() {
//         let mut nodes = node!(
//             Div::new(),
//             lay!(size: size!(300.0), padding: rect!(10.0, 20.0, 30.0, 40.0))
//         )
//         .push(node!(Div::new(), lay!(size: size_pct!(100.0, 100.0))));
//         nodes.calculate_layout(&crate::font_cache::FontCache::default(), 1.0);
//         assert_eq!(nodes.children[0].layout_result.size, size!(240.0, 260.0));
//         assert_eq!(nodes.children[0].layout_result.position.left, px!(20.0));
//         assert_eq!(nodes.children[0].layout_result.position.top, px!(10.0));
//     }

//     #[test]
//     fn test_padding_pct() {
//         let mut nodes = node!(
//             Div::new(),
//             lay!(
//                 size: size!(300.0),
//                 padding: rect_pct!(10.0, 20.0, 30.0, 40.0)
//             )
//         )
//         .push(node!(Div::new(), lay!(size: size_pct!(100.0, 100.0))));
//         nodes.calculate_layout(&crate::font_cache::FontCache::default(), 1.0);
//         assert_eq!(nodes.children[0].layout_result.size, size!(120.0, 180.0));
//         assert_eq!(nodes.children[0].layout_result.position.left, px!(60.0));
//         assert_eq!(nodes.children[0].layout_result.position.top, px!(30.0));
//     }

//     #[test]
//     fn test_margin() {
//         let mut nodes = node!(Div::new(), lay!(size: size!(300.0)))
//             .push(node!(
//                 Div::new(),
//                 lay!(
//                     size: size_pct!(50.0, 100.0),
//                     margin: rect!(5.0, 10.0, 15.0, 20.0)
//                 )
//             ))
//             .push(node!(
//                 Div::new(),
//                 lay!(
//                     size: size_pct!(50.0, 100.0),
//                     margin: rect!(15.0, 10.0, 5.0, 20.0)
//                 )
//             ));
//         nodes.calculate_layout(&crate::font_cache::FontCache::default(), 1.0);
//         assert_eq!(nodes.children[0].layout_result.size, size!(120.0, 280.0));
//         assert_eq!(nodes.children[0].layout_result.position.left, px!(10.0));
//         assert_eq!(nodes.children[0].layout_result.position.top, px!(5.0));
//         assert_eq!(nodes.children[1].layout_result.size, size!(120.0, 280.0));
//         assert_eq!(nodes.children[1].layout_result.position.left, px!(160.0));
//         assert_eq!(nodes.children[1].layout_result.position.top, px!(15.0));
//     }

//     #[test]
//     fn test_margin_pct() {
//         let mut nodes = node!(Div::new(), lay!(size: size!(300.0)))
//             .push(node!(
//                 Div::new(),
//                 lay!(
//                     size: size_pct!(50.0, 100.0),
//                     margin: rect_pct!(5.0, 10.0, 15.0, 20.0),
//                 )
//             ))
//             .push(node!(
//                 Div::new(),
//                 lay!(
//                     size: size_pct!(50.0, 100.0),
//                     margin: rect_pct!(15.0, 10.0, 5.0, 20.0),
//                 )
//             ));
//         nodes.calculate_layout(&crate::font_cache::FontCache::default(), 1.0);
//         assert_eq!(nodes.children[0].layout_result.size, size!(60.0, 240.0));
//         assert_eq!(nodes.children[0].layout_result.position.left, px!(30.0));
//         assert_eq!(nodes.children[0].layout_result.position.top, px!(15.0));
//         assert_eq!(nodes.children[1].layout_result.size, size!(60.0, 240.0));
//         assert_eq!(nodes.children[1].layout_result.position.left, px!(180.0));
//         assert_eq!(nodes.children[1].layout_result.position.top, px!(45.0));
//     }

//     #[test]
//     fn test_auto() {
//         let mut nodes = node!(
//             Div::new(),
//             lay!(direction: Direction::Row, padding: rect!(10.0))
//         )
//         .push(node!(Div::new(), lay!(size: size!(150.0))))
//         .push(node!(Div::new(), lay!(size: size!(100.0))))
//         .push(node!(
//             Div::new(),
//             lay!(size: size!(200.0), margin: rect!(2.0))
//         ));
//         nodes.calculate_layout(&crate::font_cache::FontCache::default(), 1.0);
//         assert_eq!(
//             nodes.layout_result.size,
//             size!(
//                 10.0 + 150.0 + 100.0 + 2.0 + 200.0 + 2.0 + 10.0,
//                 10.0 + 2.0 + 200.0 + 2.0 + 10.0
//             )
//         );
//     }

//     #[test]
//     fn test_auto_no_children() {
//         let mut nodes = node!(
//             Div::new(),
//             lay!(direction: Direction::Row, min_size: size!(250.0, 300.0))
//         );
//         nodes.calculate_layout(&crate::font_cache::FontCache::default(), 1.0);
//         assert_eq!(nodes.layout_result.size, size!(250.0, 300.0));
//     }

//     #[test]
//     fn test_end_alignment() {
//         let mut nodes = node!(
//             Div::new(),
//             lay!(size: size!(300.0), direction: Direction::Row,
//                  wrap: true, axis_alignment: Alignment::End, cross_alignment: Alignment::End)
//         )
//         .push(node!(Div::new(), lay!(size: size!(150.0)))) // Child 0
//         .push(node!(Div::new(), lay!(size: size!(100.0)))) // Child 1
//         .push(node!(Div::new(), lay!(size: size!(200.0)))); // Child 2

//         nodes.calculate_layout(&crate::font_cache::FontCache::default(), 1.0);
//         assert_eq!(nodes.layout_result.size, size!(300.0));

//         assert_eq!(nodes.children[0].layout_result.position.right, px!(300.0));
//         assert_eq!(nodes.children[0].layout_result.position.bottom, px!(100.0));
//         assert_eq!(nodes.children[0].layout_result.position.left, px!(150.0));
//         assert_eq!(nodes.children[0].layout_result.position.top, px!(-50.0));

//         assert_eq!(nodes.children[1].layout_result.position.right, px!(100.0));
//         assert_eq!(nodes.children[1].layout_result.position.bottom, px!(300.0));

//         assert_eq!(nodes.children[2].layout_result.position.right, px!(300.0));
//         assert_eq!(nodes.children[2].layout_result.position.bottom, px!(300.0));
//     }

//     #[test]
//     fn test_center_alignment() {
//         let mut nodes = node!(
//             Div::new(),
//             lay!(size: size!(415.0), // This is just small enough to force a wrap
//                  direction: Direction::Row,
//                  padding: rect!(5.0), wrap: true,
//                  axis_alignment: Alignment::Center, cross_alignment: Alignment::Center)
//         )
//         .push(node!(
//             Div::new(),
//             lay!(size: size!(100.0), margin: rect!(1.0))
//         ))
//         .push(node!(
//             Div::new(),
//             lay!(size: size!(200.0), margin: rect!(1.0))
//         ))
//         .push(node!(
//             Div::new(),
//             lay!(size: size!(100.0), margin: rect!(1.0))
//         ));
//         nodes.calculate_layout(&crate::font_cache::FontCache::default(), 1.0);
//         assert_eq!(nodes.layout_result.size, size!(415.0));
//         assert_eq!(nodes.children[0].layout_result.position.left, px!(56.5));
//         assert_eq!(nodes.children[0].layout_result.position.top, px!(56.5));
//         assert_eq!(nodes.children[1].layout_result.position.left, px!(158.5));
//         assert_eq!(nodes.children[1].layout_result.position.top, px!(56.5));
//         assert_eq!(nodes.children[2].layout_result.position.left, px!(157.5));
//         assert_eq!(nodes.children[2].layout_result.position.top, px!(258.5));
//     }

//     #[test]
//     fn test_absolute_positioning() {
//         let mut nodes = node!(
//             Div::new(),
//             lay!(size: size!(300.0), direction: Direction::Row, wrap: true)
//         )
//         .push(node!(Div::new(), lay!(size: size!(150.0)))) // Child 0
//         .push(node!(Div::new(), lay!(size: size!(100.0)))) // Child 1
//         .push(node!(Div::new(), lay!(size: size!(200.0)))) // Child 2
//         .push(node!(
//             // Child 3
//             Div::new(),
//             lay!(
//                 size: size!(100.0),
//                 position_type: PositionType::Absolute,
//                 position: rect!(Auto, Auto, 10.0, 10.0)
//             )
//         ));
//         nodes.calculate_layout(&crate::font_cache::FontCache::default(), 1.0);
//         assert_eq!(nodes.layout_result.size, size!(300.0));
//         assert_eq!(nodes.children[0].layout_result.position.left, px!(0.0));
//         assert_eq!(nodes.children[0].layout_result.position.top, px!(0.0));
//         assert_eq!(nodes.children[1].layout_result.position.left, px!(150.0));
//         assert_eq!(nodes.children[1].layout_result.position.top, px!(0.0));
//         assert_eq!(nodes.children[2].layout_result.position.left, px!(0.0));
//         assert_eq!(nodes.children[2].layout_result.position.top, px!(150.0));
//         assert_eq!(nodes.children[3].layout_result.position.left, px!(190.0));
//         assert_eq!(nodes.children[3].layout_result.position.top, px!(190.0));
//     }
// }

use std::hash::Hash;
use std::ops::Neg;

use crate::component::{Component, ComponentHasher, RenderContext};
use crate::event;
use crate::layout::*;
use crate::renderables::rect::InstanceBuilder;
use crate::renderables::{Rect, Renderable};
use crate::style::{HorizontalPosition, StyleVal, Styled, VerticalPosition};
use crate::types::*;

use mctk_macros::{component, state_component_impl};

const MIN_BAR_SIZE: f32 = 10.0;

#[derive(Debug, Default)]
pub struct DivState {
    scroll_position: Point,
    x_scroll_bar: Option<AABB>,
    y_scroll_bar: Option<AABB>,
    over_y_bar: bool,
    y_bar_pressed: bool,
    over_x_bar: bool,
    x_bar_pressed: bool,
    drag_start_position: Point,
    scaled_scroll_bar_width: f32,
}

#[component(State = "DivState", Styled = "Scroll", Internal)]
#[derive(Debug, Default)]
pub struct Div {
    pub background: Option<Color>,
    pub border_color: Option<Color>,
    pub border_width: Option<f32>,
    pub radius: Option<(f32, f32, f32, f32)>,
}

impl Div {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bg<C: Into<Color>>(mut self, bg: C) -> Self {
        self.background = Some(bg.into());
        self
    }

    pub fn border<C: Into<Color>>(
        mut self,
        color: C,
        width: f32,
        radius: (f32, f32, f32, f32),
    ) -> Self {
        self.border_color = Some(color.into());
        self.border_width = Some(width);
        self.radius = Some(radius);
        self
    }

    pub fn scroll_x(mut self) -> Self {
        self = self.style("x", true);
        self.state = Some(DivState::default());
        self
    }

    pub fn scroll_y(mut self) -> Self {
        self = self.style("y", true);
        self.state = Some(DivState::default());
        self
    }

    fn x_scrollable(&self) -> bool {
        self.style_val("x").unwrap().into()
    }

    fn y_scrollable(&self) -> bool {
        self.style_val("y").unwrap().into()
    }

    fn scrollable(&self) -> bool {
        self.x_scrollable() || self.y_scrollable()
    }

    fn handle_drag_start(&mut self) {
        let x_bar_pressed = self.state_ref().over_x_bar;
        let y_bar_pressed = self.state_ref().over_y_bar;
        let drag_start = self.state_ref().scroll_position;
        self.state_mut().x_bar_pressed = x_bar_pressed;
        self.state_mut().y_bar_pressed = y_bar_pressed;
        self.state_mut().drag_start_position = drag_start;
    }

    fn handle_on_drag(
        &mut self,
        current_physical_aabb: AABB,
        current_inner_scale: Option<Scale>,
        physical_delta: Point,
    ) {
        if self.scrollable() {
            let start_position = self.state_ref().drag_start_position;
            let size = current_physical_aabb.size();
            let inner_scale = current_inner_scale.unwrap();
            let mut scroll_position = self.state_ref().scroll_position;

            if self.state_ref().y_bar_pressed {
                let drag = physical_delta.y;
                let delta_position = drag * (inner_scale.height / size.height);
                let max_position = inner_scale.height - size.height;
                scroll_position.y = (start_position.y + delta_position)
                    .round()
                    .min(max_position)
                    .max(0.0);
            }

            if self.state_ref().x_bar_pressed {
                let drag = physical_delta.x;
                let delta_position = drag * (inner_scale.width / size.width);
                let max_position = inner_scale.width - size.width;
                scroll_position.x = (start_position.x + delta_position)
                    .round()
                    .min(max_position)
                    .max(0.0);
            }

            if self.y_scrollable() {
                let drag = physical_delta.y.neg();
                let delta_position = drag * (inner_scale.height / size.height);
                let max_position = inner_scale.height - size.height;
                scroll_position.y = (start_position.y + delta_position)
                    .round()
                    .min(max_position)
                    .max(0.0);
            }

            if self.x_scrollable() {
                let drag = physical_delta.x.neg();
                let delta_position = drag * (inner_scale.width / size.width);
                let max_position = inner_scale.width - size.width;
                scroll_position.x = (start_position.x + delta_position)
                    .round()
                    .min(max_position)
                    .max(0.0);
            }

            self.state_mut().scroll_position = scroll_position;
        }
    }

    fn handle_drag_end(&mut self) {
        if self.scrollable() {
            self.state_mut().x_bar_pressed = false;
            self.state_mut().y_bar_pressed = false;
        }
    }
}

#[state_component_impl(DivState)]
impl Component for Div {
    fn render_hash(&self, hasher: &mut ComponentHasher) {
        if self.state.is_some() {
            self.state_ref().scroll_position.hash(hasher);
            self.state_ref().over_y_bar.hash(hasher);
            self.state_ref().over_x_bar.hash(hasher);
            self.state_ref().y_bar_pressed.hash(hasher);
            self.state_ref().x_bar_pressed.hash(hasher);
        }
        if let Some(color) = self.background {
            color.hash(hasher);
        }
        // Maybe TODO: Should hash scroll_descriptor
    }

    fn on_scroll(&mut self, event: &mut event::Event<event::Scroll>) {
        if self.scrollable() {
            let mut scroll_position = self.state_ref().scroll_position;
            let mut scrolled = false;
            let size = event.current_physical_aabb().size();
            let inner_scale = event.current_inner_scale().unwrap();

            if self.y_scrollable() {
                if event.input.y > 0.0 {
                    let max_position = inner_scale.height - size.height;
                    if scroll_position.y < max_position {
                        scroll_position.y += event.input.y;
                        scroll_position.y = scroll_position.y.min(max_position);
                        scrolled = true;
                    }
                } else if event.input.y < 0.0 && scroll_position.y > 0.0 {
                    if scroll_position.y + size.height > inner_scale.height {
                        scroll_position.y = inner_scale.height - size.height;
                    }
                    scroll_position.y += event.input.y;
                    scroll_position.y = scroll_position.y.max(0.0);
                    scrolled = true;
                }
            }

            if self.x_scrollable() {
                if event.input.x > 0.0 {
                    let max_position = inner_scale.width - size.width;
                    if scroll_position.x < max_position {
                        scroll_position.x += event.input.x;
                        scroll_position.x = scroll_position.x.min(max_position);
                        scrolled = true;
                    }
                } else if event.input.x < 0.0 && scroll_position.x > 0.0 {
                    if scroll_position.x + size.width > inner_scale.width {
                        scroll_position.x = inner_scale.width - size.width;
                    }
                    scroll_position.x += event.input.x;
                    scroll_position.x = scroll_position.x.max(0.0);
                    scrolled = true;
                }
            }

            if scrolled {
                self.state_mut().scroll_position = scroll_position;
                event.stop_bubbling();
            }
        }
    }

    fn on_mouse_motion(&mut self, event: &mut event::Event<event::MouseMotion>) {
        if self.scrollable() {
            let over_y_bar = self
                .state_ref()
                .y_scroll_bar
                .map(|b| b.is_under(event.relative_physical_position()))
                .unwrap_or(false);
            let over_x_bar = self
                .state_ref()
                .x_scroll_bar
                .map(|b| b.is_under(event.relative_physical_position()))
                .unwrap_or(false);

            if self.state_ref().over_y_bar != over_y_bar
                || self.state_ref().over_x_bar != over_x_bar
            {
                self.state_mut().over_y_bar = over_y_bar;
                self.state_mut().over_x_bar = over_x_bar;
            }
            event.stop_bubbling();
        }
    }

    fn on_mouse_leave(&mut self, _event: &mut event::Event<event::MouseLeave>) {
        if self.scrollable() {
            self.state_mut().over_y_bar = false;
            self.state_mut().over_x_bar = false;
        }
    }

    fn on_drag_start(&mut self, event: &mut event::Event<event::DragStart>) {
        if self.scrollable() {
            self.handle_drag_start();
            event.stop_bubbling();
        }
    }

    fn on_touch_drag_start(&mut self, event: &mut event::Event<event::TouchDragStart>) {
        if self.scrollable() {
            self.handle_drag_start();
            event.stop_bubbling();
        }
    }

    fn on_drag_end(&mut self, _event: &mut event::Event<event::DragEnd>) {
        self.handle_drag_end();
    }

    fn on_touch_drag_end(&mut self, _event: &mut event::Event<event::TouchDragEnd>) {
        self.handle_drag_end();
    }

    fn on_drag(&mut self, event: &mut event::Event<event::Drag>) {
        self.handle_on_drag(
            event.current_physical_aabb(),
            event.current_inner_scale(),
            event.physical_delta(),
        );
    }

    fn on_touch_drag(&mut self, event: &mut event::Event<event::TouchDrag>) {
        self.handle_on_drag(
            event.current_physical_aabb(),
            event.current_inner_scale(),
            event.physical_delta(),
        );
    }

    fn scroll_position(&self) -> Option<ScrollPosition> {
        if self.scrollable() {
            let p = self.state_ref().scroll_position;
            Some(ScrollPosition {
                x: if self.x_scrollable() { Some(p.x) } else { None },
                y: if self.y_scrollable() { Some(p.y) } else { None },
            })
        } else {
            None
        }
    }

    fn frame_bounds(&self, aabb: AABB, inner_scale: Option<Scale>) -> AABB {
        let mut aabb = aabb;
        if self.scrollable() {
            let inner_scale = inner_scale.unwrap();
            let scaled_width = self.state_ref().scaled_scroll_bar_width;
            let size = aabb.size();
            let max_position = inner_scale - size;

            if self.y_scrollable() && max_position.height > 0.0 {
                if self.style_val("y_bar_position")
                    == Some(StyleVal::HorizontalPosition(HorizontalPosition::Left))
                {
                    aabb.pos.x += scaled_width;
                } else {
                    aabb.bottom_right.x -= scaled_width;
                }
            }

            if self.x_scrollable() && max_position.width > 0.0 {
                if self.style_val("x_bar_position")
                    == Some(StyleVal::VerticalPosition(VerticalPosition::Top))
                {
                    aabb.pos.y += scaled_width;
                } else {
                    aabb.bottom_right.y -= scaled_width;
                }
            }
        }

        aabb
    }

    fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
        let mut rs = vec![];
        let border_width = self
            .border_width
            .map_or(0.0, |x| (x * context.scale_factor.floor()).round());

        if let Some(bg) = self.background {
            // println!("Background color {:?} {:?}", bg, context.scissor);
            let mut rect_instance = InstanceBuilder::default()
                .pos(Pos {
                    x: context.aabb.pos.x,
                    y: context.aabb.pos.y,
                    z: 0.1,
                })
                .scale(context.aabb.size())
                .color(bg)
                .build()
                .unwrap();
            if let Some(radius) = self.radius {
                rect_instance.radius = radius;
            };

            rs.push(Renderable::Rect(Rect::from_instance_data(rect_instance)))
        }

        if let (Some(color), Some(width), Some(radius)) =
            (self.border_color, self.border_width, self.radius)
        {
            let rect_instance = InstanceBuilder::default()
                .pos(context.aabb.pos)
                .scale(context.aabb.size())
                .border_color(color)
                .border_size(width)
                .radius(radius)
                .build()
                .unwrap();
            rs.push(Renderable::Rect(Rect::from_instance_data(rect_instance)))
        }

        if self.scrollable() {
            let scroll_position = self.state_ref().scroll_position;
            let inner_scale = context.inner_scale.unwrap();
            let size = context.aabb.size();
            let scaled_width = self.style_val("bar_width").unwrap().f32() * context.scale_factor;
            self.state_mut().scaled_scroll_bar_width = scaled_width;

            let max_position = inner_scale - size;

            if self.y_scrollable() {
                if max_position.height > 0.0 {
                    let x = if self.style_val("y_bar_position")
                        == Some(StyleVal::HorizontalPosition(HorizontalPosition::Left))
                    {
                        0.0
                    } else {
                        size.width - scaled_width
                    };

                    let x_scroll_bar = self.x_scrollable() && max_position.width > 0.0;
                    let bar_background_height =
                        size.height - if x_scroll_bar { scaled_width } else { 0.0 };
                    let bar_y_offset = if x_scroll_bar
                        && self.style_val("x_bar_position")
                            == Some(StyleVal::VerticalPosition(VerticalPosition::Top))
                    {
                        scaled_width
                    } else {
                        0.0
                    };

                    let bar_background = Rect::new(
                        Pos {
                            x,
                            y: bar_y_offset,
                            z: 0.1, // above background
                        },
                        Scale {
                            width: scaled_width,
                            height: bar_background_height,
                        },
                        self.style_val("bar_background_color").into(),
                    );

                    let height = (bar_background_height * (size.height / inner_scale.height))
                        .max(MIN_BAR_SIZE);
                    let mut y = (bar_background_height - height)
                        * (scroll_position.y / max_position.height)
                        + bar_y_offset;
                    if height + y > bar_background_height {
                        y = bar_background_height - height;
                    }

                    let bar_aabb = AABB::new(
                        Pos {
                            x: x + 2.0,
                            y,
                            z: 0.2, // above bar background
                        },
                        Scale {
                            width: scaled_width - 4.0,
                            height,
                        },
                    );
                    let color: Color = if self.state_ref().y_bar_pressed {
                        self.style_val("bar_active_color").into()
                    } else if self.state_ref().over_y_bar {
                        self.style_val("bar_highlight_color").into()
                    } else {
                        self.style_val("bar_color").into()
                    };
                    let bar = Rect::new(bar_aabb.pos, bar_aabb.size(), color);
                    self.state_mut().y_scroll_bar = Some(bar_aabb);
                    rs.push(Renderable::Rect(bar_background));
                    rs.push(Renderable::Rect(bar));
                } else {
                    self.state_mut().y_scroll_bar = None;
                }
            }

            if self.x_scrollable() {
                if max_position.width > 0.0 {
                    let y = if self.style_val("x_bar_position")
                        == Some(StyleVal::VerticalPosition(VerticalPosition::Top))
                    {
                        0.0
                    } else {
                        size.height - scaled_width
                    };

                    let y_scroll_bar = self.y_scrollable() && max_position.height > 0.0;
                    let bar_background_width =
                        size.width - if y_scroll_bar { scaled_width } else { 0.0 };
                    let bar_x_offset = if y_scroll_bar
                        && self.style_val("y_bar_position")
                            == Some(StyleVal::HorizontalPosition(HorizontalPosition::Left))
                    {
                        scaled_width
                    } else {
                        0.0
                    };

                    let bar_background = Rect::new(
                        Pos {
                            x: bar_x_offset,
                            y,
                            z: 0.1, // above background
                        },
                        Scale {
                            width: bar_background_width,
                            height: scaled_width,
                        },
                        self.style_val("bar_background_color").into(),
                    );

                    let width =
                        (bar_background_width * (size.width / inner_scale.width)).max(MIN_BAR_SIZE);
                    let mut x = (bar_background_width - width)
                        * (scroll_position.x / max_position.width)
                        + bar_x_offset;
                    if width + x > bar_background_width {
                        x = bar_background_width - width;
                    }

                    let bar_aabb = AABB::new(
                        Pos {
                            x,
                            y: y + 2.0,
                            z: 0.2, // above bar background
                        },
                        Scale {
                            width,
                            height: scaled_width - 4.0,
                        },
                    );
                    let color = if self.state_ref().x_bar_pressed {
                        self.style_val("bar_active_color").into()
                    } else if self.state_ref().over_x_bar {
                        self.style_val("bar_highlight_color").into()
                    } else {
                        self.style_val("bar_color").into()
                    };
                    let bar = Rect::new(bar_aabb.pos, bar_aabb.size(), color);
                    self.state_mut().x_scroll_bar = Some(bar_aabb);
                    rs.push(Renderable::Rect(bar_background));
                    rs.push(Renderable::Rect(bar));
                } else {
                    self.state_mut().x_scroll_bar = None;
                }
            }
        }

        Some(rs)
    }
}

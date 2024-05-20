use crate::component::{Component, ComponentHasher, RenderContext};
use crate::event::{self, Event};
use crate::layout::*;
use crate::renderables::{Rect, Renderable};
use crate::style::{HorizontalPosition, StyleVal, Styled, VerticalPosition};
use crate::types::*;
use std::cmp;
use std::hash::Hash;
use std::ops::Neg;

use mctk_macros::{component, state_component_impl};

#[derive(Debug, Default, Clone)]
pub struct CarouselItem {}

#[derive(Debug, Default)]
pub struct TransitionPositions {
    pub from: Point,
    pub to: Point,
    pub velocity: f32,
}

#[derive(Debug, Default)]
pub struct CarouselState {
    scroll_position: Point,
    drag_start_position: Point,
    dragged_over_child: Option<AABB>,
    transition_positions: Option<TransitionPositions>,
}

#[component(State = "CarouselState", Styled = "Scroll", Internal)]
#[derive(Debug, Default)]
pub struct Carousel {}

impl Carousel {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn scroll_x(mut self) -> Self {
        self = self.style("x", true);
        self.state = Some(CarouselState::default());
        self
    }

    pub fn scroll_y(mut self) -> Self {
        self = self.style("y", true);
        self.state = Some(CarouselState::default());
        self
    }

    fn x_scrollable(&self) -> bool {
        // println!("x_scrollable {:?}", self.style_val("x").unwrap());
        self.style_val("x").unwrap().into()
    }

    fn y_scrollable(&self) -> bool {
        self.style_val("y").unwrap().into()
    }

    fn scrollable(&self) -> bool {
        self.x_scrollable() || self.y_scrollable()
    }
}
#[state_component_impl(CarouselState)]
impl Component for Carousel {
    fn on_tick(&mut self, event: &mut Event<event::Tick>) {
        //Update scroll position based on velocity and frames per seconds
        if let Some(TransitionPositions { from, to, velocity }) =
            self.state_ref().transition_positions
        {
            let mut scroll_position = self.state_ref().scroll_position;
            let distance = from.dist(to).round();
            let distance_scrolled = scroll_position.dist(from).round();

            // println!(
            //     "{:?} {:?} {:?}",
            //     distance,
            //     scroll_position,
            //     distance_scrolled != distance
            // );
            // println!(
            //     "from {:?} to {:?} scolling position {:?}",
            //     from, to, scroll_position
            // );

            if distance != 0. && scroll_position != to && distance_scrolled != distance {
                if to.x >= from.x {
                    scroll_position.x += distance * velocity;
                } else {
                    scroll_position.x -= distance * velocity;
                }

                self.state_mut().scroll_position = scroll_position;
            } else {
                self.state_mut().transition_positions = None;
            }
        }
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
    fn render_hash(&self, hasher: &mut ComponentHasher) {
        if self.state.is_some() {
            self.state_ref().scroll_position.hash(hasher);
            self.state_ref().drag_start_position.hash(hasher);
        }
    }

    fn on_drag_start(&mut self, event: &mut Event<event::DragStart>) {
        //println!("drag start from {:?}", self.state_ref().scroll_position);
        // println!(
        //     "over child {:?} current child width {:?}",
        //     event.over_child_n(),
        //     event.over_child_n_aabb()
        // );
        let drag_start = self.state_ref().scroll_position;
        self.state_mut().drag_start_position = drag_start;
        self.state_mut().dragged_over_child = event.over_child_n_aabb();
        self.state_mut().transition_positions = None;
        event.stop_bubbling();
    }
    fn on_drag_end(&mut self, event: &mut Event<event::DragEnd>) {
        let dragged_on_child = self.state_ref().dragged_over_child.unwrap();
        let child_width = dragged_on_child.width();

        let from_position = self.state_ref().scroll_position;
        let prev_slide_x = (from_position.x / child_width).floor() * child_width;
        let next_slide_x = prev_slide_x + child_width;
        println!(
            "from_position {} prev_slide_x {} next_slide_x {} {}",
            from_position.x, prev_slide_x, next_slide_x, child_width
        );
        let mut to_position = Point::default();
        to_position.x = match (from_position.x.abs() % child_width) >= (0.50 * child_width) {
            true => next_slide_x.floor(),
            false => prev_slide_x.floor(),
        };

        //println!("to_position.x {:?}", to_position.x);
        self.state_mut().transition_positions = Some(TransitionPositions {
            from: from_position,
            to: to_position,
            velocity: 0.02,
        });
        event.stop_bubbling();
    }

    fn on_drag(&mut self, event: &mut Event<event::Drag>) {
        let start_position = self.state_ref().drag_start_position;
        let size = event.current_physical_aabb().size();
        let inner_scale = event.current_inner_scale().unwrap();
        let mut scroll_position = self.state_ref().scroll_position;
        let drag = event.physical_delta().x.neg();

        //println!("dragging delta {:?}", drag);

        let delta_position = drag;
        let max_position = inner_scale.width - size.width;
        //println!("on_drag {:?} {:?}", start_position.x, delta_position);
        scroll_position.x = (start_position.x + delta_position)
            .round()
            .min(max_position)
            .max(0.0);

        self.state_mut().scroll_position = scroll_position;

        // event.stop_bubbling();
    }
}

use mctk_macros::component;

use crate::component::{Component, ComponentHasher, Message, RenderContext};

use crate::event::{self, Event};
use crate::renderables::types::{Point, Size};
use crate::renderables::{
    circle::InstanceBuilder as CircleInstanceBuilder, line::InstanceBuilder as LineInstanceBuilder,
    rect::InstanceBuilder as RectInstanceBuilder,
};
use crate::renderables::{Circle, Line, Rect, Renderable};
use crate::{lay, msg, node, size, size_pct, types::*, Node};
use std::hash::Hash;
use std::ops::Neg;

#[derive(Debug, Default)]
struct SliderState {}

#[component(State = "SliderState", Styled, Internal)]
pub struct Slider {
    pub value: i32,
    pub on_slide: Option<Box<dyn Fn(i32) -> Message + Send + Sync>>,
}

#[derive(Debug)]
enum SliderMsg {
    ValueChanged(i32),
}

impl Default for Slider {
    fn default() -> Self {
        Self {
            value: 0,
            on_slide: None,
            state: Some(SliderState::default()),
            dirty: false,
            class: Default::default(),
            style_overrides: Default::default(),
        }
    }
}

impl std::fmt::Debug for Slider {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Slider")
            .field("value", &self.value)
            .finish()
    }
}

impl Slider {
    pub fn new(value: i32) -> Self {
        Self {
            value,
            on_slide: None,
            state: Some(SliderState::default()),
            dirty: false,
            class: Default::default(),
            style_overrides: Default::default(),
        }
    }

    pub fn on_slide(mut self, f: Box<dyn Fn(i32) -> Message + Send + Sync>) -> Self {
        self.on_slide = Some(f);
        self
    }
}

impl Component for Slider {
    fn render_hash(&self, hasher: &mut ComponentHasher) {
        (self.value as i32).hash(hasher);
        // (self.state).hash(hasher);
    }

    fn update(&mut self, msg: Message) -> Vec<Message> {
        let mut m: Vec<Message> = vec![];
        match msg.downcast_ref::<SliderMsg>() {
            Some(SliderMsg::ValueChanged(value)) => {
                //println!("slider update value {:?}", value);
                if let Some(slide_fn) = &self.on_slide {
                    m.push(slide_fn(*value));
                }
            }
            _ => (),
        }
        m
    }

    fn on_mouse_down(&mut self, event: &mut Event<event::MouseDown>) {
        event.stop_bubbling();
        let click_position = event.relative_logical_position();
        println!("mouse down postion is {:?}", click_position);

        let slider_width = event.current_aabb.unwrap().width();
        let value_changed = click_position.x / slider_width * 100.;
        if let Some(slide_fn) = &self.on_slide {
            event.emit(slide_fn(value_changed.min(100.).max(0.) as i32));
        }
    }

    fn on_touch_down(&mut self, event: &mut Event<event::TouchDown>) {
        event.stop_bubbling();
        let click_position = event.relative_logical_position();
        println!("touch down postion is {:?}", click_position);

        let slider_width = event.current_aabb.unwrap().width();
        let value_changed = click_position.x.neg() / slider_width * 100.;
        if let Some(slide_fn) = &self.on_slide {
            event.emit(slide_fn(value_changed.min(100.).max(0.) as i32));
        }
    }

    fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
        let width = context.aabb.width();
        let height = context.aabb.height();
        let AABB { pos, .. } = context.aabb;

        let mut rs = vec![];

        //Outer box
        let rect_instance_data = RectInstanceBuilder::default()
            .pos(pos)
            .scale(Scale { width, height })
            .color(Color::TRANSPARENT)
            .build()
            .unwrap();
        rs.push(Renderable::Rect(Rect::from_instance_data(
            rect_instance_data,
        )));

        let start = Pos {
            x: pos.x,
            y: pos.y + height / 2.,
            z: 0.,
        };

        let end = Pos {
            x: pos.x + width,
            y: pos.y + height / 2.,
            z: 0.,
        };

        //Horizontal BG
        let line_instance_data = LineInstanceBuilder::default()
            .from(start)
            .to(end)
            .color(Color::rgb(64., 64., 68.))
            .width(4.0)
            .build()
            .unwrap();
        rs.push(Renderable::Line(Line::from_instance_data(
            line_instance_data,
        )));

        let filled_end = Pos {
            x: pos.x + width * self.value as f32 / 100.,
            y: pos.y + height / 2.,
            z: 0.,
        };

        //Horizontal Line
        let line_instance_data = LineInstanceBuilder::default()
            .from(start)
            .to(filled_end)
            .color(Color::WHITE)
            .width(4.0)
            .build()
            .unwrap();
        rs.push(Renderable::Line(Line::from_instance_data(
            line_instance_data,
        )));

        //Circle
        // let radius = 10.;
        // let circle_instance_data = CircleInstanceBuilder::default()
        //     .origin(Pos {
        //         x: pos.x + radius,
        //         y: pos.y + height / 2.,
        //         z: 0.,
        //     })
        //     .radius(radius)
        //     .build()
        //     .unwrap();
        // rs.push(Renderable::Circle(Circle::from_instance_data(
        //     circle_instance_data,
        // )));

        // let mut pointer = Pointer {};
        // let x = pointer.render(context).unwrap();

        Some(rs)
    }

    fn view(&self) -> Option<Node> {
        //println!("Slider view {}", self.value);

        Some(node!(Pointer {
            value: self.value,
        }, [ size_pct: [96, 100] ]))
    }
}

pub struct Pointer {
    pub value: i32,
}

impl std::fmt::Debug for Pointer {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Pointer")
            .field("value", &self.value)
            .finish()
    }
}

impl Component for Pointer {
    fn render_hash(&self, hasher: &mut ComponentHasher) {
        (self.value as i32).hash(hasher);
        // (self.state).hash(hasher);
    }

    fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
        //println!("Pointer render {}", self.value);

        let width = context.aabb.width();
        let height = context.aabb.height();
        let AABB { pos, .. } = context.aabb;
        let mut rs = vec![];

        let radius = 9.;
        let circle_instance_data = CircleInstanceBuilder::default()
            .origin(Pos {
                x: pos.x + radius / 2. + self.value as f32 * width / 100.,
                y: pos.y + height / 2.,
                z: 0.,
            })
            .radius(radius)
            .build()
            .unwrap();
        rs.push(Renderable::Circle(Circle::from_instance_data(
            circle_instance_data,
        )));

        Some(rs)
    }

    fn on_drag_start(&mut self, event: &mut Event<event::DragStart>) {
        //println!("Drag start. Got child {:?}", event.over_subchild_n(),);
        event.stop_bubbling();
    }

    fn on_drag(&mut self, event: &mut Event<event::Drag>) {
        //println!("Dragging {:?}", event.physical_mouse_position());
        // println!(
        //     "Delta {:?} {:?} {:?} {:?}",
        //     event.logical_delta(),
        //     event.physical_delta(),
        //     event.bounded_physical_delta(),
        //     event.bounded_logical_delta(),
        // );

        let slider_position = event.relative_logical_position();
        let slider_width = event.current_aabb.unwrap().width();

        let value_changed = slider_position.x / slider_width * 100.;

        //println!("value_changed{:?}", value_changed as i32);
        event.emit(msg!(SliderMsg::ValueChanged(
            value_changed.min(100.).max(0.) as i32
        )));
    }

    fn on_drag_end(&mut self, event: &mut Event<event::DragEnd>) {
        //println!("Drag stop at {:?}", event.relative_logical_position());
    }
}

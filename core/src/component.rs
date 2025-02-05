use std::any::Any;
use std::fmt;

use crate::event::{self, Event};
use crate::font_cache::FontCache;
use crate::layout::*;
use crate::node::Node;
use crate::renderables::types::Canvas;
use crate::renderables::Renderable;
use crate::renderer::Caches;
use crate::types::*;
use crate::window::Window;
use ahash::AHasher;
use smithay_client_toolkit::reexports::calloop;

/// A `Box<dyn Any>` type, used to convey information from a [`Component`] to one of its parent nodes. Passed to [`Event#emit`][Event#method.emit].
pub type Message = Box<dyn Any>;
#[doc(hidden)]
// Only used by `replace_state` and `take_state`, which are not meant to be implemented by the user.
pub type State = Box<dyn Any>;
/// A concrete implementor of [`std::hash::Hasher`], used by [`Component#props_hash`][Component#method.props_hash] and [`#render_hash`][Component#method.render_hash].
///
/// [`AHasher`] is used, since it makes it easier to create reproducible hashes.
pub type ComponentHasher = AHasher;

/// Wrap the input in a [`Box#new`][Box#method.new]. Convenience for [`Message`] creation.
#[macro_export]
macro_rules! msg {
    ($e:expr) => {
        Box::new($e)
    };
}

/// Passed to [`Component#render`][Component#method.render], with context required for rendering.
#[derive(Clone)]
pub struct RenderContext<'a> {
    /// The `AABB` that contains the given [`Component`] instance.
    pub aabb: AABB,
    /// For scrollable Components (Components that return a `Some` value for [`#scroll_position`][Component#method.scroll_position]), this is the size of the child Nodes.
    pub inner_scale: Option<Scale>,
    /// The caches used by the renderer.
    pub caches: Caches<'a>,
    /// The value previously returned by [`Component#render`][Component#method.render] of the given instance.
    pub prev_state: Option<Vec<Renderable>>,
    /// The scale factor of the current monitor. Renderables should be scaled by this value.
    pub scale_factor: f32,
}

/// The primary interface of Lemna. Components are the -- optionally stateful -- elements that are drawn on a window that a user interacts with.
///
/// Implementing methods are optional, since defaults are provided for all. Provided methods will either do nothing -- returning an empty value like `None`, `vec![]`, or false where the signature has a return value -- or else the default behavior will be noted.
pub trait Component: fmt::Debug {
    /// Called every draw phase, Components return a Node which contains its child Component. If you wish for a Component to have multiple children, then wrap them in a [`Div`][crate::widgets::Div] (or some other container Component).
    ///
    /// In this fashion, Components can be built from other Components (for instance, a button can be build from a [`RoundedRect`][crate::widgets::RoundedRect] and a [`Text`][crate::widgets::Text]), and an app can be built from an even larger assemblage of Components.
    ///
    /// Not all Components need implement `view`. Some Components are built up from [`renderables`][crate::renderables] -- graphical primitives -- returned in the [`#render`][Component#method.render] method.
    ///
    /// Do not perform expensive computations in `view`. Use [`#init`][Component#method.init] or [`#new_props`][Component#method.new_props] instead.
    fn view(&self) -> Option<Node> {
        None
    }

    /// Called when a Node is first instantiated. Any computations (particularly expensive ones) that aren't related to [viewing][Component#view] or [rendering][Component#method.render] should be made here or in [`#new_props`][Component#method.new_props].
    fn init(&mut self) {}

    /// Called during the View phase any time [`#props_hash`][Component#method.props_hash] generates a new value relative to the Node's previous incarnation.
    fn new_props(&mut self) {}

    /// Called when a child Node has emitted a [`Message`] via [`Event#emit`][Event#method.emit], or if a child has passed on a `Message` from one of its descendants. The return value will be passed to the `update` of a Component's parent Node.
    ///
    /// By default this forwards any incoming Messages, returning `vec![msg]`.
    fn update(&mut self, msg: Message) -> Vec<Message> {
        vec![msg]
    }

    /// Called while rendering, Components may return [`renderables`][crate::renderables] -- graphical primitives -- from this method. In this way they can efficiently draw to the screen in ways that other Components are unable to.
    ///
    /// Many of the built-in [`widgets`][crate::widgets], like [`RoundedRect`][crate::widgets::RoundedRect] and [`Text`][crate::widgets::Text], implement the `render` method.
    fn render(&mut self, _context: RenderContext) -> Option<Vec<Renderable>> {
        None
    }

    /// Called to determine whether anything about the Component that will effect rendering has changed. If a Node's `render_hash` differs from the `render_hash` is previous incarnation had created, then [`#render`][Component#method.render] will be called.
    ///
    /// Defaults to [`#props_hash`][Component#method.props_hash].
    fn render_hash(&self, hasher: &mut ComponentHasher) {
        self.props_hash(hasher);
    }

    /// Called to determine whether the inputs to the Component have changed, and thus whether [`#new_props`][Component#method.new_props] should be called. Mutate the `hasher` (you will almost certainly want to import the [`std::hash::Hash`] trait, to make this method available on implementing types).
    ///
    /// There's no need to implement this method unless `new_props` is also implemented, or if it is the desired value for [`#render_hash`][Component#method.render_hash].
    fn props_hash(&self, _hasher: &mut ComponentHasher) {}

    /// Some Components are designed to have others embedded in them. If you don't return anything from the [`#view`][Component#method.view] method, then you can [`Node#push`][crate::Node#method.push] children onto the Node of Container.
    /// Otherwise, if you return a `Some` value from both `#view` and this method, then the value returned here is the index into the child node that [`Node#push`][crate::Node#method.push] will push children into.
    /// For instance `Some(vec![0, 1])` will cause children to be attached to second child of the first Node returned by `view`. A Node with that index _must_ exist after the call to this Component's `view`. In other words, it cannot be the index of a Node that's created by a child's `#view` method.
    ///
    /// `Some(vec![])` will attach children to the root of this container's node, after the one returned by `view`, if any.
    fn container(&self) -> Option<Vec<usize>> {
        None
    }

    /// Implemented by the `component` attribute macro
    #[doc(hidden)]
    fn replace_state(&mut self, _other: State) {}

    /// Implemented by the `component` attribute macro
    #[doc(hidden)]
    fn take_state(&mut self) -> Option<State> {
        None
    }

    /// Implemented by the `component` attribute macro
    #[doc(hidden)]
    fn is_dirty(&mut self) -> bool {
        false
    }

    /// Return the set of event types that you wish this Component to be sent. This lets
    /// a Component to receive key events even if it isn't focused on the root node.
    fn register(&mut self) -> Vec<event::Register> {
        vec![]
    }

    /// Is the `mouse_position` over this Component? Implement if the Component has
    /// non-rectangular geometry. Otherwise will default to `aabb.is_under(mouse_position)`.
    fn is_mouse_over(&self, mouse_position: Point, aabb: AABB) -> bool {
        aabb.is_under(mouse_position)
    }

    /// Is the `touch_position` over this Component? Implement if the Component has
    /// non-rectangular geometry. Otherwise will default to `aabb.is_under(touch_position)`.
    fn is_touch_over(&self, touch_position: Point, aabb: AABB) -> bool {
        aabb.is_under(touch_position)
    }

    /// TODO: Why does this exist? aabb is `inclusive_aabb`, which has something
    /// to do with scrollables, but why does that exist?
    fn is_mouse_maybe_over(&self, mouse_position: Point, aabb: AABB) -> bool {
        aabb.is_under(mouse_position)
    }

    /// Called during layout, this can be used to set the size of the Component
    /// based on some intrinsic properties, by returning a desired `(width, height)`. `None` values for width or height indicate that the layout engine should determine the size.
    ///
    /// The input `width` and `height` is the size that the layout engine believes the component should have, if it does have an opinion. The size returned should not exceed the `max_` width or height. The [`FontCache`] is also provided, so that text layout can inform the size of the Component. If laying out text, you should cache the glyphs so that you don't need to compute them every time `fill_bounds` is called.
    fn fill_bounds(
        &mut self,
        _width: Option<f32>,
        _height: Option<f32>,
        _max_width: Option<f32>,
        _max_height: Option<f32>,
        _font_cache: &mut FontCache,
        _scale_factor: f32,
    ) -> (Option<f32>, Option<f32>) {
        (None, None)
    }

    /// Give the Component full control over its own [`AABB`]. When this returns `true`, [`#set_aabb`][Component#method.set_aabb] will be called while drawing a given Node.
    fn full_control(&self) -> bool {
        false
    }

    /// Mutate `aabb` to set it to a new size or position. This Node's parent's `AABB`, information about its children (`AABB`, inner scale, and [`#focus`][Component#method.focus]), as well as the frame it exist in (either the window, or the scrollable frame) are provided. "Inner scale" is the size of the contents of a scrollable Component. Children's AABBs can also be mutated to adjust their size and position.
    ///
    /// Will only have an affect if [`#full_control`][Component#method.full_control] returns `true`.
    fn set_aabb(
        &mut self,
        _aabb: &mut AABB,
        _parent_aabb: AABB,
        _children: Vec<(&mut AABB, Option<Scale>, Option<Point>)>,
        _frame: AABB,
        _scale_factor: f32,
    ) {
    }

    /// Called when the child of a full control Node. This is used to communicate a position to the parent's [`#set_aabb`][Component#method.set_aabb], so that it can position/scroll itself appropriately.
    ///
    /// This is useful if e.g. creating a text box, and scrolling needs to be controlled by a cursor.
    fn focus(&self) -> Option<Point> {
        None
    }

    /// Return a `Some` value to make the Component considered scrollable. Return the current amount that the Component is scrolled by.
    ///
    /// The children of scrollable nodes are rendered in the position dictated by this response, and occluded by [`#frame_bounds`][Component#method.frame_bounds].
    fn scroll_position(&self) -> Option<ScrollPosition> {
        None
    }

    /// Should only be overridden by scrollable containers. Used to limit the bounds of the scrollable area.
    /// Should return an [`AABB`] that is inside the bounds of the input `aabb` which belongs to the current Node. `inner_scale` is the size of its child Nodes.
    ///
    /// By default this returns `aabb`.
    fn frame_bounds(&self, aabb: AABB, _inner_scale: Option<Scale>) -> AABB {
        aabb
    }

    /// Specifies spacing between its children
    fn spacing(&self) -> Scale {
        Scale::new(0.0, 0.0)
    }

    // Event handlers
    /// Handle mouse click events. These events will only be sent if the mouse is over the Component.
    fn on_click(&mut self, _event: &mut Event<event::Click>) {}
    /// Handle mouse double click events. These events will only be sent if the mouse is over the Component.
    fn on_double_click(&mut self, _event: &mut Event<event::DoubleClick>) {}
    /// Handle mouse down events. These events will only be sent if the mouse is over the Component.
    fn on_mouse_down(&mut self, _event: &mut Event<event::MouseDown>) {}
    /// Handle mouse up events. These events will only be sent if the mouse is over the Component.
    fn on_mouse_up(&mut self, _event: &mut Event<event::MouseUp>) {}
    /// Handle mouse-enter events. These events occur when the mouse first moves over the Component.
    fn on_mouse_enter(&mut self, _event: &mut Event<event::MouseEnter>) {}
    /// Handle mouse-leave events. These events occur when the mouse stops being over the Component.
    fn on_mouse_leave(&mut self, _event: &mut Event<event::MouseLeave>) {}
    /// Handle mouse motion events. These events will only be sent if the mouse is over the Component.
    fn on_mouse_motion(&mut self, _event: &mut Event<event::MouseMotion>) {}
    /// Handle touch down events. These events will only be sent if the touch is over the Component.
    fn on_touch_down(&mut self, _event: &mut Event<event::TouchDown>) {}
    /// Handle touch up events. These events will only be sent if the touch is over the Component.
    fn on_touch_up(&mut self, _event: &mut Event<event::TouchUp>) {}
    /// Handle touch up events. These events will only be sent if the touch is over the Component.
    fn on_touch_motion(&mut self, _event: &mut Event<event::TouchMotion>) {}
    /// Handle touch cancel events. These events will only be sent if the touch is over the Component.
    fn on_touch_cancel(&mut self, _event: &mut Event<event::TouchCancel>) {}
    /// Handle scroll events. These events will only be sent if the mouse is over the Component.
    fn on_scroll(&mut self, _event: &mut Event<event::Scroll>) {}
    /// Handle mouse drag events (i.e. the user clicks a mouse button over the Component and starts moving it). These events will only be sent if the mouse is over the Component.
    fn on_drag(&mut self, _event: &mut Event<event::Drag>) {}
    /// Handle touch drag events (i.e. the user touches over the Component and starts moving). These events will only be sent if the touch is over the Component.
    fn on_touch_drag(&mut self, _event: &mut Event<event::TouchDrag>) {}
    /// Handle the start of a mouse drag events (i.e. the user clicks a mouse button over the Component and starts moving it). These events will only be sent if the mouse is over the Component.
    fn on_drag_start(&mut self, _event: &mut Event<event::DragStart>) {}
    /// Handle the start of a touch drag events (i.e. the user  touches the Component and starts moving). These events will only be sent if the touch is over the Component.
    fn on_touch_drag_start(&mut self, _event: &mut Event<event::TouchDragStart>) {}
    /// Handle the end of a mouse drag events (i.e. the user clicks a mouse button over the Component and starts moving it). These events will only be sent if the mouse is over the Component.
    fn on_drag_end(&mut self, _event: &mut Event<event::DragEnd>) {}
    /// Handle the end of a touch drag events (i.e. the user touches over the Component and starts moving). These events will only be sent if the touch is over the Component.
    fn on_touch_drag_end(&mut self, _event: &mut Event<event::TouchDragEnd>) {}
    /// Handle focus events. This event occurs when [`Event#Focus`][crate::Event#method.focus] is called on an event belonging to this component.
    fn on_focus(&mut self, _event: &mut Event<event::Focus>) {}
    /// Handle blue events. This event occurs when this component loses its focus, either by another component gaining focus, or [`Event#blur`][crate::Event#method.blur] being called on an event belonging to this component.
    fn on_blur(&mut self, _event: &mut Event<event::Blur>) {}
    /// Handle tick events, which occur regularly on a short interval
    /// (window backend dependent). This can be used to create animated effects.
    fn on_tick(&mut self, _event: &mut Event<event::Tick>) {}
    /// Handle key down events. These events will only be sent if this component is focused or the [`Component#register`][crate::Component#method.register] method returns [`Register::KeyDown`][crate::event::Register].
    fn on_key_down(&mut self, _event: &mut Event<event::KeyDown>) {}
    /// Handle key up events. These events will only be sent if this component is focused or the [`Component#register`][crate::Component#method.register] method returns [`Register::KeyUp`][crate::event::Register].
    fn on_key_up(&mut self, _event: &mut Event<event::KeyUp>) {}
    /// Handle key press events. These events will only be sent if this component is focused or the [`Component#register`][crate::Component#method.register] method returns [`Register::KeyPress`][crate::event::Register].
    fn on_key_press(&mut self, _event: &mut Event<event::KeyPress>) {}
    /// Handle text entry events. These events will only be sent if this component is focused.
    fn on_text_entry(&mut self, _event: &mut Event<event::TextEntry>) {}
    /// Handle a drag and drop event moving over the component.
    fn on_drag_target(&mut self, _event: &mut Event<event::DragTarget>) {}
    /// Handle a drag and drop event the first it moves over this component.
    fn on_drag_enter(&mut self, _event: &mut Event<event::DragEnter>) {}
    /// Handle a drag and drop event leaving this component.
    fn on_drag_leave(&mut self, _event: &mut Event<event::DragLeave>) {}
    /// Handle a drag and drop event dropping onto this component.
    fn on_drag_drop(&mut self, _event: &mut Event<event::DragDrop>) {}
    #[doc(hidden)]
    fn on_menu_select(&mut self, _event: &mut Event<event::MenuSelect>) {}
}

pub trait RootComponent<A> {
    // Called when a root node is first instantiated, this method will only be called for root components. This is called after state is init()
    fn root(&mut self, window: &dyn Any, app_params: &dyn Any) {}
}

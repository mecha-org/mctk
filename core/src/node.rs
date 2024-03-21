use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::component::*;
use crate::event::{self, Event, EventInput};
use crate::font_cache::FontCache;
use crate::renderables::Renderable;
use crate::renderer::Caches;
use crate::types::*;
// use crate::font_cache::FontCache;
use crate::layout::*;
// use crate::render::{Caches, Renderable};

static NODE_ID_ATOMIC: AtomicU64 = AtomicU64::new(1);

// (<Event that the node desires to receive>, <Node ID>)
pub(crate) type Registration = (event::Register, u64);

fn new_node_id() -> u64 {
    NODE_ID_ATOMIC.fetch_add(1, Ordering::SeqCst)
}

/// Constructor for [`Node`].
///
/// There a 5 ways to call `node`:
///
/// - With just a [`Component`] instance:
///```ignore
/// node!(COMPONENT)
///```
///
/// - With a Component, and -- in square braces -- arguments that will be passed to [`lay!`][crate::lay].
///```ignore
/// node!(COMPONENT, [LAY_MACRO_ARGS])
///```
///
/// - With a Component, arguments for [`lay!`][crate::lay], and a [`key`][Node#method.key].
///```ignore
/// node!(COMPONENT, [LAY_MACRO_ARGS], KEY)
///```
///
/// - With a Component, and a [`Layout`].
///```ignore
/// node!(COMPONENT, LAYOUT)
///```
///
/// - With a Component, a [`Layout`], and a [`key`][Node#method.key].
///```ignore
/// node!(COMPONENT, LAYOUT, KEY)
///```
/// All five call [`Node#new`][Node#method.new] and wrap the [`Component`] in a [`Box::new`][Box#method.new].
#[macro_export]
macro_rules! node {
    ($component:expr $(,)*) => {
        node!($component, $crate::layout::Layout::default())
    };
    ($component:expr, [ $( $tt:tt )* ] $(,)*) => {
        node!(
            $component,
            $crate::lay!($($tt)*),
            $crate::mctk_macros::static_id!()
        )
    };
    ($component:expr, [ $( $tt:tt )* ], $key:expr) => {
        node!(
            $component,
            $crate::lay!($($tt)*),
            $key
        )
    };
    ($component:expr, $layout:expr $(,)*) => {
        node!($component, $layout, $crate::mctk_macros::static_id!())
    };
    ($component:expr, $layout:expr, $key:expr) => {
        $crate::Node::new(Box::new($component), $key, $layout)
    };
}

/// An instance of a [`Component`] situated within the app, along with a [`Layout`]. Construct with the [`node`] macro.
///
/// When combined together, `Node`s form a graph that represents the application: the graph is responsible for handling events, it knows how to render itself, and it holds all of the required state. See the [tutorial][crate] for an explanation of how to use Nodes to create an application.
pub struct Node {
    pub(crate) id: u64,
    pub(crate) component: Box<dyn Component + Send + Sync>,
    pub(crate) render_cache: Option<Vec<Renderable>>,
    pub(crate) children: Vec<Node>,
    pub(crate) layout: Layout,
    pub(crate) layout_result: LayoutResult,
    pub(crate) aabb: AABB,
    pub(crate) inclusive_aabb: AABB,
    // TODO: Marking a node dirty should propagate to all its parents.
    //   Clean nodes can be fully recycled instead of performing a `view`
    // pub(crate) dirty: bool,
    /// If the node is scrollable, how big are its children?
    pub(crate) inner_scale: Option<Scale>,
    pub(crate) props_hash: u64,
    pub(crate) render_hash: u64,
    pub(crate) key: u64,
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Node")
            .field("id", &self.id)
            .field("component", &self.component)
            .field("render_cache", &self.render_cache)
            // .field("layout", &self.layout) // This is often TMI
            .field("aabb", &self.aabb)
            .field("inclusive_aabb", &self.inclusive_aabb)
            .field("props_hash", &self.props_hash)
            .field("render_hash", &self.render_hash)
            .field("key", &self.key)
            .field("children", &self.children)
            .finish()
    }
}

fn expand_aabb(a: &mut AABB, b: AABB) {
    if a.pos.x > b.pos.x {
        a.pos.x = b.pos.x;
    }
    if a.bottom_right.x < b.bottom_right.x {
        a.bottom_right.x = b.bottom_right.x;
    }
    if a.pos.y > b.pos.y {
        a.pos.y = b.pos.y;
    }
    if a.bottom_right.y < b.bottom_right.y {
        a.bottom_right.y = b.bottom_right.y;
    }
}

impl Node {
    /// Constructor. In most cases it will be more convenient to use the [`node`] macro, which calls this method.
    pub fn new(component: Box<dyn Component + Send + Sync>, key: u64, layout: Layout) -> Self {
        Self {
            id: 0,
            component,
            layout,
            key,
            aabb: Default::default(),
            inclusive_aabb: Default::default(),
            // dirty: false,
            inner_scale: None,
            layout_result: Default::default(),
            children: vec![],
            render_cache: None,
            props_hash: u64::max_value(),
            render_hash: u64::max_value(),
        }
    }

    /// Add a Node to the children of the current one, returns itself. Can be chained.
    pub fn push(mut self, node: Self) -> Self {
        self.children.push(node);
        self
    }

    /// Set the key of the current Node, returns itself. `key` must be set on Nodes that are part of a dynamically-generated list of Nodes, pushed to some parent. The key should be unique within that set of child nodes, and it should be stable for the lifetime of the Node. This is used to associate state between the previously generated Node graph and a newly generated one.
    pub fn key(mut self, key: u64) -> Self {
        self.key = key;
        self
    }

    pub(crate) fn view(
        &mut self,
        mut prev: Option<&mut Self>,
        registrations: &mut Vec<Registration>,
    ) {
        // TODO: skip non-visible (out of frame) nodes
        // Set up state and props
        let mut hasher = ComponentHasher::new_with_keys(0, 0);
        if let Some(prev) = &mut prev {
            self.id = prev.id;
            if let Some(state) = prev.component.take_state() {
                self.component.replace_state(state);
            }

            self.component.props_hash(&mut hasher);
            self.props_hash = hasher.finish();

            if self.props_hash != prev.props_hash {
                self.component.new_props();
            } // Maybe TODO: If nodes were clonable, it could make sense to clone them here rather than create them with `view`
        } else {
            self.id = new_node_id();
            self.component.init();
            self.component.props_hash(&mut hasher);
            self.props_hash = hasher.finish();
        }

        // Create children
        if let Some(mut child) = self.component.view() {
            if let Some(indexes) = self.component.container() {
                // Pull out the children that were pushed onto this node, since we need to moves
                // them to the correct position.
                let mut container_children = vec![];
                container_children.append(&mut self.children);
                if indexes.is_empty() {
                    // Push onto the root
                    self.children.push(child);
                    self.children.append(&mut container_children);
                } else {
                    // Find the subchild to push the container_children onto
                    assert_eq!(indexes[0], 0, "The first index returned by Component#container must be 0, since #view can only return one Node.");
                    let mut dest = &mut child;
                    for i in indexes[1..].iter() {
                        dest = &mut dest.children[*i];
                    }
                    dest.children.append(&mut container_children);
                    self.children.push(child);
                }
            } else {
                if !self.children.is_empty() {
                    panic!("Tried to add a child to a non-container node: {:?}", self);
                }

                self.children.push(child);
            }
        }

        // View children
        if let Some(prev) = prev.as_mut() {
            let prev_children = &mut prev.children;
            for child in self.children.iter_mut() {
                child.view(
                    prev_children.iter_mut().find(|x| x.key == child.key),
                    registrations,
                )
            }
        } else {
            for child in self.children.iter_mut() {
                child.view(None, registrations)
            }
        }

        // Children's registrations come first, so they can prevent bubbling
        registrations.append(
            &mut self
                .component
                .register()
                .drain(..)
                .map(|r| (r, self.id))
                .collect::<Vec<_>>(),
        );
    }

    fn set_aabb(
        &mut self,
        parent_pos: Pos,
        parent_aabb: AABB,
        mut parent_scroll_pos: ScrollPosition,
        parent_full_control: bool,
        frame: AABB,
        scale_factor: f32,
    ) {
        let full_control = self.component.full_control();

        if !parent_full_control {
            self.aabb = self.layout_result.into();
            self.aabb *= scale_factor;
            self.aabb = self.aabb.round();
            if let Some(s) = self.inner_scale.as_mut() {
                s.width = (s.width * scale_factor).round();
                s.height = (s.height * scale_factor).round();
            }
        }
        self.aabb.pos += parent_pos;
        self.aabb.bottom_right += parent_pos.into();
        self.aabb.pos.z = (self.layout.z_index.unwrap_or((parent_pos.z + 1.0).into())
            + self.layout.z_index_increment) as f32;

        if full_control {
            let children: Vec<(&mut AABB, Option<Scale>, Option<Point>)> = self
                .children
                .iter_mut()
                .map(|c| {
                    c.aabb = c.layout_result.into();
                    c.aabb *= scale_factor;
                    c.aabb = c.aabb.round();
                    if let Some(s) = c.inner_scale.as_mut() {
                        s.width = (s.width * scale_factor).round();
                        s.height = (s.height * scale_factor).round();
                    }

                    (&mut c.aabb, c.inner_scale, c.component.focus())
                })
                .collect();
            self.component
                .set_aabb(&mut self.aabb, parent_aabb, children, frame, scale_factor);
        }

        self.inclusive_aabb = self.aabb;
        if let Some(scale) = self.inner_scale {
            self.inclusive_aabb.set_scale_mut(scale.width, scale.height);
        }

        let mut child_base_pos = self.aabb.pos;

        if let Some(mut x) = self.scroll_x() {
            let width = self.aabb.width();
            let inner_width = self.inner_scale.unwrap().width;
            if x + width > inner_width {
                x = inner_width - width;
            }

            parent_scroll_pos.x = Some(x);
            child_base_pos.x -= x;
        }

        if let Some(mut y) = self.scroll_y() {
            let height = self.aabb.height();
            let inner_height = self.inner_scale.unwrap().height;
            if y + height > inner_height {
                y = inner_height - height;
            }

            parent_scroll_pos.y = Some(y);
            child_base_pos.y -= y;
        }

        let scrollable = self.scrollable();
        for child in self.children.iter_mut() {
            let mut scroll_offset: Size = parent_scroll_pos.into();
            if !child.layout.position.top.resolved() && !child.layout.position.bottom.resolved() {
                scroll_offset.height = Dimension::Px(0.0);
            }
            if !child.layout.position.left.resolved() && !child.layout.position.right.resolved() {
                scroll_offset.width = Dimension::Px(0.0);
            }

            let mut child_base_pos = child_base_pos;
            child_base_pos.x += f32::from(scroll_offset.width);
            child_base_pos.y += f32::from(scroll_offset.height);

            child.set_aabb(
                child_base_pos,
                self.aabb,
                parent_scroll_pos,
                full_control,
                if scrollable { self.aabb } else { frame },
                scale_factor,
            );
            if !scrollable {
                expand_aabb(&mut self.inclusive_aabb, child.inclusive_aabb);
            }
        }
    }

    pub(crate) fn layout(&mut self, _prev: &Self, font_cache: &FontCache, scale_factor: f32) {
        self.calculate_layout(font_cache, scale_factor);
        self.set_aabb(
            Pos::default(),
            self.aabb,
            ScrollPosition::default(),
            false,
            (AABB::from(self.layout_result) * scale_factor).round(),
            scale_factor,
        );
    }

    /// Return whether to redraw the screen
    pub(crate) fn render(
        &mut self,
        caches: Caches,
        prev: Option<&mut Self>,
        scale_factor: f32,
    ) -> bool {
        // TODO: skip non-visible nodes
        let mut hasher = ComponentHasher::new_with_keys(0, 0);
        if let Some(prev) = prev {
            let mut ret = false;
            self.component.render_hash(&mut hasher);
            self.aabb.size().hash(&mut hasher);
            self.inner_scale.hash(&mut hasher);
            self.render_hash = hasher.finish();

            //temporary commented to solve carousel
            //if self.render_hash != prev.render_hash
            if true {
                let context = RenderContext {
                    aabb: self.aabb,
                    inner_scale: self.inner_scale,
                    caches: caches.clone(),
                    prev_state: prev.render_cache.take(),
                    scale_factor,
                };
                self.render_cache = self.component.render(context);
                ret = true;
            } else {
                self.render_cache = prev.render_cache.take();
            }

            let prev_children = &mut prev.children;
            for child in self.children.iter_mut() {
                ret |= child.render(
                    caches.clone(),
                    prev_children.iter_mut().find(|x| x.key == child.key),
                    scale_factor,
                )
            }

            ret
        } else {
            let context = RenderContext {
                aabb: self.aabb,
                inner_scale: self.inner_scale,
                caches: caches.clone(),
                prev_state: None,
                scale_factor,
            };
            self.render_cache = self.component.render(context);
            self.component.render_hash(&mut hasher);
            self.render_hash = hasher.finish();

            for child in self.children.iter_mut() {
                child.render(caches.clone(), None, scale_factor);
            }

            true
        }
    }

    pub(crate) fn scroll_x(&self) -> Option<f32> {
        self.component.scroll_position().and_then(|p| p.x)
    }

    pub(crate) fn scroll_y(&self) -> Option<f32> {
        self.component.scroll_position().and_then(|p| p.y)
    }

    pub(crate) fn scrollable(&self) -> bool {
        self.scroll_x().is_some() || self.scroll_y().is_some()
    }

    pub(crate) fn iter_renderables(&self) -> NodeRenderableIterator<'_> {
        NodeRenderableIterator {
            queue: vec![self],
            current_frame: vec![],
            frame_queue: vec![],
            i: 0,
        }
    }

    // Events

    /// Used to handle input specific event handlers that rely on the event knowing what is under the mouse (e.g. `mouse_motion`)
    /// First find the (ordered by z-index) nodes under the mouse (highest z-index last),
    /// then pass the list to `_handle_event_under_mouse`, which will only handle the last
    /// event on the list. It recursively moves through the nodes that may be under the mouse
    /// and pops off the `nodes_under` list when it handles that node. We repeat until there
    /// is nothing left in `nodes_under`. If an event handler has caused the event to stop bubbling,
    /// we can stop early.
    fn handle_event_under_mouse<E: EventInput>(
        &mut self,
        event: &mut Event<E>,
        handler: fn(&mut Self, &mut Event<E>),
    ) {
        let mut nodes_under = self.nodes_under(event, false);
        while !nodes_under.is_empty() && event.bubbles {
            self._handle_event_under_mouse(event, handler, &mut nodes_under, false);
        }
    }

    fn _handle_event_under_mouse<E: EventInput>(
        &mut self,
        event: &mut Event<E>,
        handler: fn(&mut Self, &mut Event<E>),
        node_order: &mut Vec<(u64, f32)>,
        use_touch: bool,
    ) -> Vec<Message> {
        let mut event_target_position = event.mouse_position;

        // switch to touch position
        if use_touch {
            event_target_position = event.touch_position;
        }

        let mut m: Vec<Message> = vec![];
        event.over_child_n = None;
        event.over_subchild_n = None;
        for (n, child) in self.children.iter_mut().enumerate() {
            if child
                .component
                .is_mouse_maybe_over(event_target_position, child.inclusive_aabb)
            {
                for message in child
                    ._handle_event_under_mouse(event, handler, node_order, use_touch)
                    .drain(..)
                {
                    m.append(&mut self.component.update(message));
                    if self.component.is_dirty() {
                        event.dirty();
                    }
                }
                if child
                    .component
                    .is_mouse_over(event_target_position, child.aabb)
                {
                    event.over_subchild_n = event.over_child_n;
                    event.over_child_n = Some(n);
                    event.over_child_n_aabb = Some(child.aabb);
                }
            }
        }

        if event.bubbles
            && Some(self.id) == node_order.last().map(|x| x.0)
            && self
                .component
                .is_mouse_over(event_target_position, self.aabb)
        {
            node_order.pop();
            event.current_node_id = Some(self.id);
            event.current_aabb = Some(self.aabb);
            event.current_inner_scale = self.inner_scale;
            handler(self, event);
            if self.component.is_dirty() {
                event.dirty();
            }
            m.append(&mut event.messages);
        } else if Some(self.id) == node_order.last().map(|x| x.0) {
            node_order.pop();
        }

        m
    }

    /// Used to handle input specific event handlers that rely on the event knowing what is under the touch
    /// First find the (ordered by z-index) nodes under the touch (highest z-index last),
    /// then pass the list to `_handle_event_under_touch`, which will only handle the last
    /// event on the list. It recursively moves through the nodes that may be under the touch
    /// and pops off the `nodes_under` list when it handles that node. We repeat until there
    /// is nothing left in `nodes_under`. If an event handler has caused the event to stop bubbling,
    /// we can stop early.
    fn handle_event_under_touch<E: EventInput>(
        &mut self,
        event: &mut Event<E>,
        handler: fn(&mut Self, &mut Event<E>),
    ) {
        let mut nodes_under = self.nodes_under(event, true);
        while !nodes_under.is_empty() && event.bubbles {
            self._handle_event_under_mouse(event, handler, &mut nodes_under, true);
        }
    }

    fn nodes_under<E: EventInput>(&self, event: &Event<E>, use_touch: bool) -> Vec<(u64, f32)> {
        let mut collector: Vec<(u64, f32)> = vec![];

        self._nodes_under(event, &mut collector, use_touch);
        // Maybe TODO: Discard siblings?
        collector.sort_by(|(m, _), (n, _)| m.partial_cmp(n).unwrap());
        collector
    }

    fn _nodes_under<E: EventInput>(
        &self,
        event: &Event<E>,
        collector: &mut Vec<(u64, f32)>,
        use_touch: bool,
    ) {
        let mut event_target_position = event.mouse_position;

        // switch to touch position
        if use_touch {
            event_target_position = event.touch_position;
        }

        if self
            .component
            .is_mouse_over(event_target_position, self.aabb)
        {
            collector.push((self.id, self.aabb.pos.z))
        }

        let is_mouse_over = self.component.is_mouse_over(
            event_target_position,
            self.component.frame_bounds(self.aabb, self.inner_scale),
        );

        if self.scrollable() && !is_mouse_over {
            return;
        }

        for child in self.children.iter() {
            if child
                .component
                .is_mouse_maybe_over(event_target_position, child.inclusive_aabb)
            {
                child._nodes_under(event, collector, use_touch);
            }
        }
    }

    // fn get_target(&mut self, target: u64) -> Option<&mut Self> {
    //     let mut stack: Vec<&mut Self> = vec![];
    //     let mut current = self;
    //     loop {
    //         if current.id == target {
    //             return Some(current);
    //         }
    //         if !current.children.is_empty() {
    //             stack.append(&mut current.children.iter_mut().collect());
    //         }
    //         if stack.is_empty() {
    //             return None;
    //         } else {
    //             current = stack.pop().unwrap();
    //         }
    //     }
    // }

    fn get_target_from_stack(&mut self, target: &[usize]) -> &mut Self {
        let mut current = self;
        for t in target.iter() {
            current = &mut current.children[*t];
        }
        current
    }

    pub(crate) fn get_target_stack(&self, target: u64) -> Option<Vec<usize>> {
        struct Frame<'a> {
            node: &'a Node,
            child: usize,
        }

        let mut stack: Vec<Frame> = vec![];
        let mut current = Frame {
            node: self,
            child: 0,
        };
        loop {
            if current.node.id == target {
                // Unwind
                return Some(stack.iter().map(|f| f.child - 1).collect());
            }
            if current.child < current.node.children.len() {
                stack.push(Frame {
                    node: current.node,
                    child: current.child + 1,
                });
                current = Frame {
                    node: &current.node.children[current.child],
                    child: 0,
                };
            } else if stack.is_empty() {
                return None;
            } else {
                current = stack.pop().unwrap();
            }
        }
    }

    fn handle_targeted_event<E: EventInput>(
        &mut self,
        event: &mut Event<E>,
        handler: fn(&mut Self, &mut Event<E>),
    ) {
        match event.target {
            Some(0) => {
                // If the target is the root node, allow registration to accept the event
                let matching_registrations = event.matching_registrations();
                if matching_registrations.is_empty() {
                    // Go ahead and send to the root, if there are no registrations
                    self.handle_targeted_event_inner(event, handler)
                } else {
                    for node_id in event.matching_registrations().iter() {
                        // We don't reset this event, since we want to carry forward any signals: dirty, focus
                        event.target = Some(*node_id);
                        self.handle_targeted_event_inner(event, handler);
                        if !event.bubbles {
                            break;
                        }
                    }
                }
            }
            Some(_) => self.handle_targeted_event_inner(event, handler),
            None => (),
        }
    }

    fn handle_targeted_event_inner<E: EventInput>(
        &mut self,
        event: &mut Event<E>,
        handler: fn(&mut Self, &mut Event<E>),
    ) {
        if let Some(mut stack) = self.get_target_stack(event.target.unwrap()) {
            let node = self.get_target_from_stack(&stack);
            event.current_node_id = Some(node.id);
            event.current_aabb = Some(node.aabb);
            event.current_inner_scale = node.inner_scale;
            handler(node, event);
            if self.component.is_dirty() {
                event.dirty();
            }
            if stack.is_empty() {
                return;
            }
            stack.pop();

            loop {
                let node = self.get_target_from_stack(&stack);
                let mut dirty = false;
                let mut next_messages: Vec<Message> = vec![];
                for message in event.messages.drain(..) {
                    next_messages.append(&mut node.component.update(message));
                    if node.component.is_dirty() {
                        dirty = true;
                    }
                }
                if dirty {
                    event.dirty();
                }
                if next_messages.is_empty() || stack.is_empty() {
                    return;
                }
                event.messages = next_messages;
                stack.pop();
            }
        }
    }

    pub(crate) fn send_messages(
        &mut self,
        mut target_stack: Vec<usize>,
        messages: &mut Vec<Message>,
    ) -> bool {
        let mut dirty = false;
        loop {
            let node = self.get_target_from_stack(&target_stack);
            let mut next_messages: Vec<Message> = vec![];
            for message in messages.drain(..) {
                next_messages.append(&mut node.component.update(message));
                if node.component.is_dirty() {
                    dirty = true;
                }
            }
            if next_messages.is_empty() || target_stack.is_empty() {
                return dirty;
            }
            *messages = next_messages;
            target_stack.pop();
        }
    }

    pub(crate) fn mouse_motion(&mut self, event: &mut Event<event::MouseMotion>) {
        self.handle_event_under_mouse(event, |node, e| {
            e.target = Some(node.id);
            node.component.on_mouse_motion(e)
        });
    }

    pub(crate) fn scroll(&mut self, event: &mut Event<event::Scroll>) {
        self.handle_event_under_mouse(event, |node, e| node.component.on_scroll(e));
    }

    pub(crate) fn mouse_down(&mut self, event: &mut Event<event::MouseDown>) {
        self.handle_event_under_mouse(event, |node, e| node.component.on_mouse_down(e));
    }

    pub(crate) fn mouse_up(&mut self, event: &mut Event<event::MouseUp>) {
        self.handle_event_under_mouse(event, |node, e| node.component.on_mouse_up(e));
    }

    pub(crate) fn mouse_enter(&mut self, event: &mut Event<event::MouseEnter>) {
        self.handle_targeted_event(event, |node, e| node.component.on_mouse_enter(e));
    }

    pub(crate) fn mouse_leave(&mut self, event: &mut Event<event::MouseLeave>) {
        self.handle_targeted_event(event, |node, e| node.component.on_mouse_leave(e));
    }

    pub(crate) fn click(&mut self, event: &mut Event<event::Click>) {
        self.handle_event_under_mouse(event, |node, e| node.component.on_click(e));
    }

    pub(crate) fn tap(&mut self, event: &mut Event<event::Click>) {
        self.handle_event_under_touch(event, |node, e| node.component.on_click(e));
    }

    pub(crate) fn double_click(&mut self, event: &mut Event<event::DoubleClick>) {
        self.handle_event_under_mouse(event, |node, e| node.component.on_double_click(e));
    }

    pub(crate) fn double_tap(&mut self, event: &mut Event<event::DoubleClick>) {
        self.handle_event_under_touch(event, |node, e| node.component.on_double_click(e));
    }

    pub(crate) fn focus(&mut self, event: &mut Event<event::Focus>) {
        self.handle_targeted_event(event, |node, e| node.component.on_focus(e));
    }

    pub(crate) fn blur(&mut self, event: &mut Event<event::Blur>) {
        self.handle_targeted_event(event, |node, e| node.component.on_blur(e));
    }

    pub(crate) fn key_down(&mut self, event: &mut Event<event::KeyDown>) {
        self.handle_targeted_event(event, |node, e| node.component.on_key_down(e));
    }

    pub(crate) fn key_up(&mut self, event: &mut Event<event::KeyUp>) {
        self.handle_targeted_event(event, |node, e| node.component.on_key_up(e));
    }

    pub(crate) fn key_press(&mut self, event: &mut Event<event::KeyPress>) {
        self.handle_targeted_event(event, |node, e| node.component.on_key_press(e));
    }

    pub(crate) fn touch_down(&mut self, event: &mut Event<event::TouchDown>) {
        self.handle_event_under_touch(event, |node, e| node.component.on_touch_down(e));
    }

    pub(crate) fn touch_up(&mut self, event: &mut Event<event::TouchUp>) {
        self.handle_event_under_touch(event, |node, e| node.component.on_touch_up(e));
    }

    pub(crate) fn touch_moved(&mut self, event: &mut Event<event::TouchMoved>) {
        self.handle_event_under_touch(event, |node, e| node.component.on_touch_moved(e));
    }

    pub(crate) fn touch_cancel(&mut self, event: &mut Event<event::TouchCancel>) {
        self.handle_targeted_event(event, |node, e| node.component.on_touch_cancel(e));
    }

    pub(crate) fn text_entry(&mut self, event: &mut Event<event::TextEntry>) {
        self.handle_targeted_event(event, |node, e| node.component.on_text_entry(e));
    }

    pub(crate) fn drag(&mut self, event: &mut Event<event::Drag>) {
        self.handle_targeted_event(event, |node, e| node.component.on_drag(e));
    }

    pub(crate) fn drag_start(&mut self, event: &mut Event<event::DragStart>) {
        self.handle_event_under_mouse(event, |node, e| {
            e.target = Some(node.id);
            node.component.on_drag_start(e)
        });
    }

    pub(crate) fn drag_end(&mut self, event: &mut Event<event::DragEnd>) {
        self.handle_targeted_event(event, |node, e| node.component.on_drag_end(e));
    }

    // DND
    pub(crate) fn drag_target(&mut self, event: &mut Event<event::DragTarget>) {
        self.handle_event_under_mouse(event, |node, e| {
            e.target = Some(node.id);
            node.component.on_drag_target(e)
        });
    }

    pub(crate) fn drag_enter(&mut self, event: &mut Event<event::DragEnter>) {
        self.handle_targeted_event(event, |node, e| node.component.on_drag_enter(e));
    }

    pub(crate) fn drag_leave(&mut self, event: &mut Event<event::DragLeave>) {
        self.handle_targeted_event(event, |node, e| node.component.on_drag_leave(e));
    }

    pub(crate) fn drag_drop(&mut self, event: &mut Event<event::DragDrop>) {
        self.handle_targeted_event(event, |node, e| node.component.on_drag_drop(e));
    }

    pub(crate) fn menu_select(&mut self, event: &mut Event<event::MenuSelect>) {
        self.handle_targeted_event(event, |node, e| node.component.on_menu_select(e));
    }

    pub(crate) fn tick(&mut self, event: &mut Event<event::Tick>) -> Vec<Message> {
        let mut m: Vec<Message> = vec![];

        for child in self.children.iter_mut() {
            for message in child.tick(event).drain(..) {
                m.append(&mut self.component.update(message));
            }
        }

        event.current_node_id = Some(self.id);
        event.current_aabb = Some(self.aabb);
        event.current_inner_scale = self.inner_scale;
        self.component.on_tick(event);
        if self.component.is_dirty() {
            event.dirty();
        }
        m.append(&mut event.messages);

        m
    }
}

pub(crate) type ScrollFrame = AABB;

pub(crate) struct NodeRenderableIterator<'a> {
    queue: Vec<&'a Node>,
    current_frame: Vec<ScrollFrame>,
    frame_queue: Vec<(&'a Node, Vec<ScrollFrame>)>,
    i: usize,
}

impl<'a> Iterator for NodeRenderableIterator<'a> {
    type Item = (&'a Renderable, &'a AABB, Vec<ScrollFrame>);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(n) = self.queue.pop() {
            if let Some(c) = &n.render_cache {
                let i = self.i;

                if i == c.len() {
                    self.i = 0;
                    if n.scrollable() {
                        let mut f = self.current_frame.clone();
                        f.push(n.component.frame_bounds(n.aabb, n.inner_scale));
                        self.frame_queue.push((n, f));
                    } else {
                        self.queue.extend(n.children.iter().collect::<Vec<&Node>>());
                    }
                } else {
                    self.i += 1;
                    self.queue.push(n);
                    return Some((&c[i], &n.aabb, self.current_frame.clone()));
                }
            } else if n.scrollable() {
                let mut f = self.current_frame.clone();
                f.push(n.component.frame_bounds(n.aabb, n.inner_scale));
                self.frame_queue.push((n, f));
            } else {
                self.queue.extend(n.children.iter().collect::<Vec<&Node>>());
            }

            if self.queue.is_empty() && !self.frame_queue.is_empty() {
                let (n, f) = self.frame_queue.pop().unwrap();
                self.current_frame = f;
                self.queue.extend(n.children.iter().collect::<Vec<&Node>>());
            }
        }
        None
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::render::{Renderable, Renderer};
//     use crate::window::Window;
//     use raw_window_handle::{
//         HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
//     };

//     pub struct TestWindow {}
//     impl Window for TestWindow {
//         fn logical_size(&self) -> PixelSize {
//             PixelSize {
//                 width: 100,
//                 height: 100,
//             }
//         }

//         fn physical_size(&self) -> PixelSize {
//             PixelSize {
//                 width: 100,
//                 height: 100,
//             }
//         }

//         fn scale_factor(&self) -> f32 {
//             1.0
//         }
//     }
//     unsafe impl HasRawWindowHandle for TestWindow {
//         fn raw_window_handle(&self) -> RawWindowHandle {
//             panic!("Can't get windows handle in a test")
//         }
//     }

//     unsafe impl HasRawDisplayHandle for TestWindow {
//         fn raw_display_handle(&self) -> RawDisplayHandle {
//             panic!("Can't get windows handle in a test")
//         }
//     }

//     #[derive(Debug)]
//     pub struct TestRenderer {}
//     impl Renderer for TestRenderer {
//         fn new<W: Window>(_window: &W) -> Self {
//             Self {}
//         }
//     }

//     mod container {
//         use super::*;
//         #[derive(Debug)]
//         pub struct Container {}

//         impl Component for Container {}
//     }

//     mod test_button {
//         use super::*;
//         #[derive(Debug)]
//         pub struct TestButton<M> {
//             label: String,
//             on_click: Option<M>,
//         }

//         impl<M> TestButton<M> {
//             pub fn new(label: &str) -> Self {
//                 Self {
//                     on_click: None,
//                     label: label.to_string(),
//                 }
//             }
//             pub fn on_click(mut self, msg: M) -> Self {
//                 self.on_click = Some(msg);
//                 self
//             }
//         }

//         impl<M: 'static + fmt::Debug + Clone> Component for TestButton<M> {
//             fn on_click(&mut self, event: &mut Event<event::Click>) {
//                 // println!("ON CLICK {}", &self.label);
//                 if let Some(msg) = &self.on_click {
//                     event.emit(Box::new(msg.clone()));
//                 }
//             }

//             fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
//                 Some(vec![Renderable::Inc {
//                     repr: self.label.clone(),
//                     i: context.prev_state.map_or(1, |r| match r[0] {
//                         Renderable::Inc { i, .. } => i + 1,
//                         _ => panic!(),
//                     }),
//                 }])
//             }
//         }
//     }

//     pub fn container(key: u64) -> Node {
//         Node::new(Box::new(container::Container {}), key, Layout::default())
//     }

//     mod widget {
//         use super::*;
//         #[derive(Debug)]
//         pub struct Widget {
//             pub prop: usize,
//             pub state: Option<WidgetState>,
//         }
//         #[derive(Debug)]
//         pub struct WidgetState {
//             pub bar: usize,
//         }
//         #[derive(Debug, Clone)]
//         pub enum Msg {
//             APressed,
//             BPressed,
//         }
//         impl Widget {
//             pub fn new(prop: usize) -> Self {
//                 Self { prop, state: None }
//             }
//         }

//         impl Component for Widget {
//             fn view(&self) -> Option<Node> {
//                 Some(
//                     container(0)
//                         .push(Node::new(
//                             Box::new(
//                                 test_button::TestButton::new("Button A").on_click(Msg::APressed),
//                             ),
//                             0,
//                             Layout::default(),
//                         ))
//                         .push(Node::new(
//                             Box::new(
//                                 test_button::TestButton::new("Button B").on_click(Msg::BPressed),
//                             ),
//                             1,
//                             Layout::default(),
//                         )),
//                 )
//             }

//             fn update(&mut self, message: Message) -> Vec<Message> {
//                 let msg = match message.downcast_ref::<Msg>().unwrap() {
//                     Msg::APressed => test_app::AppMessage::IncFoo(2),
//                     Msg::BPressed => test_app::AppMessage::DecFoo(1),
//                 };
//                 vec![Box::new(msg)]
//             }

//             fn replace_state(&mut self, other_state: State) {
//                 let s = other_state.downcast::<WidgetState>().unwrap();
//                 self.state = Some(*s);
//             }

//             fn take_state(&mut self) -> Option<State> {
//                 if let Some(s) = self.state.take() {
//                     Some(Box::new(s))
//                 } else {
//                     None
//                 }
//             }
//         }
//     }

//     mod test_app {
//         use super::*;

//         #[derive(Debug, Default)]
//         pub struct TestApp {
//             pub state: Option<AppState>,
//         }

//         #[derive(Debug)]
//         pub struct AppState {
//             pub foo: usize,
//         }

//         #[derive(Debug)]
//         pub enum AppMessage {
//             IncFoo(usize),
//             DecFoo(usize),
//         }

//         impl TestApp {
//             fn state_mut(&mut self) -> &mut AppState {
//                 self.state.as_mut().unwrap()
//             }
//         }

//         impl Component for TestApp {
//             fn init(&mut self) {
//                 self.state = Some(test_app::AppState { foo: 1 });
//             }

//             fn view(&self) -> Option<Node> {
//                 let foo = self.state.as_ref().unwrap().foo;
//                 Some(container(0).push(Node::new(
//                     Box::new(widget::Widget::new(foo)),
//                     0,
//                     Layout::default(),
//                 )))
//             }

//             fn update(&mut self, message: Message) -> Vec<Message> {
//                 match message.downcast_ref::<AppMessage>().unwrap() {
//                     AppMessage::IncFoo(x) => self.state_mut().foo += x,
//                     AppMessage::DecFoo(x) => self.state_mut().foo -= x,
//                 };
//                 vec![]
//             }

//             fn replace_state(&mut self, other_state: State) {
//                 let s = other_state.downcast::<AppState>().unwrap();
//                 self.state = Some(*s);
//             }

//             fn take_state(&mut self) -> Option<State> {
//                 if let Some(s) = self.state.take() {
//                     Some(Box::new(s))
//                 } else {
//                     None
//                 }
//             }

//             fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
//                 Some(vec![Renderable::Inc {
//                     repr: format!("{}", self.state.as_ref().map(|s| s.foo).unwrap()),
//                     i: context.prev_state.map_or(1, |r| match r[0] {
//                         Renderable::Inc { i, .. } => i + 1,
//                         _ => panic!(),
//                     }),
//                 }])
//             }

//             fn render_hash(&self, hasher: &mut ComponentHasher) {
//                 if let Some(s) = self.state.as_ref() {
//                     s.foo.hash(hasher)
//                 }
//             }
//         }
//     }

//     #[test]
//     fn test_caching() {
//         let renderer = TestRenderer {};
//         let mut n = Node::new(Box::new(test_app::TestApp::default()), 0, Layout::default());
//         n.view(None, &mut vec![]);
//         //n.layout();
//         n.render(renderer.caches(), None, 1.0);
//         //println!("{:#?}", n);
//         assert_eq!(
//             n.render_cache,
//             Some(vec![Renderable::Inc {
//                 repr: "1".to_string(),
//                 i: 1
//             }])
//         );
//         assert_eq!(
//             n.children[0].children[0].children[0].children[0].render_cache,
//             Some(vec![Renderable::Inc {
//                 repr: "Button A".to_string(),
//                 i: 1
//             }])
//         );

//         assert_eq!(n.iter_renderables().count(), 3);

//         let mut event = Event::new(
//             event::Click(crate::input::MouseButton::Left),
//             &crate::event::EventCache::new(1.0),
//         );
//         n.click(&mut event);

//         let mut new_n = Node::new(Box::new(test_app::TestApp::default()), 0, Layout::default());
//         new_n.view(Some(&mut n), &mut vec![]);
//         assert_eq!(n.id, new_n.id);
//         assert_eq!(n.children[0].id, new_n.children[0].id);

//         //new_n.layout();
//         new_n.render(renderer.caches(), Some(&mut n), 1.0);
//         //println!("{:#?}", new_n);
//         assert_eq!(
//             new_n.render_cache,
//             Some(vec![Renderable::Inc {
//                 repr: "2".to_string(),
//                 i: 2
//             }])
//         );
//         // Button did not need to be re-rendered
//         assert_eq!(
//             new_n.children[0].children[0].children[0].children[0].render_cache,
//             Some(vec![Renderable::Inc {
//                 repr: "Button A".to_string(),
//                 i: 1
//             }])
//         );
//     }

//     mod test_scroll_app {
//         use super::*;

//         #[derive(Debug)]
//         pub struct Div {
//             name: String,
//             scrollable: bool,
//         }

//         impl Component for Div {
//             fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
//                 Some(vec![Renderable::Inc {
//                     repr: format!("Div {}", &self.name),
//                     i: context.prev_state.map_or(1, |r| match r[0] {
//                         Renderable::Inc { i, .. } => i + 1,
//                         _ => panic!(),
//                     }),
//                 }])
//             }

//             fn scroll_position(&self) -> Option<ScrollPosition> {
//                 if self.scrollable {
//                     Some(ScrollPosition {
//                         x: Some(0.0),
//                         y: Some(50.0),
//                     })
//                 } else {
//                     None
//                 }
//             }
//         }

//         #[derive(Debug, Default)]
//         pub struct TestApp {}

//         impl Component for TestApp {
//             fn view(&self) -> Option<Node> {
//                 Some(
//                     Node::new(
//                         Box::new(Div {
//                             name: "Top".to_string(),
//                             scrollable: false,
//                         }),
//                         0,
//                         Layout::default(),
//                     )
//                     .push(
//                         Node::new(
//                             Box::new(Div {
//                                 name: "Scroll".to_string(),
//                                 scrollable: true,
//                             }),
//                             0,
//                             Layout {
//                                 size: Size {
//                                     width: Dimension::Px(100.0),
//                                     height: Dimension::Px(100.0),
//                                 },
//                                 direction: Direction::Row,
//                                 ..Default::default()
//                             },
//                         )
//                         .push(
//                             Node::new(
//                                 Box::new(Div {
//                                     name: "Column A".to_string(),
//                                     scrollable: false,
//                                 }),
//                                 0,
//                                 Layout {
//                                     size: Size {
//                                         width: Dimension::Px(100.0),
//                                         height: Dimension::Auto,
//                                     },
//                                     direction: Direction::Column,
//                                     ..Default::default()
//                                 },
//                             )
//                             .push(Node::new(
//                                 Box::new(Div {
//                                     name: "A1".to_string(),
//                                     scrollable: false,
//                                 }),
//                                 0,
//                                 Layout {
//                                     size: Size {
//                                         width: Dimension::Auto,
//                                         height: Dimension::Px(75.0),
//                                     },
//                                     ..Default::default()
//                                 },
//                             ))
//                             .push(Node::new(
//                                 Box::new(Div {
//                                     name: "A2".to_string(),
//                                     scrollable: false,
//                                 }),
//                                 1,
//                                 Layout {
//                                     size: Size {
//                                         width: Dimension::Auto,
//                                         height: Dimension::Px(75.0),
//                                     },
//                                     ..Default::default()
//                                 },
//                             )),
//                         )
//                         .push(
//                             Node::new(
//                                 Box::new(Div {
//                                     name: "Column B".to_string(),
//                                     scrollable: false,
//                                 }),
//                                 0,
//                                 Layout {
//                                     size: Size {
//                                         width: Dimension::Px(100.0),
//                                         height: Dimension::Auto,
//                                     },
//                                     direction: Direction::Column,
//                                     ..Default::default()
//                                 },
//                             )
//                             .push(Node::new(
//                                 Box::new(Div {
//                                     name: "B1".to_string(),
//                                     scrollable: false,
//                                 }),
//                                 0,
//                                 Layout {
//                                     size: Size {
//                                         width: Dimension::Auto,
//                                         height: Dimension::Px(75.0),
//                                     },
//                                     ..Default::default()
//                                 },
//                             ))
//                             .push(Node::new(
//                                 Box::new(Div {
//                                     name: "B2".to_string(),
//                                     scrollable: false,
//                                 }),
//                                 1,
//                                 Layout {
//                                     size: Size {
//                                         width: Dimension::Auto,
//                                         height: Dimension::Px(75.0),
//                                     },
//                                     ..Default::default()
//                                 },
//                             )),
//                         ),
//                     ),
//                 )
//             }

//             fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
//                 Some(vec![Renderable::Inc {
//                     repr: "ScrollApp".to_string(),
//                     i: context.prev_state.map_or(1, |r| match r[0] {
//                         Renderable::Inc { i, .. } => i + 1,
//                         _ => panic!(),
//                     }),
//                 }])
//             }
//         }
//     }

//     #[test]
//     fn test_scroll() {
//         let renderer = TestRenderer {};
//         let m = Node::new(
//             Box::new(test_scroll_app::TestApp::default()),
//             0,
//             Layout::default(),
//         );
//         let mut n = Node::new(
//             Box::new(test_scroll_app::TestApp::default()),
//             0,
//             lay!(size: size!(300.0)),
//         );
//         n.view(None, &mut vec![]);
//         n.layout(&m, &renderer.caches().font.read().unwrap(), 1.0);

//         // Expect the inner_scale to be a real size
//         let scroll_node = &mut n.children[0].children[0];
//         assert_eq!(scroll_node.aabb.size(), [100.0, 100.0].into());
//         assert_eq!(scroll_node.inner_scale.unwrap(), [200.0, 150.0].into());

//         // Expect renderables to be laid out in the right order, with the correct Frames
//         n.render(renderer.caches(), None, 1.0);
//         let renderables = n.iter_renderables().collect::<Vec<_>>();
//         assert_eq!(renderables.len(), 9);
//         // First three (App, Top Div, Scroll Div) do not have Frames
//         assert_eq!(renderables[0].2.len(), 0);
//         assert_eq!(renderables[2].2.len(), 0);
//         // The rest have Frames
//         assert_eq!(renderables[3].2.len(), 1);
//         assert_eq!(renderables[8].2.len(), 1);
//     }

//     mod test_registration_app {
//         use super::*;

//         #[derive(Debug)]
//         pub struct Registerer {
//             registration: event::Register,
//         }

//         impl Component for Registerer {
//             fn register(&mut self) -> Vec<event::Register> {
//                 vec![self.registration]
//             }
//         }

//         #[derive(Debug, Default)]
//         pub struct TestApp {}

//         impl Component for TestApp {
//             fn view(&self) -> Option<Node> {
//                 Some(
//                     node!(Registerer {
//                         registration: event::Register::KeyDown
//                     })
//                     .push(node!(Registerer {
//                         registration: event::Register::KeyUp,
//                     }))
//                     .push(node!(Registerer {
//                         registration: event::Register::KeyPress,
//                     })),
//                 )
//             }
//         }
//     }

//     #[test]
//     fn test_registration() {
//         let mut n = Node::new(
//             Box::new(test_registration_app::TestApp::default()),
//             0,
//             Layout::default(),
//         );

//         let mut registrations: Vec<(event::Register, u64)> = vec![];
//         n.view(None, &mut registrations);
//         assert_eq!(registrations.len(), 3);
//         assert_eq!(registrations[0].0, event::Register::KeyUp);
//         assert_eq!(registrations[1].0, event::Register::KeyPress);
//         assert_eq!(registrations[2].0, event::Register::KeyDown);
//     }
// }

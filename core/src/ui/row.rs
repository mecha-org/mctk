use crate::ui::component::{
    AlignItems, Canvas, CanvasComponent, Dimension, JustifyContent, Point, Rect, Size,
};

use super::component::Margin;

pub struct Row {
    pub align_items: AlignItems,
    pub justify_content: JustifyContent,
    pub height: Dimension,
    pub width: Dimension,
    pub margin: Margin,
    pub children: Vec<Box<dyn CanvasComponent>>,
}

impl Row {
    pub fn new(width: Dimension, height: Dimension) -> Self {
        Self {
            height,
            width,
            margin: Margin::default(),
            align_items: AlignItems::Start,
            justify_content: JustifyContent::Start,
            children: vec![],
        }
    }

    pub fn push(&mut self, child: Box<dyn CanvasComponent>) -> bool {
        self.children.push(child);
        true
    }
}
impl CanvasComponent for Row {
    fn render(&self, canvas: &mut Canvas, rect: Rect) {
        let Rect { origin, size } = rect;
        let o_x = origin.x;
        let o_y = origin.y;
        let r_w = size.width;
        let r_h = size.height;
        let mut start_x = o_x;

        let mut static_width = 0.0;
        let mut auto_count = 0.0;
        for child in self.children.iter() {
            match child.width() {
                Dimension::Abs(v) => {
                    static_width = static_width + v + child.margin().left + child.margin().right;
                }
                Dimension::Auto => {
                    auto_count += 1.0;
                }
            }
        }
        let auto_width = r_w - static_width / auto_count;
        for child in self.children.iter() {
            //add left margin
            start_x += child.margin().left;
            let width = match child.width() {
                Dimension::Abs(d) => d,
                Dimension::Auto => auto_width,
            };
            let height = match child.height() {
                Dimension::Abs(d) => d,
                Dimension::Auto => 0.0,
            };

            //Align Items
            let align_item_point = match self.align_items {
                AlignItems::Start => Size::new(0.0, 0.0),
                AlignItems::End => Size::new(0.0, r_h - height),
                AlignItems::Center => Size::new(0.0, (r_h - height) / 2.0),
                AlignItems::Stretch => Size::new(0.0, 0.0),
            };

            let justify_item_point = match self.justify_content {
                JustifyContent::Start => Size::new(0.0, 0.0),
                JustifyContent::Center => Size::new(0.0, 0.0),
                JustifyContent::SpaceBetween => Size::new(0.0, 0.0),
                JustifyContent::SpaceAround => Size::new(0.0, 0.0),
                JustifyContent::SpaceEvenly => Size::new(0.0, 0.0),
            };

            let target_rect = Rect::new(
                Point::new(start_x, o_y).add_size(&align_item_point),
                Size::new(width, height),
            );
            child.render(canvas, target_rect);

            start_x = start_x + width + child.margin().right;
        }
    }

    fn height(&self) -> Dimension {
        self.height
    }

    fn width(&self) -> Dimension {
        self.width
    }

    fn margin(&self) -> Margin {
        self.margin
    }
}

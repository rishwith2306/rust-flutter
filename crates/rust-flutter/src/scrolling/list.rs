use super::{ScrollError, ScrollPosition, SingleChildScrollView, viewport::rebased_offset};
use crate::{
    egui::{Vec2, vec2},
    element::{BuildContext, Key, RenderBox, RenderContext, RenderObjectWidget, Widget},
    gestures::GestureBinding,
    layout::*,
};
use std::{ops::Range, rc::Rc};

type ItemBuilder = dyn Fn(&BuildContext, usize) -> Widget;
type ItemKey = dyn Fn(usize) -> Key;
/// Fixed-item-extent lazy list. Owns only its cached item subtrees; the caller
/// owns the scroll position and binding. Both parent axes must be bounded.
#[derive(Clone)]
pub struct ListView {
    position: ScrollPosition,
    binding: GestureBinding,
    count: usize,
    extent: f32,
    total: f32,
    exact_total: f64,
    axis: Axis,
    reverse: bool,
    cache: f32,
    limit: usize,
    keyboard: bool,
    builder: Rc<ItemBuilder>,
    key: Rc<ItemKey>,
}
impl ListView {
    pub fn builder(
        position: &ScrollPosition,
        binding: &GestureBinding,
        item_count: usize,
        item_extent: f32,
        builder: impl Fn(&BuildContext, usize) -> Widget + 'static,
    ) -> Result<Self, ScrollError> {
        if position.is_disposed() || binding.is_disposed() {
            return Err(ScrollError::Disposed);
        }
        if !item_extent.is_finite() || item_extent <= 0.0 {
            return Err(ScrollError::InvalidItemExtent);
        }
        let total = item_count as f64 * f64::from(item_extent);
        if total > f64::from(f32::MAX) {
            return Err(ScrollError::ContentExtentOverflow);
        }
        Ok(Self {
            position: position.clone(),
            binding: binding.clone(),
            count: item_count,
            extent: item_extent,
            total: total as f32,
            exact_total: total,
            axis: Axis::Vertical,
            reverse: false,
            cache: 250.0,
            limit: 4096,
            keyboard: true,
            builder: Rc::new(builder),
            key: Rc::new(|index| Key::U64(index as u64)),
        })
    }
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }
    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }
    pub fn cache_extent(mut self, points: f32) -> Result<Self, ScrollError> {
        if !points.is_finite() || points < 0.0 {
            return Err(ScrollError::InvalidCacheExtent);
        }
        self.cache = points;
        Ok(self)
    }
    /// Bound realized widgets per layout. A denser range returns a layout error
    /// instead of silently omitting visible items. Default: 4096.
    pub fn max_realized_items(mut self, limit: usize) -> Result<Self, ScrollError> {
        if limit == 0 {
            return Err(ScrollError::InvalidRealizedItemLimit);
        }
        self.limit = limit;
        Ok(self)
    }
    pub fn item_key(mut self, key: impl Fn(usize) -> Key + 'static) -> Self {
        self.key = Rc::new(key);
        self
    }
    pub fn keyboard_scrolling(mut self, enabled: bool) -> Self {
        self.keyboard = enabled;
        self
    }
    pub fn into_widget(self) -> Result<Widget, ScrollError> {
        if self.position.is_disposed() || self.binding.is_disposed() {
            return Err(ScrollError::Disposed);
        }
        let source = self.position.clone();
        Ok(Widget::listenable_builder(source, move |_, _| {
            let config = self.clone();
            Widget::layout_builder(move |context, constraints| config.build(context, constraints))
        }))
    }
    fn range(&self, size: Vec2) -> Range<usize> {
        if size.x <= 0.0 || size.y <= 0.0 || self.count == 0 {
            return 0..0;
        }
        let viewport = main(self.axis, size);
        let pixels = rebased_offset(
            self.position.pixels(),
            self.total,
            self.exact_total,
            viewport,
        );
        let start = if self.reverse {
            (self.exact_total - f64::from(viewport) - pixels).max(0.0)
        } else {
            pixels
        };
        let from =
            ((start - f64::from(self.cache)).max(0.0) / f64::from(self.extent)).floor() as usize;
        let to = ((start + f64::from(viewport) + f64::from(self.cache)).min(self.exact_total)
            / f64::from(self.extent))
        .ceil() as usize;
        from.min(self.count)..to.min(self.count)
    }
    fn build(&self, context: &BuildContext, constraints: BoxConstraints) -> Widget {
        if self.position.is_disposed() || self.binding.is_disposed() {
            return error(LayoutError::InvalidConfiguration(
                "list position or binding is disposed",
            ));
        }
        let size = constraints.max();
        if !size.is_finite() {
            return error(LayoutError::UnboundedSize);
        }
        let range = self.range(size);
        let count = range.end.saturating_sub(range.start);
        if count > self.limit {
            return error(LayoutError::InvalidConfiguration(
                "lazy list exceeds its configured realized item limit",
            ));
        }
        let origin = range.start as f64 * f64::from(self.extent);
        let mut children = Vec::new();
        if children.try_reserve(count).is_err() {
            return error(LayoutError::InvalidConfiguration(
                "cannot allocate lazy list items",
            ));
        }
        for index in range {
            children.push((self.builder)(context, index).keyed((self.key)(index)));
        }
        let content = Widget::render_object(
            Content {
                axis: self.axis,
                extent: self.extent,
                total: self.total,
            },
            children,
        );
        let viewport = SingleChildScrollView::new(&self.position, &self.binding).and_then(|view| {
            view.axis(self.axis)
                .reverse(self.reverse)
                .keyboard_scrolling(self.keyboard)
                .rebased_origin(origin, self.exact_total)
                .into_widget(content)
        });
        match viewport {
            Ok(view) => Widget::constrained(
                BoxConstraints::tight(size).expect("bounded parent constraints"),
                view,
            ),
            Err(_) => error(LayoutError::InvalidConfiguration(
                "list viewport is unavailable",
            )),
        }
    }
}
fn error(error: LayoutError) -> Widget {
    Widget::measured(move |_| Err(error.clone()), |_| {})
}
fn main(axis: Axis, size: Vec2) -> f32 {
    if axis == Axis::Vertical {
        size.y
    } else {
        size.x
    }
}
fn size_for(axis: Axis, main: f32, cross: f32) -> Vec2 {
    if axis == Axis::Vertical {
        vec2(cross, main)
    } else {
        vec2(main, cross)
    }
}
#[derive(Clone)]
struct Content {
    axis: Axis,
    extent: f32,
    total: f32,
}
struct ContentObject(Content);
impl RenderObjectWidget for Content {
    type Object = ContentObject;
    fn create_render_object(&self, _: &RenderContext) -> Self::Object {
        ContentObject(self.clone())
    }
    fn update_render_object(&self, object: &mut Self::Object, _: &RenderContext) {
        object.0 = self.clone();
    }
}
impl RenderBox for ContentObject {
    fn layout(
        &mut self,
        c: BoxConstraints,
        children: &mut BoxChildren<'_>,
        _: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        let cross = if self.0.axis == Axis::Vertical {
            c.max().x
        } else {
            c.max().y
        };
        if !cross.is_finite() {
            return Err(LayoutError::UnboundedSize);
        }
        let size = c.constrain(size_for(self.0.axis, self.0.total, cross))?;
        let item = BoxConstraints::tight(size_for(self.0.axis, self.0.extent, cross))?;
        for index in 0..children.len() {
            children.layout(index, item)?;
            let offset = (index as f64 * f64::from(self.0.extent)) as f32;
            children.place(index, size_for(self.0.axis, offset, 0.0))?;
        }
        Ok(BoxGeometry::new(size))
    }
}

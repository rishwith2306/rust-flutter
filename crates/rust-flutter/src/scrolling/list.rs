use super::keep_alive::{Entry, Registry, Scope};
use super::{ScrollError, ScrollPosition, SingleChildScrollView, viewport::rebased_offset};
use crate::{
    egui::{Vec2, vec2},
    element::{
        BuildContext, Key, RenderBox, RenderContext, RenderObjectWidget, RetainedBuilder, State,
        StateContext, StatefulWidget, Widget, retained_child,
    },
    gestures::GestureBinding,
    layout::*,
};
use std::{
    collections::{HashMap, HashSet},
    ops::Range,
    rc::Rc,
};

type ItemBuilder = dyn Fn(&BuildContext, usize) -> Widget;
type ItemKey = dyn Fn(usize) -> Key;
type FindIndex = dyn Fn(&Key) -> Option<usize>;
/// Fixed-item-extent lazy list. Owns cached and requested kept-alive subtrees; the caller
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
    drag_coefficient: f64,
    builder: Rc<ItemBuilder>,
    key: Rc<ItemKey>,
    find_index: Option<Rc<FindIndex>>,
    automatic_keep_alives: bool,
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
            drag_coefficient: 0.135,
            builder: Rc::new(builder),
            key: Rc::new(|index| Key::U64(index as u64)),
            find_index: None,
            automatic_keep_alives: true,
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
    /// Bound realized widgets, including kept-alive items, per layout. A denser range returns a layout error
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
    /// Set exponential friction drag for touch and pointer-release flings.
    /// Valid coefficients are strictly between zero and one; default is 0.135.
    pub fn drag_coefficient(mut self, coefficient: f64) -> Result<Self, ScrollError> {
        if !coefficient.is_finite() || !(0.0..1.0).contains(&coefficient) {
            return Err(ScrollError::InvalidDragCoefficient);
        }
        self.drag_coefficient = coefficient;
        Ok(self)
    }
    pub fn into_widget(self) -> Result<Widget, ScrollError> {
        if self.position.is_disposed() || self.binding.is_disposed() {
            return Err(ScrollError::Disposed);
        }
        Ok(Widget::stateful(ListRoot(self)))
    }
    /// Map a previously built key to its current index after a data reorder.
    /// Returning None (or an index whose key differs) releases that cached item.
    pub fn find_child_index_callback(
        mut self,
        find: impl Fn(&Key) -> Option<usize> + 'static,
    ) -> Self {
        self.find_index = Some(Rc::new(find));
        self
    }
    /// Enable descendant keep-alive requests. Disabling also creates an item
    /// scope boundary, so requests cannot accidentally retain an outer list row.
    pub fn add_automatic_keep_alives(mut self, enabled: bool) -> Self {
        self.automatic_keep_alives = enabled;
        self
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
    fn build(
        &self,
        _context: &BuildContext,
        constraints: BoxConstraints,
        registry: &Rc<Registry>,
    ) -> Widget {
        if self.position.is_disposed() || self.binding.is_disposed() {
            return error(
                registry,
                LayoutError::InvalidConfiguration("list position or binding is disposed"),
            );
        }
        let size = constraints.max();
        if !size.is_finite() {
            return error(registry, LayoutError::UnboundedSize);
        }
        let range = self.range(size);
        let count = range.end.saturating_sub(range.start);
        if count > self.limit {
            return error(
                registry,
                LayoutError::InvalidConfiguration(
                    "lazy list exceeds its configured realized item limit",
                ),
            );
        }
        let origin = range.start as f64 * f64::from(self.extent);
        // User key/index callbacks run without borrowing the registry. Limit work
        // to previously realized items plus the current viewport/cache range.
        let old = registry.entries.borrow().clone();
        let mut entries = HashMap::new();
        let mut rows = Vec::new();
        let mut keys = HashSet::new();
        for index in range.clone() {
            let key = (self.key)(index);
            // Duplicate sibling keys are rejected by normal element validation
            // before any of these descriptions can mount.
            keys.insert(key.clone());
            let entry = old
                .get(&key)
                .filter(|e| e.valid.get() && self.automatic_keep_alives)
                .cloned()
                .unwrap_or_else(|| Entry::new(index, registry));
            if entry.index.replace(index) != index {
                entry.dirty.set(true);
            }
            entries.insert(key.clone(), entry.clone());
            rows.push((index, key, entry, false));
        }
        if self.automatic_keep_alives {
            for (key, entry) in &old {
                if keys.contains(key) || entry.requests.get() == 0 || !entry.valid.get() {
                    continue;
                }
                let index = self
                    .find_index
                    .as_ref()
                    .map_or_else(|| Some(entry.index.get()), |find| find(key));
                if let Some(index) = index.filter(|i| *i < self.count && (self.key)(*i) == *key) {
                    if range.contains(&index) {
                        continue;
                    }
                    if entry.index.replace(index) != index {
                        entry.dirty.set(true);
                    }
                    keys.insert(key.clone());
                    entries.insert(key.clone(), entry.clone());
                    rows.push((index, key.clone(), entry.clone(), true));
                }
            }
        }
        if rows.len() > self.limit {
            return error(
                registry,
                LayoutError::InvalidConfiguration(
                    "lazy list exceeds its configured realized item limit including kept-alive items",
                ),
            );
        }
        for (key, entry) in &old {
            if !entries.get(key).is_some_and(|next| Rc::ptr_eq(next, entry)) {
                entry.valid.set(false);
            }
        }
        *registry.entries.borrow_mut() = entries;
        rows.sort_unstable_by_key(|row| row.0);
        let mut children = Vec::new();
        let mut offsets = Vec::new();
        if children.try_reserve(rows.len()).is_err() || offsets.try_reserve(rows.len()).is_err() {
            return error(
                registry,
                LayoutError::InvalidConfiguration("cannot allocate lazy list items"),
            );
        }
        for (index, key, entry, parked) in rows {
            let child = if !parked || entry.dirty.replace(false) {
                entry.dirty.set(false);
                let builder = self.builder.clone();
                let key = key.clone();
                Some(Rc::new(move |context: &BuildContext| {
                    builder(context, index).keyed(key.clone())
                }) as RetainedBuilder)
            } else {
                None
            };
            let scope = Scope(self.automatic_keep_alives.then_some(entry));
            children.push(
                Widget::offstage(parked, Widget::inherited(scope, retained_child(child)))
                    .keyed(key),
            );
            offsets.push(if parked {
                0.0
            } else {
                ((index as f64 * f64::from(self.extent)) - origin) as f32
            });
        }
        let content = Widget::render_object(
            Content {
                axis: self.axis,
                extent: self.extent,
                total: self.total,
                offsets,
            },
            children,
        );
        let viewport = SingleChildScrollView::new(&self.position, &self.binding).and_then(|view| {
            view.axis(self.axis)
                .reverse(self.reverse)
                .keyboard_scrolling(self.keyboard)
                .drag_coefficient(self.drag_coefficient)?
                .rebased_origin(origin, self.exact_total)
                .into_widget(content)
        });
        match viewport {
            Ok(view) => Widget::constrained(
                BoxConstraints::tight(size).expect("bounded parent constraints"),
                view,
            ),
            Err(_) => error(
                registry,
                LayoutError::InvalidConfiguration("list viewport is unavailable"),
            ),
        }
    }
}
struct ListRoot(ListView);
struct ListState {
    registry: Option<Rc<Registry>>,
}
impl StatefulWidget for ListRoot {
    type State = ListState;
    fn create_state(&self) -> Self::State {
        ListState { registry: None }
    }
}
impl State<ListRoot> for ListState {
    fn init_state(&mut self, _: &ListRoot, context: &StateContext<Self>) {
        let handle = context.handle();
        self.registry = Some(Registry::new(move || {
            let _ = handle.request_rebuild();
        }));
    }
    fn did_update_widget(&mut self, _: &ListRoot, _: &ListRoot, _: &StateContext<Self>) {
        for entry in self.registry.as_ref().unwrap().entries.borrow().values() {
            entry.dirty.set(true);
        }
    }
    fn build(&mut self, widget: &ListRoot, _: &StateContext<Self>) -> Widget {
        let config = widget.0.clone();
        let registry = self.registry.as_ref().unwrap().clone();
        Widget::listenable_builder(config.position.clone(), move |_, _| {
            let config = config.clone();
            let registry = registry.clone();
            Widget::layout_builder(move |context, constraints| {
                config.build(context, constraints, &registry)
            })
        })
    }
    fn dispose(&mut self) {
        if let Some(registry) = &self.registry {
            registry.close();
        }
    }
}
fn error(registry: &Registry, error: LayoutError) -> Widget {
    // The error description replaces the item subtree. Invalidate its registry
    // before unmount so external requests cannot revive absent parked children.
    registry.clear();
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
    offsets: Vec<f32>,
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
            let offset = self.0.offsets[index];
            children.place(index, size_for(self.0.axis, offset, 0.0))?;
        }
        Ok(BoxGeometry::new(size))
    }
}

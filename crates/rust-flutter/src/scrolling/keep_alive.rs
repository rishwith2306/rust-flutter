use crate::{
    element::{
        BuildContext, DependencyError, Key, RenderBox, RenderContext, RenderObjectWidget, Widget,
    },
    layout::*,
};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
};

pub(super) struct Registry {
    pub entries: RefCell<HashMap<Key, Rc<Entry>>>,
    pub active: Cell<bool>,
    wake: Box<dyn Fn()>,
}
impl Registry {
    pub fn new(wake: impl Fn() + 'static) -> Rc<Self> {
        Rc::new(Self {
            entries: RefCell::default(),
            active: Cell::new(true),
            wake: Box::new(wake),
        })
    }
    pub fn wake(&self) {
        if self.active.get() {
            (self.wake)();
        }
    }
    pub fn close(&self) {
        self.active.set(false);
        self.clear();
    }
    pub fn clear(&self) {
        let entries = std::mem::take(&mut *self.entries.borrow_mut());
        for entry in entries.into_values() {
            entry.valid.set(false);
        }
    }
}
pub(super) struct Entry {
    pub index: Cell<usize>,
    pub requests: Cell<usize>,
    pub valid: Cell<bool>,
    pub dirty: Cell<bool>,
    registry: Weak<Registry>,
}
impl Entry {
    pub fn new(index: usize, registry: &Rc<Registry>) -> Rc<Self> {
        Rc::new(Self {
            index: Cell::new(index),
            requests: Cell::new(0),
            valid: Cell::new(true),
            dirty: Cell::new(true),
            registry: Rc::downgrade(registry),
        })
    }
}
#[derive(Clone)]
pub(super) struct Scope(pub Option<Rc<Entry>>);
impl PartialEq for Scope {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        }
    }
}
/// One UI-local request to retain the nearest lazy item. Independent requests
/// are counted; dropping/releasing the final request allows off-cache disposal.
/// This Rust ownership API is not Flutter's Listenable notification protocol.
#[must_use = "dropping the handle releases this keep-alive request"]
pub struct KeepAliveHandle(Option<Rc<Entry>>);
impl KeepAliveHandle {
    pub fn request(context: &BuildContext) -> Result<Option<Self>, DependencyError> {
        Ok(context
            .depend_on::<Scope>()?
            .and_then(|scope| scope.0.as_ref().and_then(Self::acquire)))
    }
    fn acquire(entry: &Rc<Entry>) -> Option<Self> {
        if !entry.valid.get() || !entry.registry.upgrade()?.active.get() {
            return None;
        }
        entry.requests.set(
            entry
                .requests
                .get()
                .checked_add(1)
                .expect("keep-alive request count overflow"),
        );
        // A live item is already mounted. Its request count is sampled by the
        // next range build; acquiring cannot require a second initial build.
        Some(Self(Some(entry.clone())))
    }
    pub fn is_active(&self) -> bool {
        self.0.as_ref().is_some_and(|entry| {
            entry.valid.get() && entry.registry.upgrade().is_some_and(|r| r.active.get())
        })
    }
    pub fn release(&mut self) {
        if let Some(entry) = self.0.take() {
            entry.requests.set(
                entry
                    .requests
                    .get()
                    .checked_sub(1)
                    .expect("owned keep-alive request"),
            );
            if entry.requests.get() == 0
                && entry.valid.get()
                && let Some(registry) = entry.registry.upgrade()
            {
                registry.wake();
            }
        }
    }
}
impl Drop for KeepAliveHandle {
    fn drop(&mut self) {
        self.release();
    }
}

/// Declarative keep-alive request for the nearest ListView item scope. Multiple
/// descendants may request independently. Outside an enabled item scope it is a
/// normal pass-through box. It does not retain arbitrary removed widgets.
#[derive(Clone, Copy, Debug)]
pub struct KeepAlive {
    pub keep_alive: bool,
}
impl KeepAlive {
    pub fn new(keep_alive: bool) -> Self {
        Self { keep_alive }
    }
    pub fn into_widget(self, child: Widget) -> Widget {
        Widget::render_object(self, vec![child])
    }
}
impl Widget {
    pub fn keep_alive(keep_alive: bool, child: Widget) -> Widget {
        KeepAlive::new(keep_alive).into_widget(child)
    }
}
pub struct KeepAliveRenderBox {
    wanted: bool,
    request: Option<KeepAliveHandle>,
}
impl KeepAliveRenderBox {
    fn sync(&mut self, scope: Option<Rc<Scope>>) {
        let entry = scope
            .and_then(|scope| scope.0.clone())
            .filter(|e| e.valid.get());
        if !self.wanted
            || !self.request.as_ref().is_some_and(|h| {
                h.is_active()
                    && entry
                        .as_ref()
                        .is_some_and(|e| h.0.as_ref().is_some_and(|old| Rc::ptr_eq(old, e)))
            })
        {
            self.request = None;
            if self.wanted {
                self.request = entry.as_ref().and_then(KeepAliveHandle::acquire);
            }
        }
    }
}
impl RenderObjectWidget for KeepAlive {
    type Object = KeepAliveRenderBox;
    fn create_render_object(&self, ctx: &RenderContext) -> Self::Object {
        let mut object = KeepAliveRenderBox {
            wanted: self.keep_alive,
            request: None,
        };
        object.sync(ctx.read::<Scope>().ok().flatten());
        object
    }
    fn update_render_object(&self, object: &mut Self::Object, ctx: &RenderContext) {
        object.wanted = self.keep_alive;
        object.sync(ctx.depend_on::<Scope>().ok().flatten());
    }
}
fn validate(n: usize) -> Result<(), LayoutError> {
    if n == 1 {
        Ok(())
    } else {
        Err(LayoutError::InvalidConfiguration(
            "KeepAlive requires one child",
        ))
    }
}
impl RenderBox for KeepAliveRenderBox {
    fn dry_layout(
        &self,
        c: BoxConstraints,
        children: &BoxQueries<'_>,
    ) -> Result<crate::egui::Vec2, LayoutError> {
        validate(children.len())?;
        children.dry_layout(0, c)
    }
    fn intrinsic(
        &self,
        d: IntrinsicDimension,
        e: f32,
        children: &BoxQueries<'_>,
    ) -> Result<f32, LayoutError> {
        validate(children.len())?;
        children.intrinsic(0, d, e)
    }
    fn dry_baseline(
        &self,
        c: BoxConstraints,
        k: TextBaseline,
        children: &BoxQueries<'_>,
    ) -> Result<Option<f32>, LayoutError> {
        validate(children.len())?;
        children.dry_baseline(0, c, k)
    }
    fn layout(
        &mut self,
        c: BoxConstraints,
        children: &mut BoxChildren<'_>,
        ctx: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        validate(children.len())?;
        self.sync(ctx.depend_on::<Scope>().ok().flatten());
        let size = children.layout(0, c)?;
        children.place(0, crate::egui::Vec2::ZERO)?;
        let mut geometry = BoxGeometry::new(size);
        geometry.baselines = Baselines {
            alphabetic: children.baseline(0, TextBaseline::Alphabetic, true)?,
            ideographic: children.baseline(0, TextBaseline::Ideographic, true)?,
        };
        Ok(geometry)
    }
    fn dispose(&mut self) {
        self.request = None;
    }
}

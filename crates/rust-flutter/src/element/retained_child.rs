//! Internal per-item build contexts for lazy lists. A parked item can reuse its
//! child description while descendant and inherited invalidations remain active.
use super::{
    BuildContext, Widget,
    state::{Component, Description},
    widget::Kind,
};
use std::{
    any::{Any, TypeId},
    rc::Rc,
    sync::atomic::Ordering,
};
pub(crate) type RetainedBuilder = Rc<dyn Fn(&BuildContext) -> Widget>;
struct Retained(Option<RetainedBuilder>);
struct Instance {
    builder: RetainedBuilder,
    context: BuildContext,
    skip_next: bool,
}
pub(crate) fn retained_child(builder: Option<RetainedBuilder>) -> Widget {
    Widget {
        key: None,
        kind: Kind::Component(Box::new(Retained(builder))),
        children: vec![],
    }
}
impl Description for Retained {
    fn token(&self) -> TypeId {
        TypeId::of::<Self>()
    }
    fn name(&self) -> &'static str {
        "RetainedLazyChild"
    }
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
    fn mount(self: Box<Self>, context: BuildContext) -> Box<dyn Component> {
        Box::new(Instance {
            builder: self.0.expect("parked item mounted without its builder"),
            context,
            skip_next: false,
        })
    }
}
impl Component for Instance {
    fn dependencies_changed(&mut self) {
        self.skip_next = false;
    }
    fn update(&mut self, description: Box<dyn Any>) {
        if let Some(builder) = description
            .downcast::<Retained>()
            .expect("retained item description")
            .0
        {
            self.builder = builder;
            self.skip_next = false;
        } else {
            // A parent scroll update must not swallow this item's own pending
            // explicit or inherited rebuild request.
            self.skip_next = !self.context.queue.dirty.load(Ordering::Acquire);
        }
    }
    fn apply_pending(&mut self) {}
    fn state(&self) -> Option<&dyn Any> {
        None
    }
    fn build(&mut self) -> Option<Widget> {
        if std::mem::take(&mut self.skip_next) {
            None
        } else {
            Some((self.builder)(&self.context))
        }
    }
}
impl Drop for Instance {
    fn drop(&mut self) {
        self.context.queue.close();
    }
}

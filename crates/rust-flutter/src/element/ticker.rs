use super::{
    BuildContext, ElementId,
    inherited::{AmbientModel, Scope},
};
use crate::animation::{
    AnimationConfig, AnimationController, AnimationError, WeakAnimationController,
};
use std::{
    any::TypeId,
    cell::{Cell, RefCell},
    collections::HashSet,
};
/// Effective ticker permission for a subtree. Built-in nested modes combine
/// with logical AND. This does not disable gestures, paint or explicit commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickerMode {
    pub enabled: bool,
}
impl Default for TickerMode {
    fn default() -> Self {
        Self { enabled: true }
    }
}
impl super::InheritedModel for TickerMode {
    type Aspect = ();
    fn update_should_notify(&self, old: &Self) -> bool {
        self != old
    }
    fn update_should_notify_dependent(&self, old: &Self, _: &HashSet<()>) -> bool {
        self != old
    }
}
fn mode(scope: &Scope) -> bool {
    scope
        .get(&TypeId::of::<TickerMode>())
        .and_then(|link| link.value::<TickerMode>())
        .is_none_or(|value| value.enabled)
}
pub(super) struct TickerNode {
    pub enabled: bool,
    model: AmbientModel<TickerMode>,
}
impl TickerNode {
    pub fn new(id: ElementId, enabled: bool, parent: &Scope) -> Self {
        Self {
            enabled,
            model: AmbientModel::new(
                id,
                TickerMode {
                    enabled: enabled && mode(parent),
                },
            ),
        }
    }
    pub fn refresh(&mut self, parent: &Scope) {
        self.model.refresh(TickerMode {
            enabled: self.enabled && mode(parent),
        });
    }
    pub fn child_scope(&self, parent: &Scope) -> Scope {
        self.model.child_scope(parent)
    }
}
pub(super) struct AnimationOwner {
    controllers: RefCell<Vec<WeakAnimationController>>,
    enabled: Cell<bool>,
    closed: Cell<bool>,
}
impl AnimationOwner {
    pub fn new(scope: &Scope) -> Self {
        Self {
            controllers: RefCell::new(vec![]),
            enabled: Cell::new(mode(scope)),
            closed: Cell::new(false),
        }
    }
    pub fn refresh(&self, scope: &Scope) {
        let enabled = mode(scope);
        if self.closed.get() || self.enabled.replace(enabled) == enabled {
            return;
        }
        let controllers = self
            .controllers
            .borrow()
            .iter()
            .filter_map(|weak| weak.upgrade())
            .collect::<Vec<_>>();
        for controller in controllers {
            controller.set_scope_muted(!enabled);
        }
    }
    pub fn create(
        &self,
        ctx: &egui::Context,
        config: AnimationConfig,
    ) -> Result<AnimationController, AnimationError> {
        if self.closed.get() {
            return Err(AnimationError::Unmounted);
        }
        let controller = AnimationController::new(ctx, config)?;
        controller.set_scope_muted(!self.enabled.get());
        let mut registered = self.controllers.borrow_mut();
        registered.retain(|weak| {
            weak.upgrade()
                .is_some_and(|controller| !controller.is_disposed())
        });
        registered.push(controller.downgrade());
        Ok(controller)
    }
    pub fn close(&self) {
        if self.closed.replace(true) {
            return;
        }
        let controllers = std::mem::take(&mut *self.controllers.borrow_mut());
        for weak in controllers {
            if let Some(controller) = weak.upgrade() {
                controller.retire();
            }
        }
    }
}
impl BuildContext {
    /// Create once during initialization/attachment. This element owns teardown;
    /// all clones become disposed when it unmounts and inherited modes mute it.
    pub fn animation_controller(
        &self,
        config: AnimationConfig,
    ) -> Result<AnimationController, AnimationError> {
        if !self.mounted() {
            return Err(AnimationError::Unmounted);
        }
        self.dependencies.animations.create(self.egui(), config)
    }
}

use super::{ScrollError, ScrollPosition};
use crate::{
    animation::{AnimationRun, Curve},
    egui,
    notifier::{ChangeNotifier, Disposed, Listenable, Listener, Subscription},
};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    time::Duration,
};
struct Entry {
    key: Rc<()>,
    position: ScrollPosition,
    _subscription: Subscription,
}
struct Inner {
    initial: f32,
    entries: RefCell<Vec<Entry>>,
    notifier: ChangeNotifier,
    disposed: Cell<bool>,
    depth: Cell<usize>,
    command: RefCell<Rc<()>>,
}
#[derive(Clone)]
pub struct ScrollController(Rc<Inner>);
#[derive(Clone)]
pub struct WeakScrollController(Weak<Inner>);
impl WeakScrollController {
    pub fn upgrade(&self) -> Option<ScrollController> {
        self.0.upgrade().map(ScrollController)
    }
}
struct Target {
    key: Rc<()>,
    position: ScrollPosition,
    stamp: Rc<()>,
}
impl Default for ScrollController {
    fn default() -> Self {
        Self::new(0.0).expect("finite default offset")
    }
}
impl ScrollController {
    pub fn new(initial_offset: f32) -> Result<Self, ScrollError> {
        if !initial_offset.is_finite() {
            return Err(ScrollError::InvalidOffset);
        }
        Ok(Self(Rc::new(Inner {
            initial: initial_offset,
            entries: RefCell::new(vec![]),
            notifier: ChangeNotifier::new(),
            disposed: Cell::new(false),
            depth: Cell::new(0),
            command: RefCell::new(Rc::new(())),
        })))
    }
    pub fn downgrade(&self) -> WeakScrollController {
        WeakScrollController(Rc::downgrade(&self.0))
    }
    fn command(&self) -> Rc<()> {
        let command = Rc::new(());
        *self.0.command.borrow_mut() = command.clone();
        command
    }
    fn current(&self, command: &Rc<()>) -> bool {
        !self.is_disposed() && Rc::ptr_eq(command, &self.0.command.borrow())
    }
    fn check(&self) -> Result<(), ScrollError> {
        if self.is_disposed() {
            Err(ScrollError::Disposed)
        } else {
            Ok(())
        }
    }
    pub fn is_disposed(&self) -> bool {
        self.0.disposed.get()
    }
    pub fn initial_offset(&self) -> f32 {
        self.0.initial
    }
    pub fn create_position(&self, ctx: &egui::Context) -> Result<ScrollPosition, ScrollError> {
        self.check()?;
        ScrollPosition::new(ctx, self.0.initial)
    }
    pub fn positions(&self) -> Vec<ScrollPosition> {
        self.0
            .entries
            .borrow()
            .iter()
            .map(|e| e.position.clone())
            .collect()
    }
    pub fn has_clients(&self) -> bool {
        !self.0.entries.borrow().is_empty()
    }
    pub fn position(&self) -> Result<ScrollPosition, ScrollError> {
        self.check()?;
        let entries = self.0.entries.borrow();
        match entries.len() {
            0 => Err(ScrollError::NoClients),
            1 => Ok(entries[0].position.clone()),
            _ => Err(ScrollError::MultipleClients),
        }
    }
    pub fn offset(&self) -> Result<f32, ScrollError> {
        let p = self.position()?;
        if p.is_disposed() {
            return Err(ScrollError::Disposed);
        }
        Ok(p.pixels())
    }
    /// The owner must retain the attachment for as long as it uses this position.
    pub fn attach(&self, position: &ScrollPosition) -> Result<ScrollAttachment, ScrollError> {
        self.check()?;
        if position.is_disposed() {
            return Err(ScrollError::Disposed);
        }
        if self
            .0
            .entries
            .borrow()
            .iter()
            .any(|e| e.position.same_source(position))
        {
            return Err(ScrollError::AlreadyAttached);
        }
        let key = Rc::new(());
        let weak = Rc::downgrade(&self.0);
        let weak_key = Rc::downgrade(&key);
        let subscription = position
            .subscribe(Rc::new(move || {
                if let (Some(inner), Some(key)) = (weak.upgrade(), weak_key.upgrade())
                    && !inner.disposed.get()
                    && inner
                        .entries
                        .borrow()
                        .iter()
                        .any(|e| Rc::ptr_eq(&e.key, &key))
                {
                    inner.depth.set(inner.depth.get() + 1);
                    let _guard = NotifyGuard(&inner.depth);
                    let _ = inner.notifier.notify_listeners();
                }
            }))
            .map_err(|_| ScrollError::Disposed)?;
        self.0.entries.borrow_mut().push(Entry {
            key: key.clone(),
            position: position.clone(),
            _subscription: subscription,
        });
        Ok(ScrollAttachment {
            controller: Rc::downgrade(&self.0),
            key,
            position: position.clone(),
        })
    }
    fn snapshot(&self) -> Result<Vec<Target>, ScrollError> {
        self.check()?;
        let entries = self.0.entries.borrow();
        if entries.is_empty() {
            return Err(ScrollError::NoClients);
        }
        if entries.iter().any(|e| e.position.is_disposed()) {
            return Err(ScrollError::Disposed);
        }
        Ok(entries
            .iter()
            .map(|e| Target {
                key: e.key.clone(),
                position: e.position.clone(),
                stamp: e.position.command_stamp(),
            })
            .collect())
    }
    fn active(&self, key: &Rc<()>) -> bool {
        !self.is_disposed()
            && self
                .0
                .entries
                .borrow()
                .iter()
                .any(|e| Rc::ptr_eq(&e.key, key))
    }
    pub fn jump_to(&self, offset: f32) -> Result<(), ScrollError> {
        if !offset.is_finite() {
            return Err(ScrollError::InvalidOffset);
        }
        let targets = self.snapshot()?;
        let command = self.command();
        for target in targets {
            if !self.current(&command) {
                break;
            }
            if self.active(&target.key) && target.position.current(&target.stamp) {
                target.position.jump_to(offset)?;
            }
        }
        Ok(())
    }
    pub fn animate_to(
        &self,
        target: f32,
        duration: Duration,
        curve: Curve,
    ) -> Result<Vec<AnimationRun>, ScrollError> {
        if !target.is_finite() {
            return Err(ScrollError::InvalidOffset);
        }
        if duration.is_zero() {
            return Err(ScrollError::InvalidDuration);
        }
        let targets = self.snapshot()?;
        if targets.iter().any(|t| t.position.dimensions().is_none()) {
            return Err(ScrollError::NoDimensions);
        }
        let mut runs = vec![];
        let command = self.command();
        for destination in targets {
            if !self.current(&command) {
                break;
            }
            if self.active(&destination.key) && destination.position.current(&destination.stamp) {
                runs.push(destination.position.animate_to(target, duration, curve)?);
            }
        }
        Ok(runs)
    }
    /// Detaches clients without disposing their independently owned positions.
    pub fn dispose(&self) -> Result<(), ScrollError> {
        self.check()?;
        if self.0.depth.get() != 0 {
            return Err(ScrollError::DuringNotification);
        }
        self.0.disposed.set(true);
        let entries = self.0.entries.take();
        self.0.notifier.dispose().expect("dispatch depth checked");
        drop(entries);
        Ok(())
    }
}
impl Listenable for ScrollController {
    fn subscribe(&self, listener: Listener) -> Result<Subscription, Disposed> {
        if self.is_disposed() {
            Err(Disposed)
        } else {
            self.0.notifier.subscribe(listener)
        }
    }
    fn same_source(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
#[must_use = "dropping a scroll attachment detaches the position"]
pub struct ScrollAttachment {
    controller: Weak<Inner>,
    key: Rc<()>,
    position: ScrollPosition,
}
impl ScrollAttachment {
    pub fn position(&self) -> ScrollPosition {
        self.position.clone()
    }
    pub fn is_attached(&self) -> bool {
        self.controller.upgrade().is_some_and(|inner| {
            !inner.disposed.get()
                && inner
                    .entries
                    .borrow()
                    .iter()
                    .any(|e| Rc::ptr_eq(&e.key, &self.key))
        })
    }
    pub fn detach(&mut self) {
        if let Some(inner) = self.controller.upgrade() {
            let removed = {
                let mut entries = inner.entries.borrow_mut();
                entries
                    .iter()
                    .position(|e| Rc::ptr_eq(&e.key, &self.key))
                    .map(|index| entries.remove(index))
            };
            drop(removed);
        }
        self.controller = Weak::new();
    }
}
impl Drop for ScrollAttachment {
    fn drop(&mut self) {
        self.detach();
    }
}
struct NotifyGuard<'a>(&'a Cell<usize>);
impl Drop for NotifyGuard<'_> {
    fn drop(&mut self) {
        self.0.set(self.0.get() - 1);
    }
}

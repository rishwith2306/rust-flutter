use super::*;
#[path = "scene.rs"]
mod scene;
use crate::{
    egui::{Id, Pos2},
    layout::LayoutBox,
};
pub use scene::GestureFrame;
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    rc::{Rc, Weak},
};

#[derive(Clone, Default)]
pub struct GestureTargetConfig {
    pub tap: Option<(TapConfig, TapCallbacks)>,
    pub pan: Option<(PanConfig, PanCallbacks)>,
}
impl GestureTargetConfig {
    fn tap_config(&self) -> Option<TapConfig> {
        self.tap
            .as_ref()
            .filter(|(_, c)| c.on_down.is_some() || c.on_tap.is_some() || c.on_cancel.is_some())
            .map(|(c, _)| *c)
    }
    fn pan_config(&self) -> Option<PanConfig> {
        self.pan
            .as_ref()
            .filter(|(_, c)| {
                c.on_down.is_some()
                    || c.on_start.is_some()
                    || c.on_update.is_some()
                    || c.on_end.is_some()
                    || c.on_cancel.is_some()
            })
            .map(|(c, _)| *c)
    }
    pub(super) fn validate(&self) -> Result<(), BindingError> {
        let valid = |v: f32| v.is_finite() && v >= 0.0;
        if self
            .tap
            .as_ref()
            .is_some_and(|(c, _)| !valid(c.pre_accept_slop) || !valid(c.post_accept_slop))
            || self.pan.as_ref().is_some_and(|(c, _)| !valid(c.slop))
        {
            Err(BindingError::Recognizer(RecognizerError::InvalidSlop))
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingError {
    HostChanged,
    MultipleRoots,
    FrameAlreadyOpen,
    FrameExpired,
    DuplicateRoot(Id),
    InvalidTransform,
    Disposed,
    IdExhausted,
    TargetAlreadyRegistered(Id),
    TargetExpired,
    DuplicateDown(PointerId),
    MissingDown(PointerId),
    OldTimestamp(PointerId),
    InvalidPosition,
    Recognizer(RecognizerError),
    Arena(ArenaError),
}
impl std::fmt::Display for BindingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "gesture binding: {self:?}")
    }
}
impl std::error::Error for BindingError {}
#[derive(Clone, Debug, PartialEq)]
pub struct DispatchFailure {
    pub event: PointerEvent,
    pub error: BindingError,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DispatchReport {
    /// Number of attempted packets, including failed ones and reentrant packets.
    pub processed: usize,
    /// Reentrant calls enqueue work; the outer dispatch reports its outcome.
    pub queued: bool,
    pub failures: Vec<DispatchFailure>,
}
type Callback<T> = Option<Rc<dyn Fn(T)>>;
fn forward<T: 'static>(
    config: &Rc<RefCell<GestureTargetConfig>>,
    get: fn(&GestureTargetConfig) -> Callback<T>,
) -> Callback<T> {
    let config = config.clone();
    Some(Rc::new(move |value| {
        let callback = get(&config.borrow());
        if let Some(callback) = callback {
            callback(value);
        }
    }))
}
struct Target {
    generation: u64,
    config: Rc<RefCell<GestureTargetConfig>>,
    tap: Option<TapGestureRecognizer>,
    pan: Option<PanGestureRecognizer>,
}
impl Target {
    fn new(
        inner: &Inner,
        generation: u64,
        config: GestureTargetConfig,
    ) -> Result<Self, BindingError> {
        config.validate()?;
        let tap_config = config.tap_config();
        let pan_config = config.pan_config();
        let config = Rc::new(RefCell::new(config));
        let tap = tap_config
            .map(|settings| {
                TapGestureRecognizer::new(
                    &inner.arena,
                    &inner.router,
                    settings,
                    TapCallbacks {
                        on_down: forward(&config, |s| {
                            s.tap.as_ref().and_then(|(_, c)| c.on_down.clone())
                        }),
                        on_tap: forward(&config, |s| {
                            s.tap.as_ref().and_then(|(_, c)| c.on_tap.clone())
                        }),
                        on_cancel: forward(&config, |s| {
                            s.tap.as_ref().and_then(|(_, c)| c.on_cancel.clone())
                        }),
                    },
                )
            })
            .transpose()
            .map_err(BindingError::Recognizer)?;
        let pan = pan_config
            .map(|settings| {
                PanGestureRecognizer::new(
                    &inner.arena,
                    &inner.router,
                    settings,
                    PanCallbacks {
                        on_down: forward(&config, |s| {
                            s.pan.as_ref().and_then(|(_, c)| c.on_down.clone())
                        }),
                        on_start: forward(&config, |s| {
                            s.pan.as_ref().and_then(|(_, c)| c.on_start.clone())
                        }),
                        on_update: forward(&config, |s| {
                            s.pan.as_ref().and_then(|(_, c)| c.on_update.clone())
                        }),
                        on_end: forward(&config, |s| {
                            s.pan.as_ref().and_then(|(_, c)| c.on_end.clone())
                        }),
                        on_cancel: forward(&config, |s| {
                            s.pan.as_ref().and_then(|(_, c)| c.on_cancel.clone())
                        }),
                    },
                )
            })
            .transpose()
            .map_err(BindingError::Recognizer)?;
        Ok(Self {
            generation,
            config,
            tap,
            pan,
        })
    }
    fn add(
        self: &Rc<Self>,
        inner: &Inner,
        id: Id,
        event: PointerEvent,
    ) -> Result<(), BindingError> {
        if let Some(tap) = &self.tap {
            tap.add_pointer(event).map_err(BindingError::Recognizer)?;
        }
        if !inner
            .state
            .borrow()
            .targets
            .get(&id)
            .is_some_and(|t| Rc::ptr_eq(t, self))
        {
            self.cancel();
            return Ok(());
        }
        if let Some(pan) = &self.pan {
            pan.add_pointer(event).map_err(BindingError::Recognizer)?;
        }
        if !inner
            .state
            .borrow()
            .targets
            .get(&id)
            .is_some_and(|t| Rc::ptr_eq(t, self))
        {
            self.cancel();
        }
        Ok(())
    }
    fn cancel(&self) {
        if let Some(tap) = &self.tap {
            tap.cancel();
        }
        if let Some(pan) = &self.pan {
            pan.cancel();
        }
    }
}
struct Packet {
    event: PointerEvent,
    path: Vec<(Id, Weak<Target>)>,
}
#[derive(Default)]
struct State {
    next: u64,
    disposed: bool,
    targets: HashMap<Id, Rc<Target>>,
    active: BTreeMap<PointerId, PointerEvent>,
    queue: VecDeque<Packet>,
}
struct Inner {
    state: RefCell<State>,
    dispatching: Cell<bool>,
    arena: GestureArenaManager,
    router: PointerRouter<PointerEvent>,
    host: RefCell<Option<EguiHost>>,
}
struct EguiHost {
    context: crate::egui::Context,
    viewport: crate::egui::ViewportId,
    root: Option<Id>,
    frame: Option<u64>,
    collecting: Option<(u64, u64)>,
    next_lease: u64,
    previous_targets: Vec<Weak<Target>>,
}
impl Inner {
    fn remove(&self, id: Id, generation: u64) -> bool {
        let target = {
            let mut state = self.state.borrow_mut();
            if !state
                .targets
                .get(&id)
                .is_some_and(|t| t.generation == generation)
            {
                return false;
            }
            state.targets.remove(&id)
        };
        if let Some(target) = target {
            target.cancel();
        }
        true
    }
    fn shutdown(&self) {
        let targets = {
            let mut state = self.state.borrow_mut();
            state.disposed = true;
            state.active.clear();
            state.queue.clear();
            std::mem::take(&mut state.targets)
        };
        let _ = self.arena.dispose();
        for target in targets.values() {
            target.cancel();
        }
        self.router.dispose();
    }
    fn dispatch_one(&self, packet: Packet) -> Result<(), BindingError> {
        let event = packet.event;
        if self.arena.is_disposed() || self.router.is_disposed() {
            self.shutdown();
            return Err(BindingError::Disposed);
        }
        {
            let mut state = self.state.borrow_mut();
            if state.disposed {
                return Err(BindingError::Disposed);
            }
            if event.phase != PointerPhase::Cancel && !event.position.is_finite() {
                drop(state);
                if event.phase != PointerPhase::Down {
                    self.cancel_pointer(event.pointer);
                }
                return Err(BindingError::InvalidPosition);
            }
            if event.phase == PointerPhase::Down {
                if state.active.contains_key(&event.pointer) {
                    return Err(BindingError::DuplicateDown(event.pointer));
                }
            } else {
                let old = state
                    .active
                    .get(&event.pointer)
                    .ok_or(BindingError::MissingDown(event.pointer))?;
                if event.phase != PointerPhase::Cancel && event.time < old.time {
                    return Err(BindingError::OldTimestamp(event.pointer));
                }
            }
            state.active.insert(event.pointer, event);
        }
        if event.phase == PointerPhase::Down {
            for (id, target_at_hit) in packet.path {
                let target = self
                    .state
                    .borrow()
                    .targets
                    .get(&id)
                    .filter(|t| Weak::ptr_eq(&Rc::downgrade(t), &target_at_hit))
                    .cloned();
                if let Some(target) = target {
                    if let Err(error) = target.add(self, id, event) {
                        self.cancel_pointer(event.pointer);
                        if self.arena.is_disposed() || self.router.is_disposed() {
                            self.shutdown();
                        }
                        return Err(error);
                    }
                    if self.state.borrow().disposed {
                        return Ok(());
                    }
                    if self.arena.is_disposed() || self.router.is_disposed() {
                        self.shutdown();
                        return Err(BindingError::Disposed);
                    }
                }
            }
        }
        self.router.route(event.pointer, &event);
        if self.state.borrow().disposed {
            return Ok(());
        }
        if self.arena.is_disposed() || self.router.is_disposed() {
            self.shutdown();
            return Err(BindingError::Disposed);
        }
        match event.phase {
            PointerPhase::Down => self
                .arena
                .close(event.pointer)
                .map_err(BindingError::Arena)?,
            PointerPhase::Up => {
                self.arena
                    .sweep(event.pointer)
                    .map_err(BindingError::Arena)?;
                self.state.borrow_mut().active.remove(&event.pointer);
            }
            PointerPhase::Cancel => {
                self.state.borrow_mut().active.remove(&event.pointer);
                self.arena.cancel(event.pointer);
            }
            PointerPhase::Move => {}
        }
        self.arena.flush();
        Ok(())
    }
    fn cancel_pointer(&self, pointer: PointerId) {
        let last = self.state.borrow_mut().active.remove(&pointer);
        if let Some(last) = last {
            self.router.route(
                pointer,
                &PointerEvent {
                    phase: PointerPhase::Cancel,
                    buttons: PointerButtons::NONE,
                    ..last
                },
            );
        }
        self.arena.cancel(pointer);
    }
}
impl Drop for Inner {
    fn drop(&mut self) {
        self.shutdown();
    }
}
/// A shared root's pointer dispatcher. Down uses a snapshotted, frontmost/deepest
/// hit path; later samples stay captured until up/cancel. Nested dispatch is
/// queued until the whole current route/arena/cleanup packet completes.
#[derive(Clone)]
pub struct GestureBinding(Rc<Inner>);
#[derive(Clone)]
pub struct WeakGestureBinding(Weak<Inner>);
impl WeakGestureBinding {
    pub fn upgrade(&self) -> Option<GestureBinding> {
        self.0.upgrade().map(GestureBinding)
    }
}
impl Default for GestureBinding {
    fn default() -> Self {
        Self::new()
    }
}
impl GestureBinding {
    pub fn new() -> Self {
        Self(Rc::new(Inner {
            state: RefCell::default(),
            dispatching: Cell::new(false),
            arena: GestureArenaManager::new(),
            router: PointerRouter::new(),
            host: RefCell::default(),
        }))
    }
    pub fn downgrade(&self) -> WeakGestureBinding {
        WeakGestureBinding(Rc::downgrade(&self.0))
    }
    pub fn same_source(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
    pub fn arena(&self) -> &GestureArenaManager {
        &self.0.arena
    }
    pub fn router(&self) -> &PointerRouter<PointerEvent> {
        &self.0.router
    }
    pub fn is_disposed(&self) -> bool {
        self.0.state.borrow().disposed
    }
    pub fn active_pointers(&self) -> usize {
        self.0.state.borrow().active.len()
    }
    pub fn target_count(&self) -> usize {
        self.0.state.borrow().targets.len()
    }
    pub fn dispose(&self) {
        self.0.shutdown();
    }
    pub fn register_target(
        &self,
        id: Id,
        config: GestureTargetConfig,
    ) -> Result<GestureTarget, BindingError> {
        let state = self.0.state.borrow();
        if state.disposed {
            return Err(BindingError::Disposed);
        }
        if state.targets.contains_key(&id) {
            return Err(BindingError::TargetAlreadyRegistered(id));
        }
        let generation = state.next.checked_add(1).ok_or(BindingError::IdExhausted)?;
        drop(state);
        let target = Target::new(&self.0, generation, config)?;
        let mut state = self.0.state.borrow_mut();
        state.next = generation;
        state.targets.insert(id, Rc::new(target));
        Ok(GestureTarget {
            owner: Rc::downgrade(&self.0),
            id,
            generation,
        })
    }
    pub fn dispatch(
        &self,
        event: PointerEvent,
        hit_path: impl IntoIterator<Item = Id>,
    ) -> DispatchReport {
        let mut seen = HashSet::new();
        let ids: Vec<_> = hit_path.into_iter().filter(|id| seen.insert(*id)).collect();
        let path = {
            let state = self.0.state.borrow();
            ids.into_iter()
                .filter_map(|id| state.targets.get(&id).map(|t| (id, Rc::downgrade(t))))
                .collect()
        };
        self.0
            .state
            .borrow_mut()
            .queue
            .push_back(Packet { event, path });
        if self.0.dispatching.replace(true) {
            return DispatchReport {
                queued: true,
                ..Default::default()
            };
        }
        struct Guard<'a>(&'a Cell<bool>);
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _guard = Guard(&self.0.dispatching);
        let mut report = DispatchReport::default();
        loop {
            let packet = self.0.state.borrow_mut().queue.pop_front();
            let Some(packet) = packet else {
                break;
            };
            let event = packet.event;
            report.processed += 1;
            if let Err(error) = self.0.dispatch_one(packet) {
                report.failures.push(DispatchFailure { event, error });
            }
        }
        report
    }
    /// The event's global coordinates and root origin must share logical units.
    /// Geometry is consulted only for down; later events follow captured routes.
    pub fn dispatch_layout(
        &self,
        event: PointerEvent,
        geometry: &LayoutBox,
        origin: Pos2,
    ) -> DispatchReport {
        let path = if event.phase == PointerPhase::Down {
            geometry
                .hit_test_path(event.position - origin.to_vec2())
                .into_iter()
                .map(|e| e.id)
                .collect()
        } else {
            Vec::new()
        };
        self.dispatch(event, path)
    }
    /// Queue cancellation for all currently live sequences. Reentrant requests
    /// wait until the current packet completes; disposal is immediate instead.
    pub fn cancel_all(&self) -> DispatchReport {
        let events: Vec<_> = {
            let state = self.0.state.borrow();
            let pending: HashSet<_> = state
                .queue
                .iter()
                .filter(|p| p.event.phase == PointerPhase::Cancel)
                .map(|p| p.event.pointer)
                .collect();
            state
                .active
                .values()
                .filter(|e| {
                    !matches!(e.phase, PointerPhase::Up | PointerPhase::Cancel)
                        && !pending.contains(&e.pointer)
                })
                .copied()
                .collect()
        };
        let mut report = DispatchReport::default();
        for event in events {
            if !self.0.state.borrow().active.contains_key(&event.pointer) {
                continue;
            }
            let r = self.dispatch(
                PointerEvent {
                    phase: PointerPhase::Cancel,
                    buttons: PointerButtons::NONE,
                    ..event
                },
                [],
            );
            report.processed += r.processed;
            report.queued |= r.queued;
            report.failures.extend(r.failures);
        }
        report
    }
    /// Adapter for one egui viewport/root per binding. Delivers its normalized
    /// primary pointer once per frame, captures movement outside the root and
    /// cancels on focus/pointer loss, disable or other-button changes. Backend
    /// controls do not automatically join this arena.
    pub fn handle_egui(
        &self,
        ui: &crate::egui::Ui,
        geometry: &LayoutBox,
        origin: Pos2,
    ) -> Result<DispatchReport, BindingError> {
        use crate::egui::{Event, PointerButton};
        if self.is_disposed() {
            return Err(BindingError::Disposed);
        }
        if !origin.is_finite() {
            return Err(BindingError::InvalidPosition);
        }
        let frame = ui.ctx().cumulative_frame_nr();
        let viewport = ui.ctx().viewport_id();
        let mut switched = false;
        {
            let mut host = self.0.host.borrow_mut();
            if let Some(host) = host.as_mut() {
                if host.context != *ui.ctx() || host.viewport != viewport {
                    return Err(BindingError::HostChanged);
                }
                if host.collecting.is_some() {
                    return Err(BindingError::FrameAlreadyOpen);
                }
                if host.frame == Some(frame) {
                    return if host.root == Some(geometry.id) {
                        Ok(DispatchReport::default())
                    } else {
                        Err(BindingError::MultipleRoots)
                    };
                }
                switched = host.frame.is_some() && host.root.is_none();
                host.previous_targets.clear();
                host.frame = Some(frame);
                host.root = Some(geometry.id);
            } else {
                *host = Some(EguiHost {
                    context: ui.ctx().clone(),
                    viewport,
                    root: Some(geometry.id),
                    frame: Some(frame),
                    collecting: None,
                    next_lease: 0,
                    previous_targets: Vec::new(),
                });
            }
        }
        let (focused, time, events) = ui.input(|i| (i.focused, i.time, i.events.clone()));
        let mut report = if switched {
            self.cancel_all()
        } else {
            DispatchReport::default()
        };
        if !focused || !ui.is_enabled() {
            report.merge(self.cancel_all());
            return Ok(report);
        }
        let time = std::time::Duration::try_from_secs_f64(time).unwrap_or_default();
        let pointer = PointerId(0);
        let sample = |phase, position, buttons| PointerEvent {
            pointer,
            phase,
            position,
            time,
            buttons,
            device: PointerDeviceKind::Unknown,
        };
        let mut merge = |r: DispatchReport| {
            report.processed += r.processed;
            report.queued |= r.queued;
            report.failures.extend(r.failures);
        };
        for event in events {
            let active = self.0.state.borrow().active.contains_key(&pointer);
            match event {
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: true,
                    ..
                } => {
                    let p = pos - origin.to_vec2();
                    let clip = ui.clip_rect();
                    if pos.x >= clip.min.x
                        && pos.y >= clip.min.y
                        && pos.x < clip.max.x
                        && pos.y < clip.max.y
                        && p.x >= 0.0
                        && p.y >= 0.0
                        && p.x < geometry.size.x
                        && p.y < geometry.size.y
                    {
                        if active {
                            merge(self.cancel_all());
                        }
                        merge(self.dispatch_layout(
                            sample(PointerPhase::Down, pos, PointerButtons::PRIMARY),
                            geometry,
                            origin,
                        ));
                    }
                }
                Event::PointerMoved(pos) if active => merge(
                    self.dispatch(sample(PointerPhase::Move, pos, PointerButtons::PRIMARY), []),
                ),
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: false,
                    ..
                } if active => {
                    merge(self.dispatch(sample(PointerPhase::Up, pos, PointerButtons::NONE), []))
                }
                Event::PointerGone | Event::WindowFocused(false) => merge(self.cancel_all()),
                Event::PointerButton { button, .. }
                    if active && button != PointerButton::Primary =>
                {
                    merge(self.cancel_all())
                }
                _ => {}
            }
        }
        self.0.arena.flush();
        Ok(report)
    }
}
/// Owned target registration. Drop cancels its recognizers, preserving other
/// targets in the same pointer arena. Tokens do not retain their binding.
#[must_use = "dropping the gesture target unregisters it"]
pub struct GestureTarget {
    owner: Weak<Inner>,
    id: Id,
    generation: u64,
}
impl GestureTarget {
    pub fn is_active(&self) -> bool {
        self.owner.upgrade().is_some_and(|i| {
            i.state
                .borrow()
                .targets
                .get(&self.id)
                .is_some_and(|t| t.generation == self.generation)
        })
    }
    pub fn cancel(&self) -> bool {
        self.owner
            .upgrade()
            .is_some_and(|i| i.remove(self.id, self.generation))
    }
    /// Callback-only updates preserve active recognition. Settings or enabled
    /// recognizer changes replace the target and cancel its current candidates.
    pub fn update(&self, config: GestureTargetConfig) -> Result<(), BindingError> {
        config.validate()?;
        let inner = self.owner.upgrade().ok_or(BindingError::Disposed)?;
        let old = inner
            .state
            .borrow()
            .targets
            .get(&self.id)
            .filter(|t| t.generation == self.generation)
            .cloned()
            .ok_or(BindingError::TargetExpired)?;
        let same = {
            let c = old.config.borrow();
            c.tap_config() == config.tap_config() && c.pan_config() == config.pan_config()
        };
        if same {
            let dropped = std::mem::replace(&mut *old.config.borrow_mut(), config);
            drop(dropped);
        } else {
            let new = Rc::new(Target::new(&inner, self.generation, config)?);
            inner.state.borrow_mut().targets.insert(self.id, new);
            old.cancel();
        }
        Ok(())
    }
}
impl Drop for GestureTarget {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exhausted_frame_leases_do_not_leave_a_collector_open() {
        let binding = GestureBinding::new();
        let context = crate::egui::Context::default();
        drop(binding.begin_egui_frame(&context).unwrap());
        binding.0.host.borrow_mut().as_mut().unwrap().next_lease = u64::MAX;
        assert!(matches!(
            binding.begin_egui_frame(&context),
            Err(BindingError::IdExhausted)
        ));
        assert!(
            binding
                .0
                .host
                .borrow()
                .as_ref()
                .unwrap()
                .collecting
                .is_none()
        );
        assert_eq!(binding.active_pointers(), 0);
    }
    #[test]
    fn exhausted_target_ids_leave_registry_unchanged() {
        let binding = GestureBinding::new();
        binding.0.state.borrow_mut().next = u64::MAX;
        assert!(matches!(
            binding.register_target(Id::new(1), GestureTargetConfig::default()),
            Err(BindingError::IdExhausted)
        ));
        assert_eq!(binding.target_count(), 0);
        assert_eq!(binding.active_pointers(), 0);
        assert_eq!(binding.router().route_count(), 0);
    }
}

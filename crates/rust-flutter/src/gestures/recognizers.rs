use super::*;
use crate::egui::{Pos2, Vec2};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::{Rc, Weak},
    time::Duration,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerPhase {
    Down,
    Move,
    Up,
    Cancel,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerDeviceKind {
    Mouse,
    Touch,
    Stylus,
    Unknown,
}
/// Button bitmask. These initial recognizers accept exactly PRIMARY on down.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PointerButtons(pub u64);
impl PointerButtons {
    pub const NONE: Self = Self(0);
    pub const PRIMARY: Self = Self(1);
    pub const SECONDARY: Self = Self(2);
    pub const TERTIARY: Self = Self(4);
}
/// A host-normalized sample in logical global coordinates. Timestamps must be
/// monotonic within a sequence; device IDs/coordinate conversion belong to hosts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerEvent {
    pub pointer: PointerId,
    pub phase: PointerPhase,
    pub position: Pos2,
    pub time: Duration,
    pub device: PointerDeviceKind,
    pub buttons: PointerButtons,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TapConfig {
    pub pre_accept_slop: f32,
    pub post_accept_slop: f32,
}
impl Default for TapConfig {
    fn default() -> Self {
        Self {
            pre_accept_slop: 18.0,
            post_accept_slop: 18.0,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DragStartBehavior {
    /// Start at the recognition position; omit movement accumulated before it.
    #[default]
    Start,
    /// Start at down; deliver accumulated displacement as the first update.
    Down,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanConfig {
    pub slop: f32,
    pub start_behavior: DragStartBehavior,
}
impl Default for PanConfig {
    fn default() -> Self {
        Self {
            slop: 18.0,
            start_behavior: DragStartBehavior::Start,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TapDetails {
    pub down: PointerEvent,
    pub up: PointerEvent,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragUpdateDetails {
    pub event: PointerEvent,
    pub delta: Vec2,
}
/// Down is immediate candidate feedback, not an arena victory. Cancel follows a
/// candidate that cannot complete; tap requires both an up and an arena win.
#[derive(Clone, Default)]
pub struct TapCallbacks {
    pub on_down: Option<Rc<dyn Fn(PointerEvent)>>,
    pub on_tap: Option<Rc<dyn Fn(TapDetails)>>,
    pub on_cancel: Option<Rc<dyn Fn(PointerId)>>,
}
/// End currently contains the final sample, without velocity/fling estimation.
#[derive(Clone, Default)]
pub struct PanCallbacks {
    pub on_down: Option<Rc<dyn Fn(PointerEvent)>>,
    pub on_start: Option<Rc<dyn Fn(PointerEvent)>>,
    pub on_update: Option<Rc<dyn Fn(DragUpdateDetails)>>,
    pub on_end: Option<Rc<dyn Fn(PointerEvent)>>,
    pub on_cancel: Option<Rc<dyn Fn(PointerId)>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecognizerError {
    InvalidSlop,
    InvalidPosition,
    ExpectedDown,
    Disposed,
    ServicesUnavailable,
    IdExhausted,
    Arena(ArenaError),
    Router(PointerRouterError),
}
impl std::fmt::Display for RecognizerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSlop => f.write_str("gesture slop must be finite and nonnegative"),
            Self::InvalidPosition => f.write_str("pointer position must be finite"),
            Self::ExpectedDown => f.write_str("recognizer registration requires a down event"),
            Self::Disposed => f.write_str("gesture recognizer is disposed"),
            Self::ServicesUnavailable => {
                f.write_str("gesture arena or pointer router is unavailable")
            }
            Self::IdExhausted => f.write_str("recognizer sequence ID space exhausted"),
            Self::Arena(e) => e.fmt(f),
            Self::Router(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for RecognizerError {}

enum Policy {
    Tap(TapConfig, TapCallbacks),
    Pan(PanConfig, PanCallbacks),
}
struct Sequence {
    generation: u64,
    down: PointerEvent,
    last: PointerEvent,
    won: bool,
    threshold: bool,
    started: bool,
    up: bool,
    entry: GestureArenaEntry,
    route: Option<PointerRoute<PointerEvent>>,
}
#[derive(Default)]
struct State {
    next: u64,
    disposed: bool,
    active: Option<Sequence>,
}
struct Core {
    state: RefCell<State>,
    work: RefCell<VecDeque<Work>>,
    dispatching: Cell<bool>,
    arena: WeakGestureArenaManager,
    router: WeakPointerRouter<PointerEvent>,
    policy: Policy,
}
enum Work {
    Event(u64, PointerEvent),
    Accept(u64),
}
struct Member {
    owner: Weak<Core>,
    generation: u64,
}
impl GestureArenaMember for Member {
    fn accept_gesture(&self, _: PointerId) {
        if let Some(core) = self.owner.upgrade() {
            core.accept(self.generation);
        }
    }
    fn reject_gesture(&self, _: PointerId) {
        if let Some(core) = self.owner.upgrade() {
            core.cancel_generation(self.generation);
        }
    }
}
fn invoke<T>(callback: &Option<Rc<dyn Fn(T)>>, value: T) {
    if let Some(callback) = callback {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback(value)));
    }
}
fn valid_slop(slop: f32) -> bool {
    slop.is_finite() && slop >= 0.0
}
fn distance(a: Pos2, b: Pos2) -> f64 {
    (f64::from(a.x) - f64::from(b.x)).hypot(f64::from(a.y) - f64::from(b.y))
}
impl Core {
    fn new(
        arena: &GestureArenaManager,
        router: &PointerRouter<PointerEvent>,
        policy: Policy,
    ) -> Rc<Self> {
        Rc::new(Self {
            state: RefCell::default(),
            work: RefCell::default(),
            dispatching: Cell::new(false),
            arena: arena.downgrade(),
            router: router.downgrade(),
            policy,
        })
    }
    fn add(self: &Rc<Self>, event: PointerEvent) -> Result<bool, RecognizerError> {
        let generation = {
            let state = self.state.borrow();
            if state.disposed {
                return Err(RecognizerError::Disposed);
            }
            if event.phase != PointerPhase::Down {
                return Err(RecognizerError::ExpectedDown);
            }
            if !event.position.is_finite() {
                return Err(RecognizerError::InvalidPosition);
            }
            if state.active.is_some() || event.buttons != PointerButtons::PRIMARY {
                return Ok(false);
            }
            let has_callbacks = match &self.policy {
                Policy::Tap(_, cb) => {
                    cb.on_down.is_some() || cb.on_tap.is_some() || cb.on_cancel.is_some()
                }
                Policy::Pan(_, cb) => {
                    cb.on_down.is_some()
                        || cb.on_start.is_some()
                        || cb.on_update.is_some()
                        || cb.on_end.is_some()
                        || cb.on_cancel.is_some()
                }
            };
            if !has_callbacks {
                return Ok(false);
            }
            state
                .next
                .checked_add(1)
                .ok_or(RecognizerError::IdExhausted)?
        };
        let arena = self
            .arena
            .upgrade()
            .ok_or(RecognizerError::ServicesUnavailable)?;
        let router = self
            .router
            .upgrade()
            .ok_or(RecognizerError::ServicesUnavailable)?;
        let owner = Rc::downgrade(self);
        let route = router
            .add_route(event.pointer, move |pointer, event| {
                if pointer == event.pointer
                    && let Some(core) = owner.upgrade()
                {
                    core.handle(generation, *event);
                }
            })
            .map_err(RecognizerError::Router)?;
        // Register routing first so failure cannot leave an empty open arena.
        // Arena failure drops the route before any candidate-down callback.
        let entry = arena
            .add(
                event.pointer,
                Rc::new(Member {
                    owner: Rc::downgrade(self),
                    generation,
                }),
            )
            .map_err(RecognizerError::Arena)?;
        {
            let mut state = self.state.borrow_mut();
            state.next = generation;
            state.active = Some(Sequence {
                generation,
                down: event,
                last: event,
                won: false,
                threshold: false,
                started: false,
                up: false,
                entry,
                route: Some(route),
            });
        }
        match &self.policy {
            Policy::Tap(_, cb) => invoke(&cb.on_down, event),
            Policy::Pan(_, cb) => invoke(&cb.on_down, event),
        }
        Ok(true)
    }
    fn matches(&self, generation: u64) -> bool {
        self.state
            .borrow()
            .active
            .as_ref()
            .is_some_and(|s| s.generation == generation)
    }
    fn cancel_generation(&self, generation: u64) {
        let sequence = {
            let mut state = self.state.borrow_mut();
            if !state
                .active
                .as_ref()
                .is_some_and(|s| s.generation == generation)
            {
                return;
            }
            state.active.take().unwrap()
        };
        let pointer = sequence.down.pointer;
        sequence.entry.resolve(GestureDisposition::Rejected);
        drop(sequence); // owned route and captures are dropped outside state borrows
        match &self.policy {
            Policy::Tap(_, cb) => invoke(&cb.on_cancel, pointer),
            Policy::Pan(_, cb) => invoke(&cb.on_cancel, pointer),
        }
    }
    fn cancel(&self) {
        let generation = self.state.borrow().active.as_ref().map(|s| s.generation);
        if let Some(generation) = generation {
            self.cancel_generation(generation);
        }
    }
    fn dispose(&self) {
        self.state.borrow_mut().disposed = true;
        self.cancel();
    }
    fn dispatch(&self, work: Work) {
        self.work.borrow_mut().push_back(work);
        if self.dispatching.replace(true) {
            return;
        }
        struct Guard<'a>(&'a Cell<bool>);
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _guard = Guard(&self.dispatching);
        loop {
            let work = self.work.borrow_mut().pop_front();
            match work {
                Some(Work::Accept(generation)) => self.accept_now(generation),
                Some(Work::Event(generation, event)) => self.handle_now(generation, event),
                None => break,
            }
        }
    }
    fn accept(&self, generation: u64) {
        self.dispatch(Work::Accept(generation));
    }
    fn accept_now(&self, generation: u64) {
        {
            let mut state = self.state.borrow_mut();
            let Some(s) = state.active.as_mut().filter(|s| s.generation == generation) else {
                return;
            };
            s.won = true;
        }
        self.complete_or_start(generation);
    }
    fn complete_or_start(&self, generation: u64) {
        match &self.policy {
            Policy::Tap(_, cb) => {
                let completed = {
                    let mut state = self.state.borrow_mut();
                    if state
                        .active
                        .as_ref()
                        .is_some_and(|s| s.generation == generation && s.won && s.up)
                    {
                        state.active.take()
                    } else {
                        None
                    }
                };
                if let Some(s) = completed {
                    let details = TapDetails {
                        down: s.down,
                        up: s.last,
                    };
                    drop(s);
                    invoke(&cb.on_tap, details);
                }
            }
            Policy::Pan(config, cb) => {
                let start = {
                    let mut state = self.state.borrow_mut();
                    let Some(s) = state.active.as_mut().filter(|s| s.generation == generation)
                    else {
                        return;
                    };
                    if !s.won || !s.threshold || s.started {
                        return;
                    }
                    s.started = true;
                    Some((s.down, s.last))
                };
                if let Some((down, last)) = start {
                    let delta = last.position - down.position;
                    if config.start_behavior == DragStartBehavior::Down && !delta.is_finite() {
                        self.cancel_generation(generation);
                        return;
                    }
                    invoke(
                        &cb.on_start,
                        match config.start_behavior {
                            DragStartBehavior::Down => down,
                            DragStartBehavior::Start => last,
                        },
                    );
                    if self.matches(generation)
                        && config.start_behavior == DragStartBehavior::Down
                        && delta != Vec2::ZERO
                    {
                        invoke(&cb.on_update, DragUpdateDetails { event: last, delta });
                    }
                    self.finish_pan(generation);
                }
            }
        }
    }
    fn finish_pan(&self, generation: u64) {
        let completed = {
            let mut state = self.state.borrow_mut();
            if state
                .active
                .as_ref()
                .is_some_and(|s| s.generation == generation && s.started && s.up)
            {
                state.active.take()
            } else {
                None
            }
        };
        if let Some(s) = completed {
            let last = s.last;
            drop(s);
            if let Policy::Pan(_, cb) = &self.policy {
                invoke(&cb.on_end, last);
            }
        }
    }
    fn handle(&self, generation: u64, event: PointerEvent) {
        self.dispatch(Work::Event(generation, event));
    }
    fn handle_now(&self, generation: u64, event: PointerEvent) {
        // Cancel must terminate even when its geometry/timestamp is unavailable.
        if event.phase == PointerPhase::Cancel {
            self.cancel_generation(generation);
            return;
        }
        if !event.position.is_finite() {
            self.cancel_generation(generation);
            return;
        }
        let (entry, won, threshold, started, delta, route, reject) = {
            let mut state = self.state.borrow_mut();
            let Some(s) = state
                .active
                .as_mut()
                .filter(|s| s.generation == generation && !s.up)
            else {
                return;
            };
            if event.phase == PointerPhase::Down || event.time < s.last.time {
                return;
            }
            if event.device != s.down.device
                || (event.phase == PointerPhase::Move && event.buttons != s.down.buttons)
                || (event.phase == PointerPhase::Up && event.buttons != PointerButtons::NONE)
            {
                drop(state);
                self.cancel_generation(generation);
                return;
            }
            let delta = event.position - s.last.position;
            if !delta.is_finite() {
                drop(state);
                self.cancel_generation(generation);
                return;
            }
            s.last = event;
            s.up = event.phase == PointerPhase::Up;
            let moved = distance(event.position, s.down.position);
            let reject = match &self.policy {
                Policy::Tap(config, _) => {
                    moved
                        > f64::from(if s.won {
                            config.post_accept_slop
                        } else {
                            config.pre_accept_slop
                        })
                }
                Policy::Pan(config, _) => {
                    s.threshold |= moved > f64::from(config.slop);
                    s.up && !s.threshold
                }
            };
            (
                s.entry.clone(),
                s.won,
                s.threshold,
                s.started,
                delta,
                if s.up { s.route.take() } else { None },
                reject,
            )
        };
        drop(route);
        if reject {
            self.cancel_generation(generation);
            return;
        }
        match &self.policy {
            Policy::Tap(_, _) => {
                if event.phase == PointerPhase::Up {
                    if won {
                        self.complete_or_start(generation);
                    } else {
                        entry.resolve(GestureDisposition::Accepted);
                    }
                }
            }
            Policy::Pan(_, cb) => {
                if started {
                    if delta != Vec2::ZERO {
                        invoke(&cb.on_update, DragUpdateDetails { event, delta });
                    }
                    self.finish_pan(generation);
                } else if threshold {
                    if won {
                        self.complete_or_start(generation);
                    } else {
                        entry.resolve(GestureDisposition::Accepted);
                    }
                }
            }
        }
    }
}
impl Drop for Core {
    fn drop(&mut self) {
        self.cancel();
    }
}

macro_rules! recognizer_handle {
    ($name:ident, $weak:ident) => {
        #[derive(Clone)]
        pub struct $name(Rc<Core>);
        #[derive(Clone)]
        pub struct $weak(Weak<Core>);
        impl $weak {
            pub fn upgrade(&self) -> Option<$name> {
                self.0.upgrade().map($name)
            }
        }
        impl $name {
            /// False means a sequence is already tracked, there are no callbacks,
            /// or the buttons are not exactly primary. The host closes the arena after all down targets
            /// register, routes subsequent samples, then sweeps on up and flushes.
            pub fn add_pointer(&self, down: PointerEvent) -> Result<bool, RecognizerError> {
                self.0.add(down)
            }
            pub fn cancel(&self) {
                self.0.cancel();
            }
            pub fn dispose(&self) {
                self.0.dispose();
            }
            pub fn is_disposed(&self) -> bool {
                self.0.state.borrow().disposed
            }
            pub fn tracked_pointer(&self) -> Option<PointerId> {
                self.0
                    .state
                    .borrow()
                    .active
                    .as_ref()
                    .map(|s| s.down.pointer)
            }
            pub fn downgrade(&self) -> $weak {
                $weak(Rc::downgrade(&self.0))
            }
        }
    };
}
recognizer_handle!(TapGestureRecognizer, WeakTapGestureRecognizer);
recognizer_handle!(PanGestureRecognizer, WeakPanGestureRecognizer);
impl TapGestureRecognizer {
    pub fn new(
        arena: &GestureArenaManager,
        router: &PointerRouter<PointerEvent>,
        config: TapConfig,
        callbacks: TapCallbacks,
    ) -> Result<Self, RecognizerError> {
        if !valid_slop(config.pre_accept_slop) || !valid_slop(config.post_accept_slop) {
            return Err(RecognizerError::InvalidSlop);
        }
        Ok(Self(Core::new(
            arena,
            router,
            Policy::Tap(config, callbacks),
        )))
    }
}
impl PanGestureRecognizer {
    pub fn new(
        arena: &GestureArenaManager,
        router: &PointerRouter<PointerEvent>,
        config: PanConfig,
        callbacks: PanCallbacks,
    ) -> Result<Self, RecognizerError> {
        if !valid_slop(config.slop) {
            return Err(RecognizerError::InvalidSlop);
        }
        Ok(Self(Core::new(
            arena,
            router,
            Policy::Pan(config, callbacks),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exhausted_sequence_id_does_not_register_with_services() {
        let arena = GestureArenaManager::new();
        let router = PointerRouter::new();
        let tap = TapGestureRecognizer::new(
            &arena,
            &router,
            TapConfig::default(),
            TapCallbacks {
                on_tap: Some(Rc::new(|_| {})),
                ..Default::default()
            },
        )
        .unwrap();
        tap.0.state.borrow_mut().next = u64::MAX;
        let down = PointerEvent {
            pointer: PointerId(1),
            phase: PointerPhase::Down,
            position: Pos2::ZERO,
            time: Duration::ZERO,
            device: PointerDeviceKind::Touch,
            buttons: PointerButtons::PRIMARY,
        };
        assert_eq!(tap.add_pointer(down), Err(RecognizerError::IdExhausted));
        assert_eq!(arena.active_pointers(), 0);
        assert_eq!(router.route_count(), 0);
        assert_eq!(tap.tracked_pointer(), None);
    }
}

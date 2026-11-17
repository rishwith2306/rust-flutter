use rust_flutter::{
    egui::{Pos2, Vec2},
    gestures::*,
};
use std::{cell::RefCell, rc::Rc, time::Duration};

#[derive(Clone, Debug, PartialEq)]
enum Record {
    Down(&'static str),
    Cancel(&'static str),
    Tap(Pos2),
    Start(Pos2),
    Update(Vec2),
    End(Pos2),
}
type Log = Rc<RefCell<Vec<Record>>>;
fn event(pointer: u64, phase: PointerPhase, x: f32, millis: u64) -> PointerEvent {
    PointerEvent {
        pointer: PointerId(pointer),
        phase,
        position: Pos2::new(x, 0.0),
        time: Duration::from_millis(millis),
        device: PointerDeviceKind::Touch,
        buttons: if matches!(phase, PointerPhase::Up | PointerPhase::Cancel) {
            PointerButtons::NONE
        } else {
            PointerButtons::PRIMARY
        },
    }
}
fn tap_callbacks(log: &Log) -> TapCallbacks {
    let (a, b, c) = (log.clone(), log.clone(), log.clone());
    TapCallbacks {
        on_down: Some(Rc::new(move |_| a.borrow_mut().push(Record::Down("tap")))),
        on_tap: Some(Rc::new(move |details| {
            b.borrow_mut().push(Record::Tap(details.up.position))
        })),
        on_cancel: Some(Rc::new(move |_| c.borrow_mut().push(Record::Cancel("tap")))),
    }
}
fn pan_callbacks(log: &Log) -> PanCallbacks {
    let (a, b, c, d, e) = (
        log.clone(),
        log.clone(),
        log.clone(),
        log.clone(),
        log.clone(),
    );
    PanCallbacks {
        on_down: Some(Rc::new(move |_| a.borrow_mut().push(Record::Down("pan")))),
        on_start: Some(Rc::new(move |event| {
            b.borrow_mut().push(Record::Start(event.position))
        })),
        on_update: Some(Rc::new(move |details| {
            c.borrow_mut().push(Record::Update(details.delta))
        })),
        on_end: Some(Rc::new(move |event| {
            d.borrow_mut().push(Record::End(event.event.position))
        })),
        on_cancel: Some(Rc::new(move |_| e.borrow_mut().push(Record::Cancel("pan")))),
    }
}
struct Harness {
    arena: GestureArenaManager,
    router: PointerRouter<PointerEvent>,
    tap: TapGestureRecognizer,
    pan: PanGestureRecognizer,
    log: Log,
}
impl Harness {
    fn new(behavior: DragStartBehavior) -> Self {
        let arena = GestureArenaManager::new();
        let router = PointerRouter::new();
        let log = Log::default();
        let tap =
            TapGestureRecognizer::new(&arena, &router, TapConfig::default(), tap_callbacks(&log))
                .unwrap();
        let pan = PanGestureRecognizer::new(
            &arena,
            &router,
            PanConfig {
                start_behavior: behavior,
                ..Default::default()
            },
            pan_callbacks(&log),
        )
        .unwrap();
        Self {
            arena,
            router,
            tap,
            pan,
            log,
        }
    }
    fn down(&self, pointer: u64) {
        let down = event(pointer, PointerPhase::Down, 0.0, 0);
        assert!(self.tap.add_pointer(down).unwrap());
        assert!(self.pan.add_pointer(down).unwrap());
        self.arena.close(down.pointer).unwrap();
        self.arena.flush();
    }
    fn send(&self, phase: PointerPhase, x: f32, millis: u64) {
        let e = event(1, phase, x, millis);
        self.router.route(e.pointer, &e);
        if phase == PointerPhase::Up {
            self.arena.sweep(e.pointer).unwrap();
        }
        self.arena.flush();
    }
}

#[test]
fn tap_waits_for_up_and_arena_win_then_pointer_can_be_reused() {
    let h = Harness::new(DragStartBehavior::Start);
    h.down(1);
    assert_eq!(
        *h.log.borrow(),
        vec![Record::Down("tap"), Record::Down("pan")]
    );
    h.send(PointerPhase::Move, 18.0, 10); // strict greater-than slop
    h.send(PointerPhase::Up, 18.0, 20);
    assert_eq!(
        *h.log.borrow(),
        vec![
            Record::Down("tap"),
            Record::Down("pan"),
            Record::Cancel("pan"),
            Record::Tap(Pos2::new(18.0, 0.0))
        ]
    );
    assert_eq!(h.router.route_count(), 0);
    assert_eq!(h.arena.active_pointers(), 0);
    assert_eq!(h.tap.tracked_pointer(), None);
    assert_eq!(h.pan.tracked_pointer(), None);
    h.down(1);
    h.send(PointerPhase::Up, 0.0, 5);
    assert_eq!(
        h.log
            .borrow()
            .iter()
            .filter(|r| matches!(r, Record::Tap(_)))
            .count(),
        2
    );
}

#[test]
fn pan_wins_after_slop_and_tracks_displacement_outside_initial_region() {
    let h = Harness::new(DragStartBehavior::Down);
    h.down(1);
    h.send(PointerPhase::Move, 20.0, 10);
    h.send(PointerPhase::Move, 45.0, 20);
    h.send(PointerPhase::Up, 50.0, 30);
    assert_eq!(
        *h.log.borrow(),
        vec![
            Record::Down("tap"),
            Record::Down("pan"),
            Record::Cancel("tap"),
            Record::Start(Pos2::ZERO),
            Record::Update(Vec2::new(20.0, 0.0)),
            Record::Update(Vec2::new(25.0, 0.0)),
            Record::Update(Vec2::new(5.0, 0.0)),
            Record::End(Pos2::new(50.0, 0.0))
        ]
    );
    assert_eq!(h.router.route_count(), 0);
    assert_eq!(h.arena.active_pointers(), 0);
}

#[test]
fn start_behavior_omits_pre_accept_movement_and_up_can_cross_threshold() {
    let h = Harness::new(DragStartBehavior::Start);
    h.down(1);
    h.send(PointerPhase::Up, 30.0, 10);
    assert_eq!(
        *h.log.borrow(),
        vec![
            Record::Down("tap"),
            Record::Down("pan"),
            Record::Cancel("tap"),
            Record::Start(Pos2::new(30.0, 0.0)),
            Record::End(Pos2::new(30.0, 0.0))
        ]
    );
    assert_eq!(h.router.route_count(), 0);
}

#[test]
fn early_sole_pan_win_still_requires_threshold_and_cancels_below_it() {
    let h = Harness::new(DragStartBehavior::Start);
    let down = event(1, PointerPhase::Down, 0.0, 0);
    h.pan.add_pointer(down).unwrap();
    h.arena.close(down.pointer).unwrap();
    h.arena.flush();
    h.send(PointerPhase::Move, 5.0, 10);
    h.send(PointerPhase::Up, 5.0, 20);
    assert_eq!(
        *h.log.borrow(),
        vec![Record::Down("pan"), Record::Cancel("pan")]
    );
    assert_eq!(h.router.route_count(), 0);
}

#[test]
fn early_tap_win_uses_post_accept_slop_and_can_still_cancel() {
    let arena = GestureArenaManager::new();
    let router = PointerRouter::new();
    let log = Log::default();
    let tap = TapGestureRecognizer::new(
        &arena,
        &router,
        TapConfig {
            pre_accept_slop: 2.0,
            post_accept_slop: 10.0,
        },
        tap_callbacks(&log),
    )
    .unwrap();
    let down = event(1, PointerPhase::Down, 0.0, 0);
    tap.add_pointer(down).unwrap();
    arena.close(down.pointer).unwrap();
    arena.flush();
    router.route(down.pointer, &event(1, PointerPhase::Move, 8.0, 10));
    assert_eq!(tap.tracked_pointer(), Some(down.pointer));
    router.route(down.pointer, &event(1, PointerPhase::Move, 11.0, 20));
    assert_eq!(
        *log.borrow(),
        vec![Record::Down("tap"), Record::Cancel("tap")]
    );
    assert_eq!(tap.tracked_pointer(), None);
    assert_eq!(router.route_count(), 0);
}

#[test]
fn open_arena_delays_tap_completion_and_preserves_up_details() {
    let h = Harness::new(DragStartBehavior::Start);
    let down = event(1, PointerPhase::Down, 0.0, 0);
    h.tap.add_pointer(down).unwrap();
    h.pan.add_pointer(down).unwrap();
    let up = event(1, PointerPhase::Up, 2.0, 10);
    h.router.route(up.pointer, &up);
    assert_eq!(
        *h.log.borrow(),
        vec![
            Record::Down("tap"),
            Record::Down("pan"),
            Record::Cancel("pan")
        ]
    );
    assert_eq!(h.router.route_count(), 0);
    assert_eq!(h.tap.tracked_pointer(), Some(down.pointer));
    h.arena.close(down.pointer).unwrap();
    h.arena.flush();
    assert_eq!(h.log.borrow().last(), Some(&Record::Tap(up.position)));
}

#[test]
fn extra_pointers_and_non_primary_buttons_are_ignored_without_registration() {
    let h = Harness::new(DragStartBehavior::Start);
    h.down(1);
    assert!(
        !h.tap
            .add_pointer(event(2, PointerPhase::Down, 0.0, 0))
            .unwrap()
    );
    assert!(
        !h.pan
            .add_pointer(event(2, PointerPhase::Down, 0.0, 0))
            .unwrap()
    );
    assert_eq!(h.router.route_count(), 2);
    assert!(h.arena.arena(PointerId(2)).is_none());
    h.send(PointerPhase::Cancel, 0.0, 10);
    let mut other = event(3, PointerPhase::Down, 0.0, 0);
    other.buttons = PointerButtons::SECONDARY;
    assert!(!h.tap.add_pointer(other).unwrap());
    assert!(!h.pan.add_pointer(other).unwrap());
    other.buttons = PointerButtons(3);
    assert!(!h.tap.add_pointer(other).unwrap());
    assert_eq!(h.router.route_count(), 0);
}

#[test]
fn explicit_cancel_and_arena_disposal_notify_once_and_clear_routes() {
    let h = Harness::new(DragStartBehavior::Start);
    h.down(1);
    h.tap.cancel();
    h.tap.cancel();
    h.arena.dispose().unwrap();
    assert_eq!(
        *h.log.borrow(),
        vec![
            Record::Down("tap"),
            Record::Down("pan"),
            Record::Cancel("tap"),
            Record::Cancel("pan")
        ]
    );
    assert_eq!(h.router.route_count(), 0);
    assert_eq!(h.pan.tracked_pointer(), None);
}

#[test]
fn recognizer_disposal_and_final_handle_drop_end_tracking() {
    let arena = GestureArenaManager::new();
    let router = PointerRouter::new();
    let log = Log::default();
    let tap = TapGestureRecognizer::new(&arena, &router, TapConfig::default(), tap_callbacks(&log))
        .unwrap();
    let clone = tap.clone();
    let weak = tap.downgrade();
    let down = event(1, PointerPhase::Down, 0.0, 0);
    tap.add_pointer(down).unwrap();
    drop(tap);
    assert_eq!(router.route_count(), 1);
    drop(clone);
    assert!(weak.upgrade().is_none());
    assert_eq!(router.route_count(), 0);
    assert_eq!(
        *log.borrow(),
        vec![Record::Down("tap"), Record::Cancel("tap")]
    );
    let tap = TapGestureRecognizer::new(
        &arena,
        &router,
        TapConfig::default(),
        tap_callbacks(&Log::default()),
    )
    .unwrap();
    tap.dispose();
    tap.dispose();
    assert!(tap.is_disposed());
    assert_eq!(tap.add_pointer(down), Err(RecognizerError::Disposed));
}

#[test]
fn invalid_configuration_samples_and_service_failures_do_not_leak_registrations() {
    let arena = GestureArenaManager::new();
    let router = PointerRouter::new();
    for slop in [f32::NAN, f32::INFINITY, -1.0] {
        assert!(matches!(
            PanGestureRecognizer::new(
                &arena,
                &router,
                PanConfig {
                    slop,
                    ..Default::default()
                },
                PanCallbacks::default()
            ),
            Err(RecognizerError::InvalidSlop)
        ));
    }
    let tap = TapGestureRecognizer::new(
        &arena,
        &router,
        TapConfig::default(),
        tap_callbacks(&Log::default()),
    )
    .unwrap();
    assert_eq!(
        tap.add_pointer(event(1, PointerPhase::Move, 0.0, 0)),
        Err(RecognizerError::ExpectedDown)
    );
    assert_eq!(
        tap.add_pointer(event(1, PointerPhase::Down, f32::NAN, 0)),
        Err(RecognizerError::InvalidPosition)
    );
    router.dispose();
    assert_eq!(
        tap.add_pointer(event(1, PointerPhase::Down, 0.0, 0)),
        Err(RecognizerError::Router(PointerRouterError::Disposed))
    );
    assert_eq!(arena.active_pointers(), 0);
    let router = PointerRouter::new();
    let tap = TapGestureRecognizer::new(
        &arena,
        &router,
        TapConfig::default(),
        tap_callbacks(&Log::default()),
    )
    .unwrap();
    arena.add(PointerId(1), Rc::new(Dummy)).unwrap();
    arena.close(PointerId(1)).unwrap();
    assert_eq!(
        tap.add_pointer(event(1, PointerPhase::Down, 0.0, 0)),
        Err(RecognizerError::Arena(ArenaError::Closed))
    );
    assert_eq!(router.route_count(), 0);
    drop(arena);
    assert_eq!(
        tap.add_pointer(event(2, PointerPhase::Down, 0.0, 0)),
        Err(RecognizerError::ServicesUnavailable)
    );
}
struct Dummy;
#[test]
fn recognizers_without_callbacks_do_not_compete_or_track() {
    let arena = GestureArenaManager::new();
    let router = PointerRouter::new();
    let tap = TapGestureRecognizer::new(
        &arena,
        &router,
        TapConfig::default(),
        TapCallbacks::default(),
    )
    .unwrap();
    let pan = PanGestureRecognizer::new(
        &arena,
        &router,
        PanConfig::default(),
        PanCallbacks::default(),
    )
    .unwrap();
    let down = event(1, PointerPhase::Down, 0.0, 0);
    assert!(!tap.add_pointer(down).unwrap());
    assert!(!pan.add_pointer(down).unwrap());
    assert_eq!(router.route_count(), 0);
    assert_eq!(arena.active_pointers(), 0);
}
impl GestureArenaMember for Dummy {
    fn accept_gesture(&self, _: PointerId) {}
    fn reject_gesture(&self, _: PointerId) {}
}

#[test]
fn losing_to_an_external_contender_cancels_without_recognized_callbacks() {
    let h = Harness::new(DragStartBehavior::Start);
    let down = event(1, PointerPhase::Down, 0.0, 0);
    h.tap.add_pointer(down).unwrap();
    h.pan.add_pointer(down).unwrap();
    let winner = h.arena.add(down.pointer, Rc::new(Dummy)).unwrap();
    h.arena.close(down.pointer).unwrap();
    winner.resolve(GestureDisposition::Accepted);
    assert_eq!(
        *h.log.borrow(),
        vec![
            Record::Down("tap"),
            Record::Down("pan"),
            Record::Cancel("tap"),
            Record::Cancel("pan")
        ]
    );
    assert_eq!(h.router.route_count(), 0);
    assert_eq!(h.arena.active_pointers(), 0);
}

#[test]
fn pending_pan_claim_with_up_completes_only_after_arena_closes() {
    let h = Harness::new(DragStartBehavior::Down);
    let down = event(1, PointerPhase::Down, 0.0, 0);
    h.pan.add_pointer(down).unwrap();
    h.arena.add(down.pointer, Rc::new(Dummy)).unwrap();
    h.router
        .route(down.pointer, &event(1, PointerPhase::Up, 30.0, 10));
    assert_eq!(*h.log.borrow(), vec![Record::Down("pan")]);
    assert_eq!(h.router.route_count(), 0);
    h.arena.close(down.pointer).unwrap();
    assert_eq!(
        *h.log.borrow(),
        vec![
            Record::Down("pan"),
            Record::Start(Pos2::ZERO),
            Record::Update(Vec2::new(30.0, 0.0)),
            Record::End(Pos2::new(30.0, 0.0))
        ]
    );
    assert_eq!(h.pan.tracked_pointer(), None);
}

#[test]
fn changed_buttons_device_or_invalid_move_cancels_and_regressing_time_is_ignored() {
    for change in 0..4 {
        let h = Harness::new(DragStartBehavior::Start);
        h.down(1);
        h.send(PointerPhase::Move, 1.0, 10);
        h.send(PointerPhase::Move, 80.0, 5); // old sample must not claim pan
        assert_eq!(h.log.borrow().len(), 2);
        let mut e = event(1, PointerPhase::Move, 2.0, 20);
        match change {
            0 => e.buttons = PointerButtons::NONE,
            1 => e.device = PointerDeviceKind::Mouse,
            2 => e.position.x = f32::NAN,
            _ => {
                e.phase = PointerPhase::Cancel;
                e.time = Duration::ZERO;
                e.position.x = f32::NAN;
            }
        }
        h.router.route(e.pointer, &e);
        h.arena.flush();
        assert_eq!(h.router.route_count(), 0);
        assert_eq!(h.tap.tracked_pointer(), None);
        assert_eq!(h.pan.tracked_pointer(), None);
        assert_eq!(
            h.log
                .borrow()
                .iter()
                .filter(|r| matches!(r, Record::Cancel(_)))
                .count(),
            2
        );
    }
}

#[test]
fn mismatched_payload_pointer_is_not_delivered_to_a_registration() {
    let h = Harness::new(DragStartBehavior::Start);
    h.down(1);
    h.router
        .route(PointerId(1), &event(2, PointerPhase::Up, 0.0, 10));
    assert_eq!(h.tap.tracked_pointer(), Some(PointerId(1)));
    assert_eq!(h.log.borrow().len(), 2);
    h.send(PointerPhase::Up, 0.0, 20);
    assert!(matches!(h.log.borrow().last(), Some(Record::Tap(_))));
}

#[test]
fn on_start_can_dispose_without_update_or_end_and_pan_can_be_canceled_after_winning() {
    let h = Harness::new(DragStartBehavior::Down);
    let target = Rc::new(RefCell::new(None::<WeakPanGestureRecognizer>));
    let t = target.clone();
    let mut callbacks = pan_callbacks(&h.log);
    let old = callbacks.on_start.take().unwrap();
    callbacks.on_start = Some(Rc::new(move |e| {
        old(e);
        t.borrow().as_ref().unwrap().upgrade().unwrap().dispose();
    }));
    let pan = PanGestureRecognizer::new(
        &h.arena,
        &h.router,
        PanConfig {
            start_behavior: DragStartBehavior::Down,
            ..Default::default()
        },
        callbacks,
    )
    .unwrap();
    *target.borrow_mut() = Some(pan.downgrade());
    pan.add_pointer(event(1, PointerPhase::Down, 0.0, 0))
        .unwrap();
    h.arena.close(PointerId(1)).unwrap();
    h.arena.flush();
    h.router
        .route(PointerId(1), &event(1, PointerPhase::Move, 30.0, 10));
    assert_eq!(
        *h.log.borrow(),
        vec![
            Record::Down("pan"),
            Record::Start(Pos2::ZERO),
            Record::Cancel("pan")
        ]
    );
    assert_eq!(h.router.route_count(), 0);
}

#[test]
fn reentrant_input_during_start_is_queued_after_accumulated_update() {
    let h = Harness::new(DragStartBehavior::Down);
    let weak = h.router.downgrade();
    let mut callbacks = pan_callbacks(&h.log);
    let old = callbacks.on_start.take().unwrap();
    callbacks.on_start = Some(Rc::new(move |e| {
        old(e);
        let router = weak.upgrade().unwrap();
        router.route(e.pointer, &event(1, PointerPhase::Up, 40.0, 20));
    }));
    let pan = PanGestureRecognizer::new(
        &h.arena,
        &h.router,
        PanConfig {
            start_behavior: DragStartBehavior::Down,
            ..Default::default()
        },
        callbacks,
    )
    .unwrap();
    pan.add_pointer(event(1, PointerPhase::Down, 0.0, 0))
        .unwrap();
    h.arena.close(PointerId(1)).unwrap();
    h.arena.flush();
    h.router
        .route(PointerId(1), &event(1, PointerPhase::Move, 30.0, 10));
    assert_eq!(
        *h.log.borrow(),
        vec![
            Record::Down("pan"),
            Record::Start(Pos2::ZERO),
            Record::Update(Vec2::new(30.0, 0.0)),
            Record::Update(Vec2::new(10.0, 0.0)),
            Record::End(Pos2::new(40.0, 0.0))
        ]
    );
    assert_eq!(h.router.route_count(), 0);
}

#[test]
fn reentrant_cancel_and_restart_do_not_complete_or_remove_the_new_sequence() {
    let arena = GestureArenaManager::new();
    let router = PointerRouter::new();
    let log = Log::default();
    let target = Rc::new(RefCell::new(None::<WeakTapGestureRecognizer>));
    let t = target.clone();
    let r = router.downgrade();
    let mut callbacks = tap_callbacks(&log);
    let old = callbacks.on_tap.take().unwrap();
    callbacks.on_tap = Some(Rc::new(move |details| {
        old(details);
        let tap = t.borrow().as_ref().unwrap().upgrade().unwrap();
        tap.add_pointer(event(1, PointerPhase::Down, 10.0, 30))
            .unwrap();
        r.upgrade()
            .unwrap()
            .route(PointerId(1), &event(1, PointerPhase::Move, 11.0, 40));
    }));
    let tap = TapGestureRecognizer::new(&arena, &router, TapConfig::default(), callbacks).unwrap();
    *target.borrow_mut() = Some(tap.downgrade());
    tap.add_pointer(event(1, PointerPhase::Down, 0.0, 0))
        .unwrap();
    arena.close(PointerId(1)).unwrap();
    arena.flush();
    router.route(PointerId(1), &event(1, PointerPhase::Up, 1.0, 10));
    assert_eq!(tap.tracked_pointer(), Some(PointerId(1)));
    assert_eq!(router.route_count(), 1);
    assert_eq!(
        *log.borrow(),
        vec![
            Record::Down("tap"),
            Record::Tap(Pos2::new(1.0, 0.0)),
            Record::Down("tap")
        ]
    );
    tap.cancel();
    assert_eq!(router.route_count(), 0);
}

#[test]
fn pan_callback_panic_does_not_prevent_end_or_cleanup() {
    let h = Harness::new(DragStartBehavior::Down);
    let mut callbacks = pan_callbacks(&h.log);
    callbacks.on_update = Some(Rc::new(|_| panic!("expected callback panic")));
    let pan = PanGestureRecognizer::new(
        &h.arena,
        &h.router,
        PanConfig {
            start_behavior: DragStartBehavior::Down,
            ..Default::default()
        },
        callbacks,
    )
    .unwrap();
    pan.add_pointer(event(1, PointerPhase::Down, 0.0, 0))
        .unwrap();
    h.arena.close(PointerId(1)).unwrap();
    h.arena.flush();
    h.router
        .route(PointerId(1), &event(1, PointerPhase::Up, 30.0, 20));
    assert_eq!(
        *h.log.borrow(),
        vec![
            Record::Down("pan"),
            Record::Start(Pos2::ZERO),
            Record::End(Pos2::new(30.0, 0.0))
        ]
    );
    assert_eq!(h.router.route_count(), 0);
}

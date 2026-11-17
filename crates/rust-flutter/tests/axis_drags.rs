use rust_flutter::{
    egui::{self, Id, pos2, vec2},
    element::{ElementTree, FlexItem, FlexStyle, Widget},
    gestures::*,
    layout::*,
    scrolling::*,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
fn e(phase: PointerPhase, x: f32, y: f32, ms: u64) -> PointerEvent {
    PointerEvent {
        pointer: PointerId(1),
        phase,
        position: pos2(x, y),
        time: Duration::from_millis(ms),
        device: PointerDeviceKind::Touch,
        buttons: if matches!(phase, PointerPhase::Up | PointerPhase::Cancel) {
            PointerButtons::NONE
        } else {
            PointerButtons::PRIMARY
        },
    }
}
#[derive(Default)]
struct Log {
    starts: Vec<PointerEvent>,
    updates: Vec<DragUpdateDetails>,
    ends: Vec<DragEndDetails>,
    cancels: usize,
}
fn callbacks(log: &Rc<RefCell<Log>>) -> PanCallbacks {
    let a = log.clone();
    let b = log.clone();
    let c = log.clone();
    let d = log.clone();
    PanCallbacks {
        on_start: Some(Rc::new(move |v| a.borrow_mut().starts.push(v))),
        on_update: Some(Rc::new(move |v| b.borrow_mut().updates.push(v))),
        on_end: Some(Rc::new(move |v| c.borrow_mut().ends.push(v))),
        on_cancel: Some(Rc::new(move |_| d.borrow_mut().cancels += 1)),
        ..Default::default()
    }
}
struct Harness {
    arena: GestureArenaManager,
    router: PointerRouter<PointerEvent>,
    pan: PanGestureRecognizer,
    log: Rc<RefCell<Log>>,
}
impl Harness {
    fn new(axis: Option<DragAxis>, behavior: DragStartBehavior, fling: FlingConfig) -> Self {
        let arena = GestureArenaManager::new();
        let router = PointerRouter::new();
        let log = Rc::new(RefCell::new(Log::default()));
        let pan = PanGestureRecognizer::new(
            &arena,
            &router,
            PanConfig {
                axis,
                start_behavior: behavior,
                fling,
                ..Default::default()
            },
            callbacks(&log),
        )
        .unwrap();
        Self {
            arena,
            router,
            pan,
            log,
        }
    }
    fn down(&self) {
        self.pan
            .add_pointer(e(PointerPhase::Down, 0.0, 0.0, 0))
            .unwrap();
        self.arena.close(PointerId(1)).unwrap();
        self.arena.flush();
    }
    fn send(&self, phase: PointerPhase, x: f32, y: f32, ms: u64) {
        self.router.route(PointerId(1), &e(phase, x, y, ms));
        if phase == PointerPhase::Up {
            self.arena.sweep(PointerId(1)).unwrap();
        }
        self.arena.flush();
    }
}
#[test]
fn cross_axis_motion_does_not_start_a_drag_even_after_sole_arena_acceptance() {
    let h = Harness::new(
        Some(DragAxis::Vertical),
        DragStartBehavior::Down,
        FlingConfig::default(),
    );
    h.down();
    h.send(PointerPhase::Move, 100.0, 0.0, 10);
    h.send(PointerPhase::Up, 200.0, 0.0, 20);
    let l = h.log.borrow();
    assert!(l.starts.is_empty() && l.updates.is_empty() && l.ends.is_empty());
    assert_eq!(l.cancels, 1);
    assert_eq!(h.router.route_count(), 0);
}
#[test]
fn axis_slop_is_strict_and_reversals_use_net_selected_displacement() {
    let h = Harness::new(
        Some(DragAxis::Vertical),
        DragStartBehavior::Down,
        FlingConfig::default(),
    );
    h.down();
    for (x, y, t) in [(100.0, 18.0, 10), (200.0, 10.0, 20), (300.0, -18.0, 30)] {
        h.send(PointerPhase::Move, x, y, t);
        assert!(h.log.borrow().starts.is_empty());
    }
    h.send(PointerPhase::Move, 400.0, -19.0, 40);
    let l = h.log.borrow();
    assert_eq!(l.starts.len(), 1);
    assert_eq!(l.updates[0].delta, vec2(0.0, -19.0));
    assert_eq!(l.updates[0].primary_delta, Some(-19.0));
}
#[test]
fn axis_updates_project_cross_motion_and_preserve_raw_event_positions() {
    let h = Harness::new(
        Some(DragAxis::Horizontal),
        DragStartBehavior::Down,
        FlingConfig::default(),
    );
    h.down();
    h.send(PointerPhase::Move, 20.0, 50.0, 10);
    h.send(PointerPhase::Move, 25.0, 100.0, 20);
    h.send(PointerPhase::Move, 25.0, 150.0, 30);
    let l = h.log.borrow();
    assert_eq!(l.updates.len(), 2);
    assert_eq!(l.updates[0].delta, vec2(20.0, 0.0));
    assert_eq!(l.updates[1].delta, vec2(5.0, 0.0));
    assert_eq!(l.updates[1].primary_delta, Some(5.0));
    assert_eq!(l.updates[1].event.position, pos2(25.0, 100.0));
}
#[test]
fn start_behavior_omits_threshold_motion_for_axis_drag() {
    let h = Harness::new(
        Some(DragAxis::Vertical),
        DragStartBehavior::Start,
        FlingConfig::default(),
    );
    h.down();
    h.send(PointerPhase::Move, 100.0, 20.0, 10);
    assert!(h.log.borrow().updates.is_empty());
    assert_eq!(h.log.borrow().starts[0].position, pos2(100.0, 20.0));
    h.send(PointerPhase::Move, 200.0, 25.0, 20);
    assert_eq!(h.log.borrow().updates[0].primary_delta, Some(5.0));
}
#[test]
fn unrestricted_pan_keeps_two_dimensional_delta_and_absent_primary_fields() {
    let h = Harness::new(None, DragStartBehavior::Down, FlingConfig::default());
    h.down();
    h.send(PointerPhase::Move, 20.0, 30.0, 10);
    h.send(PointerPhase::Up, 20.0, 30.0, 100);
    let l = h.log.borrow();
    assert_eq!(l.updates[0].delta, vec2(20.0, 30.0));
    assert_eq!(l.updates[0].primary_delta, None);
    assert_eq!(l.ends[0].primary_velocity, None);
}
#[test]
fn effective_fling_uses_axis_clamp_and_keeps_unprojected_raw_estimate() {
    let h = Harness::new(
        Some(DragAxis::Vertical),
        DragStartBehavior::Down,
        FlingConfig {
            max_velocity: 500.0,
            ..Default::default()
        },
    );
    h.down();
    for i in 1..=3 {
        h.send(
            PointerPhase::Move,
            i as f32 * 1000.0,
            i as f32 * 10.0,
            i * 10,
        );
    }
    h.send(PointerPhase::Up, 3000.0, 30.0, 40);
    let l = h.log.borrow();
    let d = l.ends[0];
    assert!(d.is_fling);
    assert_eq!(d.velocity.pixels_per_second(), vec2(0.0, 500.0));
    assert_eq!(d.primary_velocity, Some(500.0));
    let raw = d.estimate.unwrap();
    assert!((raw.velocity.pixels_per_second().x - 100000.0).abs() < 1.0);
    assert!((raw.velocity.pixels_per_second().y - 1000.0).abs() < 0.01);
    assert_eq!(raw.offset, vec2(3000.0, 30.0));
}
#[test]
fn cross_axis_speed_cannot_make_a_slow_primary_axis_fling() {
    let h = Harness::new(
        Some(DragAxis::Vertical),
        DragStartBehavior::Down,
        FlingConfig {
            min_distance: 0.0,
            ..Default::default()
        },
    );
    h.down();
    h.send(PointerPhase::Move, 0.0, 25.0, 10);
    for i in 0..=3 {
        h.send(
            PointerPhase::Move,
            i as f32 * 1000.0,
            25.0 + i as f32 * 0.2,
            200 + i * 10,
        );
    }
    h.send(PointerPhase::Up, 3000.0, 25.6, 240);
    let d = h.log.borrow().ends[0];
    assert!(!d.is_fling);
    assert_eq!(d.primary_velocity, Some(0.0));
    assert!(d.estimate.unwrap().velocity.pixels_per_second().x > 8000.0);
}
#[test]
fn fling_distance_is_primary_segment_distance_not_cross_distance() {
    let h = Harness::new(
        Some(DragAxis::Vertical),
        DragStartBehavior::Down,
        FlingConfig::default(),
    );
    h.down();
    h.send(PointerPhase::Move, 0.0, 25.0, 10);
    for i in 0..=2 {
        h.send(
            PointerPhase::Move,
            i as f32 * 1000.0,
            25.0 + i as f32 * 8.5,
            200 + i * 10,
        );
    }
    h.send(PointerPhase::Up, 2000.0, 42.0, 230);
    let d = h.log.borrow().ends[0];
    assert!(!d.is_fling);
    assert_eq!(d.primary_velocity, Some(0.0));
    assert!(d.estimate.unwrap().velocity.pixels_per_second().y > 800.0);
}
#[test]
fn negative_axis_velocity_keeps_sign_and_zeroes_cross_velocity() {
    let h = Harness::new(
        Some(DragAxis::Horizontal),
        DragStartBehavior::Down,
        FlingConfig::default(),
    );
    h.down();
    for i in 1..=3 {
        h.send(
            PointerPhase::Move,
            -(i as f32) * 10.0,
            i as f32 * 1000.0,
            i * 10,
        );
    }
    h.send(PointerPhase::Up, -30.0, 3000.0, 40);
    let d = h.log.borrow().ends[0];
    assert!(d.is_fling);
    assert!((d.primary_velocity.unwrap() + 1000.0).abs() < 0.01);
    assert_eq!(d.velocity.pixels_per_second().y, 0.0);
}
#[test]
fn orthogonal_recognizers_compete_by_the_component_that_crosses_slop() {
    let arena = GestureArenaManager::new();
    let router = PointerRouter::new();
    let v = Rc::new(RefCell::new(Log::default()));
    let h = Rc::new(RefCell::new(Log::default()));
    let vertical =
        VerticalDragGestureRecognizer::new(&arena, &router, PanConfig::default(), callbacks(&v))
            .unwrap();
    let horizontal =
        HorizontalDragGestureRecognizer::new(&arena, &router, PanConfig::default(), callbacks(&h))
            .unwrap();
    vertical
        .add_pointer(e(PointerPhase::Down, 0.0, 0.0, 0))
        .unwrap();
    horizontal
        .add_pointer(e(PointerPhase::Down, 0.0, 0.0, 0))
        .unwrap();
    arena.close(PointerId(1)).unwrap();
    router.route(PointerId(1), &e(PointerPhase::Move, 30.0, 5.0, 10));
    assert_eq!(h.borrow().starts.len(), 1);
    assert!(v.borrow().starts.is_empty());
    assert_eq!(v.borrow().cancels, 1);
}
#[test]
fn simultaneous_diagonal_thresholds_use_registration_order_not_angle_dominance() {
    let arena = GestureArenaManager::new();
    let router = PointerRouter::new();
    let v = Rc::new(RefCell::new(Log::default()));
    let h = Rc::new(RefCell::new(Log::default()));
    let vertical =
        VerticalDragGestureRecognizer::new(&arena, &router, PanConfig::default(), callbacks(&v))
            .unwrap();
    let horizontal =
        HorizontalDragGestureRecognizer::new(&arena, &router, PanConfig::default(), callbacks(&h))
            .unwrap();
    vertical
        .add_pointer(e(PointerPhase::Down, 0.0, 0.0, 0))
        .unwrap();
    horizontal
        .add_pointer(e(PointerPhase::Down, 0.0, 0.0, 0))
        .unwrap();
    arena.close(PointerId(1)).unwrap();
    router.route(PointerId(1), &e(PointerPhase::Move, 100.0, 20.0, 10));
    assert_eq!(v.borrow().starts.len(), 1);
    assert!(h.borrow().starts.is_empty());
}
#[test]
fn named_wrapper_forces_axis_and_weak_handle_disposal_is_reentrant() {
    let arena = GestureArenaManager::new();
    let router = PointerRouter::new();
    let weak = Rc::new(RefCell::new(None::<WeakVerticalDragGestureRecognizer>));
    let slot = weak.clone();
    let updates = Rc::new(Cell::new(0));
    let count = updates.clone();
    let v = VerticalDragGestureRecognizer::new(
        &arena,
        &router,
        PanConfig {
            axis: Some(DragAxis::Horizontal),
            start_behavior: DragStartBehavior::Down,
            ..Default::default()
        },
        PanCallbacks {
            on_start: Some(Rc::new(move |_| {
                slot.borrow().as_ref().unwrap().upgrade().unwrap().dispose()
            })),
            on_update: Some(Rc::new(move |_| count.set(count.get() + 1))),
            ..Default::default()
        },
    )
    .unwrap();
    *weak.borrow_mut() = Some(v.downgrade());
    v.add_pointer(e(PointerPhase::Down, 0.0, 0.0, 0)).unwrap();
    arena.close(PointerId(1)).unwrap();
    arena.flush();
    router.route(PointerId(1), &e(PointerPhase::Move, 100.0, 0.0, 10));
    assert!(!v.is_disposed());
    router.route(PointerId(1), &e(PointerPhase::Move, 100.0, 20.0, 20));
    assert!(v.is_disposed());
    assert_eq!(updates.get(), 0);
    assert_eq!(router.route_count(), 0);
}
#[test]
fn binding_axis_change_cancels_active_drag_without_reusing_the_old_sequence() {
    let b = GestureBinding::new();
    let id = Id::new("drag");
    let log = Rc::new(RefCell::new(Log::default()));
    let config = |axis| GestureTargetConfig {
        pan: Some((
            PanConfig {
                axis: Some(axis),
                ..Default::default()
            },
            callbacks(&log),
        )),
        ..Default::default()
    };
    let target = b.register_target(id, config(DragAxis::Vertical)).unwrap();
    b.dispatch(e(PointerPhase::Down, 0.0, 0.0, 0), [id]);
    b.dispatch(e(PointerPhase::Move, 0.0, 30.0, 10), []);
    assert_eq!(log.borrow().starts.len(), 1);
    target.update(config(DragAxis::Horizontal)).unwrap();
    assert_eq!(log.borrow().cancels, 1);
    b.dispatch(e(PointerPhase::Up, 0.0, 30.0, 20), []);
    b.dispatch(e(PointerPhase::Down, 0.0, 0.0, 30), [id]);
    b.dispatch(e(PointerPhase::Move, 30.0, 0.0, 40), []);
    assert_eq!(log.borrow().starts.len(), 2);
}
#[test]
fn nested_orthogonal_scroll_views_route_drag_to_matching_parent_axis() {
    let ctx = egui::Context::default();
    ctx.begin_pass(egui::RawInput::default());
    let b = GestureBinding::new();
    let outer = ScrollPosition::new(&ctx, 0.0).unwrap();
    let inner = ScrollPosition::new(&ctx, 0.0).unwrap();
    let child = SingleChildScrollView::new(&inner, &b)
        .unwrap()
        .into_widget(Widget::leaf(vec2(80.0, 500.0), |_| {}))
        .unwrap();
    let mut row = FlexStyle::new(Axis::Horizontal);
    row.main_size = MainAxisSize::Min;
    let content = Widget::flex(
        row,
        vec![
            FlexItem::fixed(Widget::constrained(
                BoxConstraints::tight(vec2(80.0, 100.0)).unwrap(),
                child,
            )),
            FlexItem::fixed(Widget::leaf(vec2(500.0, 100.0), |_| {})),
        ],
    );
    let mut tree = ElementTree::new(
        &ctx,
        SingleChildScrollView::new(&outer, &b)
            .unwrap()
            .axis(Axis::Horizontal)
            .into_widget(content)
            .unwrap(),
    )
    .unwrap();
    let g = tree
        .layout(BoxConstraints::tight(vec2(80.0, 100.0)).unwrap())
        .unwrap();
    let _ = ctx.end_pass();
    for event in [
        e(PointerPhase::Down, 60.0, 50.0, 0),
        e(PointerPhase::Move, 20.0, 55.0, 10),
        e(PointerPhase::Up, 20.0, 55.0, 100),
    ] {
        assert!(
            b.dispatch_layout(event, &g, pos2(0.0, 0.0))
                .failures
                .is_empty()
        );
    }
    assert_eq!((outer.pixels(), inner.pixels()), (40.0, 0.0));
    assert_eq!(inner.activity(), ScrollActivity::Idle);
}

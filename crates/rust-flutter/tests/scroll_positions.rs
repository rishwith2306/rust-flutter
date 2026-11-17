use rust_flutter::{
    animation::{AnimationOutcome, Curve},
    egui,
    layout::Axis,
    notifier::*,
    scrolling::*,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
fn frame(ctx: &egui::Context, t: f64) {
    ctx.begin_pass(egui::RawInput {
        time: Some(t),
        ..Default::default()
    });
    let _ = ctx.end_pass();
}
fn setup() -> (egui::Context, ScrollPosition) {
    let ctx = egui::Context::default();
    frame(&ctx, 0.0);
    let p = ScrollPosition::new(&ctx, 0.0).unwrap();
    (ctx, p)
}
fn dims(max: f32) -> ScrollDimensions {
    ScrollDimensions::new(0.0, max, 100.0, Axis::Vertical).unwrap()
}
fn ready() -> (egui::Context, ScrollPosition) {
    let (ctx, p) = setup();
    p.apply_dimensions(dims(300.0)).unwrap();
    (ctx, p)
}
fn counter() -> (Rc<Cell<usize>>, Listener) {
    let c = Rc::new(Cell::new(0));
    let cc = c.clone();
    (c, Rc::new(move || cc.set(cc.get() + 1)))
}
fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 0.001, "{a} != {b}");
}
#[test]
fn initial_offset_waits_for_dimensions_then_clamps_and_metrics_are_wide() {
    let (ctx, _) = setup();
    let p = ScrollPosition::new(&ctx, 400.0).unwrap();
    assert!(p.metrics().is_none());
    assert_eq!(p.pixels(), 400.0);
    p.apply_dimensions(dims(300.0)).unwrap();
    let m = p.metrics().unwrap();
    assert_eq!(m.pixels, 300.0);
    assert_eq!(m.extent_before(), 300.0);
    assert_eq!(m.extent_after(), 0.0);
    assert_eq!(m.extent_inside(), 100.0);
    assert!(m.at_edge());
    p.apply_dimensions(ScrollDimensions::new(f32::MIN, f32::MAX, 0.0, Axis::Horizontal).unwrap())
        .unwrap();
    p.jump_to(f32::MAX).unwrap();
    let m = p.metrics().unwrap();
    assert!(m.extent_before().is_finite());
    assert!(m.extent_before() > f64::from(f32::MAX));
    assert_eq!(m.dimensions.axis(), Axis::Horizontal);
}
#[test]
fn dimensions_and_offset_validation_are_atomic() {
    let (ctx, p) = ready();
    let run = p
        .animate_to(200.0, Duration::from_secs(1), Curve::Linear)
        .unwrap();
    for (min, max, viewport) in [
        (1.0, 0.0, 1.0),
        (0.0, f32::INFINITY, 1.0),
        (0.0, 1.0, -1.0),
        (f32::NAN, 1.0, 1.0),
    ] {
        assert!(ScrollDimensions::new(min, max, viewport, Axis::Vertical).is_err());
    }
    assert_eq!(p.jump_to(f32::NAN), Err(ScrollError::InvalidOffset));
    assert!(matches!(
        p.animate_to(10.0, Duration::ZERO, Curve::Linear),
        Err(ScrollError::InvalidDuration)
    ));
    assert!(matches!(
        p.animate_to(f32::INFINITY, Duration::from_secs(1), Curve::Linear),
        Err(ScrollError::InvalidOffset)
    ));
    frame(&ctx, 1.0);
    assert_eq!(run.outcome(), Some(AnimationOutcome::Completed));
    assert_eq!(p.pixels(), 200.0);
}
#[test]
fn metrics_and_pixel_notifications_are_distinct_and_equal_values_are_quiet() {
    let (_, p) = setup();
    let (pc, pl) = counter();
    let (mc, ml) = counter();
    let _p = p.subscribe(pl).unwrap();
    let _m = p.subscribe_metrics(ml).unwrap();
    p.apply_dimensions(dims(300.0)).unwrap();
    assert_eq!((pc.get(), mc.get()), (0, 1));
    assert!(!p.apply_dimensions(dims(300.0)).unwrap());
    p.jump_to(0.0).unwrap();
    assert_eq!((pc.get(), mc.get()), (0, 1));
    p.jump_to(200.0).unwrap();
    assert_eq!((pc.get(), mc.get()), (1, 1));
    p.apply_dimensions(dims(50.0)).unwrap();
    assert_eq!((pc.get(), mc.get()), (2, 2));
    assert_eq!(p.pixels(), 50.0);
}
#[test]
fn dimensions_changes_cancel_motion_but_identical_dimensions_do_not() {
    let (ctx, p) = ready();
    let run = p
        .animate_to(300.0, Duration::from_secs(1), Curve::Linear)
        .unwrap();
    frame(&ctx, 0.5);
    assert_eq!(p.pixels(), 150.0);
    assert_eq!(p.activity(), ScrollActivity::Driven);
    assert!(!p.apply_dimensions(dims(300.0)).unwrap());
    assert_eq!(run.outcome(), None);
    p.apply_dimensions(dims(80.0)).unwrap();
    assert_eq!(run.outcome(), Some(AnimationOutcome::Canceled));
    assert_eq!(p.pixels(), 80.0);
    assert_eq!(p.activity(), ScrollActivity::Idle);
    frame(&ctx, 2.0);
    assert_eq!(p.pixels(), 80.0);
}
#[test]
fn drag_clamps_both_edges_reports_unapplied_distance_and_tracks_activity() {
    let (_, p) = ready();
    assert_eq!(p.drag_by(10.0), Err(ScrollError::NotDragging));
    p.begin_drag().unwrap();
    assert!(p.is_scrolling());
    assert_eq!(p.drag_by(-40.0).unwrap(), -40.0);
    assert_eq!(p.drag_by(400.0).unwrap(), 100.0);
    assert_eq!(p.pixels(), 300.0);
    assert_eq!(p.activity(), ScrollActivity::Drag);
    assert_eq!(p.drag_by(-50.0).unwrap(), 0.0);
    assert_eq!(p.pixels(), 250.0);
    assert_eq!(p.drag_by(f32::NAN), Err(ScrollError::InvalidOffset));
    p.cancel_activity().unwrap();
    assert!(!p.is_scrolling());
    assert_eq!(p.pixels(), 250.0);
}
#[test]
fn release_velocity_advances_friction_until_contact_then_idle() {
    let (ctx, p) = ready();
    p.begin_drag().unwrap();
    p.drag_by(50.0).unwrap();
    let run = p.end_drag(1000.0, 0.135).unwrap();
    assert_eq!(p.activity(), ScrollActivity::Ballistic);
    frame(&ctx, 0.1);
    assert!(p.pixels() > 50.0 && p.pixels() < 300.0);
    frame(&ctx, 2.0);
    assert_eq!(p.pixels(), 300.0);
    assert_eq!(run.outcome(), Some(AnimationOutcome::Completed));
    assert_eq!(p.activity(), ScrollActivity::Idle);
}
#[test]
fn negative_velocity_and_zero_release_have_explicit_completion() {
    let (ctx, p) = ready();
    p.jump_to(200.0).unwrap();
    p.begin_drag().unwrap();
    let run = p.end_drag(-1000.0, 0.135).unwrap();
    frame(&ctx, 2.0);
    assert_eq!(p.pixels(), 0.0);
    assert_eq!(run.outcome(), Some(AnimationOutcome::Completed));
    p.begin_drag().unwrap();
    let zero = p.end_drag(0.0, 0.135).unwrap();
    assert_eq!(zero.outcome(), Some(AnimationOutcome::Completed));
    assert!(!p.is_scrolling());
    p.begin_drag().unwrap();
    assert!(p.end_drag(10.0, 1.0).is_err());
    assert_eq!(p.activity(), ScrollActivity::Drag);
    assert_eq!(p.pixels(), 0.0);
}
#[test]
fn animate_to_is_frame_driven_and_drag_or_jump_interrupts() {
    let (ctx, p) = ready();
    let run = p
        .animate_to(200.0, Duration::from_secs(1), Curve::Linear)
        .unwrap();
    frame(&ctx, 0.5);
    close(p.pixels(), 100.0);
    p.begin_drag().unwrap();
    assert_eq!(run.outcome(), Some(AnimationOutcome::Canceled));
    p.drag_by(20.0).unwrap();
    let release = p.end_drag(100.0, 0.135).unwrap();
    p.jump_to(50.0).unwrap();
    assert_eq!(release.outcome(), Some(AnimationOutcome::Canceled));
    assert_eq!(p.activity(), ScrollActivity::Idle);
    frame(&ctx, 2.0);
    assert_eq!(p.pixels(), 50.0);
}
#[test]
fn position_commands_before_dimensions_are_explicit() {
    let (_, p) = setup();
    assert_eq!(p.begin_drag(), Err(ScrollError::NoDimensions));
    assert!(matches!(
        p.animate_to(10.0, Duration::from_secs(1), Curve::Linear),
        Err(ScrollError::NoDimensions)
    ));
    p.jump_to(-10.0).unwrap();
    assert_eq!(p.pixels(), -10.0);
    p.apply_dimensions(dims(0.0)).unwrap();
    assert_eq!(p.pixels(), 0.0);
}
#[test]
fn controller_requires_single_client_for_offset_and_tokens_own_attachments() {
    let (ctx, p) = ready();
    let c = ScrollController::new(20.0).unwrap();
    assert_eq!(c.offset(), Err(ScrollError::NoClients));
    let mut a = c.attach(&p).unwrap();
    assert!(c.has_clients() && a.is_attached());
    assert!(matches!(c.attach(&p), Err(ScrollError::AlreadyAttached)));
    let p2 = c.create_position(&ctx).unwrap();
    assert_eq!(p2.pixels(), 20.0);
    let b = c.attach(&p2).unwrap();
    assert_eq!(c.offset(), Err(ScrollError::MultipleClients));
    drop(b);
    assert_eq!(c.offset(), Ok(0.0));
    a.detach();
    a.detach();
    assert!(!a.is_attached());
    assert!(!c.has_clients());
    assert_eq!(p.pixels(), 0.0);
}
#[test]
fn controller_forwards_pixels_but_not_metrics_or_attachment_changes() {
    let (_, p) = setup();
    let c = ScrollController::default();
    let (calls, listener) = counter();
    let _s = c.subscribe(listener).unwrap();
    let a = c.attach(&p).unwrap();
    p.apply_dimensions(dims(300.0)).unwrap();
    assert_eq!(calls.get(), 0);
    p.jump_to(50.0).unwrap();
    assert_eq!(calls.get(), 1);
    drop(a);
    p.jump_to(100.0).unwrap();
    assert_eq!(calls.get(), 1);
}
#[test]
fn controller_broadcasts_clamp_per_client_and_animation_preflight_is_atomic() {
    let (ctx, p) = ready();
    let q = ScrollPosition::new(&ctx, 0.0).unwrap();
    let c = ScrollController::default();
    let _a = c.attach(&p).unwrap();
    let _b = c.attach(&q).unwrap();
    assert!(matches!(
        c.animate_to(200.0, Duration::from_secs(1), Curve::Linear),
        Err(ScrollError::NoDimensions)
    ));
    assert_eq!(p.pixels(), 0.0);
    q.apply_dimensions(dims(80.0)).unwrap();
    c.jump_to(100.0).unwrap();
    assert_eq!((p.pixels(), q.pixels()), (100.0, 80.0));
    let runs = c
        .animate_to(300.0, Duration::from_secs(1), Curve::Linear)
        .unwrap();
    assert_eq!(runs.len(), 2);
    frame(&ctx, 1.0);
    assert_eq!((p.pixels(), q.pixels()), (300.0, 80.0));
    assert!(
        runs.iter()
            .all(|r| r.outcome() == Some(AnimationOutcome::Completed))
    );
}
#[test]
fn reentrant_detachment_skips_a_later_client_without_borrow_panics() {
    let (ctx, p) = ready();
    let q = ScrollPosition::new(&ctx, 0.0).unwrap();
    q.apply_dimensions(dims(300.0)).unwrap();
    let c = ScrollController::default();
    let _a = c.attach(&p).unwrap();
    let b = Rc::new(RefCell::new(Some(c.attach(&q).unwrap())));
    let detach = b.clone();
    let _s = c
        .subscribe(Rc::new(move || {
            detach.borrow_mut().take();
        }))
        .unwrap();
    c.jump_to(50.0).unwrap();
    assert_eq!((p.pixels(), q.pixels()), (50.0, 0.0));
    assert_eq!(c.positions().len(), 1);
}
#[test]
fn newer_controller_broadcast_supersedes_older_pending_clients() {
    let (ctx, p) = ready();
    let q = ScrollPosition::new(&ctx, 0.0).unwrap();
    q.apply_dimensions(dims(300.0)).unwrap();
    let c = ScrollController::default();
    let _a = c.attach(&p).unwrap();
    let _b = c.attach(&q).unwrap();
    let once = Rc::new(Cell::new(true));
    let weak = c.downgrade();
    let _s = c
        .subscribe(Rc::new(move || {
            if once.replace(false) {
                weak.upgrade().unwrap().jump_to(70.0).unwrap();
            }
        }))
        .unwrap();
    c.jump_to(50.0).unwrap();
    assert_eq!((p.pixels(), q.pixels()), (70.0, 70.0));
}
#[test]
fn direct_position_command_also_supersedes_pending_broadcast_target() {
    let (ctx, p) = ready();
    let q = ScrollPosition::new(&ctx, 0.0).unwrap();
    q.apply_dimensions(dims(300.0)).unwrap();
    let c = ScrollController::default();
    let _a = c.attach(&p).unwrap();
    let _b = c.attach(&q).unwrap();
    let weak = q.downgrade();
    let _s = p
        .subscribe(Rc::new(move || {
            weak.upgrade().unwrap().jump_to(90.0).unwrap()
        }))
        .unwrap();
    c.jump_to(50.0).unwrap();
    assert_eq!((p.pixels(), q.pixels()), (50.0, 90.0));
}
#[test]
fn jump_callbacks_can_start_new_motion_without_stale_idle_overwrite() {
    let (_, p) = ready();
    let weak = p.downgrade();
    let once = Rc::new(Cell::new(true));
    let _s = p
        .subscribe(Rc::new(move || {
            if once.replace(false) {
                weak.upgrade()
                    .unwrap()
                    .animate_to(200.0, Duration::from_secs(1), Curve::Linear)
                    .unwrap();
            }
        }))
        .unwrap();
    p.jump_to(50.0).unwrap();
    assert_eq!(p.activity(), ScrollActivity::Driven);
}
#[test]
fn disposal_is_rejected_during_notification_and_controller_does_not_own_position_lifetime() {
    let (_, p) = ready();
    let c = ScrollController::default();
    let a = c.attach(&p).unwrap();
    let weak = c.downgrade();
    let wp = p.downgrade();
    let _s = c
        .subscribe(Rc::new(move || {
            assert_eq!(
                weak.upgrade().unwrap().dispose(),
                Err(ScrollError::DuringNotification)
            );
            assert_eq!(
                wp.upgrade().unwrap().dispose(),
                Err(ScrollError::DuringNotification)
            );
        }))
        .unwrap();
    p.jump_to(40.0).unwrap();
    c.dispose().unwrap();
    assert!(!a.is_attached());
    assert!(!p.is_disposed());
    p.jump_to(50.0).unwrap();
    assert!(c.subscribe(Rc::new(|| {})).is_err());
    assert_eq!(c.offset(), Err(ScrollError::Disposed));
}
#[test]
fn position_disposal_and_final_drop_cancel_runs_and_remove_weak_ownership() {
    let (ctx, p) = ready();
    let run = p
        .animate_to(300.0, Duration::from_secs(1), Curve::Linear)
        .unwrap();
    p.dispose().unwrap();
    assert_eq!(run.outcome(), Some(AnimationOutcome::Canceled));
    assert_eq!(p.jump_to(20.0), Err(ScrollError::Disposed));
    assert!(p.subscribe(Rc::new(|| {})).is_err());
    let q = ScrollPosition::new(&ctx, 0.0).unwrap();
    q.apply_dimensions(dims(100.0)).unwrap();
    let run = q
        .animate_to(100.0, Duration::from_secs(1), Curve::Linear)
        .unwrap();
    let weak = q.downgrade();
    drop(q);
    assert!(weak.upgrade().is_none());
    assert_eq!(run.outcome(), Some(AnimationOutcome::Canceled));
}
#[test]
fn activity_notifier_reports_drag_ballistic_and_idle_without_pixel_motion() {
    let (_, p) = ready();
    let states = Rc::new(RefCell::new(vec![]));
    let log = states.clone();
    let weak = p.downgrade();
    let _s = p
        .subscribe_activity(Rc::new(move || {
            log.borrow_mut().push(weak.upgrade().unwrap().activity())
        }))
        .unwrap();
    p.begin_drag().unwrap();
    p.end_drag(0.0, 0.135).unwrap();
    assert_eq!(
        *states.borrow(),
        vec![ScrollActivity::Drag, ScrollActivity::Idle]
    );
}

#[test]
fn position_rebuilds_retained_builder_geometry_without_replacing_elements() {
    use rust_flutter::{
        element::{ElementTree, Widget},
        layout::BoxConstraints,
    };
    let (ctx, p) = ready();
    let mut tree = ElementTree::new(
        &ctx,
        Widget::value_listenable_builder(p.clone(), |_, pixels| {
            Widget::leaf(egui::vec2(20.0 + pixels, 10.0), |_| {})
        }),
    )
    .unwrap();
    let constraints = BoxConstraints::loose(egui::vec2(400.0, 100.0)).unwrap();
    assert_eq!(tree.layout(constraints).unwrap().size.x, 20.0);
    let ids = tree.elements().iter().map(|e| e.id).collect::<Vec<_>>();
    p.animate_to(200.0, Duration::from_secs(1), Curve::Linear)
        .unwrap();
    frame(&ctx, 0.5);
    assert_eq!(tree.layout(constraints).unwrap().size.x, 120.0);
    assert_eq!(
        tree.elements().iter().map(|e| e.id).collect::<Vec<_>>(),
        ids
    );
}

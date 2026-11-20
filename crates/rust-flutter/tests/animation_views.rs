use rust_flutter::{animation::*, egui, notifier::*};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
fn source() -> (egui::Context, AnimationController) {
    let ctx = egui::Context::default();
    ctx.begin_pass(egui::RawInput {
        time: Some(0.0),
        ..Default::default()
    });
    let _ = ctx.end_pass();
    let c = AnimationController::new(
        &ctx,
        AnimationConfig {
            duration: Duration::from_secs(1),
            ..Default::default()
        },
    )
    .unwrap();
    (ctx, c)
}
#[test]
fn immutable_sources_support_generic_values_without_retaining_or_firing_listeners() {
    let value = AlwaysStoppedAnimation::new(String::from("fixed"));
    assert_eq!(value.value(), "fixed");
    assert_eq!(value.status(), AnimationStatus::Forward);
    assert!(value.same_source(&value.clone()));
    assert!(!value.same_source(&AlwaysStoppedAnimation::new(String::from("fixed"))));
    let captured = Rc::new(Cell::new(0));
    let held = captured.clone();
    let _v = value.subscribe(Rc::new(move || held.set(1))).unwrap();
    let held = captured.clone();
    let _s = value
        .subscribe_status(Rc::new(move || held.set(2)))
        .unwrap();
    assert_eq!(Rc::strong_count(&captured), 1);
    assert_eq!(captured.get(), 0);
    assert!(value.is_animating()); // Status-derived helper; this source has no clock.
}
#[test]
fn reverse_maps_values_and_all_four_statuses() {
    let (_, p) = source();
    let r = ReverseAnimation::new(p.clone()).unwrap();
    assert_eq!(r.value(), 1.0);
    assert!(r.is_completed());
    p.forward().unwrap();
    assert_eq!(r.status(), AnimationStatus::Reverse);
    p.set_value(0.25).unwrap();
    assert_eq!(r.value(), 0.75);
    p.set_value(1.0).unwrap();
    assert_eq!(r.value(), 0.0);
    assert!(r.is_dismissed());
    p.reverse().unwrap();
    assert_eq!(r.status(), AnimationStatus::Forward);
    let twice = ReverseAnimation::new(r.clone()).unwrap();
    assert_eq!(twice.status(), p.status());
    assert_eq!(twice.value(), p.value());
}
#[test]
fn status_listeners_observe_reversed_status_and_disposal_preserves_parent() {
    let (_, p) = source();
    let r = ReverseAnimation::new(p.clone()).unwrap();
    let statuses = Rc::new(RefCell::new(vec![]));
    let log = statuses.clone();
    let weak = r.downgrade();
    let _s = r
        .subscribe_status(Rc::new(move || {
            log.borrow_mut().push(weak.upgrade().unwrap().status())
        }))
        .unwrap();
    p.forward().unwrap();
    p.set_value(1.0).unwrap();
    p.reverse().unwrap();
    assert_eq!(
        *statuses.borrow(),
        vec![
            AnimationStatus::Reverse,
            AnimationStatus::Dismissed,
            AnimationStatus::Forward
        ]
    );
    r.dispose().unwrap();
    assert!(!p.is_disposed());
    assert!(r.try_value().is_err());
    assert!(r.subscribe(Rc::new(|| {})).is_err());
    p.set_value(0.5).unwrap();
    assert_eq!(statuses.borrow().len(), 3);
}
#[test]
fn reverse_supports_finite_overshoot_and_propagates_checked_parent_errors() {
    let over = ReverseAnimation::new(AlwaysStoppedAnimation::new(1.5_f32)).unwrap();
    assert_eq!(over.value(), -0.5);
    let bad = ReverseAnimation::new(AlwaysStoppedAnimation::new(f32::NAN)).unwrap();
    assert!(bad.try_value().is_err());
    let (_, p) = source();
    let r = ReverseAnimation::new(p.clone()).unwrap();
    p.dispose().unwrap();
    assert_eq!(r.try_value(), Err(AnimationError::Disposed));
    assert!(ReverseAnimation::new(p).is_err());
}
#[test]
fn source_drop_cleans_links_even_when_tokens_outlive_the_view() {
    let (_, p) = source();
    let r = ReverseAnimation::new(p).unwrap();
    let weak = r.downgrade();
    let held = Rc::new(Cell::new(0));
    let capture = held.clone();
    let token = r.subscribe(Rc::new(move || capture.set(1))).unwrap();
    assert_eq!(Rc::strong_count(&held), 2);
    drop(r);
    assert!(weak.upgrade().is_none());
    assert_eq!(Rc::strong_count(&held), 1);
    drop(token);
}
#[test]
fn controller_frames_drive_reverse_and_typed_chains() {
    let (ctx, p) = source();
    let r = ReverseAnimation::new(p.clone()).unwrap();
    let size = r
        .clone()
        .drive(ValueTween::new(0.0_f32, 100.0).unwrap())
        .unwrap();
    p.forward().unwrap();
    ctx.begin_pass(egui::RawInput {
        time: Some(0.25),
        ..Default::default()
    });
    let _ = ctx.end_pass();
    assert_eq!(r.value(), 0.75);
    assert_eq!(size.value(), 75.0);
    ctx.begin_pass(egui::RawInput {
        time: Some(1.0),
        ..Default::default()
    });
    let _ = ctx.end_pass();
    assert_eq!(size.value(), 0.0);
    assert_eq!(size.status(), AnimationStatus::Dismissed);
}

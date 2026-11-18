use rust_flutter::{animation::*, egui, element::*, layout::*, notifier::*};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
#[derive(Clone)]
struct Source(Rc<SourceData>);
struct SourceData {
    value: Cell<f32>,
    status: Cell<AnimationStatus>,
    values: ChangeNotifier,
    statuses: ChangeNotifier,
    disposed: Cell<bool>,
    dispose_on_status: Cell<bool>,
    fire_value_on_subscribe: Cell<bool>,
    live_values: Rc<Cell<usize>>,
    live_statuses: Rc<Cell<usize>>,
}
impl Source {
    fn new() -> Self {
        Self(Rc::new(SourceData {
            value: Cell::new(0.0),
            status: Cell::new(AnimationStatus::Dismissed),
            values: ChangeNotifier::new(),
            statuses: ChangeNotifier::new(),
            disposed: Cell::new(false),
            dispose_on_status: Cell::new(false),
            fire_value_on_subscribe: Cell::new(false),
            live_values: Rc::default(),
            live_statuses: Rc::default(),
        }))
    }
    fn publish(&self, value: f32, status: AnimationStatus) {
        let old_value = self.0.value.replace(value);
        let old_status = self.0.status.replace(status);
        if old_value != value {
            self.0.values.notify_listeners().unwrap();
        }
        if old_status != status {
            self.0.statuses.notify_listeners().unwrap();
        }
    }
    fn listen(&self, listener: Listener, status: bool) -> Result<Subscription, Disposed> {
        if self.is_disposed() {
            return Err(Disposed);
        }
        let (notifier, count) = if status {
            (&self.0.statuses, self.0.live_statuses.clone())
        } else {
            (&self.0.values, self.0.live_values.clone())
        };
        let immediate = listener.clone();
        let subscription = notifier.subscribe(listener)?;
        count.set(count.get() + 1);
        let token = Subscription::new(move || {
            count.set(count.get() - 1);
            drop(subscription);
        });
        if !status && self.0.fire_value_on_subscribe.get() {
            immediate();
        }
        Ok(token)
    }
}
impl Listenable for Source {
    fn subscribe(&self, l: Listener) -> Result<Subscription, Disposed> {
        self.listen(l, false)
    }
    fn same_source(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl ValueListenable for Source {
    type Value = f32;
    fn value(&self) -> f32 {
        self.0.value.get()
    }
}
impl Animation<f32> for Source {
    fn status(&self) -> AnimationStatus {
        self.0.status.get()
    }
    fn subscribe_status(&self, l: Listener) -> Result<Subscription, Disposed> {
        let subscription = self.listen(l, true)?;
        if self.0.dispose_on_status.replace(false) {
            self.0.disposed.set(true);
        }
        Ok(subscription)
    }
    fn is_disposed(&self) -> bool {
        self.0.disposed.get()
    }
}
struct FnCurve(Rc<dyn Fn(f32) -> Result<f32, AnimationError>>);
impl ParametricCurve for FnCurve {
    fn transform(&self, t: f32) -> Result<f32, AnimationError> {
        (self.0)(t)
    }
}
fn cube() -> Cubic {
    Cubic::new(1.0 / 3.0, 0.0, 2.0 / 3.0, 0.0).unwrap()
}
fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-5, "{a} != {b}");
}
#[test]
fn cubic_solves_x_before_y_and_handles_flat_derivatives_endpoints_and_overshoot() {
    let curve = Cubic::new(0.42, 0.0, 0.58, 1.0).unwrap();
    close(curve.transform(0.25).unwrap(), 0.1291619);
    close(curve.transform(0.5).unwrap(), 0.5);
    close(curve.transform(0.75).unwrap(), 0.8708381);
    for t in [0.0, 0.001, 0.25, 0.5, 0.75, 0.999, 1.0] {
        close(
            Cubic::new(0.0, 0.0, 0.0, 1.0)
                .unwrap()
                .transform(t)
                .unwrap(),
            if t == 0.0 {
                0.0
            } else {
                let s = t.cbrt();
                3.0 * s * s - 2.0 * s * s * s
            },
        );
        close(cube().transform(t).unwrap(), t * t * t);
    }
    close(
        Cubic::new(1.0, 0.0, 0.0, 1.0)
            .unwrap()
            .transform(0.5)
            .unwrap(),
        0.5,
    );
    let tiny = f32::from_bits(1);
    let root = f64::from(tiny).cbrt();
    let expected = (3.0 * root - 3.0 * root * root + root * root * root) as f32;
    let actual = Cubic::new(0.0, 1.0, 0.0, 1.0)
        .unwrap()
        .transform(tiny)
        .unwrap();
    assert!((actual / expected - 1.0).abs() < 1e-6);
    let overshoot = Cubic::new(0.25, 2.0, 0.75, 2.0).unwrap();
    assert!(overshoot.transform(0.5).unwrap() > 1.0);
    assert_eq!(overshoot.transform(0.0).unwrap(), 0.0);
    assert_eq!(overshoot.transform(1.0).unwrap(), 1.0);
    assert!(
        Cubic::new(0.25, f32::MAX, 0.75, f32::MAX)
            .unwrap()
            .transform(0.5)
            .unwrap()
            .is_finite()
    );
}
#[test]
fn invalid_controls_and_progress_are_checked_and_flipped_curve_inverts_both_axes() {
    for controls in [
        [-0.1, 0.0, 0.5, 1.0],
        [0.0, 0.0, 1.1, 1.0],
        [0.0, f32::NAN, 1.0, 1.0],
        [0.0, 0.0, 1.0, f32::INFINITY],
    ] {
        assert!(Cubic::new(controls[0], controls[1], controls[2], controls[3]).is_err());
    }
    for t in [-1.0, f32::NAN, f32::INFINITY, 1.1] {
        assert!(cube().transform(t).is_err());
        assert!(ParametricCurve::transform(&Curve::Linear, t).is_err());
    }
    let flipped = FlippedCurve(cube());
    close(flipped.transform(0.25).unwrap(), 1.0 - 0.75_f32.powi(3));
    close(flipped.transform(0.0).unwrap(), 0.0);
    close(flipped.transform(1.0).unwrap(), 1.0);
}
#[test]
fn direction_is_latched_until_terminal_status_and_reverse_curve_is_optional() {
    let p = Source::new();
    let c = CurvedAnimation::new(p.clone(), cube()).unwrap();
    c.set_reverse_curve(Some(Rc::new(Curve::EaseOut))).unwrap();
    p.publish(0.5, AnimationStatus::Forward);
    close(c.value(), 0.125);
    p.publish(0.4, AnimationStatus::Reverse);
    close(c.value(), 0.064);
    p.publish(0.6, AnimationStatus::Forward);
    close(c.value(), 0.216);
    p.publish(1.0, AnimationStatus::Completed);
    assert!(c.is_completed());
    p.publish(0.5, AnimationStatus::Reverse);
    close(c.value(), 0.875);
    p.publish(0.6, AnimationStatus::Forward);
    close(c.value(), 0.936);
    p.publish(0.0, AnimationStatus::Dismissed);
    assert!(c.is_dismissed());
    p.publish(0.5, AnimationStatus::Forward);
    close(c.value(), 0.125);
    c.set_reverse_curve(None).unwrap();
    p.publish(1.0, AnimationStatus::Completed);
    p.publish(0.5, AnimationStatus::Reverse);
    close(c.value(), 0.125);
}
#[test]
fn constructing_mid_reverse_latches_reverse_and_config_edits_do_not_emit_notifications() {
    let p = Source::new();
    p.publish(0.5, AnimationStatus::Reverse);
    let c = CurvedAnimation::new(p.clone(), cube()).unwrap();
    c.set_reverse_curve(Some(Rc::new(Curve::EaseOut))).unwrap();
    close(c.value(), 0.875);
    let hits = Rc::new(Cell::new(0));
    let count = hits.clone();
    let _s = c
        .subscribe(Rc::new(move || count.set(count.get() + 1)))
        .unwrap();
    c.set_reverse_curve(Some(Rc::new(Curve::Linear))).unwrap();
    close(c.value(), 0.5);
    assert_eq!(hits.get(), 0);
    c.set_curve(Curve::EaseOut).unwrap();
    p.publish(0.0, AnimationStatus::Dismissed);
    p.publish(0.5, AnimationStatus::Forward);
    close(c.value(), 0.875);
}
#[test]
fn value_and_status_registration_order_and_reentrant_disposal_are_safe() {
    let p = Source::new();
    let c = CurvedAnimation::new(p.clone(), cube()).unwrap();
    let clone = c.clone();
    assert!(c.same_source(&clone));
    assert!(!c.same_source(&CurvedAnimation::new(p.clone(), cube()).unwrap()));
    let events = Rc::new(RefCell::new(vec![]));
    let log = events.clone();
    let sample = c.clone();
    let _v = c
        .subscribe(Rc::new(move || {
            log.borrow_mut()
                .push((false, sample.value(), sample.status()))
        }))
        .unwrap();
    let log = events.clone();
    let sample = c.clone();
    let _s = c
        .subscribe_status(Rc::new(move || {
            log.borrow_mut()
                .push((true, sample.value(), sample.status()))
        }))
        .unwrap();
    p.publish(0.5, AnimationStatus::Forward);
    assert_eq!(events.borrow().len(), 2);
    for (index, (status_event, value, status)) in events.borrow().iter().enumerate() {
        assert_eq!(*status_event, index == 1);
        close(*value, 0.125);
        assert_eq!(*status, AnimationStatus::Forward);
    }
    let stop = c.clone();
    let _stop = c
        .subscribe(Rc::new(move || stop.dispose().unwrap()))
        .unwrap();
    p.publish(0.75, AnimationStatus::Forward);
    assert!(c.is_disposed());
    assert!(!p.is_disposed());
    assert_eq!(p.0.live_values.get(), 0);
    assert_eq!(p.0.live_statuses.get(), 0);
    assert!(c.subscribe(Rc::new(|| {})).is_err());
    assert!(c.dispose().is_err());
    assert!(c.try_value().is_err());
    p.publish(1.0, AnimationStatus::Completed);
}
#[test]
fn dropping_derived_source_releases_parent_registrations_even_if_tokens_outlive_it() {
    let p = Source::new();
    let c = CurvedAnimation::new(p.clone(), cube()).unwrap();
    let weak = c.downgrade();
    let value = c.subscribe(Rc::new(|| {})).unwrap();
    let status = c.subscribe_status(Rc::new(|| {})).unwrap();
    assert_eq!(p.0.live_values.get(), 1);
    assert_eq!(p.0.live_statuses.get(), 2);
    drop(c);
    assert!(weak.upgrade().is_none());
    assert_eq!(p.0.live_values.get(), 0);
    assert_eq!(p.0.live_statuses.get(), 0);
    drop(value);
    drop(status);
}
#[test]
fn derived_chains_accept_general_animation_parents_and_keep_overshoot_unclamped() {
    let p = Rc::new(Source::new());
    let first = CurvedAnimation::new(p.clone(), cube()).unwrap();
    let second = CurvedAnimation::new(first.clone(), cube()).unwrap();
    p.publish(0.5, AnimationStatus::Forward);
    close(second.value(), 0.001953125);
    let over = CurvedAnimation::new(p.clone(), Cubic::new(0.25, 2.0, 0.75, 2.0).unwrap()).unwrap();
    assert!(over.value() > 1.0);
    let invalid = CurvedAnimation::new(over, Curve::Linear).unwrap();
    assert!(invalid.try_value().is_err());
    first.dispose().unwrap();
    assert!(second.is_disposed());
    second.dispose().unwrap();
}
#[test]
fn custom_curve_errors_panics_invalid_outputs_and_bad_endpoints_are_observable() {
    let p = Source::new();
    let c = CurvedAnimation::new(p.clone(), Curve::Linear).unwrap();
    p.publish(0.5, AnimationStatus::Forward);
    c.set_curve(FnCurve(Rc::new(|_| {
        Err(AnimationError::InvalidConfiguration("custom failure"))
    })))
    .unwrap();
    assert_eq!(
        c.try_value(),
        Err(AnimationError::InvalidConfiguration("custom failure"))
    );
    c.set_curve(FnCurve(Rc::new(|_| panic!("bad curve"))))
        .unwrap();
    assert_eq!(c.try_value(), Err(AnimationError::CurvePanicked));
    c.set_curve(FnCurve(Rc::new(|_| Ok(f32::NAN)))).unwrap();
    assert!(c.try_value().is_err());
    c.set_curve(FnCurve(Rc::new(|_| Ok(0.5)))).unwrap();
    p.publish(1.0, AnimationStatus::Completed);
    assert!(c.try_value().is_err());
    p.publish(2.0, AnimationStatus::Forward);
    assert!(c.try_value().is_err());
}
#[test]
fn reentrant_curve_replacement_parent_motion_and_disposal_cannot_return_stale_samples() {
    let p = Source::new();
    p.publish(0.5, AnimationStatus::Forward);
    let c = CurvedAnimation::new(p.clone(), Curve::Linear).unwrap();
    let weak = c.downgrade();
    c.set_curve(FnCurve(Rc::new(move |t| {
        weak.upgrade().unwrap().set_curve(Curve::EaseOut).unwrap();
        Ok(t)
    })))
    .unwrap();
    assert_eq!(c.try_value(), Err(AnimationError::Superseded));
    close(c.value(), 0.875);
    let parent = p.clone();
    c.set_curve(FnCurve(Rc::new(move |t| {
        parent.publish(0.75, AnimationStatus::Forward);
        Ok(t)
    })))
    .unwrap();
    assert_eq!(c.try_value(), Err(AnimationError::Superseded));
    let weak = c.downgrade();
    c.set_curve(FnCurve(Rc::new(move |t| {
        weak.upgrade().unwrap().dispose().unwrap();
        Ok(t)
    })))
    .unwrap();
    assert_eq!(c.try_value(), Err(AnimationError::Disposed));
}
#[test]
fn disposed_parent_rejects_new_derived_sources_and_can_be_cleaned_up_afterwards() {
    let p = Source::new();
    let c = CurvedAnimation::new(p.clone(), Curve::Linear).unwrap();
    p.0.disposed.set(true);
    assert!(CurvedAnimation::new(p.clone(), Curve::Linear).is_err());
    assert!(c.is_disposed());
    assert_eq!(c.try_value(), Err(AnimationError::Disposed));
    c.dispose().unwrap();
    assert_eq!(p.0.live_statuses.get(), 0);
}
#[test]
fn existing_controller_frames_drive_derived_widget_values_and_reverse_latching() {
    let ctx = egui::Context::default();
    let frame = |time| {
        ctx.begin_pass(egui::RawInput {
            time: Some(time),
            ..Default::default()
        });
        let _ = ctx.end_pass();
    };
    frame(0.0);
    let p = AnimationController::new(
        &ctx,
        AnimationConfig {
            duration: Duration::from_secs(1),
            ..Default::default()
        },
    )
    .unwrap();
    let c = CurvedAnimation::new(p.clone(), cube()).unwrap();
    c.set_reverse_curve(Some(Rc::new(Curve::EaseOut))).unwrap();
    p.forward().unwrap();
    frame(0.5);
    close(c.value(), 0.125);
    p.reverse().unwrap();
    frame(0.75);
    close(c.value(), 0.015625);
    frame(1.0);
    assert!(c.is_dismissed());
    p.forward().unwrap();
    frame(2.0);
    assert!(c.is_completed());
    p.reverse().unwrap();
    frame(2.5);
    close(c.value(), 0.875);
    let mut tree = ElementTree::new(
        &ctx,
        Widget::value_listenable_builder(c.clone(), |_, v| {
            Widget::leaf(egui::vec2(100.0 * v, 20.0), |_| {})
        }),
    )
    .unwrap();
    let bounds = BoxConstraints::loose(egui::vec2(200.0, 40.0)).unwrap();
    close(tree.layout(bounds).unwrap().size.x, 87.5);
    p.set_value(0.25).unwrap();
    close(tree.layout(bounds).unwrap().size.x, 57.8125);
    tree.unmount();
    c.dispose().unwrap();
    p.dispose().unwrap();
}

#[test]
fn synchronous_subscription_callbacks_and_parent_disposal_during_attach_leave_no_registrations() {
    let p = Source::new();
    p.0.dispose_on_status.set(true);
    assert!(CurvedAnimation::new(p.clone(), Curve::Linear).is_err());
    assert_eq!(p.0.live_statuses.get(), 0);
    let p = Source::new();
    let c = CurvedAnimation::new(p.clone(), Curve::Linear).unwrap();
    p.0.fire_value_on_subscribe.set(true);
    let stop = c.clone();
    assert!(
        c.subscribe(Rc::new(move || stop.dispose().unwrap()))
            .is_err()
    );
    assert_eq!(p.0.live_values.get(), 0);
    assert_eq!(p.0.live_statuses.get(), 0);
}
#[test]
fn duplicate_listener_tokens_are_independent_and_equal_mapped_values_still_follow_parent_events() {
    let p = Source::new();
    let c = CurvedAnimation::new(
        p.clone(),
        FnCurve(Rc::new(|t| Ok(if t == 0.0 || t == 1.0 { t } else { 0.5 }))),
    )
    .unwrap();
    let count = Rc::new(Cell::new(0));
    let hits = count.clone();
    let callback: Listener = Rc::new(move || hits.set(hits.get() + 1));
    let first = c.subscribe(callback.clone()).unwrap();
    let second = c.subscribe(callback).unwrap();
    p.publish(0.25, AnimationStatus::Forward);
    assert_eq!(count.get(), 2);
    drop(first);
    p.publish(0.5, AnimationStatus::Forward);
    assert_eq!(count.get(), 3);
    drop(second);
    p.publish(0.75, AnimationStatus::Forward);
    assert_eq!(count.get(), 3);
}

#[test]
fn checked_chains_propagate_parent_errors_and_reentrant_parent_sampling_releases_child_borrows() {
    let source = Source::new();
    source.publish(0.5, AnimationStatus::Forward);
    let parent = CurvedAnimation::new(source, Curve::Linear).unwrap();
    let child = CurvedAnimation::new(parent.clone(), Curve::Linear).unwrap();
    parent
        .set_curve(FnCurve(Rc::new(|_| {
            Err(AnimationError::InvalidConfiguration("parent curve failed"))
        })))
        .unwrap();
    assert_eq!(
        child.try_value(),
        Err(AnimationError::InvalidConfiguration("parent curve failed"))
    );
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let weak = child.downgrade();
    parent
        .set_curve(FnCurve(Rc::new(move |t| {
            let call = count.get();
            count.set(call + 1);
            if call == 1 {
                weak.upgrade().unwrap().set_curve(Curve::EaseOut).unwrap();
            }
            Ok(t)
        })))
        .unwrap();
    assert_eq!(child.try_value(), Err(AnimationError::Superseded));
    close(child.value(), 0.875);
}

#[test]
fn recursive_custom_curve_sampling_is_rejected_without_leaking_the_sample_guard() {
    let p = Source::new();
    p.publish(0.5, AnimationStatus::Forward);
    let c = CurvedAnimation::new(p, Curve::Linear).unwrap();
    let weak = c.downgrade();
    c.set_curve(FnCurve(Rc::new(move |_| {
        weak.upgrade().unwrap().try_value()
    })))
    .unwrap();
    assert_eq!(c.try_value(), Err(AnimationError::Superseded));
    c.set_curve(Curve::EaseOut).unwrap();
    close(c.value(), 0.875);
}

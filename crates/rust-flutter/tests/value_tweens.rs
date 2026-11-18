use rust_flutter::{
    animation::*, decoration::BoxDecoration, egui, element::*, layout::*, notifier::*,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
fn controller(value: f32) -> (egui::Context, AnimationController) {
    let ctx = egui::Context::default();
    ctx.begin_pass(egui::RawInput {
        time: Some(0.0),
        ..Default::default()
    });
    let _ = ctx.end_pass();
    let source = AnimationController::new(
        &ctx,
        AnimationConfig {
            value: Some(value),
            duration: Duration::from_secs(1),
            ..Default::default()
        },
    )
    .unwrap();
    (ctx, source)
}
fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-5, "{a} != {b}");
}
#[test]
fn typed_tweens_preserve_exact_endpoints_and_extrapolate_finite_scalars_and_geometry() {
    let scalar = ValueTween::new(-f32::MAX, f32::MAX).unwrap();
    assert_eq!(scalar.transform(0.0).unwrap(), -f32::MAX);
    assert_eq!(scalar.transform(1.0).unwrap(), f32::MAX);
    assert_eq!(scalar.transform(0.5).unwrap(), 0.0);
    assert!(scalar.transform(2.0).is_err());
    close(
        ValueTween::new(10.0_f32, 20.0)
            .unwrap()
            .transform(1.5)
            .unwrap(),
        25.0,
    );
    assert_eq!(
        ValueTween::new(2.0_f64, 4.0)
            .unwrap()
            .transform(-1.0)
            .unwrap(),
        0.0
    );
    let vector = ValueTween::new(egui::vec2(2.0, 4.0), egui::vec2(6.0, 8.0)).unwrap();
    assert_eq!(vector.transform(1.5).unwrap(), egui::vec2(8.0, 10.0));
    let position = ValueTween::new(egui::pos2(0.0, 4.0), egui::pos2(4.0, 8.0)).unwrap();
    assert_eq!(position.transform(0.5).unwrap(), egui::pos2(2.0, 6.0));
    let color = ValueTween::new(egui::Color32::BLACK, egui::Color32::WHITE).unwrap();
    assert_eq!(color.transform(-1.0).unwrap(), egui::Color32::BLACK);
    assert_eq!(color.transform(2.0).unwrap(), egui::Color32::WHITE);
    assert!(ValueTween::new(f32::NAN, 1.0).is_err());
    assert!(vector.transform(f32::NAN).is_err());
}
#[test]
fn extrapolation_rejects_invalid_geometry_instead_of_clamping_it_or_creating_nan() {
    let rect = ValueTween::new(
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1.0, 1.0)),
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(3.0, 3.0)),
    )
    .unwrap();
    assert!(rect.transform(-1.0).is_err());
    assert!(
        ValueTween::new(Insets::all(1.0), Insets::all(3.0))
            .unwrap()
            .transform(-1.0)
            .is_err()
    );
    assert!(
        ValueTween::new(
            egui::Stroke::new(1.0_f32, egui::Color32::WHITE),
            egui::Stroke::new(3.0_f32, egui::Color32::WHITE)
        )
        .unwrap()
        .transform(-1.0)
        .is_err()
    );
    let a = BoxDecoration::solid(egui::Color32::BLACK);
    let mut b = a;
    b.border = egui::Stroke::new(1.0_f32, egui::Color32::WHITE);
    assert!(ValueTween::new(a, b).unwrap().transform(-1.0).is_err());
}
#[derive(Clone, PartialEq)]
struct Normalized(f32);
impl TweenValue for Normalized {
    fn interpolate(&self, b: &Self, t: f32) -> Self {
        assert!((0.0..=1.0).contains(&t));
        Self(self.0 + (b.0 - self.0) * t)
    }
}
#[test]
fn existing_custom_interpolation_contracts_remain_normalized_unless_they_opt_in() {
    let tween = ValueTween::new(Normalized(0.0), Normalized(10.0)).unwrap();
    assert_eq!(tween.transform(0.5).unwrap().0, 5.0);
    assert!(tween.transform(2.0).is_err());
}
#[test]
fn shared_endpoint_edits_update_existing_driven_values_without_replacing_source_or_notifying() {
    let (_, p) = controller(0.5);
    let tween = ValueTween::new(10.0_f32, 20.0).unwrap();
    let driven = tween.clone().animate(p.clone()).unwrap();
    let clone = driven.clone();
    assert!(driven.same_source(&clone));
    close(driven.value(), 15.0);
    let calls = Rc::new(Cell::new(0));
    let hits = calls.clone();
    let _s = driven
        .subscribe(Rc::new(move || hits.set(hits.get() + 1)))
        .unwrap();
    tween.set_end(30.0).unwrap();
    close(driven.value(), 20.0);
    assert_eq!(calls.get(), 0);
    tween.set_begin(0.0).unwrap();
    assert_eq!(tween.begin(), 0.0);
    assert_eq!(tween.end(), 30.0);
    close(driven.value(), 15.0);
    p.set_value(0.75).unwrap();
    close(driven.value(), 22.5);
    assert_eq!(calls.get(), 1);
    assert!(tween.set_end(f32::NAN).is_err());
    assert_eq!(tween.end(), 30.0);
}
#[test]
fn animate_drive_evaluate_and_chain_compose_in_parent_then_child_order() {
    let (_, p) = controller(0.5);
    let tween = ValueTween::new(10.0_f32, 20.0).unwrap();
    close(tween.evaluate(&p).unwrap(), 15.0);
    close(p.clone().drive(tween.clone()).unwrap().value(), 15.0);
    let cube = Cubic::new(1.0 / 3.0, 0.0, 2.0 / 3.0, 0.0).unwrap();
    let chained = tween.chain(CurveTween::new(cube));
    close(chained.transform(0.5).unwrap(), 11.25);
    let intermediate = CurveTween::new(cube).animate(p.clone()).unwrap();
    close(
        chained.animate(p).unwrap().value(),
        intermediate
            .drive(ValueTween::new(10.0_f32, 20.0).unwrap())
            .unwrap()
            .value(),
    );
}
#[test]
fn weighted_sequences_remap_each_section_and_choose_next_section_at_exact_boundaries() {
    let sequence = TweenSequence::new(vec![
        SequenceItem::new(ValueTween::new(0.0_f32, 10.0).unwrap(), 2.0).unwrap(),
        SequenceItem::new(ConstantTween(99.0_f32), 1.0).unwrap(),
        SequenceItem::new(ValueTween::new(20.0_f32, 30.0).unwrap(), 1.0).unwrap(),
    ])
    .unwrap();
    for (t, expected) in [
        (0.0, 0.0),
        (0.25, 5.0),
        (0.5, 99.0),
        (0.625, 99.0),
        (0.75, 20.0),
        (0.875, 25.0),
        (1.0, 30.0),
    ] {
        close(sequence.transform(t).unwrap(), expected);
    }
    let strings = TweenSequence::new(vec![
        SequenceItem::new(ConstantTween(String::from("first")), 1.0).unwrap(),
        SequenceItem::new(ConstantTween(String::from("second")), 1.0).unwrap(),
    ])
    .unwrap();
    assert_eq!(strings.transform(0.5).unwrap(), "second");
    assert!(sequence.transform(-0.1).is_err());
}
#[test]
fn sequence_weights_handle_large_finite_totals_and_reject_invalid_or_collapsed_sections() {
    let sequence = TweenSequence::new(vec![
        SequenceItem::new(ConstantTween(1.0_f32), f64::MAX).unwrap(),
        SequenceItem::new(ConstantTween(2.0_f32), f64::MAX).unwrap(),
    ])
    .unwrap();
    assert_eq!(sequence.transform(0.5).unwrap(), 2.0);
    for weight in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(SequenceItem::new(ConstantTween(0.0_f32), weight).is_err());
    }
    assert!(TweenSequence::<f32>::new(vec![]).is_err());
    assert!(
        TweenSequence::new(vec![
            SequenceItem::new(ConstantTween(0.0_f32), f64::MAX).unwrap(),
            SequenceItem::new(ConstantTween(1.0_f32), f64::MIN_POSITIVE).unwrap()
        ])
        .is_err()
    );
}
struct FnMap(Rc<dyn Fn(f32) -> Result<f32, AnimationError>>);
impl Animatable for FnMap {
    type Value = f32;
    fn transform(&self, t: f32) -> Result<f32, AnimationError> {
        (self.0)(t)
    }
}
struct FnCurve(Rc<dyn Fn(f32) -> Result<f32, AnimationError>>);
impl ParametricCurve for FnCurve {
    fn transform(&self, t: f32) -> Result<f32, AnimationError> {
        (self.0)(t)
    }
}
#[test]
fn curve_tweens_are_mutable_checked_and_guard_reentrant_replacement_and_recursive_sampling() {
    let curve = CurveTween::new(Curve::Linear);
    let clone = curve.clone();
    curve.set_curve(Curve::EaseOut);
    close(clone.transform(0.5).unwrap(), 0.875);
    let weak = curve.downgrade();
    curve.set_curve(FnCurve(Rc::new(move |t| {
        weak.upgrade().unwrap().set_curve(Curve::Linear);
        Ok(t)
    })));
    assert_eq!(curve.transform(0.5), Err(AnimationError::Superseded));
    close(curve.transform(0.5).unwrap(), 0.5);
    let weak = curve.downgrade();
    curve.set_curve(FnCurve(Rc::new(move |t| {
        weak.upgrade().unwrap().transform(t)
    })));
    assert_eq!(curve.transform(0.5), Err(AnimationError::Superseded));
    curve.set_curve(FnCurve(Rc::new(|_| panic!("curve"))));
    assert_eq!(curve.transform(0.5), Err(AnimationError::CurvePanicked));
    curve.set_curve(FnCurve(Rc::new(|_| Ok(f32::NAN))));
    assert!(curve.transform(0.5).is_err());
    curve.set_curve(FnCurve(Rc::new(|_| Ok(0.5))));
    assert!(curve.transform(0.0).is_err());
}
#[test]
fn driven_lifetime_forwards_value_status_and_disposes_during_callbacks_without_disposing_parent() {
    let (_, p) = controller(0.0);
    let a = p
        .clone()
        .drive(ValueTween::new(0.0_f32, 100.0).unwrap())
        .unwrap();
    let events = Rc::new(RefCell::new(vec![]));
    let log = events.clone();
    let weak = a.downgrade();
    let _v = a
        .subscribe(Rc::new(move || {
            log.borrow_mut()
                .push((false, weak.upgrade().unwrap().value()))
        }))
        .unwrap();
    let log = events.clone();
    let weak = a.downgrade();
    let _s = a
        .subscribe_status(Rc::new(move || {
            log.borrow_mut()
                .push((true, weak.upgrade().unwrap().value()))
        }))
        .unwrap();
    p.set_value(0.25).unwrap();
    assert_eq!(*events.borrow(), vec![(false, 25.0), (true, 25.0)]);
    let weak = a.downgrade();
    let _stop = a
        .subscribe(Rc::new(move || weak.upgrade().unwrap().dispose().unwrap()))
        .unwrap();
    p.set_value(0.5).unwrap();
    assert!(a.is_disposed());
    assert!(!p.is_disposed());
    assert!(a.try_value().is_err());
    assert!(a.subscribe(Rc::new(|| {})).is_err());
    assert!(a.dispose().is_err());
    let count = events.borrow().len();
    p.set_value(1.0).unwrap();
    assert_eq!(events.borrow().len(), count);
}
#[test]
fn dropping_driven_source_cancels_links_even_when_subscription_tokens_outlive_it() {
    let (_, p) = controller(0.0);
    let a = p.drive(ConstantTween(egui::vec2(1.0, 2.0))).unwrap();
    let weak = a.downgrade();
    let sentinel = Rc::new(Cell::new(0));
    let held = sentinel.clone();
    let v = a.subscribe(Rc::new(move || held.set(1))).unwrap();
    let held = sentinel.clone();
    let s = a.subscribe_status(Rc::new(move || held.set(2))).unwrap();
    assert_eq!(Rc::strong_count(&sentinel), 3);
    drop(a);
    assert!(weak.upgrade().is_none());
    assert_eq!(Rc::strong_count(&sentinel), 1);
    drop(v);
    drop(s);
}
#[test]
fn driven_sampling_reports_mapping_panics_parent_changes_disposal_and_recursive_calls() {
    let (_, p) = controller(0.5);
    let panic = p
        .clone()
        .drive(FnMap(Rc::new(|_| panic!("mapping"))))
        .unwrap();
    assert_eq!(
        panic.try_value(),
        Err(AnimationError::InterpolationPanicked)
    );
    let parent = p.clone();
    let changed = p
        .clone()
        .drive(FnMap(Rc::new(move |t| {
            parent.set_value(0.75).unwrap();
            Ok(t)
        })))
        .unwrap();
    assert_eq!(changed.try_value(), Err(AnimationError::Superseded));
    let holder: Rc<RefCell<Option<WeakDrivenAnimation<AnimationController, FnMap>>>> =
        Rc::default();
    let saved = holder.clone();
    let recursive = p
        .clone()
        .drive(FnMap(Rc::new(move |_| {
            saved
                .borrow()
                .as_ref()
                .unwrap()
                .upgrade()
                .unwrap()
                .try_value()
        })))
        .unwrap();
    *holder.borrow_mut() = Some(recursive.downgrade());
    assert_eq!(recursive.try_value(), Err(AnimationError::Superseded));
    let holder: Rc<RefCell<Option<WeakDrivenAnimation<AnimationController, FnMap>>>> =
        Rc::default();
    let saved = holder.clone();
    let disposed = p
        .drive(FnMap(Rc::new(move |t| {
            saved
                .borrow()
                .as_ref()
                .unwrap()
                .upgrade()
                .unwrap()
                .dispose()
                .unwrap();
            Ok(t)
        })))
        .unwrap();
    *holder.borrow_mut() = Some(disposed.downgrade());
    assert_eq!(disposed.try_value(), Err(AnimationError::Disposed));
}
#[derive(Clone)]
struct Reentrant {
    value: f32,
    hook: Option<Rc<dyn Fn()>>,
}
impl PartialEq for Reentrant {
    fn eq(&self, b: &Self) -> bool {
        self.value == b.value
    }
}
impl TweenValue for Reentrant {
    fn interpolate(&self, b: &Self, t: f32) -> Self {
        if let Some(h) = &self.hook {
            h();
        }
        Self {
            value: self.value + (b.value - self.value) * t,
            hook: None,
        }
    }
}
fn plain(value: f32) -> Reentrant {
    Reentrant { value, hook: None }
}
#[test]
fn mutable_endpoint_callbacks_cannot_overwrite_newer_values_and_recursive_interpolation_is_checked()
{
    let holder: Rc<RefCell<Option<WeakValueTween<Reentrant>>>> = Rc::default();
    let saved = holder.clone();
    let begin = Reentrant {
        value: 0.0,
        hook: Some(Rc::new(move || {
            saved
                .borrow()
                .as_ref()
                .unwrap()
                .upgrade()
                .unwrap()
                .set_end(plain(20.0))
                .unwrap();
        })),
    };
    let tween = ValueTween::new(begin, plain(10.0)).unwrap();
    *holder.borrow_mut() = Some(tween.downgrade());
    assert!(matches!(
        tween.transform(0.5),
        Err(AnimationError::Superseded)
    ));
    assert_eq!(tween.end().value, 20.0);
    tween.set_begin(plain(0.0)).unwrap();
    assert_eq!(tween.transform(0.5).unwrap().value, 10.0);
    let weak = tween.downgrade();
    tween
        .set_begin(Reentrant {
            value: 0.0,
            hook: Some(Rc::new(move || {
                assert!(matches!(
                    weak.upgrade().unwrap().transform(0.5),
                    Err(AnimationError::Superseded)
                ));
            })),
        })
        .unwrap();
    assert_eq!(tween.transform(0.5).unwrap().value, 10.0);
}
#[test]
fn overshooting_curve_outputs_drive_valid_typed_values_and_checked_failures_propagate_through_chains()
 {
    let (_, p) = controller(0.5);
    let curve = CurvedAnimation::new(p, Cubic::new(0.25, 2.0, 0.75, 2.0).unwrap()).unwrap();
    let vector = curve
        .clone()
        .drive(ValueTween::new(egui::vec2(0.0, 0.0), egui::vec2(10.0, 20.0)).unwrap())
        .unwrap();
    assert!(vector.value().x > 10.0);
    let invalid = curve.drive(CurveTween::new(Curve::Linear)).unwrap();
    assert!(invalid.try_value().is_err());
    let nested = invalid
        .drive(ValueTween::new(0.0_f32, 1.0).unwrap())
        .unwrap();
    assert!(nested.try_value().is_err());
}
#[test]
fn controller_frames_and_typed_builders_animate_retained_geometry() {
    let (ctx, p) = controller(0.0);
    let driven = p
        .clone()
        .drive(ValueTween::new(egui::vec2(12.0, 8.0), egui::vec2(24.0, 16.0)).unwrap())
        .unwrap();
    let mut t = ElementTree::new(
        &ctx,
        Widget::value_listenable_builder(driven.clone(), |_, size| Widget::leaf(size, |_| {})),
    )
    .unwrap();
    let c = BoxConstraints::loose(egui::vec2(100.0, 100.0)).unwrap();
    assert_eq!(t.layout(c).unwrap().size, egui::vec2(12.0, 8.0));
    p.forward().unwrap();
    ctx.begin_pass(egui::RawInput {
        time: Some(0.5),
        ..Default::default()
    });
    let _ = ctx.end_pass();
    assert_eq!(t.layout(c).unwrap().size, egui::vec2(18.0, 12.0));
    ctx.begin_pass(egui::RawInput {
        time: Some(1.0),
        ..Default::default()
    });
    let _ = ctx.end_pass();
    assert_eq!(t.layout(c).unwrap().size, egui::vec2(24.0, 16.0));
    t.unmount();
    driven.dispose().unwrap();
    p.dispose().unwrap();
}

#[test]
fn forwarding_links_are_canceled_before_an_owned_parent_is_dropped() {
    struct OwnedParent {
        value: ValueNotifier<f32>,
        closed: Rc<Cell<bool>>,
    }
    impl Drop for OwnedParent {
        fn drop(&mut self) {
            self.closed.set(true);
        }
    }
    impl Listenable for OwnedParent {
        fn subscribe(&self, listener: Listener) -> Result<Subscription, Disposed> {
            let subscription = self.value.subscribe(listener)?;
            let closed = self.closed.clone();
            Ok(Subscription::new(move || {
                assert!(
                    !closed.get(),
                    "parent dropped before its forwarding link was canceled"
                );
                drop(subscription);
            }))
        }
        fn same_source(&self, other: &Self) -> bool {
            self.value.same_source(&other.value)
        }
    }
    impl ValueListenable for OwnedParent {
        type Value = f32;
        fn value(&self) -> f32 {
            self.value.get()
        }
    }
    impl Animation<f32> for OwnedParent {
        fn status(&self) -> AnimationStatus {
            AnimationStatus::Forward
        }
        fn subscribe_status(&self, _: Listener) -> Result<Subscription, Disposed> {
            Ok(Subscription::new(|| {}))
        }
    }
    let closed = Rc::new(Cell::new(false));
    let driven = OwnedParent {
        value: ValueNotifier::new(0.5),
        closed: closed.clone(),
    }
    .drive(ConstantTween(1.0_f32))
    .unwrap();
    let token = driven.subscribe(Rc::new(|| {})).unwrap();
    drop(driven);
    assert!(closed.get());
    drop(token);
}
#[test]
fn checked_scalar_samples_and_new_driven_sources_reject_disposed_parents() {
    let (_, parent) = controller(0.5);
    parent.dispose().unwrap();
    assert_eq!(Animation::try_value(&parent), Err(AnimationError::Disposed));
    assert!(parent.drive(ConstantTween(1.0_f32)).is_err());
}

#[test]
fn callback_mappings_accept_application_values_propagate_errors_and_isolate_panics() {
    let labels = CallbackAnimatable::new(|t| Ok(format!("progress {t:.1}")));
    assert_eq!(labels.transform(0.5).unwrap(), "progress 0.5");
    assert!(labels.transform(f32::NAN).is_err());
    let failed = CallbackAnimatable::<f32>::new(|_| {
        Err(AnimationError::InvalidConfiguration("callback failure"))
    });
    assert_eq!(
        failed.transform(0.5),
        Err(AnimationError::InvalidConfiguration("callback failure"))
    );
    let panicked = CallbackAnimatable::<f32>::new(|_| panic!("callback"));
    assert_eq!(
        panicked.transform(0.5),
        Err(AnimationError::InterpolationPanicked)
    );
    let holder: Rc<RefCell<Option<CallbackAnimatable<f32>>>> = Rc::default();
    let saved = Rc::downgrade(&holder);
    let recursive = CallbackAnimatable::new(move |t| {
        saved
            .upgrade()
            .unwrap()
            .borrow()
            .as_ref()
            .unwrap()
            .transform(t)
    });
    *holder.borrow_mut() = Some(recursive.clone());
    assert_eq!(recursive.transform(0.5), Err(AnimationError::Superseded));
}

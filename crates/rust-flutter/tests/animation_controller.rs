use rust_flutter::{animation::*, egui, element::*, layout::BoxConstraints, notifier::*};
use std::{
    cell::{Cell, RefCell},
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
    time::Duration,
};
fn frame(ctx: &egui::Context, time: f64) -> egui::FullOutput {
    ctx.begin_pass(egui::RawInput {
        time: Some(time),
        ..Default::default()
    });
    ctx.end_pass()
}
fn controller(ctx: &egui::Context) -> AnimationController {
    AnimationController::new(
        ctx,
        AnimationConfig {
            duration: Duration::from_secs(1),
            ..Default::default()
        },
    )
    .unwrap()
}
fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 0.00001, "{a} != {b}");
}
#[test]
fn frame_driver_advances_without_manual_sampling_and_reports_terminal_status() {
    let ctx = egui::Context::default();
    frame(&ctx, 0.0);
    let c = controller(&ctx);
    let values = Rc::new(Cell::new(0));
    let calls = values.clone();
    let _subscription = c
        .subscribe(Rc::new(move || calls.set(calls.get() + 1)))
        .unwrap();
    let statuses = Rc::new(RefCell::new(vec![]));
    let log = statuses.clone();
    let weak = c.downgrade();
    let _status = c
        .subscribe_status(Rc::new(move || {
            log.borrow_mut().push(weak.upgrade().unwrap().status())
        }))
        .unwrap();
    let run = c.forward().unwrap();
    assert_eq!(c.value(), 0.0);
    assert_eq!(c.status(), AnimationStatus::Forward);
    assert!(c.is_animating());
    frame(&ctx, 0.25);
    close(c.value(), 0.25);
    frame(&ctx, 0.25);
    assert_eq!(values.get(), 1);
    frame(&ctx, 1.0);
    assert_eq!(c.value(), 1.0);
    assert!(!c.is_animating());
    assert_eq!(run.outcome(), Some(AnimationOutcome::Completed));
    assert_eq!(
        *statuses.borrow(),
        vec![AnimationStatus::Forward, AnimationStatus::Completed]
    );
}
#[test]
fn reverse_uses_remaining_distance_and_reverse_duration_and_stop_preserves_status() {
    let ctx = egui::Context::default();
    frame(&ctx, 0.0);
    let c = AnimationController::new(
        &ctx,
        AnimationConfig {
            duration: Duration::from_secs(2),
            reverse_duration: Some(Duration::from_secs(4)),
            value: Some(0.5),
            ..Default::default()
        },
    )
    .unwrap();
    let forward = c.forward().unwrap();
    frame(&ctx, 0.5);
    close(c.value(), 0.75);
    let reverse = c.reverse().unwrap();
    assert_eq!(forward.outcome(), Some(AnimationOutcome::Canceled));
    frame(&ctx, 1.5);
    close(c.value(), 0.5);
    c.stop(false).unwrap();
    assert_eq!(reverse.outcome(), Some(AnimationOutcome::Completed));
    assert_eq!(c.status(), AnimationStatus::Reverse);
    assert!(!c.is_animating());
    frame(&ctx, 3.0);
    close(c.value(), 0.5);
    let done = c.reverse().unwrap();
    frame(&ctx, 5.0);
    assert_eq!(c.value(), 0.0);
    assert_eq!(c.status(), AnimationStatus::Dismissed);
    assert_eq!(done.outcome(), Some(AnimationOutcome::Completed));
}
#[test]
fn animate_to_and_back_have_directional_status_independent_of_target_order() {
    let ctx = egui::Context::default();
    frame(&ctx, 0.0);
    let c = controller(&ctx);
    c.set_value(1.0).unwrap();
    let run = c
        .animate_to(0.5, Some(Duration::from_secs(2)), Curve::EaseInOut)
        .unwrap();
    frame(&ctx, 1.0);
    close(c.value(), 0.75);
    assert_eq!(c.status(), AnimationStatus::Forward);
    frame(&ctx, 2.0);
    assert_eq!(run.outcome(), Some(AnimationOutcome::Completed));
    assert_eq!(c.status(), AnimationStatus::Completed);
    let run = c
        .animate_back(0.8, Some(Duration::from_secs(1)), Curve::Linear)
        .unwrap();
    frame(&ctx, 3.0);
    close(c.value(), 0.8);
    assert_eq!(run.outcome(), Some(AnimationOutcome::Completed));
    assert_eq!(c.status(), AnimationStatus::Dismissed);
    c.set_value(0.4).unwrap();
    assert_eq!(c.status(), AnimationStatus::Reverse);
    c.reset().unwrap();
    c.set_value(0.5).unwrap();
    assert_eq!(c.status(), AnimationStatus::Reverse);
}
#[test]
fn repeat_preserves_starting_phase_alternates_and_finishes_at_requested_leg() {
    let ctx = egui::Context::default();
    frame(&ctx, 0.0);
    let c = controller(&ctx);
    c.set_value(0.25).unwrap();
    let run = c
        .repeat(RepeatOptions {
            reverse: true,
            count: Some(2),
            ..Default::default()
        })
        .unwrap();
    frame(&ctx, 0.5);
    close(c.value(), 0.75);
    frame(&ctx, 1.0);
    close(c.value(), 0.75);
    assert_eq!(c.status(), AnimationStatus::Reverse);
    frame(&ctx, 1.75);
    assert_eq!(c.value(), 0.0);
    assert_eq!(c.status(), AnimationStatus::Dismissed);
    assert_eq!(run.outcome(), Some(AnimationOutcome::Completed));
    assert!(!c.is_animating());
    let run = c
        .repeat(RepeatOptions {
            min: Some(0.2),
            max: Some(0.8),
            count: Some(3),
            ..Default::default()
        })
        .unwrap();
    frame(&ctx, 5.0);
    close(c.value(), 0.8);
    assert_eq!(c.status(), AnimationStatus::Completed);
    assert_eq!(run.outcome(), Some(AnimationOutcome::Completed));
}
#[test]
fn muting_suppresses_ticks_but_elapsed_time_is_applied_on_unmute() {
    let ctx = egui::Context::default();
    frame(&ctx, 0.0);
    let c = controller(&ctx);
    let run = c.forward().unwrap();
    frame(&ctx, 0.2);
    close(c.value(), 0.2);
    c.set_muted(true).unwrap();
    frame(&ctx, 2.0);
    close(c.value(), 0.2);
    assert!(c.is_animating());
    assert_eq!(run.outcome(), None);
    c.set_muted(false).unwrap();
    frame(&ctx, 2.1);
    assert_eq!(c.value(), 1.0);
    assert_eq!(run.outcome(), Some(AnimationOutcome::Completed));
}
#[test]
fn immediate_runs_and_set_value_clamp_notify_and_cancel_current_playback() {
    let ctx = egui::Context::default();
    frame(&ctx, 0.0);
    let c = controller(&ctx);
    let calls = Rc::new(Cell::new(0));
    let log = calls.clone();
    let _sub = c
        .subscribe(Rc::new(move || log.set(log.get() + 1)))
        .unwrap();
    c.set_value(-3.0).unwrap();
    c.set_value(0.0).unwrap();
    assert_eq!(calls.get(), 2);
    assert_eq!(c.value(), 0.0);
    let run = c.forward().unwrap();
    c.set_value(2.0).unwrap();
    assert_eq!(run.outcome(), Some(AnimationOutcome::Canceled));
    assert_eq!(c.value(), 1.0);
    let run = c
        .animate_back(0.4, Some(Duration::ZERO), Curve::Linear)
        .unwrap();
    close(c.value(), 0.4);
    assert_eq!(run.outcome(), Some(AnimationOutcome::Completed));
    assert!(!c.is_animating());
    c.reset().unwrap();
    assert_eq!(c.value(), 0.0);
}
#[test]
fn invalid_inputs_leave_existing_run_intact_and_extreme_bounds_interpolate_finitely() {
    let ctx = egui::Context::default();
    frame(&ctx, 0.0);
    let c = controller(&ctx);
    let run = c.forward().unwrap();
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(c.set_value(value).is_err());
        assert!(c.animate_to(value, None, Curve::Linear).is_err());
    }
    for options in [
        RepeatOptions {
            min: Some(2.0),
            ..Default::default()
        },
        RepeatOptions {
            max: Some(-1.0),
            ..Default::default()
        },
        RepeatOptions {
            period: Some(Duration::ZERO),
            ..Default::default()
        },
        RepeatOptions {
            count: Some(0),
            ..Default::default()
        },
    ] {
        assert!(c.repeat(options).is_err());
    }
    assert_eq!(run.outcome(), None);
    frame(&ctx, 1.0);
    assert_eq!(run.outcome(), Some(AnimationOutcome::Completed));
    let c = AnimationController::new(
        &ctx,
        AnimationConfig {
            lower_bound: -f32::MAX,
            upper_bound: f32::MAX,
            duration: Duration::from_secs(2),
            ..Default::default()
        },
    )
    .unwrap();
    c.forward().unwrap();
    frame(&ctx, 2.0);
    assert!(c.value().is_finite());
    close(c.value(), 0.0);
    assert!(
        AnimationController::new(
            &ctx,
            AnimationConfig {
                upper_bound: 0.0,
                ..Default::default()
            }
        )
        .is_err()
    );
}
struct WakeCount(AtomicUsize);
impl Wake for WakeCount {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
#[test]
fn futures_wake_for_completion_cancellation_and_final_controller_drop() {
    let ctx = egui::Context::default();
    frame(&ctx, 0.0);
    let c = controller(&ctx);
    let mut run = c.forward().unwrap();
    let count = Arc::new(WakeCount(AtomicUsize::new(0)));
    let waker = Waker::from(count.clone());
    let mut task = Context::from_waker(&waker);
    assert_eq!(Pin::new(&mut run).poll(&mut task), Poll::Pending);
    assert_eq!(Pin::new(&mut run).poll(&mut task), Poll::Pending);
    frame(&ctx, 1.0);
    assert_eq!(count.0.load(Ordering::SeqCst), 1);
    assert_eq!(
        Pin::new(&mut run).poll(&mut task),
        Poll::Ready(AnimationOutcome::Completed)
    );
    let run = c.reverse().unwrap();
    c.dispose().unwrap();
    assert_eq!(run.outcome(), Some(AnimationOutcome::Canceled));
    assert_eq!(c.forward().err(), Some(AnimationError::Disposed));
    assert!(c.subscribe(Rc::new(|| {})).is_err());
    let c = controller(&ctx);
    let weak = c.downgrade();
    let run = c.forward().unwrap();
    drop(c);
    assert!(weak.upgrade().is_none());
    assert_eq!(run.outcome(), Some(AnimationOutcome::Canceled));
    frame(&ctx, 2.0);
}
#[test]
fn reentrant_listener_can_replace_run_and_disposal_is_rejected_during_dispatch() {
    let ctx = egui::Context::default();
    frame(&ctx, 0.0);
    let c = controller(&ctx);
    let weak = c.downgrade();
    let once = Rc::new(Cell::new(false));
    let flag = once.clone();
    let replacement = Rc::new(RefCell::new(None));
    let saved = replacement.clone();
    let _sub = c
        .subscribe(Rc::new(move || {
            let c = weak.upgrade().unwrap();
            if !flag.replace(true) {
                assert_eq!(c.dispose(), Err(AnimationError::DuringNotification));
                *saved.borrow_mut() = Some(c.reverse().unwrap());
            }
        }))
        .unwrap();
    let first = c.forward().unwrap();
    frame(&ctx, 0.5);
    assert_eq!(first.outcome(), Some(AnimationOutcome::Canceled));
    assert_eq!(c.status(), AnimationStatus::Reverse);
    frame(&ctx, 1.0);
    assert_eq!(c.value(), 0.0);
    assert_eq!(
        replacement.borrow().as_ref().unwrap().outcome(),
        Some(AnimationOutcome::Completed)
    );
    c.dispose().unwrap();
}
#[test]
fn controller_value_listenable_rebuilds_retained_widgets_from_automatic_frame_ticks() {
    let ctx = egui::Context::default();
    frame(&ctx, 0.0);
    let c = controller(&ctx);
    let mut tree = ElementTree::new(
        &ctx,
        Widget::value_listenable_builder(c.clone(), |_, value| {
            Widget::leaf(egui::vec2(20.0 + value * 40.0, 10.0), |_| {})
        }),
    )
    .unwrap();
    let constraints = BoxConstraints::loose(egui::vec2(100.0, 100.0)).unwrap();
    assert_eq!(tree.layout(constraints).unwrap().size.x, 20.0);
    let ids = tree.elements().iter().map(|e| e.id).collect::<Vec<_>>();
    c.forward().unwrap();
    frame(&ctx, 0.5);
    assert_eq!(tree.layout(constraints).unwrap().size.x, 40.0);
    assert_eq!(
        tree.elements().iter().map(|e| e.id).collect::<Vec<_>>(),
        ids
    );
}

#[test]
fn controllers_are_isolated_by_context_and_viewport_and_repeat_can_run_indefinitely() {
    let ctx = egui::Context::default();
    frame(&ctx, 0.0);
    let c = controller(&ctx);
    let run = c.forward().unwrap();
    let other = egui::Context::default();
    frame(&other, 0.0);
    let other_controller = controller(&other);
    other_controller.forward().unwrap();
    frame(&other, 0.5);
    close(other_controller.value(), 0.5);
    assert_eq!(c.value(), 0.0);
    let viewport = egui::ViewportId::from_hash_of("secondary");
    let mut input = egui::RawInput {
        viewport_id: viewport,
        time: Some(0.5),
        ..Default::default()
    };
    input
        .viewports
        .insert(viewport, egui::ViewportInfo::default());
    ctx.begin_pass(input);
    let _ = ctx.end_pass();
    assert_eq!(c.value(), 0.0);
    assert_eq!(run.outcome(), None);
    frame(&ctx, 1.0);
    assert_eq!(c.value(), 1.0);
    c.reset().unwrap();
    let run = c.repeat(RepeatOptions::default()).unwrap();
    frame(&ctx, 3.25);
    close(c.value(), 0.25);
    assert_eq!(run.outcome(), None);
    c.stop(true).unwrap();
    assert_eq!(run.outcome(), Some(AnimationOutcome::Canceled));
}

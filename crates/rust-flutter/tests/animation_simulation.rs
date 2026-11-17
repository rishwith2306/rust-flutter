use rust_flutter::{animation::*, egui, notifier::*, physics::*};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

fn frame(ctx: &egui::Context, time: f64) {
    ctx.begin_pass(egui::RawInput {
        time: Some(time),
        ..Default::default()
    });
    let _ = ctx.end_pass();
}
fn setup() -> (egui::Context, AnimationController) {
    let ctx = egui::Context::default();
    frame(&ctx, 0.0);
    let c = AnimationController::new(&ctx, AnimationConfig::default()).unwrap();
    (ctx, c)
}
struct Model<F: Fn(Duration) -> Result<SimulationSample, PhysicsError>>(F);
impl<F: Fn(Duration) -> Result<SimulationSample, PhysicsError>> Simulation for Model<F> {
    fn sample(&self, t: Duration) -> Result<SimulationSample, PhysicsError> {
        (self.0)(t)
    }
    fn tolerance(&self) -> Tolerance {
        Tolerance::default()
    }
}
fn model(
    f: impl Fn(Duration) -> Result<SimulationSample, PhysicsError> + 'static,
) -> Rc<dyn Simulation> {
    Rc::new(Model(f))
}
fn linear(t: Duration) -> Result<SimulationSample, PhysicsError> {
    Ok(SimulationSample {
        position: t.as_secs_f64(),
        velocity: 1.0,
        done: t >= Duration::from_secs(1),
    })
}
fn close(a: f32, b: f64) {
    assert!((f64::from(a) - b).abs() < 1e-6, "{a} != {b}");
}

#[test]
fn spring_drives_frames_and_completes_with_directional_status() {
    let (ctx, c) = setup();
    let spring = SpringSimulation::with_options(
        SpringDescription {
            mass: 1.0,
            stiffness: 100.0,
            damping: 20.0,
        },
        0.0,
        0.8,
        0.0,
        Tolerance::default(),
        true,
    )
    .unwrap();
    let run = c.animate_with(Rc::new(spring)).unwrap();
    assert_eq!(c.status(), AnimationStatus::Forward);
    frame(&ctx, 0.1);
    close(
        c.value(),
        spring.sample(Duration::from_millis(100)).unwrap().position,
    );
    frame(&ctx, 2.0);
    assert_eq!(c.value(), 0.8);
    assert_eq!(c.status(), AnimationStatus::Completed);
    assert_eq!(run.outcome(), Some(AnimationOutcome::Completed));
    assert!(!c.is_animating());
    let reverse = c
        .animate_back_with(Rc::new(
            SpringSimulation::with_options(
                SpringDescription {
                    mass: 1.0,
                    stiffness: 100.0,
                    damping: 20.0,
                },
                0.8,
                0.2,
                0.0,
                Tolerance::default(),
                true,
            )
            .unwrap(),
        ))
        .unwrap();
    assert_eq!(c.status(), AnimationStatus::Reverse);
    frame(&ctx, 4.0);
    assert_eq!(c.value(), 0.2);
    assert_eq!(c.status(), AnimationStatus::Dismissed);
    assert_eq!(reverse.outcome(), Some(AnimationOutcome::Completed));
}
#[test]
fn clamping_does_not_end_an_unfinished_simulation_or_infer_direction() {
    let (ctx, c) = setup();
    let run = c
        .animate_back_with(model(|t| {
            Ok(SimulationSample {
                position: 2.0 + t.as_secs_f64(),
                velocity: 1.0,
                done: t >= Duration::from_secs(2),
            })
        }))
        .unwrap();
    assert_eq!(c.value(), 1.0);
    assert_eq!(c.status(), AnimationStatus::Reverse);
    frame(&ctx, 1.0);
    assert!(c.is_animating());
    assert_eq!(run.outcome(), None);
    frame(&ctx, 2.0);
    assert_eq!(run.outcome(), Some(AnimationOutcome::Completed));
    assert_eq!(c.status(), AnimationStatus::Dismissed);
}
#[test]
fn initially_done_samples_finish_immediately_and_notify() {
    let (_, c) = setup();
    let calls = Rc::new(Cell::new(0));
    let log = calls.clone();
    let _s = c
        .subscribe(Rc::new(move || log.set(log.get() + 1)))
        .unwrap();
    let run = c
        .animate_with(model(|_| {
            Ok(SimulationSample {
                position: 0.4,
                velocity: 0.0,
                done: true,
            })
        }))
        .unwrap();
    assert_eq!(run.outcome(), Some(AnimationOutcome::Completed));
    assert_eq!(calls.get(), 1);
    assert!(!c.is_animating());
    close(c.value(), 0.4);
}
#[test]
fn rejected_initial_samples_preserve_existing_playback() {
    let (ctx, c) = setup();
    let old = c.forward().unwrap();
    assert!(matches!(
        c.animate_with(model(|_| Err(PhysicsError::NumericalRange))),
        Err(AnimationError::Simulation(PhysicsError::NumericalRange))
    ));
    for (position, velocity) in [(f64::NAN, 0.0), (0.0, f64::INFINITY)] {
        assert!(matches!(
            c.animate_with(model(move |_| Ok(SimulationSample {
                position,
                velocity,
                done: false
            }))),
            Err(AnimationError::Simulation(PhysicsError::NonFinite))
        ));
    }
    assert!(matches!(
        c.animate_with(model(|_| panic!("preflight"))),
        Err(AnimationError::SimulationPanicked)
    ));
    assert_eq!(old.outcome(), None);
    frame(&ctx, 1.0);
    assert_eq!(old.outcome(), Some(AnimationOutcome::Completed));
}
#[test]
fn later_failures_resolve_failed_and_keep_last_finite_value_and_direction() {
    for mode in 0..3 {
        let (ctx, c) = setup();
        let run = c
            .animate_back_with(model(move |t| {
                if t.is_zero() {
                    Ok(SimulationSample {
                        position: 0.6,
                        velocity: 0.0,
                        done: false,
                    })
                } else {
                    match mode {
                        0 => Err(PhysicsError::NumericalRange),
                        1 => panic!("later"),
                        _ => Ok(SimulationSample {
                            position: f64::NAN,
                            velocity: 0.0,
                            done: true,
                        }),
                    }
                }
            }))
            .unwrap();
        frame(&ctx, 0.1);
        assert!(!c.is_animating());
        close(c.value(), 0.6);
        assert_eq!(c.status(), AnimationStatus::Reverse);
        let error = match mode {
            0 => AnimationError::Simulation(PhysicsError::NumericalRange),
            1 => AnimationError::SimulationPanicked,
            _ => AnimationError::Simulation(PhysicsError::NonFinite),
        };
        assert_eq!(run.outcome(), Some(AnimationOutcome::Failed(error)));
        let next = c.forward().unwrap();
        frame(&ctx, 1.0);
        assert_eq!(next.outcome(), Some(AnimationOutcome::Completed));
    }
}
#[test]
fn duplicate_frames_do_not_resample_and_muting_catches_up() {
    let (ctx, c) = setup();
    let times = Rc::new(RefCell::new(vec![]));
    let log = times.clone();
    let run = c
        .animate_with(model(move |t| {
            log.borrow_mut().push(t);
            linear(t)
        }))
        .unwrap();
    frame(&ctx, 0.2);
    frame(&ctx, 0.2);
    assert_eq!(times.borrow().len(), 2);
    c.set_muted(true).unwrap();
    frame(&ctx, 2.0);
    assert_eq!(times.borrow().len(), 2);
    assert_eq!(run.outcome(), None);
    c.set_muted(false).unwrap();
    frame(&ctx, 2.1);
    assert_eq!(run.outcome(), Some(AnimationOutcome::Completed));
    assert_eq!(c.value(), 1.0);
}
#[test]
fn simulation_commands_can_replace_or_stop_playback_without_stale_writes() {
    for stop in [false, true] {
        let (ctx, c) = setup();
        let weak = c.downgrade();
        let run = c
            .animate_with(model(move |t| {
                if !t.is_zero() {
                    let c = weak.upgrade().unwrap();
                    if stop {
                        c.stop(true).unwrap();
                    } else {
                        c.set_value(0.75).unwrap();
                        c.reverse().unwrap();
                    }
                }
                linear(t)
            }))
            .unwrap();
        frame(&ctx, 0.1);
        assert_eq!(run.outcome(), Some(AnimationOutcome::Canceled));
        if stop {
            assert!(!c.is_animating());
            assert_eq!(c.value(), 0.0);
        } else {
            assert!(c.is_animating());
            assert_eq!(c.value(), 0.75);
            assert_eq!(c.status(), AnimationStatus::Reverse);
        }
    }
}
#[test]
fn simulation_can_dispose_its_controller_while_sampling() {
    let (ctx, c) = setup();
    let weak = c.downgrade();
    let run = c
        .animate_with(model(move |t| {
            if !t.is_zero() {
                weak.upgrade().unwrap().dispose().unwrap();
            }
            linear(t)
        }))
        .unwrap();
    frame(&ctx, 0.1);
    assert!(c.is_disposed());
    assert_eq!(c.value(), 0.0);
    assert_eq!(run.outcome(), Some(AnimationOutcome::Canceled));
}
#[test]
fn preflight_reentrant_commands_supersede_outer_start_including_idle_stop() {
    for stop in [false, true] {
        let (_, c) = setup();
        let weak = c.downgrade();
        let result = c.animate_with(model(move |t| {
            let c = weak.upgrade().unwrap();
            if stop {
                c.stop(true).unwrap();
            } else {
                c.set_value(0.7).unwrap();
            }
            linear(t)
        }));
        assert!(matches!(result, Err(AnimationError::Superseded)));
        close(c.value(), if stop { 0.0 } else { 0.7 });
        assert!(!c.is_animating());
    }
}
#[test]
fn sampling_can_start_another_custom_model_and_reject_obsolete_results() {
    let (ctx, c) = setup();
    let weak = c.downgrade();
    let next = Rc::new(RefCell::new(None));
    let log = next.clone();
    let run = c
        .animate_with(model(move |t| {
            if !t.is_zero() {
                *log.borrow_mut() = Some(
                    weak.upgrade()
                        .unwrap()
                        .animate_back_with(model(|_| {
                            Ok(SimulationSample {
                                position: 0.9,
                                velocity: 0.0,
                                done: true,
                            })
                        }))
                        .unwrap(),
                );
            }
            linear(t)
        }))
        .unwrap();
    frame(&ctx, 0.1);
    assert_eq!(c.value(), 0.9);
    assert_eq!(c.status(), AnimationStatus::Dismissed);
    assert_eq!(run.outcome(), Some(AnimationOutcome::Canceled));
    assert_eq!(
        next.borrow().as_ref().unwrap().outcome(),
        Some(AnimationOutcome::Completed)
    );
}
#[test]
fn destructor_reentrancy_runs_outside_borrows_on_completion_and_replacement() {
    struct DropModel(rust_flutter::animation::WeakAnimationController);
    impl Simulation for DropModel {
        fn sample(&self, t: Duration) -> Result<SimulationSample, PhysicsError> {
            linear(t)
        }
        fn tolerance(&self) -> Tolerance {
            Tolerance::default()
        }
    }
    impl Drop for DropModel {
        fn drop(&mut self) {
            self.0.upgrade().unwrap().set_value(0.3).unwrap();
        }
    }
    for complete in [false, true] {
        let (ctx, c) = setup();
        let run = c.animate_with(Rc::new(DropModel(c.downgrade()))).unwrap();
        if complete {
            frame(&ctx, 1.0);
        } else {
            c.forward().unwrap();
        }
        close(c.value(), 0.3);
        assert!(!c.is_animating());
        assert_eq!(
            run.outcome(),
            Some(if complete {
                AnimationOutcome::Completed
            } else {
                AnimationOutcome::Canceled
            })
        );
    }
}
#[test]
fn finite_f64_samples_clamp_before_f32_conversion_and_final_drop_cancels() {
    let (_, c) = setup();
    let run = c
        .animate_with(model(|_| {
            Ok(SimulationSample {
                position: f64::MAX,
                velocity: f64::MAX,
                done: false,
            })
        }))
        .unwrap();
    assert_eq!(c.value(), 1.0);
    drop(c);
    assert_eq!(run.outcome(), Some(AnimationOutcome::Canceled));
}
#[test]
fn errors_from_obsolete_samples_do_not_fail_a_replacement() {
    let (ctx, c) = setup();
    let weak = c.downgrade();
    let next = Rc::new(RefCell::new(None));
    let log = next.clone();
    let run = c
        .animate_with(model(move |t| {
            if !t.is_zero() {
                let c = weak.upgrade().unwrap();
                c.set_value(0.4).unwrap();
                *log.borrow_mut() = Some(c.forward().unwrap());
                return Err(PhysicsError::NumericalRange);
            }
            linear(t)
        }))
        .unwrap();
    frame(&ctx, 0.1);
    assert_eq!(run.outcome(), Some(AnimationOutcome::Canceled));
    assert_eq!(next.borrow().as_ref().unwrap().outcome(), None);
    close(c.value(), 0.4);
}
#[test]
fn notifying_listeners_can_replace_custom_playback() {
    let (ctx, c) = setup();
    let weak = c.downgrade();
    let armed = Rc::new(Cell::new(false));
    let a = armed.clone();
    let _sub = c
        .subscribe(Rc::new(move || {
            if a.replace(false) {
                weak.upgrade().unwrap().set_value(0.8).unwrap();
            }
        }))
        .unwrap();
    let run = c.animate_with(model(linear)).unwrap();
    armed.set(true);
    frame(&ctx, 0.1);
    assert_eq!(run.outcome(), Some(AnimationOutcome::Canceled));
    assert_eq!(c.value(), 0.8);
}

#[test]
fn failed_future_wakes_and_resolves_once() {
    use std::{
        future::Future,
        pin::Pin,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        task::{Context, Poll, Wake, Waker},
    };
    struct Counter(AtomicUsize);
    impl Wake for Counter {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    let (ctx, c) = setup();
    let mut run = c
        .animate_with(model(|t| {
            if t.is_zero() {
                linear(t)
            } else {
                Err(PhysicsError::NumericalRange)
            }
        }))
        .unwrap();
    let count = Arc::new(Counter(AtomicUsize::new(0)));
    let waker = Waker::from(count.clone());
    let mut task = Context::from_waker(&waker);
    assert!(Pin::new(&mut run).poll(&mut task).is_pending());
    frame(&ctx, 0.1);
    assert_eq!(count.0.load(Ordering::SeqCst), 1);
    assert_eq!(
        Pin::new(&mut run).poll(&mut task),
        Poll::Ready(AnimationOutcome::Failed(AnimationError::Simulation(
            PhysicsError::NumericalRange
        )))
    );
    frame(&ctx, 0.2);
    assert_eq!(count.0.load(Ordering::SeqCst), 1);
}

#[test]
fn preflight_disposal_is_reported_without_installing_motion() {
    let (_, c) = setup();
    let weak = c.downgrade();
    assert!(matches!(
        c.animate_with(model(move |t| {
            weak.upgrade().unwrap().dispose().unwrap();
            linear(t)
        })),
        Err(AnimationError::Disposed)
    ));
    assert!(c.is_disposed());
    assert!(!c.is_animating());
}

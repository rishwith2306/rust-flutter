use rust_flutter::{
    egui::{Vec2, pos2, vec2},
    gestures::*,
};
use std::{cell::RefCell, rc::Rc, time::Duration};
fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
fn close(a: f64, b: f64) {
    assert!(
        (a - b).abs() < 0.0001_f64.max(b.abs() * 0.00001),
        "{a} != {b}"
    );
}
fn linear() -> VelocityTracker {
    let mut t = VelocityTracker::new(PointerDeviceKind::Touch);
    for i in 0..9 {
        t.add_position(ms(i * 10), pos2(i as f32 * 10.0, i as f32 * -5.0))
            .unwrap();
    }
    t
}
#[test]
fn constant_two_dimensional_motion_has_correct_units_fit_and_offset() {
    let t = linear();
    let e = t.estimate().unwrap();
    assert_eq!(e.status, VelocityEstimateStatus::Fitted);
    close(e.velocity.pixels_per_second().x as f64, 1000.0);
    close(e.velocity.pixels_per_second().y as f64, -500.0);
    close(e.confidence, 1.0);
    assert_eq!(e.duration, ms(80));
    assert_eq!(e.offset, vec2(80.0, -40.0));
    assert_eq!(e.samples, 9);
    assert_eq!(t.kind(), PointerDeviceKind::Touch);
}
#[test]
fn quadratic_motion_estimates_instantaneous_end_velocity_not_average() {
    let mut t = VelocityTracker::new(PointerDeviceKind::Mouse);
    for i in 0..9 {
        let time = i as f32 * 0.01;
        t.add_position(
            ms(i * 10),
            pos2(1000.0 * time * time, -500.0 * time * time + 300.0 * time),
        )
        .unwrap();
    }
    let e = t.estimate().unwrap();
    close(e.velocity.pixels_per_second().x as f64, 160.0);
    close(e.velocity.pixels_per_second().y as f64, 220.0);
    close(e.confidence, 1.0);
}
#[test]
fn sparse_equal_time_and_stationary_samples_are_explicitly_distinguished() {
    let mut t = VelocityTracker::new(PointerDeviceKind::Stylus);
    assert!(t.estimate().is_none());
    assert_eq!(t.velocity(), Velocity::ZERO);
    t.add_position(ms(0), pos2(0.0, 0.0)).unwrap();
    t.add_position(ms(10), pos2(10.0, 0.0)).unwrap();
    assert_eq!(
        t.estimate().unwrap().status,
        VelocityEstimateStatus::Unavailable
    );
    t.add_position(ms(20), pos2(20.0, 0.0)).unwrap();
    t.add_position(ms(20), pos2(40.0, 0.0)).unwrap();
    assert_eq!(t.len(), 3);
    close(t.velocity().pixels_per_second().x as f64, 4000.0);
    t.clear();
    assert!(t.is_empty());
    for i in 0..4 {
        t.add_position(ms(i * 10), pos2(5.0, 8.0)).unwrap();
    }
    let e = t.estimate().unwrap();
    assert_eq!(e.status, VelocityEstimateStatus::Fitted);
    assert_eq!(e.velocity, Velocity::ZERO);
    assert_eq!(e.confidence, 1.0);
}
#[test]
fn stopping_uses_explicit_query_time_and_gap_boundary() {
    let t = linear();
    assert_eq!(
        t.estimate_at(ms(120)).unwrap().status,
        VelocityEstimateStatus::Fitted
    );
    let e = t.estimate_at(ms(120) + Duration::from_nanos(1)).unwrap();
    assert_eq!(e.status, VelocityEstimateStatus::Stopped);
    assert_eq!(e.velocity, Velocity::ZERO);
    assert_eq!(e.offset, Vec2::ZERO);
    assert_eq!(e.duration, Duration::ZERO);
    assert!(t.estimate_at(ms(79)).is_none());
    assert_eq!(t.estimate().unwrap().samples, 9);
}
#[test]
fn history_capacity_horizon_and_motion_gap_bound_the_used_segment() {
    let mut t = VelocityTracker::with_config(
        PointerDeviceKind::Touch,
        VelocityTrackerConfig {
            history_size: 3,
            horizon: ms(1000),
            stop_gap: ms(1000),
        },
    )
    .unwrap();
    for i in 0..10 {
        t.add_position(ms(i * 10), pos2(i as f32 * 10.0, 0.0))
            .unwrap();
    }
    assert_eq!(t.len(), 3);
    assert_eq!(t.estimate().unwrap().duration, ms(20));
    assert_eq!(t.estimate().unwrap().offset, vec2(20.0, 0.0));
    let mut t = VelocityTracker::with_config(
        PointerDeviceKind::Touch,
        VelocityTrackerConfig {
            history_size: 256,
            ..Default::default()
        },
    )
    .unwrap();
    for i in 0..21 {
        t.add_position(ms(i * 10), pos2(i as f32 * 10.0, 0.0))
            .unwrap();
    }
    assert_eq!(t.len(), 11);
    assert_eq!(t.estimate().unwrap().duration, ms(100));
    t.add_position(ms(241), pos2(500.0, 0.0)).unwrap();
    assert_eq!(t.len(), 1);
    assert_eq!(
        t.estimate().unwrap().status,
        VelocityEstimateStatus::Unavailable
    );
}
#[test]
fn invalid_samples_do_not_mutate_previous_history_or_estimate() {
    let mut t = linear();
    let before = t.estimate();
    assert_eq!(
        t.add_position(ms(90), pos2(f32::NAN, 0.0)),
        Err(VelocityError::InvalidPosition)
    );
    assert_eq!(
        t.add_position(ms(79), pos2(1.0, 0.0)),
        Err(VelocityError::OldTimestamp)
    );
    assert_eq!(t.estimate(), before);
    assert_eq!(t.len(), 9);
}
#[test]
fn large_time_epoch_preserves_small_inter_sample_intervals() {
    let mut t = VelocityTracker::new(PointerDeviceKind::Touch);
    let epoch = Duration::from_secs(u64::MAX - 1);
    for i in 0..9 {
        t.add_position(epoch + ms(i * 10), pos2(i as f32 * 10.0, 0.0))
            .unwrap();
    }
    close(t.velocity().pixels_per_second().x as f64, 1000.0);
    assert_eq!(t.estimate().unwrap().duration, ms(80));
}
#[test]
fn noisy_fit_reports_confidence_without_nonfinite_output() {
    let mut t = VelocityTracker::new(PointerDeviceKind::Unknown);
    for (i, x) in [0.0, 100.0, -100.0, 0.0, 100.0].into_iter().enumerate() {
        t.add_position(ms(i as u64 * 10), pos2(x, 0.0)).unwrap();
    }
    let e = t.estimate().unwrap();
    assert_eq!(e.status, VelocityEstimateStatus::Fitted);
    assert!(e.confidence >= 0.0 && e.confidence < 0.9);
    assert!(e.velocity.pixels_per_second().is_finite());
}
#[test]
fn unrepresentable_geometry_and_degenerate_timing_never_invent_velocity() {
    let mut t = VelocityTracker::new(PointerDeviceKind::Touch);
    for (time, x) in [(0, -f32::MAX), (10, 0.0), (20, f32::MAX)] {
        t.add_position(ms(time), pos2(x, 0.0)).unwrap();
    }
    assert!(t.estimate().is_none());
    assert_eq!(t.velocity(), Velocity::ZERO);
    let mut t = VelocityTracker::with_config(
        PointerDeviceKind::Unknown,
        VelocityTrackerConfig {
            horizon: Duration::MAX,
            stop_gap: Duration::MAX,
            ..Default::default()
        },
    )
    .unwrap();
    for (time, x) in [
        (Duration::ZERO, 0.0),
        (Duration::from_nanos(1), 1.0),
        (Duration::from_secs(u64::MAX - 1), 2.0),
    ] {
        t.add_position(time, pos2(x, 0.0)).unwrap();
    }
    assert_eq!(
        t.estimate().unwrap().status,
        VelocityEstimateStatus::Unavailable
    );
    assert_eq!(t.velocity(), Velocity::ZERO);
}
#[test]
fn magnitude_clamping_preserves_direction_and_handles_extreme_vectors() {
    let v = Velocity::new(vec2(3.0, 4.0)).unwrap();
    close(v.clamp_magnitude(10.0, 20.0).unwrap().speed(), 10.0);
    assert_eq!(v.clamp_magnitude(0.0, 10.0).unwrap(), v);
    assert_eq!(
        Velocity::ZERO.clamp_magnitude(10.0, 20.0).unwrap(),
        Velocity::ZERO
    );
    let v = Velocity::new(vec2(f32::MAX, f32::MAX))
        .unwrap()
        .clamp_magnitude(0.0, 8000.0)
        .unwrap();
    close(v.speed(), 8000.0);
    close(
        v.pixels_per_second().x as f64,
        v.pixels_per_second().y as f64,
    );
    assert!(Velocity::new(vec2(f32::INFINITY, 0.0)).is_err());
    assert!(v.clamp_magnitude(2.0, 1.0).is_err());
    assert!(v.clamp_magnitude(-1.0, 1.0).is_err());
}
#[test]
fn fling_requires_speed_and_recent_distance_and_caps_effective_velocity() {
    let e = linear().estimate().unwrap();
    let fling = FlingConfig {
        max_velocity: 100.0,
        ..Default::default()
    }
    .classify(&e)
    .unwrap();
    assert!(fling.is_fling);
    close(fling.velocity.speed(), 100.0);
    for c in [
        FlingConfig {
            min_distance: 90.0,
            ..Default::default()
        },
        FlingConfig {
            min_velocity: 2000.0,
            ..Default::default()
        },
    ] {
        assert!(!c.classify(&e).unwrap().is_fling);
    }
    let boundary = VelocityEstimate {
        offset: vec2(18.0, 0.0),
        ..e
    };
    assert!(!FlingConfig::default().classify(&boundary).unwrap().is_fling);
    let boundary = VelocityEstimate {
        velocity: Velocity::new(vec2(50.0, 0.0)).unwrap(),
        ..e
    };
    assert!(!FlingConfig::default().classify(&boundary).unwrap().is_fling);
    let mut e = e;
    e.status = VelocityEstimateStatus::Unavailable;
    assert_eq!(
        FlingConfig::default().classify(&e).unwrap().velocity,
        Velocity::ZERO
    );
    e.confidence = f64::NAN;
    assert_eq!(
        FlingConfig::default().classify(&e),
        Err(VelocityError::InvalidEstimate)
    );
}
#[test]
fn invalid_tracker_and_fling_configuration_is_rejected_before_use() {
    for c in [
        VelocityTrackerConfig {
            history_size: 2,
            ..Default::default()
        },
        VelocityTrackerConfig {
            history_size: 257,
            ..Default::default()
        },
        VelocityTrackerConfig {
            horizon: Duration::ZERO,
            ..Default::default()
        },
        VelocityTrackerConfig {
            stop_gap: Duration::ZERO,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            VelocityTracker::with_config(PointerDeviceKind::Touch, c),
            Err(VelocityError::InvalidConfiguration)
        ));
    }
    for c in [
        FlingConfig {
            min_velocity: f32::NAN,
            ..Default::default()
        },
        FlingConfig {
            max_velocity: 1.0,
            ..Default::default()
        },
        FlingConfig {
            min_distance: -1.0,
            ..Default::default()
        },
    ] {
        assert!(c.validate().is_err());
    }
}
fn event(phase: PointerPhase, x: f32, time: u64) -> PointerEvent {
    PointerEvent {
        pointer: PointerId(1),
        phase,
        position: pos2(x, 0.0),
        time: ms(time),
        device: PointerDeviceKind::Touch,
        buttons: if phase == PointerPhase::Up {
            PointerButtons::NONE
        } else {
            PointerButtons::PRIMARY
        },
    }
}
fn pan_fixture() -> (
    GestureArenaManager,
    PointerRouter<PointerEvent>,
    PanGestureRecognizer,
    Rc<RefCell<Vec<DragEndDetails>>>,
) {
    let a = GestureArenaManager::new();
    let r = PointerRouter::new();
    let output = Rc::new(RefCell::new(Vec::new()));
    let o = output.clone();
    let p = PanGestureRecognizer::new(
        &a,
        &r,
        PanConfig::default(),
        PanCallbacks {
            on_end: Some(Rc::new(move |d| o.borrow_mut().push(d))),
            ..Default::default()
        },
    )
    .unwrap();
    (a, r, p, output)
}
fn drive(
    a: &GestureArenaManager,
    r: &PointerRouter<PointerEvent>,
    p: &PanGestureRecognizer,
    up_time: u64,
    up_x: f32,
) {
    p.add_pointer(event(PointerPhase::Down, 0.0, 0)).unwrap();
    a.close(PointerId(1)).unwrap();
    a.flush();
    for i in 1..=8 {
        r.route(
            PointerId(1),
            &event(PointerPhase::Move, i as f32 * 10.0, i * 10),
        );
    }
    r.route(PointerId(1), &event(PointerPhase::Up, up_x, up_time));
    a.sweep(PointerId(1)).unwrap();
    a.flush();
}
#[test]
fn pan_end_exposes_raw_fit_and_effective_fling_without_stationary_up_bias() {
    let (a, r, p, out) = pan_fixture();
    drive(&a, &r, &p, 90, 80.0);
    let d = out.borrow()[0];
    assert!(d.is_fling);
    close(d.velocity.pixels_per_second().x as f64, 1000.0);
    assert_eq!(d.event.phase, PointerPhase::Up);
    assert_eq!(d.estimate.unwrap().duration, ms(80));
    assert_eq!(r.route_count(), 0);
}
#[test]
fn pan_end_after_release_pause_is_zero_and_changed_up_contributes_motion() {
    let (a, r, p, out) = pan_fixture();
    drive(&a, &r, &p, 130, 80.0);
    assert!(!out.borrow()[0].is_fling);
    assert_eq!(
        out.borrow()[0].estimate.unwrap().status,
        VelocityEstimateStatus::Stopped
    );
    drive(&a, &r, &p, 90, 90.0);
    assert!(out.borrow()[1].is_fling);
    close(
        out.borrow()[1].velocity.pixels_per_second().x as f64,
        1000.0,
    );
    assert_eq!(out.borrow()[1].estimate.unwrap().duration, ms(90));
}
#[test]
fn deferred_arena_win_preserves_the_original_release_estimate() {
    struct Other;
    impl GestureArenaMember for Other {
        fn accept_gesture(&self, _: PointerId) {}
        fn reject_gesture(&self, _: PointerId) {}
    }
    let (a, r, p, out) = pan_fixture();
    p.add_pointer(event(PointerPhase::Down, 0.0, 0)).unwrap();
    a.add(PointerId(1), Rc::new(Other)).unwrap();
    for i in 1..=8 {
        r.route(
            PointerId(1),
            &event(PointerPhase::Move, i as f32 * 10.0, i * 10),
        );
    }
    r.route(PointerId(1), &event(PointerPhase::Up, 80.0, 90));
    assert!(out.borrow().is_empty());
    std::thread::sleep(ms(50)); // Deliberately exceed the stop gap before arbitration.
    a.close(PointerId(1)).unwrap();
    assert!(out.borrow()[0].is_fling);
    close(out.borrow()[0].velocity.speed(), 1000.0);
}
#[test]
fn invalid_pan_velocity_settings_do_not_register_or_reset_existing_target() {
    let a = GestureArenaManager::new();
    let r = PointerRouter::new();
    let config = PanConfig {
        velocity: VelocityTrackerConfig {
            history_size: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(matches!(
        PanGestureRecognizer::new(&a, &r, config, PanCallbacks::default()),
        Err(RecognizerError::Velocity(
            VelocityError::InvalidConfiguration
        ))
    ));
    assert_eq!(a.active_pointers(), 0);
    assert_eq!(r.route_count(), 0);
    let b = GestureBinding::new();
    let target = b
        .register_target(
            rust_flutter::egui::Id::new(1),
            GestureTargetConfig::default(),
        )
        .unwrap();
    assert!(
        target
            .update(GestureTargetConfig {
                pan: Some((config, PanCallbacks::default())),
                ..Default::default()
            })
            .is_err()
    );
    assert!(target.is_active());
    assert_eq!(b.target_count(), 1);
}

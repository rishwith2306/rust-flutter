use rust_flutter::physics::*;
use std::time::Duration;

fn time(t: f64) -> Duration {
    Duration::from_secs_f64(t)
}
fn close(a: f64, b: f64, tolerance: f64) {
    assert!(
        (a - b).abs() <= tolerance,
        "{a} != {b}, tolerance {tolerance}"
    );
}
fn spring(ratio: f64) -> SpringSimulation {
    SpringSimulation::new(
        SpringDescription::with_damping_ratio(1.0, 100.0, ratio).unwrap(),
        10.0,
        20.0,
        3.0,
    )
    .unwrap()
}

#[test]
fn exponential_friction_matches_analytic_integral_in_both_directions() {
    for v in [-100.0, 100.0] {
        let motion = FrictionSimulation::new((-2.0_f64).exp(), 7.0, v).unwrap();
        for t in [0.0_f64, 0.01, 0.2, 1.0, 10.0] {
            let s = motion.sample(time(t)).unwrap();
            close(s.velocity, v * (-2.0 * t).exp(), 1e-12);
            close(s.position, 7.0 + v / 2.0 * (1.0 - (-2.0 * t).exp()), 1e-12);
        }
        close(motion.final_position(), 7.0 + v / 2.0, 1e-12);
    }
}
#[test]
fn tiny_time_friction_avoids_cancellation_and_huge_time_remains_finite() {
    let motion = FrictionSimulation::new(1.0 - 1e-10, 0.0, 100.0).unwrap();
    close(
        motion.sample(Duration::from_nanos(1)).unwrap().position,
        1e-7,
        1e-20,
    );
    let final_sample = motion.sample(Duration::MAX).unwrap();
    assert_eq!(final_sample.position, motion.final_position());
    assert_eq!(final_sample.velocity, 0.0);
    assert!(final_sample.done);
}
#[test]
fn friction_tolerance_is_strict_and_done_does_not_snap() {
    let tolerance = Tolerance {
        velocity: 10.0,
        ..Default::default()
    };
    let motion = FrictionSimulation::with_tolerance(0.5, 0.0, 10.0, tolerance).unwrap();
    assert!(!motion.sample(Duration::ZERO).unwrap().done);
    let s = motion.sample(time(1.0)).unwrap();
    assert!(s.done);
    assert_eq!(s.velocity, 5.0);
    assert!(s.position < motion.final_position());
    let rest = FrictionSimulation::new(0.5, 9.0, 0.0).unwrap();
    assert_eq!(
        rest.sample(Duration::MAX).unwrap(),
        SimulationSample {
            position: 9.0,
            velocity: 0.0,
            done: true
        }
    );
}
#[test]
fn bounded_friction_contacts_stop_and_inward_motion_is_allowed() {
    for sign in [-1.0, 1.0] {
        let motion = FrictionSimulation::new(0.5, 0.0, sign * 100.0).unwrap();
        let bounded = BoundedFrictionSimulation::new(motion, -10.0, 10.0).unwrap();
        let mid = bounded.sample(time(0.05)).unwrap();
        assert!(!mid.done);
        assert_eq!(mid.velocity.signum(), sign);
        assert_eq!(
            bounded.sample(time(1.0)).unwrap(),
            SimulationSample {
                position: sign * 10.0,
                velocity: 0.0,
                done: true
            }
        );
    }
    let motion = FrictionSimulation::new(0.5, 10.0, -100.0).unwrap();
    assert!(
        !BoundedFrictionSimulation::new(motion, 0.0, 10.0)
            .unwrap()
            .sample(Duration::ZERO)
            .unwrap()
            .done
    );
    let outward = FrictionSimulation::new(0.5, 10.0, 100.0).unwrap();
    assert!(
        BoundedFrictionSimulation::new(outward, 0.0, 10.0)
            .unwrap()
            .sample(Duration::ZERO)
            .unwrap()
            .done
    );
    let fixed = BoundedFrictionSimulation::new(outward, 10.0, 10.0).unwrap();
    assert_eq!(fixed.sample(Duration::ZERO).unwrap().velocity, 0.0);
}
#[test]
fn friction_and_bounds_reject_invalid_or_unrepresentable_inputs() {
    for drag in [0.0, 1.0, -0.1, 2.0, f64::NAN, f64::INFINITY] {
        assert!(matches!(
            FrictionSimulation::new(drag, 0.0, 1.0),
            Err(PhysicsError::InvalidDrag)
        ));
    }
    assert!(FrictionSimulation::new(0.5, f64::INFINITY, 0.0).is_err());
    assert!(FrictionSimulation::new(0.999, 0.0, f64::MAX).is_err());
    let m = FrictionSimulation::new(0.5, 0.0, 1.0).unwrap();
    for (min, max) in [
        (1.0, 2.0),
        (1.0, -1.0),
        (f64::NAN, 1.0),
        (-1.0, f64::INFINITY),
    ] {
        assert!(BoundedFrictionSimulation::new(m, min, max).is_err());
    }
}
#[test]
fn tolerance_rejects_zero_negative_and_nonfinite_fields() {
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        for field in 0..3 {
            let mut t = Tolerance::default();
            match field {
                0 => t.distance = bad,
                1 => t.velocity = bad,
                _ => t.time = bad,
            }
            assert_eq!(t.validate(), Err(PhysicsError::InvalidTolerance));
            assert!(FrictionSimulation::with_tolerance(0.5, 0.0, 1.0, t).is_err());
        }
    }
}
#[test]
fn undamped_spring_matches_harmonic_motion() {
    let m = SpringSimulation::new(
        SpringDescription {
            mass: 1.0,
            stiffness: 4.0,
            damping: 0.0,
        },
        4.0,
        1.0,
        -2.0,
    )
    .unwrap();
    assert_eq!(m.spring_type(), SpringType::Underdamped);
    for t in [0.0, 0.1, 0.5, 1.0, 5.0] {
        let (sin, cos) = (2.0_f64 * t).sin_cos();
        let s = m.sample(time(t)).unwrap();
        close(s.position, 1.0 + 3.0 * cos - sin, 1e-12);
        close(s.velocity, -6.0 * sin - 2.0 * cos, 1e-12);
        assert!(!s.done);
    }
}
#[test]
fn critical_spring_matches_closed_form() {
    let m = spring(1.0);
    assert_eq!(m.spring_type(), SpringType::Critical);
    for t in [0.0_f64, 0.01, 0.2, 1.0, 10.0] {
        let displacement = (-10.0 - 97.0 * t) * (-10.0 * t).exp();
        let s = m.sample(time(t)).unwrap();
        close(s.position, 20.0 + displacement, 1e-12);
        close(
            s.velocity,
            (-97.0 + 100.0 + 970.0 * t) * (-10.0 * t).exp(),
            1e-12,
        );
    }
}
#[test]
fn damping_regimes_obey_initial_conditions_differential_equation_and_converge() {
    for ratio in [0.1, 0.5, 1.0, 2.0, 100.0] {
        let m = spring(ratio);
        let initial = m.sample(Duration::ZERO).unwrap();
        assert_eq!(initial.position, 10.0);
        assert_eq!(initial.velocity, 3.0);
        let h = 1e-5;
        for t in [0.01, 0.1, 0.5, 2.0] {
            let left = m.sample(time(t - h)).unwrap();
            let s = m.sample(time(t)).unwrap();
            let right = m.sample(time(t + h)).unwrap();
            close(
                (right.position - left.position) / (2.0 * h),
                s.velocity,
                1e-5,
            );
            close(
                (right.velocity - left.velocity) / (2.0 * h),
                -100.0 * (s.position - 20.0) - 20.0 * ratio * s.velocity,
                1e-3,
            );
        }
        let last = m.sample(Duration::MAX).unwrap();
        assert!(last.done);
        assert_eq!(last.position, 20.0);
        assert_eq!(last.velocity, 0.0);
    }
}
#[test]
fn nearly_critical_springs_are_continuous_on_both_sides() {
    let reference = spring(1.0);
    for ratio in [1.0 - 1e-12, 1.0 + 1e-12] {
        let m = spring(ratio);
        for t in [1e-9, 0.01, 0.2, 1.0] {
            let s = m.sample(time(t)).unwrap();
            let r = reference.sample(time(t)).unwrap();
            close(s.position, r.position, 1e-8);
            close(s.velocity, r.velocity, 1e-8);
        }
    }
}
#[test]
fn spring_done_requires_both_position_and_velocity_and_snapping_is_optional() {
    let desc = SpringDescription {
        mass: 1.0,
        stiffness: 100.0,
        damping: 20.0,
    };
    let moving = SpringSimulation::new(desc, 1.0, 1.0, 10.0).unwrap();
    assert!(!moving.sample(Duration::ZERO).unwrap().done);
    let t = Tolerance {
        distance: 0.1,
        velocity: 0.1,
        ..Default::default()
    };
    let raw = SpringSimulation::with_options(desc, 0.0, 1.0, 0.0, t, false).unwrap();
    let snapped = SpringSimulation::with_options(desc, 0.0, 1.0, 0.0, t, true).unwrap();
    let s = raw.sample(time(1.0)).unwrap();
    assert!(s.done && s.position < 1.0 && s.velocity > 0.0);
    assert_eq!(
        snapped.sample(time(1.0)).unwrap(),
        SimulationSample {
            position: 1.0,
            velocity: 0.0,
            done: true
        }
    );
    assert_eq!(
        raw.sample(time(0.1)).unwrap(),
        snapped.sample(time(0.1)).unwrap()
    );
}
#[test]
fn spring_queries_are_pure_and_trait_is_object_safe() {
    let models: Vec<Box<dyn Simulation>> = vec![
        Box::new(spring(0.5)),
        Box::new(spring(2.0)),
        Box::new(FrictionSimulation::new(0.5, 0.0, 100.0).unwrap()),
    ];
    for m in models {
        let before = m.sample(time(0.1)).unwrap();
        m.sample(time(100.0)).unwrap();
        assert_eq!(m.sample(time(0.1)).unwrap(), before);
        assert_eq!(m.tolerance(), Tolerance::default());
    }
}
#[test]
fn spring_rejects_invalid_and_unrepresentable_configuration() {
    for d in [
        SpringDescription {
            mass: 0.0,
            stiffness: 1.0,
            damping: 1.0,
        },
        SpringDescription {
            mass: 1.0,
            stiffness: -1.0,
            damping: 1.0,
        },
        SpringDescription {
            mass: 1.0,
            stiffness: 1.0,
            damping: -1.0,
        },
        SpringDescription {
            mass: f64::NAN,
            stiffness: 1.0,
            damping: 1.0,
        },
        SpringDescription {
            mass: 1e-300,
            stiffness: 1e300,
            damping: 0.0,
        },
    ] {
        assert!(SpringSimulation::new(d, 0.0, 1.0, 0.0).is_err());
    }
    let desc = SpringDescription {
        mass: 1.0,
        stiffness: 1.0,
        damping: 1.0,
    };
    assert!(SpringSimulation::new(desc, f64::MAX, -f64::MAX, 0.0).is_err());
    assert!(SpringSimulation::new(desc, 0.0, 1.0, f64::NAN).is_err());
    for ratio in [-1.0, f64::NAN, f64::INFINITY] {
        assert!(SpringDescription::with_damping_ratio(1.0, 1.0, ratio).is_err());
    }
}
#[test]
fn numerical_failure_never_exposes_nonfinite_geometry() {
    let d = SpringDescription {
        mass: 1.0,
        stiffness: 1e300,
        damping: 0.0,
    };
    let m = SpringSimulation::new(d, 1e200, 0.0, 0.0).unwrap();
    assert_eq!(m.sample(time(0.1)), Err(PhysicsError::NumericalRange));
    assert_eq!(m.sample(Duration::ZERO).unwrap().position, 1e200);
    let extreme = SpringSimulation::new(
        SpringDescription {
            mass: 1.0,
            stiffness: 1.0,
            damping: 1.0,
        },
        1.0,
        1e20,
        0.0,
    )
    .unwrap();
    assert_eq!(extreme.sample(Duration::ZERO).unwrap().position, 1.0);
}

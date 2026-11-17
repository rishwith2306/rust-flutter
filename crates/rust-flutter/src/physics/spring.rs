use super::{PhysicsError, Simulation, SimulationSample, Tolerance};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringDescription {
    pub mass: f64,
    pub stiffness: f64,
    pub damping: f64,
}
impl SpringDescription {
    pub fn with_damping_ratio(mass: f64, stiffness: f64, ratio: f64) -> Result<Self, PhysicsError> {
        if !ratio.is_finite() || ratio < 0.0 {
            return Err(PhysicsError::InvalidSpring);
        }
        let damping = 2.0 * mass.sqrt() * stiffness.sqrt() * ratio;
        let spring = Self {
            mass,
            stiffness,
            damping,
        };
        spring.rates()?;
        Ok(spring)
    }
    fn rates(self) -> Result<(f64, f64), PhysicsError> {
        if !self.mass.is_finite()
            || self.mass <= 0.0
            || !self.stiffness.is_finite()
            || self.stiffness <= 0.0
            || !self.damping.is_finite()
            || self.damping < 0.0
        {
            return Err(PhysicsError::InvalidSpring);
        }
        let omega = (self.stiffness / self.mass).sqrt();
        let alpha = (self.damping / self.mass) / 2.0;
        if !omega.is_finite() || omega == 0.0 || !alpha.is_finite() || !(omega + alpha).is_finite()
        {
            return Err(PhysicsError::NumericalRange);
        }
        Ok((omega, alpha))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpringType {
    Underdamped,
    Critical,
    Overdamped,
}

#[derive(Clone, Copy, Debug)]
enum Solution {
    Under {
        alpha: f64,
        frequency: f64,
        a: f64,
        b: f64,
    },
    Critical {
        alpha: f64,
        a: f64,
        b: f64,
    },
    Over {
        slow: f64,
        fast: f64,
        offset: f64,
        b: f64,
    },
}
/// Closed-form mass/spring/damper solution. Completion requires both displacement
/// and velocity within tolerance. Optional snapping affects only done samples.
#[derive(Clone, Copy, Debug)]
pub struct SpringSimulation {
    solution: Solution,
    start: f64,
    end: f64,
    initial_velocity: f64,
    tolerance: Tolerance,
    snap_to_end: bool,
}
impl SpringSimulation {
    pub fn new(
        spring: SpringDescription,
        start: f64,
        end: f64,
        velocity: f64,
    ) -> Result<Self, PhysicsError> {
        Self::with_options(spring, start, end, velocity, Tolerance::default(), false)
    }
    pub fn with_options(
        spring: SpringDescription,
        start: f64,
        end: f64,
        velocity: f64,
        tolerance: Tolerance,
        snap_to_end: bool,
    ) -> Result<Self, PhysicsError> {
        tolerance.validate()?;
        let (omega, alpha) = spring.rates()?;
        if !start.is_finite() || !end.is_finite() || !velocity.is_finite() {
            return Err(PhysicsError::NonFinite);
        }
        let offset = start - end;
        if !offset.is_finite() {
            return Err(PhysicsError::NumericalRange);
        }
        let (solution, coefficients) = if alpha < omega {
            let frequency = (omega - alpha).sqrt() * (omega + alpha).sqrt();
            let b = (velocity + alpha * offset) / frequency;
            (
                Solution::Under {
                    alpha,
                    frequency,
                    a: offset,
                    b,
                },
                [frequency, b],
            )
        } else if alpha == omega {
            let b = velocity + alpha * offset;
            (
                Solution::Critical {
                    alpha,
                    a: offset,
                    b,
                },
                [alpha, b],
            )
        } else {
            let q = (alpha - omega).sqrt() * (alpha + omega).sqrt();
            let fast = -(alpha + q);
            // Product of roots is omega²; this avoids subtracting nearly equal
            // large alpha and q for a heavily damped spring's slow root.
            let slow = -omega * (omega / (alpha + q));
            if !fast.is_finite() || !slow.is_finite() || slow == 0.0 {
                return Err(PhysicsError::NumericalRange);
            }
            let b = (slow * offset - velocity) / (slow - fast);
            let a = offset - b;
            (
                Solution::Over {
                    slow,
                    fast,
                    offset,
                    b,
                },
                [a, b],
            )
        };
        if coefficients.iter().any(|v| !v.is_finite()) {
            return Err(PhysicsError::NumericalRange);
        }
        Ok(Self {
            solution,
            start,
            end,
            initial_velocity: velocity,
            tolerance,
            snap_to_end,
        })
    }
    pub fn spring_type(&self) -> SpringType {
        match self.solution {
            Solution::Under { .. } => SpringType::Underdamped,
            Solution::Critical { .. } => SpringType::Critical,
            Solution::Over { .. } => SpringType::Overdamped,
        }
    }
}
impl Simulation for SpringSimulation {
    fn sample(&self, elapsed: Duration) -> Result<SimulationSample, PhysicsError> {
        let t = elapsed.as_secs_f64();
        let (offset, velocity) = if elapsed.is_zero() {
            (self.start - self.end, self.initial_velocity)
        } else {
            match self.solution {
                Solution::Under {
                    alpha,
                    frequency,
                    a,
                    b,
                } => {
                    let decay = (-alpha * t).exp();
                    if decay == 0.0 {
                        (0.0, 0.0)
                    } else {
                        let (sin, cos) = (frequency * t).sin_cos();
                        let offset = (decay * a) * cos + (decay * b) * sin;
                        let velocity =
                            frequency * ((decay * b) * cos - (decay * a) * sin) - alpha * offset;
                        (offset, velocity)
                    }
                }
                Solution::Critical { alpha, a, b } => {
                    let decay = (-alpha * t).exp();
                    if decay == 0.0 {
                        (0.0, 0.0)
                    } else {
                        let offset = decay * a + b * (t * decay);
                        (offset, decay * b - alpha * offset)
                    }
                }
                Solution::Over {
                    slow,
                    fast,
                    offset: initial,
                    b,
                } => {
                    let offset = (slow * t).exp() * (initial + b * ((fast - slow) * t).exp_m1());
                    let velocity = slow * offset + (fast - slow) * (b * (fast * t).exp());
                    (offset, velocity)
                }
            }
        };
        let done =
            offset.abs() < self.tolerance.distance && velocity.abs() < self.tolerance.velocity;
        if done && self.snap_to_end {
            Ok(SimulationSample {
                position: self.end,
                velocity: 0.0,
                done: true,
            })
        } else {
            SimulationSample::checked(
                if elapsed.is_zero() {
                    self.start
                } else {
                    self.end + offset
                },
                velocity,
                done,
            )
        }
    }
    fn tolerance(&self) -> Tolerance {
        self.tolerance
    }
}

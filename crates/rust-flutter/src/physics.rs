//! Stateless, checked one-dimensional motion in logical points and seconds.
//!
//! Sampling does not schedule frames or attach motion to a widget. See
//! `docs/physics.md` for supported models and numerical limits.
mod friction;
mod spring;
pub use friction::{BoundedFrictionSimulation, FrictionSimulation};
pub use spring::{SpringDescription, SpringSimulation, SpringType};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicsError {
    InvalidTolerance,
    InvalidDrag,
    InvalidSpring,
    InvalidBounds,
    NonFinite,
    NumericalRange,
}
impl std::fmt::Display for PhysicsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "physics: {self:?}")
    }
}
impl std::error::Error for PhysicsError {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tolerance {
    pub distance: f64,
    pub velocity: f64,
    /// Reserved for time-based convergence policies; these models do not use it.
    pub time: f64,
}
impl Default for Tolerance {
    fn default() -> Self {
        Self {
            distance: 0.001,
            velocity: 0.001,
            time: 0.001,
        }
    }
}
impl Tolerance {
    pub fn validate(self) -> Result<(), PhysicsError> {
        if [self.distance, self.velocity, self.time]
            .iter()
            .all(|v| v.is_finite() && *v > 0.0)
        {
            Ok(())
        } else {
            Err(PhysicsError::InvalidTolerance)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimulationSample {
    pub position: f64,
    pub velocity: f64,
    pub done: bool,
}
impl SimulationSample {
    fn checked(position: f64, velocity: f64, done: bool) -> Result<Self, PhysicsError> {
        if !position.is_finite() || !velocity.is_finite() {
            Err(PhysicsError::NumericalRange)
        } else {
            Ok(Self {
                position,
                velocity,
                done,
            })
        }
    }
}

/// A pure query at elapsed time since the start, independent of previous queries.
/// Duration excludes negative/nonfinite times. Implementations report overflow
/// instead of returning nonfinite geometry. A done sample need not be snapped.
pub trait Simulation {
    fn sample(&self, elapsed: Duration) -> Result<SimulationSample, PhysicsError>;
    fn tolerance(&self) -> Tolerance;
}

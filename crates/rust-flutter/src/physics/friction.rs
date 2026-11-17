use super::{PhysicsError, Simulation, SimulationSample, Tolerance};
use std::time::Duration;

/// Exponential fluid drag: dv/dt = ln(drag) * v, with 0 < drag < 1.
#[derive(Clone, Copy, Debug)]
pub struct FrictionSimulation {
    position: f64,
    velocity: f64,
    rate: f64,
    final_position: f64,
    tolerance: Tolerance,
}
impl FrictionSimulation {
    pub fn new(drag: f64, position: f64, velocity: f64) -> Result<Self, PhysicsError> {
        Self::with_tolerance(drag, position, velocity, Tolerance::default())
    }
    pub fn with_tolerance(
        drag: f64,
        position: f64,
        velocity: f64,
        tolerance: Tolerance,
    ) -> Result<Self, PhysicsError> {
        tolerance.validate()?;
        if !drag.is_finite() || drag <= 0.0 || drag >= 1.0 {
            return Err(PhysicsError::InvalidDrag);
        }
        if !position.is_finite() || !velocity.is_finite() {
            return Err(PhysicsError::NonFinite);
        }
        let rate = -drag.ln();
        let travel = velocity / rate;
        let final_position = position + travel;
        if !travel.is_finite() || !final_position.is_finite() {
            return Err(PhysicsError::NumericalRange);
        }
        Ok(Self {
            position,
            velocity,
            rate,
            final_position,
            tolerance,
        })
    }
    pub fn final_position(&self) -> f64 {
        self.final_position
    }
}
impl Simulation for FrictionSimulation {
    fn sample(&self, elapsed: Duration) -> Result<SimulationSample, PhysicsError> {
        let exponent = -self.rate * elapsed.as_secs_f64();
        // exp_m1 avoids cancellation for small times. Multiplying bounded
        // travel by a [0,1] fraction avoids velocity*time intermediate overflow.
        let position = self.position + (self.velocity / self.rate) * -exponent.exp_m1();
        let velocity = self.velocity * exponent.exp();
        SimulationSample::checked(position, velocity, velocity.abs() < self.tolerance.velocity)
    }
    fn tolerance(&self) -> Tolerance {
        self.tolerance
    }
}

/// Stops on contact with either bound; an inward velocity at a bound can move.
#[derive(Clone, Copy, Debug)]
pub struct BoundedFrictionSimulation {
    motion: FrictionSimulation,
    min: f64,
    max: f64,
}
impl BoundedFrictionSimulation {
    pub fn new(motion: FrictionSimulation, min: f64, max: f64) -> Result<Self, PhysicsError> {
        if !min.is_finite()
            || !max.is_finite()
            || min > max
            || motion.position < min
            || motion.position > max
        {
            return Err(PhysicsError::InvalidBounds);
        }
        Ok(Self { motion, min, max })
    }
}
impl Simulation for BoundedFrictionSimulation {
    fn sample(&self, elapsed: Duration) -> Result<SimulationSample, PhysicsError> {
        let sample = self.motion.sample(elapsed)?;
        if self.min == self.max
            || sample.position < self.min
            || sample.position > self.max
            || (sample.position == self.min && sample.velocity <= 0.0)
            || (sample.position == self.max && sample.velocity >= 0.0)
        {
            return Ok(SimulationSample {
                position: sample.position.clamp(self.min, self.max),
                velocity: 0.0,
                done: true,
            });
        }
        Ok(sample)
    }
    fn tolerance(&self) -> Tolerance {
        self.motion.tolerance()
    }
}

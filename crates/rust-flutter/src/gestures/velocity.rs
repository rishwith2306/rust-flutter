use super::PointerDeviceKind;
use crate::egui::{Pos2, Vec2};
use std::{collections::VecDeque, time::Duration};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VelocityError {
    InvalidConfiguration,
    InvalidPosition,
    OldTimestamp,
    InvalidVelocity,
    InvalidClamp,
    InvalidEstimate,
}
impl std::fmt::Display for VelocityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "velocity: {self:?}")
    }
}
impl std::error::Error for VelocityError {}
/// Finite global logical-point velocity, independent of device pixel density.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Velocity {
    pixels_per_second: Vec2,
}
impl Velocity {
    pub const ZERO: Self = Self {
        pixels_per_second: Vec2::ZERO,
    };
    pub fn new(pixels_per_second: Vec2) -> Result<Self, VelocityError> {
        if !pixels_per_second.is_finite() {
            return Err(VelocityError::InvalidVelocity);
        }
        Ok(Self { pixels_per_second })
    }
    pub fn pixels_per_second(self) -> Vec2 {
        self.pixels_per_second
    }
    pub fn speed(self) -> f64 {
        f64::from(self.pixels_per_second.x).hypot(f64::from(self.pixels_per_second.y))
    }
    /// Preserve direction. A zero vector remains zero even with a nonzero
    /// minimum, since it has no direction. Bounds must be finite and ordered.
    pub fn clamp_magnitude(self, min: f32, max: f32) -> Result<Self, VelocityError> {
        if !min.is_finite() || !max.is_finite() || min < 0.0 || max < min {
            return Err(VelocityError::InvalidClamp);
        }
        let speed = self.speed();
        if speed == 0.0 {
            return Ok(self);
        }
        let target = speed.clamp(f64::from(min), f64::from(max));
        if target == speed {
            return Ok(self);
        }
        let factor = target / speed;
        Self::new(Vec2::new(
            (f64::from(self.pixels_per_second.x) * factor) as f32,
            (f64::from(self.pixels_per_second.y) * factor) as f32,
        ))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VelocityTrackerConfig {
    pub history_size: usize,
    pub horizon: Duration,
    pub stop_gap: Duration,
}
impl Default for VelocityTrackerConfig {
    fn default() -> Self {
        Self {
            history_size: 20,
            horizon: Duration::from_millis(100),
            stop_gap: Duration::from_millis(40),
        }
    }
}
impl VelocityTrackerConfig {
    pub fn validate(self) -> Result<(), VelocityError> {
        if !(3..=256).contains(&self.history_size)
            || self.horizon.is_zero()
            || self.stop_gap.is_zero()
        {
            Err(VelocityError::InvalidConfiguration)
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VelocityEstimateStatus {
    Fitted,
    Unavailable,
    Stopped,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VelocityEstimate {
    pub velocity: Velocity,
    pub confidence: f64,
    pub duration: Duration,
    /// Time from newest recorded motion sample to this query.
    pub age: Duration,
    pub offset: Vec2,
    pub samples: usize,
    pub status: VelocityEstimateStatus,
}
#[derive(Clone, Copy, Debug)]
struct Sample {
    time: Duration,
    position: Pos2,
}
/// Bounded timestamped history. Equal-time samples replace the newest sample;
/// invalid/older samples are rejected without changing history. No wall clock
/// is read. Query explicitly at release time to account for a stationary pause.
#[derive(Clone, Debug)]
pub struct VelocityTracker {
    kind: PointerDeviceKind,
    config: VelocityTrackerConfig,
    samples: VecDeque<Sample>,
}
impl VelocityTracker {
    pub fn new(kind: PointerDeviceKind) -> Self {
        Self {
            kind,
            config: VelocityTrackerConfig::default(),
            samples: VecDeque::with_capacity(20),
        }
    }
    pub fn with_config(
        kind: PointerDeviceKind,
        config: VelocityTrackerConfig,
    ) -> Result<Self, VelocityError> {
        config.validate()?;
        Ok(Self {
            kind,
            config,
            samples: VecDeque::with_capacity(config.history_size),
        })
    }
    pub fn kind(&self) -> PointerDeviceKind {
        self.kind
    }
    pub fn config(&self) -> VelocityTrackerConfig {
        self.config
    }
    pub fn len(&self) -> usize {
        self.samples.len()
    }
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
    pub fn clear(&mut self) {
        self.samples.clear();
    }
    pub fn add_position(&mut self, time: Duration, position: Pos2) -> Result<(), VelocityError> {
        if !position.is_finite() {
            return Err(VelocityError::InvalidPosition);
        }
        if let Some(last) = self.samples.back() {
            if time < last.time {
                return Err(VelocityError::OldTimestamp);
            }
            if time == last.time {
                self.samples.back_mut().unwrap().position = position;
                return Ok(());
            }
            if time - last.time > self.config.stop_gap {
                self.samples.clear();
            }
        }
        self.samples.push_back(Sample { time, position });
        while self.samples.len() > self.config.history_size
            || time - self.samples.front().unwrap().time > self.config.horizon
        {
            self.samples.pop_front();
        }
        Ok(())
    }
    pub fn estimate(&self) -> Option<VelocityEstimate> {
        self.estimate_at(self.samples.back()?.time)
    }
    /// None means empty history, a query before the latest sample, or a result
    /// not representable in finite Vec2 coordinates. Sparse/degenerate fits and
    /// stopped motion return explicit status rather than an invented velocity.
    pub fn estimate_at(&self, now: Duration) -> Option<VelocityEstimate> {
        let newest = self.samples.back()?;
        if now < newest.time {
            return None;
        }
        if now - newest.time > self.config.stop_gap {
            return Some(VelocityEstimate {
                velocity: Velocity::ZERO,
                confidence: 1.0,
                duration: Duration::ZERO,
                age: now - newest.time,
                offset: Vec2::ZERO,
                samples: 0,
                status: VelocityEstimateStatus::Stopped,
            });
        }
        let oldest = self.samples.front()?;
        let offset = finite_vec(
            f64::from(newest.position.x) - f64::from(oldest.position.x),
            f64::from(newest.position.y) - f64::from(oldest.position.y),
        )?;
        let duration = newest.time - oldest.time;
        let mut estimate = VelocityEstimate {
            velocity: Velocity::ZERO,
            confidence: 1.0,
            duration,
            age: now - newest.time,
            offset,
            samples: self.len(),
            status: VelocityEstimateStatus::Unavailable,
        };
        if self.len() < 3 || duration.is_zero() {
            return Some(estimate);
        }
        let span = duration.as_secs_f64();
        // Normalize before fitting; absolute Duration epochs never become f64.
        let times: Vec<_> = self
            .samples
            .iter()
            .map(|s| -(newest.time - s.time).as_secs_f64() / span)
            .collect();
        let Some((q, r)) = qr(&times) else {
            return Some(estimate);
        };
        let x: Vec<_> = self
            .samples
            .iter()
            .map(|s| f64::from(s.position.x) - f64::from(newest.position.x))
            .collect();
        let y: Vec<_> = self
            .samples
            .iter()
            .map(|s| f64::from(s.position.y) - f64::from(newest.position.y))
            .collect();
        let (cx, confidence_x) = fit(&times, &x, &q, &r);
        let (cy, confidence_y) = fit(&times, &y, &q, &r);
        estimate.velocity = Velocity::new(finite_vec(cx[1] / span, cy[1] / span)?).ok()?;
        estimate.confidence = (confidence_x * confidence_y).clamp(0.0, 1.0);
        estimate.status = VelocityEstimateStatus::Fitted;
        Some(estimate)
    }
    pub fn velocity(&self) -> Velocity {
        self.estimate().map_or(Velocity::ZERO, |e| e.velocity)
    }
    pub fn velocity_at(&self, now: Duration) -> Velocity {
        self.estimate_at(now).map_or(Velocity::ZERO, |e| e.velocity)
    }
}
fn finite_vec(x: f64, y: f64) -> Option<Vec2> {
    let vector = Vec2::new(x as f32, y as f32);
    vector.is_finite().then_some(vector)
}
type Matrix = [[f64; 3]; 3];
fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
// Two-pass modified Gram-Schmidt on [1,t,t²]. Query-only allocation is bounded
// by the validated history cap; no normal equations square the condition number.
fn qr(times: &[f64]) -> Option<([Vec<f64>; 3], Matrix)> {
    let mut q: [Vec<f64>; 3] = std::array::from_fn(|_| Vec::new());
    let mut r = [[0.0; 3]; 3];
    for col in 0..3 {
        let mut v: Vec<_> = times
            .iter()
            .map(|t| match col {
                0 => 1.0,
                1 => *t,
                _ => t * t,
            })
            .collect();
        for _ in 0..2 {
            for i in 0..col {
                let projection = dot(&q[i], &v);
                r[i][col] += projection;
                for (v, q) in v.iter_mut().zip(&q[i]) {
                    *v -= projection * q;
                }
            }
        }
        let norm = dot(&v, &v).sqrt();
        if norm <= 1e-12 {
            return None;
        }
        r[col][col] = norm;
        q[col] = v.into_iter().map(|v| v / norm).collect();
    }
    Some((q, r))
}
fn fit(times: &[f64], values: &[f64], q: &[Vec<f64>; 3], r: &Matrix) -> ([f64; 3], f64) {
    let mut coefficients = [0.0; 3];
    for i in (0..3).rev() {
        let other: f64 = ((i + 1)..3).map(|j| r[i][j] * coefficients[j]).sum();
        coefficients[i] = (dot(&q[i], values) - other) / r[i][i];
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let total: f64 = values.iter().map(|v| (v - mean).powi(2)).sum();
    let residual: f64 = times
        .iter()
        .zip(values)
        .map(|(t, y)| (y - ((coefficients[2] * t + coefficients[1]) * t + coefficients[0])).powi(2))
        .sum();
    let confidence = if total == 0.0 {
        1.0
    } else {
        (1.0 - residual / total).clamp(0.0, 1.0)
    };
    (coefficients, confidence)
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlingConfig {
    pub min_velocity: f32,
    pub max_velocity: f32,
    pub min_distance: f32,
}
impl Default for FlingConfig {
    fn default() -> Self {
        Self {
            min_velocity: 50.0,
            max_velocity: 8000.0,
            min_distance: 18.0,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fling {
    pub velocity: Velocity,
    pub is_fling: bool,
}
impl FlingConfig {
    pub fn validate(self) -> Result<(), VelocityError> {
        if !self.min_velocity.is_finite()
            || !self.max_velocity.is_finite()
            || !self.min_distance.is_finite()
            || self.min_velocity < 0.0
            || self.max_velocity < self.min_velocity
            || self.min_distance < 0.0
        {
            Err(VelocityError::InvalidConfiguration)
        } else {
            Ok(())
        }
    }
    pub fn classify(self, estimate: &VelocityEstimate) -> Result<Fling, VelocityError> {
        self.validate()?;
        if !estimate.offset.is_finite()
            || !estimate.confidence.is_finite()
            || !(0.0..=1.0).contains(&estimate.confidence)
        {
            return Err(VelocityError::InvalidEstimate);
        }
        let distance = f64::from(estimate.offset.x).hypot(f64::from(estimate.offset.y));
        let qualifies = estimate.status == VelocityEstimateStatus::Fitted
            && estimate.velocity.speed() > f64::from(self.min_velocity)
            && distance > f64::from(self.min_distance);
        let velocity = if qualifies {
            estimate
                .velocity
                .clamp_magnitude(self.min_velocity, self.max_velocity)?
        } else {
            Velocity::ZERO
        };
        Ok(Fling {
            velocity,
            is_fling: velocity.speed() > 0.0,
        })
    }
}

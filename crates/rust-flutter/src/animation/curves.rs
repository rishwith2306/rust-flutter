use super::{AnimationError, Curve};
use std::rc::Rc;

/// Checked mapping of normalized progress. Outputs may overshoot 0…1, but must
/// be finite and map the two endpoints exactly. User implementations may fail.
pub trait ParametricCurve: 'static {
    fn transform(&self, progress: f32) -> Result<f32, AnimationError>;
}
pub(super) fn progress(t: f32) -> Result<(), AnimationError> {
    if t.is_finite() && (0.0..=1.0).contains(&t) {
        Ok(())
    } else {
        Err(AnimationError::InvalidConfiguration(
            "curve progress must be finite and within 0..1",
        ))
    }
}
impl ParametricCurve for Curve {
    fn transform(&self, t: f32) -> Result<f32, AnimationError> {
        progress(t)?;
        Ok(Curve::transform(*self, t))
    }
}
impl<C: ParametricCurve + ?Sized> ParametricCurve for Rc<C> {
    fn transform(&self, t: f32) -> Result<f32, AnimationError> {
        (**self).transform(t)
    }
}
/// Cubic Bézier easing, solving x(s)=progress before evaluating y(s). The x
/// controls must lie in 0…1; finite y controls may overshoot that interval.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cubic {
    controls: [f32; 4],
}
impl Cubic {
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Result<Self, AnimationError> {
        progress(x1)?;
        progress(x2)?;
        if !y1.is_finite() || !y2.is_finite() {
            return Err(AnimationError::InvalidConfiguration(
                "cubic controls must be finite",
            ));
        }
        Ok(Self {
            controls: [x1, y1, x2, y2],
        })
    }
    pub fn control_points(&self) -> [f32; 4] {
        self.controls
    }
    pub fn transform(&self, t: f32) -> Result<f32, AnimationError> {
        <Self as ParametricCurve>::transform(self, t)
    }
}
impl ParametricCurve for Cubic {
    fn transform(&self, t: f32) -> Result<f32, AnimationError> {
        progress(t)?;
        if t == 0.0 || t == 1.0 {
            return Ok(t);
        }
        fn bezier(s: f64, a: f32, b: f32) -> f64 {
            let u = 1.0 - s;
            3.0 * u * u * s * f64::from(a) + 3.0 * u * s * s * f64::from(b) + s * s * s
        }
        let [x1, y1, x2, y2] = self.controls;
        let mut lo = 0.0;
        let mut hi = 1.0;
        // Bisection remains stable at flat derivatives and crossed x controls.
        // f64 intermediates also avoid overflow for finite extreme y controls.
        for _ in 0..128 {
            let s = (lo + hi) * 0.5;
            if s == lo || s == hi {
                return Ok(bezier(s, y1, y2) as f32);
            }
            let x = bezier(s, x1, x2);
            if x == f64::from(t) {
                return Ok(bezier(s, y1, y2) as f32);
            }
            if x < f64::from(t) {
                lo = s;
            } else {
                hi = s;
            }
        }
        Ok(bezier((lo + hi) * 0.5, y1, y2) as f32)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct FlippedCurve<C>(pub C);
impl<C: ParametricCurve> ParametricCurve for FlippedCurve<C> {
    fn transform(&self, t: f32) -> Result<f32, AnimationError> {
        progress(t)?;
        let value = self.0.transform(1.0 - t)?;
        if !value.is_finite() {
            return Err(AnimationError::InvalidConfiguration(
                "curve output must be finite",
            ));
        }
        Ok(1.0 - value)
    }
}

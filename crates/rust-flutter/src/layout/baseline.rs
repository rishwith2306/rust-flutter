use super::*;
/// Baseline coordinate system. These are declared metrics, not inferred font data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextBaseline {
    Alphabetic,
    Ideographic,
}
/// Real baseline distances from a box's top. None means no real baseline.
/// Finite distances outside the box are allowed, including negative values.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Baselines {
    pub alphabetic: Option<f32>,
    pub ideographic: Option<f32>,
}
impl Baselines {
    pub const NONE: Self = Self {
        alphabetic: None,
        ideographic: None,
    };
    pub fn get(self, kind: TextBaseline) -> Option<f32> {
        match kind {
            TextBaseline::Alphabetic => self.alphabetic,
            TextBaseline::Ideographic => self.ideographic,
        }
    }
    pub fn with(mut self, kind: TextBaseline, value: Option<f32>) -> Self {
        match kind {
            TextBaseline::Alphabetic => self.alphabetic = value,
            TextBaseline::Ideographic => self.ideographic = value,
        };
        self
    }
    pub(super) fn validate(self) -> Result<(), LayoutError> {
        if self
            .alphabetic
            .iter()
            .chain(self.ideographic.iter())
            .any(|v| !v.is_finite())
        {
            Err(LayoutError::InvalidConfiguration(
                "baseline distances must be finite",
            ))
        } else {
            Ok(())
        }
    }
}
impl LayoutBox {
    pub fn baseline(&self, kind: TextBaseline) -> Option<f32> {
        self.baselines.get(kind)
    }
    /// Set only_real to false to use the box's bottom when no real baseline exists.
    pub fn distance_to_baseline(&self, kind: TextBaseline, only_real: bool) -> Option<f32> {
        self.baseline(kind)
            .or_else(|| (!only_real).then_some(self.size.y))
    }
    pub(super) fn inherit_baselines(&mut self, highest: bool) -> Result<(), LayoutError> {
        let mut baselines = Baselines::NONE;
        for kind in [TextBaseline::Alphabetic, TextBaseline::Ideographic] {
            let mut value: Option<f32> = None;
            for child in &self.children {
                if let Some(distance) = child.layout.baseline(kind) {
                    let distance = distance + child.offset.y;
                    if !distance.is_finite() {
                        return Err(LayoutError::InvalidConfiguration(
                            "translated baseline must be finite",
                        ));
                    }
                    value =
                        Some(value.map_or(
                            distance,
                            |old| if highest { old.min(distance) } else { old },
                        ));
                }
            }
            baselines = baselines.with(kind, value);
        }
        self.baselines = baselines;
        Ok(())
    }
}
impl BoxNode {
    pub fn dry_baseline(
        &self,
        c: BoxConstraints,
        kind: TextBaseline,
    ) -> Result<Option<f32>, LayoutError> {
        self.validate(&mut HashSet::new(), 0)?;
        self.compute_dry_baseline(c, kind)
    }
    pub(super) fn compute_dry_baseline(
        &self,
        c: BoxConstraints,
        kind: TextBaseline,
    ) -> Result<Option<f32>, LayoutError> {
        let result = match &self.kind {
            BoxKind::Leaf { .. } | BoxKind::Measured(_) => None,
            BoxKind::BaselineLeaf { baselines, .. } => {
                baselines.validate()?;
                baselines.get(kind)
            }
            BoxKind::Custom { layout, children } => layout.dry_baseline(c, kind, children)?,
            BoxKind::IgnoreBaseline { .. } => None,
            BoxKind::Baseline {
                distance,
                baseline,
                child,
            } => {
                validate_target(*distance)?;
                if let Some(requested) = child.compute_dry_baseline(c.loosen(), kind)? {
                    let aligned = match child.compute_dry_baseline(c.loosen(), *baseline)? {
                        Some(real) => real,
                        None => child.compute_dry_layout(c.loosen())?.y,
                    };
                    Some(requested + (distance - aligned))
                } else {
                    None
                }
            }
            BoxKind::Padding { insets, child } => child
                .compute_dry_baseline(c.deflate(*insets)?, kind)?
                .map(|b| b + insets.top),
            BoxKind::Constrained { constraints, child } => {
                child.compute_dry_baseline(constraints.enforce(c), kind)?
            }
            BoxKind::IntrinsicWidth {
                step_width,
                step_height,
                child,
            } => child.compute_dry_baseline(
                query::intrinsic_constraints(c, child, Some((*step_width, *step_height)))?,
                kind,
            )?,
            BoxKind::IntrinsicHeight { child } => {
                child.compute_dry_baseline(query::intrinsic_constraints(c, child, None)?, kind)?
            }
            BoxKind::Align {
                alignment, child, ..
            } => {
                let baseline = child.compute_dry_baseline(c.loosen(), kind)?;
                if let Some(b) = baseline {
                    let size = self.compute_dry_layout(c)?;
                    let child_size = child.compute_dry_layout(c.loosen())?;
                    Some(b + alignment.offset(size - child_size)?.y)
                } else {
                    None
                }
            }
            BoxKind::Flex(flex) => flex.dry_geometry(c, Some(kind))?.baseline(kind),
            BoxKind::Stack(stack) => stack.dry_baseline(c, kind)?,
        };
        if result.is_some_and(|v| !v.is_finite()) {
            Err(LayoutError::InvalidConfiguration(
                "dry baseline must be finite",
            ))
        } else {
            Ok(result)
        }
    }
}
pub(super) fn validate_target(distance: f32) -> Result<(), LayoutError> {
    if distance.is_finite() {
        Ok(())
    } else {
        Err(LayoutError::InvalidConfiguration(
            "baseline target must be finite",
        ))
    }
}
pub(super) fn baseline_size(
    c: BoxConstraints,
    size: Vec2,
    baseline: Option<f32>,
    target: f32,
) -> Result<(Vec2, Vec2), LayoutError> {
    validate_target(target)?;
    let top = target - baseline.unwrap_or(size.y);
    if !top.is_finite() {
        return Err(LayoutError::InvalidConfiguration(
            "baseline child offset must be finite",
        ));
    }
    let height = top + size.y;
    if !height.is_finite() {
        return Err(LayoutError::UnboundedSize);
    }
    Ok((
        c.constrain(egui::vec2(size.x, height.max(0.0)))?,
        egui::vec2(0.0, top),
    ))
}

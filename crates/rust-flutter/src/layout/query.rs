//! Read-only size protocols. These never fall back to a mutable layout callback.
use super::*;
use egui::vec2;

/// Content contribution to query. Width queries take a height argument; height
/// queries take a width. The opposite extent may be infinite.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IntrinsicDimension {
    MinWidth,
    MaxWidth,
    MinHeight,
    MaxHeight,
}
impl IntrinsicDimension {
    pub(crate) fn axis(self) -> Axis {
        match self {
            Self::MinWidth | Self::MaxWidth => Axis::Horizontal,
            _ => Axis::Vertical,
        }
    }
    pub(crate) fn for_axis(self, axis: Axis) -> Self {
        match (self, axis) {
            (Self::MinWidth | Self::MinHeight, Axis::Horizontal) => Self::MinWidth,
            (Self::MinWidth | Self::MinHeight, Axis::Vertical) => Self::MinHeight,
            (_, Axis::Horizontal) => Self::MaxWidth,
            (_, Axis::Vertical) => Self::MaxHeight,
        }
    }
}
/// Immutable access to child dry layout and intrinsic sizes. Querying a child
/// does not count as laying it out or placing it in the wet BoxChildren protocol.
pub struct BoxQueries<'a> {
    nodes: &'a [BoxNode],
}
impl<'a> BoxQueries<'a> {
    pub(super) fn new(nodes: &'a [BoxNode]) -> Self {
        Self { nodes }
    }
    fn node(&self, index: usize) -> Result<&BoxNode, LayoutError> {
        self.nodes
            .get(index)
            .ok_or(LayoutError::InvalidConfiguration(
                "size-query child index is out of bounds",
            ))
    }
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
    pub fn id(&self, index: usize) -> Result<Id, LayoutError> {
        Ok(self.node(index)?.id)
    }
    pub fn dry_layout(&self, index: usize, c: BoxConstraints) -> Result<Vec2, LayoutError> {
        self.node(index)?.compute_dry_layout(c)
    }
    pub fn dry_baseline(
        &self,
        index: usize,
        c: BoxConstraints,
        kind: TextBaseline,
    ) -> Result<Option<f32>, LayoutError> {
        self.node(index)?.compute_dry_baseline(c, kind)
    }
    pub fn intrinsic(
        &self,
        index: usize,
        dimension: IntrinsicDimension,
        extent: f32,
    ) -> Result<f32, LayoutError> {
        self.node(index)?.compute_intrinsic(dimension, extent)
    }
}
fn factors(width: Option<f32>, height: Option<f32>) -> Result<(), LayoutError> {
    if width
        .iter()
        .chain(height.iter())
        .any(|v| !v.is_finite() || *v < 0.0)
    {
        Err(LayoutError::InvalidConfiguration(
            "alignment size factors must be finite and nonnegative",
        ))
    } else {
        Ok(())
    }
}
pub(super) fn intrinsic_argument(extent: f32) -> Result<(), LayoutError> {
    if extent.is_nan() || extent < 0.0 {
        Err(LayoutError::InvalidConfiguration(
            "intrinsic extent must be nonnegative and not NaN",
        ))
    } else {
        Ok(())
    }
}
pub(super) fn snapped(value: f32, step: Option<f32>) -> Result<f32, LayoutError> {
    if step.is_some_and(|s| !s.is_finite() || s <= 0.0) {
        return Err(LayoutError::InvalidConfiguration(
            "intrinsic steps must be finite and positive",
        ));
    }
    let value = step.map_or(value, |step| {
        ((f64::from(value) / f64::from(step)).ceil() * f64::from(step)) as f32
    });
    if value.is_finite() {
        Ok(value)
    } else {
        Err(LayoutError::InvalidSize)
    }
}
pub(super) fn intrinsic_constraints(
    c: BoxConstraints,
    child: &BoxNode,
    steps: Option<(Option<f32>, Option<f32>)>,
) -> Result<BoxConstraints, LayoutError> {
    let (width, height) = if let Some((sw, sh)) = steps {
        snapped(0.0, sw)?;
        snapped(0.0, sh)?;
        (
            if c.min.x == c.max.x {
                None
            } else {
                Some(snapped(
                    child.compute_intrinsic(IntrinsicDimension::MaxWidth, c.max.y)?,
                    sw,
                )?)
            },
            if sh.is_none() {
                None
            } else {
                Some(snapped(
                    child.compute_intrinsic(IntrinsicDimension::MaxHeight, c.max.x)?,
                    sh,
                )?)
            },
        )
    } else {
        (
            None,
            if c.min.y == c.max.y {
                None
            } else {
                Some(child.compute_intrinsic(IntrinsicDimension::MaxHeight, c.max.x)?)
            },
        )
    };
    Ok(BoxConstraints::tight_for(width, height)?.enforce(c))
}
impl BoxNode {
    /// Read-only desired size. Missing pure callbacks return UnsupportedQuery.
    pub fn dry_layout(&self, c: BoxConstraints) -> Result<Vec2, LayoutError> {
        self.validate(&mut HashSet::new(), 0)?;
        self.compute_dry_layout(c)
    }
    /// Query a content contribution without performing layout or painting.
    pub fn intrinsic(
        &self,
        dimension: IntrinsicDimension,
        extent: f32,
    ) -> Result<f32, LayoutError> {
        self.validate(&mut HashSet::new(), 0)?;
        self.compute_intrinsic(dimension, extent)
    }
    pub fn min_intrinsic_width(&self, height: f32) -> Result<f32, LayoutError> {
        self.intrinsic(IntrinsicDimension::MinWidth, height)
    }
    pub fn max_intrinsic_width(&self, height: f32) -> Result<f32, LayoutError> {
        self.intrinsic(IntrinsicDimension::MaxWidth, height)
    }
    pub fn min_intrinsic_height(&self, width: f32) -> Result<f32, LayoutError> {
        self.intrinsic(IntrinsicDimension::MinHeight, width)
    }
    pub fn max_intrinsic_height(&self, width: f32) -> Result<f32, LayoutError> {
        self.intrinsic(IntrinsicDimension::MaxHeight, width)
    }
    pub(super) fn compute_dry_layout(&self, c: BoxConstraints) -> Result<Vec2, LayoutError> {
        let size = match &self.kind {
            BoxKind::BaselineLeaf {
                preferred,
                baselines,
            } => {
                baselines.validate()?;
                c.constrain(*preferred)?
            }
            BoxKind::Baseline {
                distance,
                baseline,
                child,
            } => {
                baseline::validate_target(*distance)?;
                let size = child.compute_dry_layout(c.loosen())?;
                baseline::baseline_size(
                    c,
                    size,
                    child.compute_dry_baseline(c.loosen(), *baseline)?,
                    *distance,
                )?
                .0
            }
            BoxKind::IgnoreBaseline { child } => child.compute_dry_layout(c)?,
            BoxKind::Leaf { preferred } => c.constrain(*preferred)?,
            BoxKind::Measured(_) => {
                return Err(LayoutError::UnsupportedQuery(
                    "measured leaf has no dry-layout callback",
                ));
            }
            BoxKind::Custom { layout, children } => layout.dry_layout(c, children)?,
            BoxKind::Padding { insets, child } => {
                c.constrain(child.compute_dry_layout(c.deflate(*insets)?)? + insets.total())?
            }
            BoxKind::IntrinsicWidth {
                step_width,
                step_height,
                child,
            } => child.compute_dry_layout(intrinsic_constraints(
                c,
                child,
                Some((*step_width, *step_height)),
            )?)?,
            BoxKind::IntrinsicHeight { child } => {
                child.compute_dry_layout(intrinsic_constraints(c, child, None)?)?
            }
            BoxKind::Constrained { constraints, child } => {
                child.compute_dry_layout(constraints.enforce(c))?
            }
            BoxKind::Align {
                alignment,
                width_factor,
                height_factor,
                child,
            } => {
                factors(*width_factor, *height_factor)?;
                let child_size = child.compute_dry_layout(c.loosen())?;
                let size = c.constrain(vec2(
                    width_factor.map_or(
                        if c.max.x.is_finite() {
                            c.max.x
                        } else {
                            child_size.x
                        },
                        |f| child_size.x * f,
                    ),
                    height_factor.map_or(
                        if c.max.y.is_finite() {
                            c.max.y
                        } else {
                            child_size.y
                        },
                        |f| child_size.y * f,
                    ),
                ))?;
                alignment.offset(size - child_size)?;
                size
            }
            BoxKind::Flex(flex) => flex.dry_layout(c)?,
            BoxKind::Stack(stack) => stack.dry_layout(c)?,
        };
        if !c.contains(size) {
            Err(LayoutError::ChildViolatedConstraints(self.id))
        } else {
            Ok(size)
        }
    }
    pub(super) fn compute_intrinsic(
        &self,
        d: IntrinsicDimension,
        extent: f32,
    ) -> Result<f32, LayoutError> {
        intrinsic_argument(extent)?;
        let axis = d.axis();
        let value = match &self.kind {
            BoxKind::Baseline { child, .. } | BoxKind::IgnoreBaseline { child } => {
                child.compute_intrinsic(d, extent)?
            }
            BoxKind::BaselineLeaf { preferred, .. } | BoxKind::Leaf { preferred } => {
                if !nonnegative(*preferred) {
                    return Err(LayoutError::InvalidSize);
                }
                axis.main(*preferred)
            }
            BoxKind::Measured(_) => {
                return Err(LayoutError::UnsupportedQuery(
                    "measured leaf has no intrinsic-size callback",
                ));
            }
            BoxKind::Custom { layout, children } => layout.intrinsic(d, extent, children)?,
            BoxKind::Padding { insets, child } => {
                insets.validate()?;
                child.compute_intrinsic(d, (extent - axis.cross(insets.total())).max(0.0))?
                    + axis.main(insets.total())
            }
            BoxKind::IntrinsicWidth {
                step_width,
                step_height,
                child,
            } => {
                snapped(0.0, *step_width)?;
                snapped(0.0, *step_height)?;
                if axis == Axis::Horizontal {
                    snapped(
                        child.compute_intrinsic(IntrinsicDimension::MaxWidth, extent)?,
                        *step_width,
                    )?
                } else {
                    let width = if extent.is_finite() {
                        extent
                    } else {
                        snapped(
                            child.compute_intrinsic(IntrinsicDimension::MaxWidth, f32::INFINITY)?,
                            *step_width,
                        )?
                    };
                    snapped(child.compute_intrinsic(d, width)?, *step_height)?
                }
            }
            BoxKind::IntrinsicHeight { child } => {
                if axis == Axis::Vertical {
                    child.compute_intrinsic(IntrinsicDimension::MaxHeight, extent)?
                } else {
                    let height = if extent.is_finite() {
                        extent
                    } else {
                        child.compute_intrinsic(IntrinsicDimension::MaxHeight, f32::INFINITY)?
                    };
                    child.compute_intrinsic(d, height)?
                }
            }
            BoxKind::Constrained { constraints, child } => {
                let min = axis.main(constraints.min);
                let max = axis.main(constraints.max);
                if min == max && min.is_finite() {
                    min
                } else {
                    let value = child.compute_intrinsic(d, extent)?;
                    if min.is_finite() {
                        value.clamp(min, max)
                    } else {
                        value
                    }
                }
            }
            BoxKind::Align {
                width_factor,
                height_factor,
                child,
                ..
            } => {
                factors(*width_factor, *height_factor)?;
                child.compute_intrinsic(d, extent)?
                    * if axis == Axis::Horizontal {
                        width_factor.unwrap_or(1.0)
                    } else {
                        height_factor.unwrap_or(1.0)
                    }
            }
            BoxKind::Flex(flex) => flex.intrinsic(d, extent)?,
            BoxKind::Stack(stack) => {
                let mut value = 0.0_f32;
                for child in &stack.children {
                    if !child.position.is_positioned() {
                        value = value.max(child.node.compute_intrinsic(d, extent)?);
                    }
                }
                value
            }
        };
        if !value.is_finite() || value < 0.0 {
            Err(LayoutError::InvalidConfiguration(
                "intrinsic result must be finite and nonnegative",
            ))
        } else {
            Ok(value)
        }
    }
}

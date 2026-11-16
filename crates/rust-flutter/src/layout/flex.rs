use super::*;
#[derive(Clone, Copy)]
struct FlexSpec {
    axis: Axis,
    main_size: MainAxisSize,
    main_alignment: MainAxisAlignment,
    cross_alignment: CrossAxisAlignment,
    text_direction: TextDirection,
    vertical_direction: VerticalDirection,
    spacing: f32,
}
impl Flex {
    fn spec(&self) -> FlexSpec {
        FlexSpec {
            axis: self.axis,
            main_size: self.main_size,
            main_alignment: self.main_alignment,
            cross_alignment: self.cross_alignment,
            text_direction: self.text_direction,
            vertical_direction: self.vertical_direction,
            spacing: self.spacing,
        }
    }
    pub(super) fn layout(
        &mut self,
        c: BoxConstraints,
    ) -> Result<(Vec2, Vec<PlacedBox>), LayoutError> {
        let spec = self.spec();
        let sizing: Vec<_> = self.children.iter().map(|child| child.sizing).collect();
        spec.layout(&sizing, c, |index, c| {
            self.children[index].node.perform_layout(c)
        })
    }
    pub(super) fn dry_layout(&self, c: BoxConstraints) -> Result<Vec2, LayoutError> {
        Ok(self.dry_geometry(c, None)?.size)
    }
    pub(super) fn dry_geometry(
        &self,
        c: BoxConstraints,
        query: Option<TextBaseline>,
    ) -> Result<LayoutBox, LayoutError> {
        let sizing: Vec<_> = self.children.iter().map(|child| child.sizing).collect();
        let align = match (self.axis, self.cross_alignment) {
            (Axis::Horizontal, CrossAxisAlignment::Baseline(kind)) => Some(kind),
            _ => None,
        };
        let (size, children) = self.spec().layout(&sizing, c, |index, c| {
            let node = &self.children[index].node;
            let mut layout = LayoutBox::new(
                node.id,
                node.compute_dry_layout(c)?,
                vec![],
                Clip::None,
                true,
            )?;
            for kind in [TextBaseline::Alphabetic, TextBaseline::Ideographic] {
                if query == Some(kind) || align == Some(kind) {
                    layout.baselines = layout
                        .baselines
                        .with(kind, node.compute_dry_baseline(c, kind)?);
                }
            }
            Ok(layout)
        })?;
        let mut geometry = LayoutBox::new(
            Id::new("dry flex geometry"),
            size,
            children,
            Clip::None,
            false,
        )?;
        geometry.inherit_baselines(self.axis == Axis::Horizontal)?;
        Ok(geometry)
    }
}
impl FlexSpec {
    fn layout(
        self,
        sizing: &[FlexSizing],
        c: BoxConstraints,
        mut measure: impl FnMut(usize, BoxConstraints) -> Result<LayoutBox, LayoutError>,
    ) -> Result<(Vec2, Vec<PlacedBox>), LayoutError> {
        if !self.spacing.is_finite() || self.spacing < 0.0 {
            return Err(LayoutError::InvalidConfiguration(
                "flex spacing must be finite and nonnegative",
            ));
        }
        let axis = self.axis;
        let max_main = axis.main(c.max);
        let max_cross = axis.cross(c.max);
        let mut factors = 0_u64;
        let mut tight_flex = false;
        for child_sizing in sizing {
            if let FlexSizing::Flexible { factor, fit } = *child_sizing {
                if factor == 0 {
                    return Err(LayoutError::InvalidConfiguration(
                        "flex factors must be greater than zero",
                    ));
                }
                factors += u64::from(factor);
                tight_flex |= fit == FlexFit::Tight;
            }
        }
        if !max_main.is_finite()
            && factors > 0
            && (self.main_size == MainAxisSize::Max || tight_flex)
        {
            return Err(LayoutError::UnboundedFlex);
        }
        if self.cross_alignment == CrossAxisAlignment::Stretch
            && !max_cross.is_finite()
            && !sizing.is_empty()
        {
            return Err(LayoutError::UnboundedStretch);
        }
        let cross_min = if self.cross_alignment == CrossAxisAlignment::Stretch {
            max_cross
        } else {
            0.0
        };
        let child_constraints =
            |min, max| BoxConstraints::new(axis.size(min, cross_min), axis.size(max, max_cross));
        let mut layouts: Vec<Option<LayoutBox>> = (0..sizing.len()).map(|_| None).collect();
        let mut used = self.spacing * (sizing.len().saturating_sub(1) as f32);
        let mut cross = 0.0_f32;
        for (index, child_sizing) in sizing.iter().enumerate() {
            if matches!(*child_sizing, FlexSizing::Fixed) {
                let layout = measure(index, child_constraints(0.0, f32::INFINITY)?)?;
                used += axis.main(layout.size);
                cross = cross.max(axis.cross(layout.size));
                layouts[index] = Some(layout);
            }
        }
        let remaining = (max_main - used).max(0.0);
        for (index, child_sizing) in sizing.iter().enumerate() {
            if let FlexSizing::Flexible { factor, fit } = *child_sizing {
                let allocated = if max_main.is_finite() {
                    remaining * (factor as f64 / factors as f64) as f32
                } else {
                    f32::INFINITY
                };
                let layout = measure(
                    index,
                    child_constraints(
                        if fit == FlexFit::Tight {
                            allocated
                        } else {
                            0.0
                        },
                        allocated,
                    )?,
                )?;
                used += axis.main(layout.size);
                cross = cross.max(axis.cross(layout.size));
                layouts[index] = Some(layout);
            }
        }
        let baseline_kind = match (axis, self.cross_alignment) {
            (Axis::Horizontal, CrossAxisAlignment::Baseline(kind)) => Some(kind),
            _ => None,
        };
        let mut group: Option<(f32, f32)> = None;
        if let Some(kind) = baseline_kind {
            for layout in layouts.iter().flatten() {
                if let Some(ascent) = layout.baseline(kind) {
                    let descent = layout.size.y - ascent;
                    group = Some(
                        group.map_or((ascent, descent), |(a, d)| (a.max(ascent), d.max(descent))),
                    );
                }
            }
        }
        if let Some((ascent, descent)) = group {
            if !(ascent + descent).is_finite() {
                return Err(LayoutError::InvalidConfiguration(
                    "baseline flex extent must be finite",
                ));
            }
            cross = cross.max(ascent + descent);
        }
        let desired = axis.size(
            if self.main_size == MainAxisSize::Max && max_main.is_finite() {
                max_main
            } else {
                used
            },
            cross,
        );
        let size = c.constrain(desired)?;
        let free = (axis.main(size) - used).max(0.0);
        let n = sizing.len();
        let (leading, between) = match self.main_alignment {
            MainAxisAlignment::Start => (0.0, 0.0),
            MainAxisAlignment::End => (free, 0.0),
            MainAxisAlignment::Center => (free / 2.0, 0.0),
            MainAxisAlignment::SpaceBetween if n > 1 => (0.0, free / (n - 1) as f32),
            MainAxisAlignment::SpaceAround if n > 0 => (free / n as f32 / 2.0, free / n as f32),
            MainAxisAlignment::SpaceEvenly => (free / (n + 1) as f32, free / (n + 1) as f32),
            _ => (0.0, 0.0),
        };
        let main_flip = if axis == Axis::Horizontal {
            self.text_direction == TextDirection::Rtl
        } else {
            self.vertical_direction == VerticalDirection::Up
        };
        let cross_flip = if axis == Axis::Horizontal {
            self.vertical_direction == VerticalDirection::Up
        } else {
            self.text_direction == TextDirection::Rtl
        };
        let mut cursor = leading;
        let mut placed = Vec::new();
        for layout in layouts.into_iter().flatten() {
            let main = if main_flip {
                axis.main(size) - cursor - axis.main(layout.size)
            } else {
                cursor
            };
            let cross_free = axis.cross(size) - axis.cross(layout.size);
            let cross = match self.cross_alignment {
                CrossAxisAlignment::Start => {
                    if cross_flip {
                        cross_free
                    } else {
                        0.0
                    }
                }
                CrossAxisAlignment::End => {
                    if cross_flip {
                        0.0
                    } else {
                        cross_free
                    }
                }
                CrossAxisAlignment::Center => cross_free / 2.0,
                CrossAxisAlignment::Stretch => 0.0,
                CrossAxisAlignment::Baseline(_) => {
                    match (group, baseline_kind.and_then(|kind| layout.baseline(kind))) {
                        (Some((ascent, _)), Some(distance)) => ascent - distance,
                        _ => 0.0,
                    }
                }
            };
            cursor += axis.main(layout.size) + self.spacing + between;
            let offset = axis.size(main, cross);
            if !offset.is_finite() {
                return Err(LayoutError::UnboundedSize);
            }
            placed.push(PlacedBox { offset, layout });
        }
        Ok((size, placed))
    }
}

impl Flex {
    pub(super) fn intrinsic(&self, d: IntrinsicDimension, extent: f32) -> Result<f32, LayoutError> {
        if !self.spacing.is_finite() || self.spacing < 0.0 {
            return Err(LayoutError::InvalidConfiguration(
                "flex spacing must be finite and nonnegative",
            ));
        }
        let mut factors = 0_u64;
        for child in &self.children {
            if let FlexSizing::Flexible { factor, .. } = child.sizing {
                if factor == 0 {
                    return Err(LayoutError::InvalidConfiguration(
                        "flex factors must be greater than zero",
                    ));
                }
                factors += u64::from(factor);
            }
        }
        let gaps = self.spacing * self.children.len().saturating_sub(1) as f32;
        if d.axis() == self.axis {
            let mut fixed = gaps;
            let mut unit = 0.0_f32;
            for child in &self.children {
                let value = child.node.compute_intrinsic(d, extent)?;
                match child.sizing {
                    FlexSizing::Fixed => fixed += value,
                    FlexSizing::Flexible { factor, .. } => unit = unit.max(value / factor as f32),
                }
            }
            Ok(fixed + unit * factors as f32)
        } else {
            // Determine main-axis allocation from max-content contributions,
            // then ask each child for the selected cross-axis contribution.
            let main_query = IntrinsicDimension::MaxWidth.for_axis(self.axis);
            let mut main: Vec<Option<f32>> = vec![None; self.children.len()];
            let mut used = gaps;
            for (i, child) in self.children.iter().enumerate() {
                if matches!(child.sizing, FlexSizing::Fixed) {
                    let size = child.node.compute_intrinsic(main_query, f32::INFINITY)?;
                    main[i] = Some(size);
                    used += size;
                }
            }
            let mut cross = 0.0_f32;
            let mut group: Option<(f32, f32)> = None;
            for (i, child) in self.children.iter().enumerate() {
                let main = match (main[i], child.sizing) {
                    (Some(size), _) => size,
                    (_, FlexSizing::Flexible { factor, .. }) if extent.is_finite() => {
                        (extent - used).max(0.0) * (factor as f64 / factors as f64) as f32
                    }
                    _ => child.node.compute_intrinsic(main_query, f32::INFINITY)?,
                };
                let child_cross = child.node.compute_intrinsic(d, main)?;
                cross = cross.max(child_cross);
                if let (Axis::Horizontal, CrossAxisAlignment::Baseline(kind)) =
                    (self.axis, self.cross_alignment)
                {
                    let (min_main, max_main) = match child.sizing {
                        FlexSizing::Flexible { fit, .. } if extent.is_finite() => {
                            (if fit == FlexFit::Tight { main } else { 0.0 }, main)
                        }
                        _ => (0.0, f32::INFINITY),
                    };
                    let constraints = BoxConstraints::new(
                        self.axis.size(min_main, 0.0),
                        self.axis.size(max_main, f32::INFINITY),
                    )?;
                    if let Some(ascent) = child.node.compute_dry_baseline(constraints, kind)? {
                        let descent = child_cross - ascent;
                        group =
                            Some(group.map_or((ascent, descent), |(a, d)| {
                                (a.max(ascent), d.max(descent))
                            }));
                    }
                }
            }
            if let Some((ascent, descent)) = group {
                cross = cross.max(ascent + descent);
            }
            Ok(cross)
        }
    }
}

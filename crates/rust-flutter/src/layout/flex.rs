use super::*;
impl Flex {
    pub(super) fn layout(
        &mut self,
        c: BoxConstraints,
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
        for child in &self.children {
            if let FlexSizing::Flexible { factor, fit } = child.sizing {
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
            && !self.children.is_empty()
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
        let mut layouts: Vec<Option<LayoutBox>> = (0..self.children.len()).map(|_| None).collect();
        let mut used = self.spacing * (self.children.len().saturating_sub(1) as f32);
        let mut cross = 0.0_f32;
        for (index, child) in self.children.iter_mut().enumerate() {
            if matches!(child.sizing, FlexSizing::Fixed) {
                let layout = child
                    .node
                    .perform_layout(child_constraints(0.0, f32::INFINITY)?)?;
                used += axis.main(layout.size);
                cross = cross.max(axis.cross(layout.size));
                layouts[index] = Some(layout);
            }
        }
        let remaining = (max_main - used).max(0.0);
        for (index, child) in self.children.iter_mut().enumerate() {
            if let FlexSizing::Flexible { factor, fit } = child.sizing {
                let allocated = if max_main.is_finite() {
                    remaining * (factor as f64 / factors as f64) as f32
                } else {
                    f32::INFINITY
                };
                let layout = child.node.perform_layout(child_constraints(
                    if fit == FlexFit::Tight {
                        allocated
                    } else {
                        0.0
                    },
                    allocated,
                )?)?;
                used += axis.main(layout.size);
                cross = cross.max(axis.cross(layout.size));
                layouts[index] = Some(layout);
            }
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
        let n = self.children.len();
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

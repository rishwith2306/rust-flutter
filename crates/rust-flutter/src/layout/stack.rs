use super::*;
impl Stack {
    pub(super) fn layout(
        &mut self,
        c: BoxConstraints,
    ) -> Result<(Vec2, Vec<PlacedBox>), LayoutError> {
        let child_c = match self.fit {
            StackFit::Loose => c.loosen(),
            StackFit::Expand => BoxConstraints::tight(finite(c.max)?)?,
            StackFit::Passthrough => c,
        };
        let mut layouts: Vec<Option<LayoutBox>> = (0..self.children.len()).map(|_| None).collect();
        let mut natural = c.min;
        let mut count = 0;
        for (index, child) in self.children.iter_mut().enumerate() {
            if !child.position.is_positioned() {
                let layout = child.node.perform_layout(child_c)?;
                natural = natural.max(layout.size);
                layouts[index] = Some(layout);
                count += 1;
            }
        }
        let size = if count == 0 {
            finite(c.max)?
        } else {
            c.constrain(natural)?
        };
        let mut placed = Vec::new();
        for (index, child) in self.children.iter_mut().enumerate() {
            let p = child.position;
            let (layout, offset) = if let Some(layout) = layouts[index].take() {
                let offset = self.alignment.offset(size - layout.size)?;
                (layout, offset)
            } else {
                let child_constraints = positioned_constraints(size, p)?;
                let layout = child.node.perform_layout(child_constraints)?;
                let offset = positioned_offset(self.alignment, size, p, layout.size)?;
                (layout, offset)
            };
            placed.push(PlacedBox { layout, offset });
        }
        Ok((size, placed))
    }
}

impl Stack {
    pub(super) fn dry_layout(&self, c: BoxConstraints) -> Result<Vec2, LayoutError> {
        self.alignment.offset(Vec2::ZERO)?;
        let child_c = match self.fit {
            StackFit::Loose => c.loosen(),
            StackFit::Expand => BoxConstraints::tight(finite(c.max)?)?,
            StackFit::Passthrough => c,
        };
        let mut size = c.min;
        let mut any = false;
        for child in &self.children {
            if !child.position.is_positioned() {
                size = size.max(child.node.compute_dry_layout(child_c)?);
                any = true;
            }
        }
        if any {
            c.constrain(size)
        } else {
            finite(c.max)
        }
    }
}

fn positioned_constraints(size: Vec2, p: Positioned) -> Result<BoxConstraints, LayoutError> {
    if [p.left, p.right, p.top, p.bottom, p.width, p.height]
        .iter()
        .flatten()
        .any(|n| !n.is_finite())
        || p.width.is_some_and(|n| n < 0.0)
        || p.height.is_some_and(|n| n < 0.0)
    {
        return Err(LayoutError::InvalidConfiguration(
            "positioned edges must be finite, sizes must also be nonnegative",
        ));
    }
    if (p.left.is_some() && p.right.is_some() && p.width.is_some())
        || (p.top.is_some() && p.bottom.is_some() && p.height.is_some())
    {
        return Err(LayoutError::InvalidConfiguration(
            "a positioned axis accepts at most two of start, end and size",
        ));
    }
    let width = p
        .width
        .or_else(|| p.left.zip(p.right).map(|(a, b)| (size.x - a - b).max(0.0)));
    let height = p
        .height
        .or_else(|| p.top.zip(p.bottom).map(|(a, b)| (size.y - a - b).max(0.0)));
    BoxConstraints::tight_for(width, height)
}
fn positioned_offset(
    alignment: Alignment,
    size: Vec2,
    p: Positioned,
    child_size: Vec2,
) -> Result<Vec2, LayoutError> {
    let fallback = alignment.offset(size - child_size)?;
    let offset = egui::vec2(
        p.left
            .unwrap_or_else(|| p.right.map_or(fallback.x, |r| size.x - r - child_size.x)),
        p.top
            .unwrap_or_else(|| p.bottom.map_or(fallback.y, |b| size.y - b - child_size.y)),
    );
    if !offset.is_finite() {
        return Err(LayoutError::UnboundedSize);
    }
    Ok(offset)
}
impl Stack {
    pub(super) fn dry_baseline(
        &self,
        c: BoxConstraints,
        kind: TextBaseline,
    ) -> Result<Option<f32>, LayoutError> {
        let size = self.dry_layout(c)?;
        let non_positioned = match self.fit {
            StackFit::Loose => c.loosen(),
            StackFit::Expand => BoxConstraints::tight(finite(c.max)?)?,
            StackFit::Passthrough => c,
        };
        let mut result: Option<f32> = None;
        for child in &self.children {
            let p = child.position;
            let child_c = if p.is_positioned() {
                positioned_constraints(size, p)?
            } else {
                non_positioned
            };
            if let Some(baseline) = child.node.compute_dry_baseline(child_c, kind)? {
                let y = if let Some(top) = p.top {
                    top
                } else {
                    let child_size = child.node.compute_dry_layout(child_c)?;
                    if p.is_positioned() {
                        positioned_offset(self.alignment, size, p, child_size)?.y
                    } else {
                        self.alignment.offset(size - child_size)?.y
                    }
                };
                let value = y + baseline;
                if !value.is_finite() {
                    return Err(LayoutError::InvalidConfiguration(
                        "translated baseline must be finite",
                    ));
                }
                result = Some(result.map_or(value, |old| old.min(value)));
            }
        }
        Ok(result)
    }
}

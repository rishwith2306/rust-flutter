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
                let layout = child
                    .node
                    .perform_layout(BoxConstraints::tight_for(width, height)?)?;
                let fallback = self.alignment.offset(size - layout.size)?;
                let offset = egui::vec2(
                    p.left.unwrap_or_else(|| {
                        p.right.map_or(fallback.x, |r| size.x - r - layout.size.x)
                    }),
                    p.top.unwrap_or_else(|| {
                        p.bottom.map_or(fallback.y, |b| size.y - b - layout.size.y)
                    }),
                );
                if !offset.is_finite() {
                    return Err(LayoutError::UnboundedSize);
                }
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

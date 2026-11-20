use super::{RenderBox, RenderContext, RenderObjectWidget, Widget};
use crate::{
    egui::{self, Vec2},
    layout::*,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WrapAlignment {
    #[default]
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}
impl WrapAlignment {
    fn distribute(self, free: f64, spacing: f64, count: usize, reversed: bool) -> (f64, f64) {
        let start = if reversed { free } else { 0.0 };
        match self {
            Self::Start => (start, spacing),
            Self::End => (free - start, spacing),
            Self::Center => (free / 2.0, spacing),
            Self::SpaceBetween if count > 1 => (0.0, spacing + free / (count - 1) as f64),
            Self::SpaceBetween => (start, spacing),
            Self::SpaceAround => (free / count as f64 / 2.0, spacing + free / count as f64),
            Self::SpaceEvenly => (
                free / (count + 1) as f64,
                spacing + free / (count + 1) as f64,
            ),
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WrapCrossAlignment {
    #[default]
    Start,
    End,
    Center,
}
/// Eager multi-child layout that starts a new run when the main axis is full.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wrap {
    pub direction: Axis,
    pub alignment: WrapAlignment,
    pub spacing: f32,
    pub run_alignment: WrapAlignment,
    pub run_spacing: f32,
    pub cross_axis_alignment: WrapCrossAlignment,
    pub text_direction: TextDirection,
    pub vertical_direction: VerticalDirection,
    pub clip: Clip,
}
impl Default for Wrap {
    fn default() -> Self {
        Self {
            direction: Axis::Horizontal,
            alignment: WrapAlignment::Start,
            spacing: 0.0,
            run_alignment: WrapAlignment::Start,
            run_spacing: 0.0,
            cross_axis_alignment: WrapCrossAlignment::Start,
            text_direction: TextDirection::Ltr,
            vertical_direction: VerticalDirection::Down,
            clip: Clip::None,
        }
    }
}
struct Run {
    start: usize,
    end: usize,
    main: f64,
    cross: f64,
}
struct Plan {
    overflow: bool,
    size: Vec2,
    offsets: Vec<Vec2>,
}
impl Wrap {
    pub fn new(direction: Axis) -> Self {
        Self {
            direction,
            ..Self::default()
        }
    }
    pub fn into_widget(self, children: Vec<Widget>) -> Widget {
        Widget::render_object(self, children)
    }
    fn validate(self) -> Result<(), LayoutError> {
        if !self.spacing.is_finite() || !self.run_spacing.is_finite() {
            Err(LayoutError::InvalidConfiguration(
                "wrap spacing must be finite",
            ))
        } else {
            Ok(())
        }
    }
    fn main(self, size: Vec2) -> f64 {
        f64::from(if self.direction == Axis::Horizontal {
            size.x
        } else {
            size.y
        })
    }
    fn cross(self, size: Vec2) -> f64 {
        f64::from(if self.direction == Axis::Horizontal {
            size.y
        } else {
            size.x
        })
    }
    fn axes(self, main: f64, cross: f64) -> Vec2 {
        if self.direction == Axis::Horizontal {
            egui::vec2(main as f32, cross as f32)
        } else {
            egui::vec2(cross as f32, main as f32)
        }
    }
    fn child_constraints(self, c: BoxConstraints) -> Result<BoxConstraints, LayoutError> {
        BoxConstraints::loose(self.axes(self.main(c.max()), f64::INFINITY))
    }
    fn plan(self, c: BoxConstraints, sizes: &[Vec2]) -> Result<Plan, LayoutError> {
        self.validate()?;
        if sizes.is_empty() {
            return Ok(Plan {
                overflow: false,
                size: c.constrain(c.min())?,
                offsets: vec![],
            });
        }
        let spacing = f64::from(self.spacing);
        let run_spacing = f64::from(self.run_spacing);
        let mut runs: Vec<Run> = vec![];
        for (i, &size) in sizes.iter().enumerate() {
            let main = self.main(size);
            let cross = self.cross(size);
            if !size.is_finite() || main < 0.0 || cross < 0.0 {
                return Err(LayoutError::InvalidSize);
            }
            if let Some(run) = runs.last_mut()
                && run.main + spacing + main <= self.main(c.max())
            {
                run.end = i + 1;
                run.main += spacing + main;
                run.cross = run.cross.max(cross);
            } else {
                runs.push(Run {
                    start: i,
                    end: i + 1,
                    main,
                    cross,
                });
            }
        }
        if runs.iter().any(|run| run.main < 0.0) {
            return Err(LayoutError::InvalidSize);
        }
        let content_main = runs.iter().map(|r| r.main).fold(0.0, f64::max);
        let content_cross =
            runs.iter().map(|r| r.cross).sum::<f64>() + run_spacing * (runs.len() - 1) as f64;
        let natural = self.axes(content_main, content_cross);
        if !natural.is_finite() {
            return Err(LayoutError::UnboundedSize);
        }
        let size = c.constrain(natural)?;
        let overflow = content_main > self.main(size) || content_cross > self.cross(size);
        let horizontal_reverse = self.text_direction == TextDirection::Rtl;
        let vertical_reverse = self.vertical_direction == VerticalDirection::Up;
        let (reverse_main, reverse_cross) = if self.direction == Axis::Horizontal {
            (horizontal_reverse, vertical_reverse)
        } else {
            (vertical_reverse, horizontal_reverse)
        };
        let free_cross = (self.cross(size) - content_cross).max(0.0);
        let (mut cross_offset, run_gap) =
            self.run_alignment
                .distribute(free_cross, run_spacing, runs.len(), reverse_cross);
        let mut offsets = vec![Vec2::ZERO; sizes.len()];
        for index in 0..runs.len() {
            let run = &runs[if reverse_cross {
                runs.len() - 1 - index
            } else {
                index
            }];
            let free_main = (self.main(size) - run.main).max(0.0);
            let (mut main_offset, gap) =
                self.alignment
                    .distribute(free_main, spacing, run.end - run.start, reverse_main);
            for index in 0..run.end - run.start {
                let child = if reverse_main {
                    run.end - 1 - index
                } else {
                    run.start + index
                };
                let remaining = run.cross - self.cross(sizes[child]);
                let alignment = match self.cross_axis_alignment {
                    WrapCrossAlignment::Center => 0.5,
                    WrapCrossAlignment::Start => {
                        if reverse_cross {
                            1.0
                        } else {
                            0.0
                        }
                    }
                    WrapCrossAlignment::End => {
                        if reverse_cross {
                            0.0
                        } else {
                            1.0
                        }
                    }
                };
                let offset = self.axes(main_offset, cross_offset + remaining * alignment);
                if !offset.is_finite() {
                    return Err(LayoutError::InvalidSize);
                }
                offsets[child] = offset;
                main_offset += self.main(sizes[child]) + gap;
            }
            cross_offset += run.cross + run_gap;
        }
        Ok(Plan {
            size,
            offsets,
            overflow,
        })
    }
    fn dry_plan(self, c: BoxConstraints, children: &BoxQueries<'_>) -> Result<Plan, LayoutError> {
        self.validate()?;
        let inner = self.child_constraints(c)?;
        let sizes = (0..children.len())
            .map(|i| children.dry_layout(i, inner))
            .collect::<Result<Vec<_>, _>>()?;
        self.plan(c, &sizes)
    }
}
impl Widget {
    pub fn wrap(style: Wrap, children: Vec<Widget>) -> Self {
        style.into_widget(children)
    }
}
impl RenderObjectWidget for Wrap {
    type Object = Self;
    fn create_render_object(&self, _: &RenderContext) -> Self {
        *self
    }
    fn update_render_object(&self, object: &mut Self, _: &RenderContext) {
        *object = *self;
    }
}
impl RenderBox for Wrap {
    fn dry_layout(
        &self,
        c: BoxConstraints,
        children: &BoxQueries<'_>,
    ) -> Result<Vec2, LayoutError> {
        Ok(self.dry_plan(c, children)?.size)
    }
    fn intrinsic(
        &self,
        d: IntrinsicDimension,
        extent: f32,
        children: &BoxQueries<'_>,
    ) -> Result<f32, LayoutError> {
        self.validate()?;
        let width = matches!(
            d,
            IntrinsicDimension::MinWidth | IntrinsicDimension::MaxWidth
        );
        let main = width == (self.direction == Axis::Horizontal);
        if main {
            let minimum = matches!(
                d,
                IntrinsicDimension::MinWidth | IntrinsicDimension::MinHeight
            );
            let mut result = 0.0_f64;
            for i in 0..children.len() {
                let value = f64::from(children.intrinsic(i, d, f32::INFINITY)?);
                result = if minimum {
                    result.max(value)
                } else {
                    result + value
                };
            }
            let result = result as f32;
            if result.is_finite() {
                Ok(result)
            } else {
                Err(LayoutError::InvalidSize)
            }
        } else {
            let c = BoxConstraints::loose(self.axes(f64::from(extent), f64::INFINITY))?;
            Ok(self.cross(self.dry_plan(c, children)?.size) as f32)
        }
    }
    fn dry_baseline(
        &self,
        c: BoxConstraints,
        kind: TextBaseline,
        children: &BoxQueries<'_>,
    ) -> Result<Option<f32>, LayoutError> {
        let plan = self.dry_plan(c, children)?;
        let inner = self.child_constraints(c)?;
        let mut result: Option<f32> = None;
        for (i, offset) in plan.offsets.iter().enumerate() {
            if let Some(b) = children.dry_baseline(i, inner, kind)? {
                let b = b + offset.y;
                if !b.is_finite() {
                    return Err(LayoutError::InvalidSize);
                }
                result = Some(result.map_or(b, |old| old.min(b)));
            }
        }
        Ok(result)
    }
    fn layout(
        &mut self,
        c: BoxConstraints,
        children: &mut BoxChildren<'_>,
        _: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        self.validate()?;
        let inner = self.child_constraints(c)?;
        let sizes = (0..children.len())
            .map(|i| children.layout(i, inner))
            .collect::<Result<Vec<_>, _>>()?;
        let plan = self.plan(c, &sizes)?;
        let mut geometry = BoxGeometry::new(plan.size);
        geometry.clip = if plan.overflow { self.clip } else { Clip::None };
        for (i, offset) in plan.offsets.into_iter().enumerate() {
            for kind in [TextBaseline::Alphabetic, TextBaseline::Ideographic] {
                if let Some(b) = children.baseline(i, kind, true)? {
                    let b = b + offset.y;
                    if !b.is_finite() {
                        return Err(LayoutError::InvalidSize);
                    }
                    let old = geometry.baselines.get(kind);
                    geometry.baselines = geometry
                        .baselines
                        .with(kind, Some(old.map_or(b, |old| old.min(b))));
                }
            }
            children.place(i, offset)?;
        }
        Ok(geometry)
    }
}

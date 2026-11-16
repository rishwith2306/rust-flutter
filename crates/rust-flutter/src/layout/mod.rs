//! Constraint-based box layout independent of egui's immediate layout algorithm.
//! Own a `BoxNode` tree, compute geometry, then paint leaves through `show_layout`.
mod baseline;
pub use baseline::{Baselines, TextBaseline};
mod custom;
mod flex;
mod query;
pub use custom::{BoxChildren, BoxGeometry, CustomBox, CustomBoxLayout};
pub use query::{BoxQueries, IntrinsicDimension};
mod stack;
mod ui;
use egui::{Id, Pos2, Rect, Vec2};
use std::{collections::HashSet, fmt, hash::Hash};
pub use ui::show_layout;

#[derive(Clone, Debug, PartialEq)]
pub enum LayoutError {
    InvalidConstraints,
    InvalidSize,
    InvalidConfiguration(&'static str),
    UnsupportedQuery(&'static str),
    UnboundedSize,
    UnboundedFlex,
    UnboundedStretch,
    ChildViolatedConstraints(Id),
    DuplicateKey(Id),
    TreeTooDeep,
}
impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConstraints => f.write_str("box constraints must satisfy 0 <= min <= max on each axis"),
            Self::InvalidSize => f.write_str("box sizes must be nonnegative and must not contain NaN"),
            Self::InvalidConfiguration(reason) => f.write_str(reason),
            Self::UnsupportedQuery(reason) => write!(f,"size query is unsupported: {reason}"),
            Self::UnboundedSize => f.write_str("layout needs a finite size; constrain the unbounded axis"),
            Self::UnboundedFlex => f.write_str("flex cannot expand in an unbounded main axis; constrain it or use loose flex with MainAxisSize::Min"),
            Self::UnboundedStretch => f.write_str("stretch requires a bounded cross axis"),
            Self::ChildViolatedConstraints(id) => write!(f, "measured box {id:?} returned a size outside its constraints"),
            Self::DuplicateKey(id) => write!(f, "duplicate box key {id:?}"),
            Self::TreeTooDeep => f.write_str("box tree exceeds 256 levels"),
        }
    }
}
impl std::error::Error for LayoutError {}

/// Normalized min/max extents in logical points. Infinity is permitted; NaN is not.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxConstraints {
    min: Vec2,
    max: Vec2,
}
impl Default for BoxConstraints {
    fn default() -> Self {
        Self {
            min: Vec2::ZERO,
            max: Vec2::INFINITY,
        }
    }
}
impl BoxConstraints {
    pub fn new(min: Vec2, max: Vec2) -> Result<Self, LayoutError> {
        if !nonnegative(min) || !nonnegative(max) || min.x > max.x || min.y > max.y {
            return Err(LayoutError::InvalidConstraints);
        }
        Ok(Self { min, max })
    }
    pub fn tight(size: Vec2) -> Result<Self, LayoutError> {
        Self::new(size, size)
    }
    pub fn loose(max: Vec2) -> Result<Self, LayoutError> {
        Self::new(Vec2::ZERO, max)
    }
    pub fn tight_for(width: Option<f32>, height: Option<f32>) -> Result<Self, LayoutError> {
        Self::new(
            egui::vec2(width.unwrap_or(0.0), height.unwrap_or(0.0)),
            egui::vec2(
                width.unwrap_or(f32::INFINITY),
                height.unwrap_or(f32::INFINITY),
            ),
        )
    }
    pub fn min(self) -> Vec2 {
        self.min
    }
    pub fn max(self) -> Vec2 {
        self.max
    }
    pub fn loosen(self) -> Self {
        Self {
            min: Vec2::ZERO,
            ..self
        }
    }
    pub fn is_tight(self) -> bool {
        self.min == self.max
    }
    pub fn contains(self, size: Vec2) -> bool {
        size.is_finite()
            && size.x >= self.min.x
            && size.y >= self.min.y
            && size.x <= self.max.x
            && size.y <= self.max.y
    }
    pub fn constrain(self, size: Vec2) -> Result<Vec2, LayoutError> {
        if !nonnegative(size) {
            return Err(LayoutError::InvalidSize);
        }
        finite(egui::vec2(
            size.x.clamp(self.min.x, self.max.x),
            size.y.clamp(self.min.y, self.max.y),
        ))
    }
    pub fn enforce(self, parent: Self) -> Self {
        Self {
            min: self.min.clamp(parent.min, parent.max),
            max: self.max.clamp(parent.min, parent.max),
        }
    }
    pub fn deflate(self, insets: Insets) -> Result<Self, LayoutError> {
        insets.validate()?;
        let space = insets.total();
        Self::new(
            (self.min - space).max(Vec2::ZERO),
            (self.max - space).max(Vec2::ZERO),
        )
    }
}
fn nonnegative(size: Vec2) -> bool {
    size.x >= 0.0 && size.y >= 0.0
}
fn finite(size: Vec2) -> Result<Vec2, LayoutError> {
    if size.is_finite() {
        Ok(size)
    } else {
        Err(LayoutError::UnboundedSize)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Insets {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}
impl Insets {
    pub const fn all(value: f32) -> Self {
        Self {
            left: value,
            top: value,
            right: value,
            bottom: value,
        }
    }
    pub fn total(self) -> Vec2 {
        egui::vec2(self.left + self.right, self.top + self.bottom)
    }
    fn validate(self) -> Result<(), LayoutError> {
        if [self.left, self.top, self.right, self.bottom]
            .iter()
            .all(|v| v.is_finite() && *v >= 0.0)
            && self.total().is_finite()
        {
            Ok(())
        } else {
            Err(LayoutError::InvalidConfiguration(
                "padding must be finite and nonnegative",
            ))
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Alignment {
    pub x: f32,
    pub y: f32,
}
impl Alignment {
    pub const TOP_LEFT: Self = Self { x: -1.0, y: -1.0 };
    pub const CENTER: Self = Self { x: 0.0, y: 0.0 };
    pub const BOTTOM_RIGHT: Self = Self { x: 1.0, y: 1.0 };
    fn offset(self, space: Vec2) -> Result<Vec2, LayoutError> {
        if !self.x.is_finite() || !self.y.is_finite() {
            return Err(LayoutError::InvalidConfiguration(
                "alignment must be finite",
            ));
        }
        let offset = space * egui::vec2((self.x + 1.0) / 2.0, (self.y + 1.0) / 2.0);
        if !offset.is_finite() {
            return Err(LayoutError::UnboundedSize);
        }
        Ok(offset)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Clip {
    None,
    #[default]
    HardEdge,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}
impl Axis {
    fn main(self, size: Vec2) -> f32 {
        if self == Self::Horizontal {
            size.x
        } else {
            size.y
        }
    }
    fn cross(self, size: Vec2) -> f32 {
        if self == Self::Horizontal {
            size.y
        } else {
            size.x
        }
    }
    fn size(self, main: f32, cross: f32) -> Vec2 {
        if self == Self::Horizontal {
            egui::vec2(main, cross)
        } else {
            egui::vec2(cross, main)
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MainAxisSize {
    Min,
    #[default]
    Max,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MainAxisAlignment {
    #[default]
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CrossAxisAlignment {
    Start,
    End,
    #[default]
    Center,
    Stretch,
    Baseline(TextBaseline),
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextDirection {
    #[default]
    Ltr,
    Rtl,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VerticalDirection {
    #[default]
    Down,
    Up,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlexFit {
    Tight,
    Loose,
}
#[derive(Clone, Copy, Debug)]
pub enum FlexSizing {
    Fixed,
    Flexible { factor: u32, fit: FlexFit },
}
pub struct FlexChild {
    pub node: BoxNode,
    pub sizing: FlexSizing,
}
impl FlexChild {
    pub fn fixed(node: BoxNode) -> Self {
        Self {
            node,
            sizing: FlexSizing::Fixed,
        }
    }
    pub fn expanded(node: BoxNode, factor: u32) -> Self {
        Self::flexible(node, factor, FlexFit::Tight)
    }
    pub fn flexible(node: BoxNode, factor: u32, fit: FlexFit) -> Self {
        Self {
            node,
            sizing: FlexSizing::Flexible { factor, fit },
        }
    }
}
pub struct Flex {
    pub axis: Axis,
    pub main_size: MainAxisSize,
    pub main_alignment: MainAxisAlignment,
    pub cross_alignment: CrossAxisAlignment,
    pub text_direction: TextDirection,
    pub vertical_direction: VerticalDirection,
    pub spacing: f32,
    pub clip: Clip,
    pub children: Vec<FlexChild>,
}
impl Flex {
    pub fn new(axis: Axis, children: Vec<FlexChild>) -> Self {
        Self {
            axis,
            children,
            main_size: MainAxisSize::Max,
            main_alignment: MainAxisAlignment::Start,
            cross_alignment: CrossAxisAlignment::Center,
            text_direction: TextDirection::Ltr,
            vertical_direction: VerticalDirection::Down,
            spacing: 0.0,
            clip: Clip::HardEdge,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StackFit {
    #[default]
    Loose,
    Expand,
    Passthrough,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Positioned {
    pub left: Option<f32>,
    pub top: Option<f32>,
    pub right: Option<f32>,
    pub bottom: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
}
impl Positioned {
    pub fn fill() -> Self {
        Self {
            left: Some(0.0),
            top: Some(0.0),
            right: Some(0.0),
            bottom: Some(0.0),
            ..Self::default()
        }
    }
    fn is_positioned(self) -> bool {
        [
            self.left,
            self.top,
            self.right,
            self.bottom,
            self.width,
            self.height,
        ]
        .iter()
        .any(Option::is_some)
    }
}
pub struct StackChild {
    pub node: BoxNode,
    pub position: Positioned,
}
impl StackChild {
    pub fn new(node: BoxNode) -> Self {
        Self {
            node,
            position: Positioned::default(),
        }
    }
    pub fn positioned(node: BoxNode, position: Positioned) -> Self {
        Self { node, position }
    }
}
pub struct Stack {
    pub alignment: Alignment,
    pub fit: StackFit,
    pub clip: Clip,
    pub children: Vec<StackChild>,
}
impl Stack {
    pub fn new(children: Vec<StackChild>) -> Self {
        Self {
            children,
            alignment: Alignment::TOP_LEFT,
            fit: StackFit::Loose,
            clip: Clip::HardEdge,
        }
    }
}

type Measure = dyn FnMut(BoxConstraints) -> Result<Vec2, LayoutError>;
pub enum BoxKind {
    BaselineLeaf {
        preferred: Vec2,
        baselines: Baselines,
    },
    Baseline {
        distance: f32,
        baseline: TextBaseline,
        child: Box<BoxNode>,
    },
    IgnoreBaseline {
        child: Box<BoxNode>,
    },
    IntrinsicWidth {
        step_width: Option<f32>,
        step_height: Option<f32>,
        child: Box<BoxNode>,
    },
    IntrinsicHeight {
        child: Box<BoxNode>,
    },
    Custom {
        layout: CustomBox,
        children: Vec<BoxNode>,
    },
    Leaf {
        preferred: Vec2,
    },
    Measured(Box<Measure>),
    Padding {
        insets: Insets,
        child: Box<BoxNode>,
    },
    Constrained {
        constraints: BoxConstraints,
        child: Box<BoxNode>,
    },
    Align {
        alignment: Alignment,
        width_factor: Option<f32>,
        height_factor: Option<f32>,
        child: Box<BoxNode>,
    },
    Flex(Flex),
    Stack(Stack),
}
/// Owned box-layout tree. Keys identify geometry/egui leaf UI consistently across frames.
/// Use [`crate::element::ElementTree`] for declarative reconciliation and component state.
pub struct BoxNode {
    pub id: Id,
    pub kind: BoxKind,
}
impl BoxNode {
    pub fn new(key: impl Hash, kind: BoxKind) -> Self {
        Self {
            id: Id::new(key),
            kind,
        }
    }
    pub fn leaf(key: impl Hash, preferred: Vec2) -> Self {
        Self::new(key, BoxKind::Leaf { preferred })
    }
    pub fn baseline_leaf(key: impl Hash, preferred: Vec2, baselines: Baselines) -> Self {
        Self::new(
            key,
            BoxKind::BaselineLeaf {
                preferred,
                baselines,
            },
        )
    }
    pub fn baseline(key: impl Hash, distance: f32, baseline: TextBaseline, child: Self) -> Self {
        Self::new(
            key,
            BoxKind::Baseline {
                distance,
                baseline,
                child: Box::new(child),
            },
        )
    }
    pub fn ignore_baseline(key: impl Hash, child: Self) -> Self {
        Self::new(
            key,
            BoxKind::IgnoreBaseline {
                child: Box::new(child),
            },
        )
    }
    pub fn measured(
        key: impl Hash,
        measure: impl FnMut(BoxConstraints) -> Result<Vec2, LayoutError> + 'static,
    ) -> Self {
        Self::new(key, BoxKind::Measured(Box::new(measure)))
    }
    pub fn padding(key: impl Hash, insets: Insets, child: Self) -> Self {
        Self::new(
            key,
            BoxKind::Padding {
                insets,
                child: Box::new(child),
            },
        )
    }
    pub fn intrinsic_width(
        key: impl Hash,
        step_width: Option<f32>,
        step_height: Option<f32>,
        child: Self,
    ) -> Self {
        Self::new(
            key,
            BoxKind::IntrinsicWidth {
                step_width,
                step_height,
                child: Box::new(child),
            },
        )
    }
    pub fn intrinsic_height(key: impl Hash, child: Self) -> Self {
        Self::new(
            key,
            BoxKind::IntrinsicHeight {
                child: Box::new(child),
            },
        )
    }
    pub fn constrained(key: impl Hash, constraints: BoxConstraints, child: Self) -> Self {
        Self::new(
            key,
            BoxKind::Constrained {
                constraints,
                child: Box::new(child),
            },
        )
    }
    pub fn align(key: impl Hash, alignment: Alignment, child: Self) -> Self {
        Self::new(
            key,
            BoxKind::Align {
                alignment,
                width_factor: None,
                height_factor: None,
                child: Box::new(child),
            },
        )
    }
    pub fn flex(key: impl Hash, flex: Flex) -> Self {
        Self::new(key, BoxKind::Flex(flex))
    }
    pub fn stack(key: impl Hash, stack: Stack) -> Self {
        Self::new(key, BoxKind::Stack(stack))
    }
    pub fn layout(&mut self, constraints: BoxConstraints) -> Result<LayoutBox, LayoutError> {
        self.validate(&mut HashSet::new(), 0)?;
        self.perform_layout(constraints)
    }
    fn validate(&self, keys: &mut HashSet<Id>, depth: usize) -> Result<(), LayoutError> {
        if depth >= 256 {
            return Err(LayoutError::TreeTooDeep);
        }
        if !keys.insert(self.id) {
            return Err(LayoutError::DuplicateKey(self.id));
        }
        match &self.kind {
            BoxKind::Custom { children, .. } => {
                for child in children {
                    child.validate(keys, depth + 1)?;
                }
            }
            BoxKind::Padding { child, .. }
            | BoxKind::Constrained { child, .. }
            | BoxKind::Align { child, .. }
            | BoxKind::IntrinsicWidth { child, .. }
            | BoxKind::IntrinsicHeight { child }
            | BoxKind::Baseline { child, .. }
            | BoxKind::IgnoreBaseline { child } => child.validate(keys, depth + 1)?,
            BoxKind::Flex(flex) => {
                for child in &flex.children {
                    child.node.validate(keys, depth + 1)?;
                }
            }
            BoxKind::Stack(stack) => {
                for child in &stack.children {
                    child.node.validate(keys, depth + 1)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn perform_layout(&mut self, constraints: BoxConstraints) -> Result<LayoutBox, LayoutError> {
        let leaf = matches!(
            self.kind,
            BoxKind::Leaf { .. } | BoxKind::BaselineLeaf { .. } | BoxKind::Measured(_)
        );
        let mut children = Vec::new();
        let mut clip = Clip::None;
        let size = match &mut self.kind {
            BoxKind::Custom { layout, children } => {
                return layout.perform_layout(self.id, constraints, children);
            }
            BoxKind::BaselineLeaf {
                preferred,
                baselines,
            } => {
                baselines.validate()?;
                constraints.constrain(*preferred)?
            }
            BoxKind::Baseline {
                distance,
                baseline,
                child,
            } => {
                baseline::validate_target(*distance)?;
                let layout = child.perform_layout(constraints.loosen())?;
                let (size, offset) = baseline::baseline_size(
                    constraints,
                    layout.size,
                    layout.baseline(*baseline),
                    *distance,
                )?;
                children.push(PlacedBox { offset, layout });
                size
            }
            BoxKind::IgnoreBaseline { child } => {
                let layout = child.perform_layout(constraints)?;
                let size = layout.size;
                children.push(PlacedBox {
                    offset: Vec2::ZERO,
                    layout,
                });
                size
            }
            BoxKind::Leaf { preferred } => constraints.constrain(*preferred)?,
            BoxKind::Measured(measure) => measure(constraints)?,
            BoxKind::IntrinsicWidth {
                step_width,
                step_height,
                child,
            } => {
                let layout = child.perform_layout(query::intrinsic_constraints(
                    constraints,
                    child,
                    Some((*step_width, *step_height)),
                )?)?;
                let size = layout.size;
                children.push(PlacedBox {
                    offset: Vec2::ZERO,
                    layout,
                });
                size
            }
            BoxKind::IntrinsicHeight { child } => {
                let layout = child.perform_layout(query::intrinsic_constraints(
                    constraints,
                    child,
                    None,
                )?)?;
                let size = layout.size;
                children.push(PlacedBox {
                    offset: Vec2::ZERO,
                    layout,
                });
                size
            }
            BoxKind::Padding { insets, child } => {
                let layout = child.perform_layout(constraints.deflate(*insets)?)?;
                let size = constraints.constrain(layout.size + insets.total())?;
                children.push(PlacedBox {
                    offset: egui::vec2(insets.left, insets.top),
                    layout,
                });
                size
            }
            BoxKind::Constrained {
                constraints: own,
                child,
            } => {
                let layout = child.perform_layout(own.enforce(constraints))?;
                let size = layout.size;
                children.push(PlacedBox {
                    offset: Vec2::ZERO,
                    layout,
                });
                size
            }
            BoxKind::Align {
                alignment,
                width_factor,
                height_factor,
                child,
            } => {
                if width_factor
                    .iter()
                    .chain(height_factor.iter())
                    .any(|v| !v.is_finite() || *v < 0.0)
                {
                    return Err(LayoutError::InvalidConfiguration(
                        "alignment size factors must be finite and nonnegative",
                    ));
                }
                let layout = child.perform_layout(constraints.loosen())?;
                let size = constraints.constrain(egui::vec2(
                    width_factor.map_or(
                        if constraints.max.x.is_finite() {
                            constraints.max.x
                        } else {
                            layout.size.x
                        },
                        |f| layout.size.x * f,
                    ),
                    height_factor.map_or(
                        if constraints.max.y.is_finite() {
                            constraints.max.y
                        } else {
                            layout.size.y
                        },
                        |f| layout.size.y * f,
                    ),
                ))?;
                let offset = alignment.offset(size - layout.size)?;
                children.push(PlacedBox { offset, layout });
                size
            }
            BoxKind::Flex(flex) => {
                clip = flex.clip;
                let result = flex.layout(constraints)?;
                children = result.1;
                result.0
            }
            BoxKind::Stack(stack) => {
                clip = stack.clip;
                let result = stack.layout(constraints)?;
                children = result.1;
                result.0
            }
        };
        if !constraints.contains(size) {
            return Err(LayoutError::ChildViolatedConstraints(self.id));
        }
        let mut layout = LayoutBox::new(self.id, size, children, clip, leaf)?;
        match &self.kind {
            BoxKind::BaselineLeaf { baselines, .. } => layout.baselines = *baselines,
            BoxKind::IgnoreBaseline { .. } => {}
            BoxKind::Flex(flex) => layout.inherit_baselines(flex.axis == Axis::Horizontal)?,
            BoxKind::Stack(_) => layout.inherit_baselines(true)?,
            _ => layout.inherit_baselines(false)?,
        }
        Ok(layout)
    }
}
#[derive(Clone, Debug)]
pub struct PlacedBox {
    pub offset: Vec2,
    pub layout: LayoutBox,
}
#[derive(Clone, Debug)]
pub struct LayoutBox {
    leaf: bool,
    paint_self: bool,
    hit_self: bool,
    pub baselines: Baselines,
    pub id: Id,
    pub size: Vec2,
    pub children: Vec<PlacedBox>,
    pub clip: Clip,
    /// Visual overflow before this box's own clip is applied.
    pub overflow: Insets,
    pub paint_bounds: Rect,
}
impl LayoutBox {
    fn new(
        id: Id,
        size: Vec2,
        children: Vec<PlacedBox>,
        clip: Clip,
        leaf: bool,
    ) -> Result<Self, LayoutError> {
        let own = Rect::from_min_size(Pos2::ZERO, size);
        let bounds = children.iter().fold(own, |bounds, child| {
            bounds.union(child.layout.paint_bounds.translate(child.offset))
        });
        let overflow = Insets {
            left: (-bounds.min.x).max(0.0),
            top: (-bounds.min.y).max(0.0),
            right: (bounds.max.x - size.x).max(0.0),
            bottom: (bounds.max.y - size.y).max(0.0),
        };
        if !bounds.is_finite()
            || [overflow.left, overflow.top, overflow.right, overflow.bottom]
                .iter()
                .any(|v| !v.is_finite())
        {
            return Err(LayoutError::UnboundedSize);
        }
        Ok(Self {
            leaf,
            paint_self: leaf,
            hit_self: leaf,
            baselines: Baselines::NONE,
            id,
            size,
            children,
            clip,
            overflow,
            paint_bounds: if clip == Clip::HardEdge { own } else { bounds },
        })
    }
    /// Topmost participating box at a local point. As with box hit testing, ancestors' bounds
    /// gate hits even when Clip::None permits painting outside them.
    pub fn hit_test(&self, point: Pos2) -> Option<Id> {
        if !(point.x >= 0.0 && point.y >= 0.0 && point.x < self.size.x && point.y < self.size.y) {
            return None;
        }
        self.children
            .iter()
            .rev()
            .find_map(|child| child.layout.hit_test(point - child.offset))
            .or_else(|| self.hit_self.then_some(self.id))
    }
    pub fn find(&self, id: Id) -> Option<&Self> {
        if self.id == id {
            return Some(self);
        }
        self.children.iter().find_map(|child| child.layout.find(id))
    }
    pub fn is_leaf(&self) -> bool {
        self.leaf
    }
    /// Locate a box relative to this root, including all intermediate offsets.
    pub fn rect_for(&self, id: Id) -> Option<Rect> {
        if self.id == id {
            return Some(Rect::from_min_size(Pos2::ZERO, self.size));
        }
        self.children.iter().find_map(|child| {
            child
                .layout
                .rect_for(id)
                .map(|rect| rect.translate(child.offset))
        })
    }
}

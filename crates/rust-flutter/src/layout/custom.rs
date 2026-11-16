//! Stateful custom box layout with validated child layout/placement.
use super::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

/// Geometry selected by a custom box. Children paint in placement order and hit
/// test in reverse order. Offsets may be negative; all geometry must be finite.
#[derive(Clone, Copy, Debug)]
pub struct BoxGeometry {
    pub size: Vec2,
    pub clip: Clip,
    pub hit_test_self: bool,
    pub baselines: Baselines,
}
impl BoxGeometry {
    pub fn new(size: Vec2) -> Self {
        Self {
            size,
            clip: Clip::None,
            hit_test_self: false,
            baselines: Baselines::NONE,
        }
    }
}
/// Layout every child, then place it exactly once. A child may be measured more
/// than once before placement. Layout errors leave the mounted tree intact.
pub struct BoxChildren<'a> {
    nodes: &'a mut [BoxNode],
    measured: Vec<Option<LayoutBox>>,
    order: Vec<usize>,
    offsets: Vec<Vec2>,
}
impl BoxChildren<'_> {
    /// Speculate without laying out or placing a child in the current pass.
    pub fn queries(&self) -> BoxQueries<'_> {
        BoxQueries::new(self.nodes)
    }
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
    pub fn id(&self, index: usize) -> Result<Id, LayoutError> {
        self.nodes
            .get(index)
            .map(|n| n.id)
            .ok_or(LayoutError::InvalidConfiguration(
                "custom child index is out of bounds",
            ))
    }
    pub fn baseline(
        &self,
        index: usize,
        kind: TextBaseline,
        only_real: bool,
    ) -> Result<Option<f32>, LayoutError> {
        let layout = self.measured.get(index).and_then(Option::as_ref).ok_or(
            LayoutError::InvalidConfiguration("baseline query requires a laid-out child"),
        )?;
        Ok(layout.distance_to_baseline(kind, only_real))
    }
    pub fn layout(
        &mut self,
        index: usize,
        constraints: BoxConstraints,
    ) -> Result<Vec2, LayoutError> {
        if self.order.contains(&index) {
            return Err(LayoutError::InvalidConfiguration(
                "cannot layout a child after placement",
            ));
        }
        let node = self
            .nodes
            .get_mut(index)
            .ok_or(LayoutError::InvalidConfiguration(
                "custom child index is out of bounds",
            ))?;
        let layout = node.perform_layout(constraints)?;
        let size = layout.size;
        self.measured[index] = Some(layout);
        Ok(size)
    }
    pub fn place(&mut self, index: usize, offset: Vec2) -> Result<(), LayoutError> {
        if !offset.is_finite() {
            return Err(LayoutError::InvalidConfiguration(
                "custom child offset must be finite",
            ));
        }
        if self.order.contains(&index) {
            return Err(LayoutError::InvalidConfiguration(
                "custom child was placed twice",
            ));
        }
        if self.measured.get(index).and_then(Option::as_ref).is_none() {
            return Err(LayoutError::InvalidConfiguration(
                "custom child must be laid out before placement",
            ));
        }
        self.order.push(index);
        self.offsets[index] = offset;
        Ok(())
    }
}

/// A layout implementation supplies an external revision if its geometry can
/// change without replacing children. The retained render-object adapter uses
/// its tree's layout revision. Call `CustomBox::invalidate` for standalone use.
pub trait CustomBoxLayout: 'static {
    fn dry_baseline(
        &self,
        _constraints: BoxConstraints,
        _kind: TextBaseline,
        _children: &BoxQueries<'_>,
    ) -> Result<Option<f32>, LayoutError> {
        Err(LayoutError::UnsupportedQuery(
            "custom box has no dry-baseline implementation",
        ))
    }
    fn dry_layout(
        &self,
        _constraints: BoxConstraints,
        _children: &BoxQueries<'_>,
    ) -> Result<Vec2, LayoutError> {
        Err(LayoutError::UnsupportedQuery(
            "custom box has no dry-layout implementation",
        ))
    }
    fn intrinsic(
        &self,
        _dimension: IntrinsicDimension,
        _extent: f32,
        _children: &BoxQueries<'_>,
    ) -> Result<f32, LayoutError> {
        Err(LayoutError::UnsupportedQuery(
            "custom box has no intrinsic-size implementation",
        ))
    }
    fn revision(&self) -> u64 {
        0
    }
    fn layout(
        &mut self,
        constraints: BoxConstraints,
        children: &mut BoxChildren<'_>,
    ) -> Result<BoxGeometry, LayoutError>;
}
struct Cache {
    id: Id,
    constraints: BoxConstraints,
    revision: u64,
    local_revision: u64,
    children: Vec<Id>,
    geometry: LayoutBox,
}
struct CustomState {
    layout: Box<dyn CustomBoxLayout>,
    cache: Option<Cache>,
}
/// Shared custom layout state persists when its surrounding BoxNode description
/// is regenerated. Clones share the same layout implementation and cache.
#[derive(Clone)]
pub struct CustomBox {
    state: Rc<RefCell<CustomState>>,
    revision: Rc<Cell<u64>>,
}
impl CustomBox {
    pub fn new(layout: impl CustomBoxLayout) -> Self {
        Self {
            state: Rc::new(RefCell::new(CustomState {
                layout: Box::new(layout),
                cache: None,
            })),
            revision: Rc::new(Cell::new(0)),
        }
    }
    pub fn invalidate(&self) {
        self.revision.set(self.revision.get().wrapping_add(1));
    }
    pub(super) fn dry_baseline(
        &self,
        c: BoxConstraints,
        kind: TextBaseline,
        children: &[BoxNode],
    ) -> Result<Option<f32>, LayoutError> {
        let state = self.state.try_borrow().map_err(|_| {
            LayoutError::InvalidConfiguration("custom layout object is already active")
        })?;
        state
            .layout
            .dry_baseline(c, kind, &BoxQueries::new(children))
    }
    pub(super) fn dry_layout(
        &self,
        c: BoxConstraints,
        children: &[BoxNode],
    ) -> Result<Vec2, LayoutError> {
        let state = self.state.try_borrow().map_err(|_| {
            LayoutError::InvalidConfiguration("custom layout object is already active")
        })?;
        state.layout.dry_layout(c, &BoxQueries::new(children))
    }
    pub(super) fn intrinsic(
        &self,
        d: IntrinsicDimension,
        extent: f32,
        children: &[BoxNode],
    ) -> Result<f32, LayoutError> {
        let state = self.state.try_borrow().map_err(|_| {
            LayoutError::InvalidConfiguration("custom layout object is already active")
        })?;
        state
            .layout
            .intrinsic(d, extent, &BoxQueries::new(children))
    }
    pub(super) fn perform_layout(
        &self,
        id: Id,
        constraints: BoxConstraints,
        nodes: &mut [BoxNode],
    ) -> Result<LayoutBox, LayoutError> {
        let mut state = self.state.try_borrow_mut().map_err(|_| {
            LayoutError::InvalidConfiguration("custom layout object is already active")
        })?;
        let revision = state.layout.revision();
        let local_revision = self.revision.get();
        let ids: Vec<_> = nodes.iter().map(|n| n.id).collect();
        if let Some(cache) = &state.cache
            && cache.id == id
            && cache.constraints == constraints
            && cache.revision == revision
            && cache.local_revision == local_revision
            && cache.children == ids
        {
            return Ok(cache.geometry.clone());
        }
        state.cache = None;
        let mut children = BoxChildren {
            measured: vec![None; nodes.len()],
            order: vec![],
            offsets: vec![Vec2::ZERO; nodes.len()],
            nodes,
        };
        let geometry = state.layout.layout(constraints, &mut children)?;
        geometry.baselines.validate()?;
        if !constraints.contains(geometry.size) {
            return Err(LayoutError::ChildViolatedConstraints(id));
        }
        if children.order.len() != children.nodes.len() {
            return Err(LayoutError::InvalidConfiguration(
                "custom layout must place every child",
            ));
        }
        let placed = children
            .order
            .into_iter()
            .map(|index| PlacedBox {
                offset: children.offsets[index],
                layout: children.measured[index]
                    .take()
                    .expect("placed child was measured"),
            })
            .collect();
        let mut layout = LayoutBox::new(
            id,
            geometry.size,
            placed,
            geometry.clip,
            children.nodes.is_empty(),
        )?;
        layout.baselines = geometry.baselines;
        layout.paint_self = true;
        layout.hit_self = geometry.hit_test_self;
        state.cache = Some(Cache {
            id,
            constraints,
            revision,
            local_revision,
            children: ids,
            geometry: layout.clone(),
        });
        Ok(layout)
    }
}

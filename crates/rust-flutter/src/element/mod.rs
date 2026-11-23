//! Declarative widgets, keyed element reconciliation and typed component state.
mod wrap;
pub use wrap::{Wrap, WrapAlignment, WrapCrossAlignment};
mod fractionally_sized_box;
pub use fractionally_sized_box::FractionallySizedBox;
mod limited_box;
pub use limited_box::LimitedBox;
mod aspect_ratio;
pub use aspect_ratio::AspectRatio;
mod inherited;
mod layout_builder;
pub use layout_builder::LayoutBuilder;
mod notifier_builder;
mod offstage;
mod retained_child;
mod shortcuts;
pub use shortcuts::Shortcuts;
mod tooltip;
pub use tooltip::Tooltip;
mod actions;
pub use actions::Actions;
mod signal_builder;
pub use offstage::Offstage;
pub(crate) use retained_child::{RetainedBuilder, retained_child};
mod render_object;
use render_object::{MountedRender, Pipeline};
pub use render_object::{RenderBox, RenderContext, RenderHandle, RenderObjectWidget};
mod ticker;
pub use ticker::TickerMode;
mod safe_area;
use safe_area::SafeAreaNode;
pub use safe_area::SafeAreaOptions;
mod semantics;
pub use semantics::Semantics;
mod directionality;
pub use directionality::Directionality;
mod state;
mod widget;
use crate::layout::{self, BoxConstraints, BoxKind, BoxNode, LayoutBox, LayoutError};
use inherited::{AmbientModel, Dependencies, Phase, Provider, Scope};
pub use inherited::{DependencyError, InheritedHandle, InheritedModel, InheritedNotifier};
pub use state::{
    BuildContext, State, StateContext, StateHandle, StatefulWidget, StatelessWidget, Unmounted,
};
use state::{Component, Queue};
use std::rc::Rc;
use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
pub use widget::{FlexItem, FlexStyle, StackItem, StackStyle, Widget};
use widget::{Kind, Paint, Token};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    String(String),
    U64(u64),
}
impl From<&str> for Key {
    fn from(value: &str) -> Self {
        Self::String(value.into())
    }
}
impl From<String> for Key {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}
impl From<u64> for Key {
    fn from(value: u64) -> Self {
        Self::U64(value)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ElementId(u64);
impl ElementId {
    pub fn value(self) -> u64 {
        self.0
    }
}
type PaintEntry = (Paint, Option<Paint>);
static NEXT_ID: AtomicU64 = AtomicU64::new(1);
#[derive(Debug, PartialEq)]
pub enum ElementError {
    DuplicateKey(Key),
    TreeTooDeep,
    EmptyTree,
    ContextChanged,
    GestureBinding(crate::gestures::BindingError),
    DisposedNotifier,
    Layout(LayoutError),
}
impl std::fmt::Display for ElementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateKey(key) => write!(f, "duplicate sibling key {key:?}"),
            Self::TreeTooDeep => f.write_str("element tree exceeds 256 levels"),
            Self::EmptyTree => f.write_str("element tree is unmounted; update it with a new root"),
            Self::DisposedNotifier => {
                f.write_str("cannot subscribe to a disposed inherited notifier")
            }
            Self::ContextChanged => f.write_str("element tree belongs to a different egui context"),
            Self::Layout(e) => e.fmt(f),
            Self::GestureBinding(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for ElementError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Layout(e) => Some(e),
            Self::GestureBinding(e) => Some(e),
            _ => None,
        }
    }
}
impl From<LayoutError> for ElementError {
    fn from(e: LayoutError) -> Self {
        Self::Layout(e)
    }
}
#[derive(Clone, Debug)]
pub struct ElementSnapshot {
    pub id: ElementId,
    pub key: Option<Key>,
    pub widget_type: &'static str,
    pub children: Vec<ElementId>,
}

fn validate(widget: &Widget, depth: usize) -> Result<(), ElementError> {
    if depth >= 256 {
        return Err(ElementError::TreeTooDeep);
    }
    let mut keys = HashSet::new();
    for child in &widget.children {
        if let Some(key) = &child.key
            && !keys.insert(key)
        {
            return Err(ElementError::DuplicateKey(key.clone()));
        }
        validate(child, depth + 1)?;
    }
    Ok(())
}
enum MountedKind {
    Render(Kind),
    Component(Box<dyn Component>),
    Inherited(Box<dyn Provider>),
    SafeArea(SafeAreaNode),
    TickerMode(ticker::TickerNode),
    RenderObject(Box<dyn MountedRender>),
}
struct Element {
    id: ElementId,
    key: Option<Key>,
    token: Token,
    name: &'static str,
    kind: MountedKind,
    children: Vec<Element>,
    queue: Arc<Queue>,
    dependencies: Rc<Dependencies>,
    last_build: u64,
}
impl Drop for Element {
    fn drop(&mut self) {
        self.children.clear();
        self.dependencies.close();
        self.queue.close();
        self.dependencies.animations.close();
    }
}
impl Element {
    fn matches(&self, widget: &Widget) -> bool {
        self.key == widget.key && self.token == widget.kind.token()
    }
    fn reconcile(
        old: Option<Self>,
        widget: Widget,
        ctx: &egui::Context,
        depth: usize,
        epoch: u64,
        scope: &Scope,
        pipeline: &Arc<Pipeline>,
    ) -> Result<Self, ElementError> {
        validate(&widget, depth)?;
        let mut element = if let Some(old) = old.filter(|old| old.matches(&widget)) {
            old
        } else {
            let id = ElementId(NEXT_ID.fetch_add(1, Ordering::Relaxed));
            let queue = Queue::new(ctx, pipeline);
            let dependencies = Dependencies::new(id, &queue, scope.clone());
            let context = BuildContext {
                dependencies: dependencies.clone(),
                id,
                queue: queue.clone(),
                egui: ctx.clone(),
            };
            let token = widget.kind.token();
            let name = widget.kind.name();
            let Widget {
                key,
                kind,
                children,
            } = widget;
            let kind = match kind {
                Kind::Component(description) => MountedKind::Component(description.mount(context)),
                Kind::Inherited(description) => {
                    MountedKind::Inherited(description.mount(id, queue.clone()))
                }
                Kind::TickerMode(enabled) => {
                    MountedKind::TickerMode(ticker::TickerNode::new(id, enabled, scope))
                }
                Kind::SafeArea(options) => {
                    MountedKind::SafeArea(SafeAreaNode::new(id, options, scope))
                }
                Kind::RenderObject(description) => {
                    MountedKind::RenderObject(description.mount(context))
                }
                other => MountedKind::Render(other),
            };
            let mut new = Self {
                id,
                key,
                token,
                name,
                kind,
                children: vec![],
                queue,
                dependencies,
                last_build: 0,
            };
            new.rebuild(children, ctx, depth, epoch, scope)?;
            return Ok(new);
        };
        let Widget { kind, children, .. } = widget;
        element.dependencies.set_scope(scope);
        match (&mut element.kind, kind) {
            (MountedKind::Component(component), Kind::Component(description)) => {
                component.update(description.into_any())
            }
            (MountedKind::Inherited(provider), Kind::Inherited(description)) => {
                provider.update(description.into_any())
            }
            (MountedKind::SafeArea(node), Kind::SafeArea(options)) => node.options = options,
            (MountedKind::TickerMode(node), Kind::TickerMode(enabled)) => node.enabled = enabled,
            (MountedKind::RenderObject(object), Kind::RenderObject(description)) => {
                object.update(description.into_any())
            }
            (slot, render) => *slot = MountedKind::Render(render),
        }
        element.rebuild(children, ctx, depth, epoch, scope)?;
        Ok(element)
    }
    fn rebuild(
        &mut self,
        described: Vec<Widget>,
        ctx: &egui::Context,
        depth: usize,
        epoch: u64,
        scope: &Scope,
    ) -> Result<(), ElementError> {
        self.queue.dirty.store(false, Ordering::Release);
        self.last_build = epoch;
        let children = if let MountedKind::Component(component) = &mut self.kind {
            component.apply_pending();
            if self.dependencies.changed.replace(false) {
                let _phase = self.dependencies.enter(Phase::Dependencies);
                component.dependencies_changed();
            }
            let _phase = self.dependencies.enter(Phase::Build);
            let Some(child) = component.build() else {
                return Ok(());
            };
            vec![child]
        } else {
            if let MountedKind::Inherited(provider) = &mut self.kind {
                provider.apply_pending()?;
            }
            described
        };
        if let MountedKind::SafeArea(node) = &mut self.kind {
            node.refresh(scope);
        }
        if let MountedKind::TickerMode(node) = &mut self.kind {
            node.refresh(scope);
        }
        let child_scope = self.child_scope(scope);
        let mut keys = HashSet::new();
        for child in &children {
            if let Some(key) = &child.key
                && !keys.insert(key)
            {
                return Err(ElementError::DuplicateKey(key.clone()));
            }
            validate(child, depth + 1)?;
        }
        let mut old: Vec<_> = std::mem::take(&mut self.children)
            .into_iter()
            .map(Some)
            .collect();
        let keyed: HashMap<_, _> = old
            .iter()
            .enumerate()
            .filter_map(|(i, child)| {
                child
                    .as_ref()
                    .and_then(|child| child.key.clone().map(|k| (k, i)))
            })
            .collect();
        let mut next = Vec::new();
        for (index, child) in children.into_iter().enumerate() {
            let previous = if let Some(key) = &child.key {
                keyed.get(key).and_then(|&index| old[index].take())
            } else {
                old.get_mut(index)
                    .filter(|old| old.as_ref().is_some_and(|element| element.key.is_none()))
                    .and_then(Option::take)
            };
            next.push(Self::reconcile(
                previous,
                child,
                ctx,
                depth + 1,
                epoch,
                &child_scope,
                &self.queue.pipeline,
            )?);
        }
        self.children = next;
        self.synchronize_render_children();
        Ok(())
    }
    fn flush(
        &mut self,
        ctx: &egui::Context,
        depth: usize,
        epoch: u64,
        scope: &Scope,
    ) -> Result<bool, ElementError> {
        let mut changed = if let MountedKind::SafeArea(node) = &mut self.kind {
            node.refresh(scope)
        } else {
            false
        };
        if let MountedKind::TickerMode(node) = &mut self.kind {
            node.refresh(scope);
        }
        self.dependencies.set_scope(scope);
        if self.last_build != epoch && self.queue.dirty.load(Ordering::Acquire) {
            if matches!(self.kind, MountedKind::Component(_)) {
                self.rebuild(vec![], ctx, depth, epoch, scope)?;
                changed = true;
            } else {
                self.queue.dirty.store(false, Ordering::Release);
                if let MountedKind::Inherited(provider) = &mut self.kind {
                    self.last_build = epoch;
                    provider.apply_pending()?;
                }
                if let MountedKind::RenderObject(object) = &mut self.kind {
                    object.mark_needs_update();
                    changed = true;
                }
            }
        }
        let child_scope = self.child_scope(scope);
        for child in &mut self.children {
            changed |= child.flush(ctx, depth + 1, epoch, &child_scope)?;
        }
        self.synchronize_render_children();
        Ok(changed)
    }
    fn render_id(&self) -> egui::Id {
        if matches!(
            self.kind,
            MountedKind::Component(_) | MountedKind::Inherited(_) | MountedKind::TickerMode(_)
        ) {
            self.children[0].render_id()
        } else {
            egui::Id::new(self.id)
        }
    }
    fn synchronize_render_children(&mut self) {
        if !matches!(self.kind, MountedKind::RenderObject(_)) {
            return;
        }
        let ids = self.children.iter().map(Self::render_id).collect();
        if let MountedKind::RenderObject(object) = &mut self.kind {
            object.synchronize(ids);
        }
    }
    fn render_node(&self, paint: &mut HashMap<egui::Id, PaintEntry>) -> BoxNode {
        if matches!(
            self.kind,
            MountedKind::Component(_) | MountedKind::Inherited(_) | MountedKind::TickerMode(_)
        ) {
            return self.children[0].render_node(paint);
        }
        if let MountedKind::RenderObject(object) = &self.kind {
            paint.insert(
                egui::Id::new(self.id),
                (object.paint(), Some(object.paint_foreground())),
            );
            return BoxNode::new(
                self.id,
                BoxKind::Custom {
                    layout: object.layout(),
                    children: self.children.iter().map(|c| c.render_node(paint)).collect(),
                },
            );
        }
        if let MountedKind::SafeArea(node) = &self.kind {
            return BoxNode::new(
                self.id,
                BoxKind::Padding {
                    insets: node.insets,
                    child: Box::new(self.children[0].render_node(paint)),
                },
            );
        }
        let children = self
            .children
            .iter()
            .map(|c| c.render_node(paint))
            .collect::<Vec<_>>();
        let MountedKind::Render(kind) = &self.kind else {
            unreachable!()
        };
        let key = self.id;
        let id = egui::Id::new(key);
        let kind = match kind {
            Kind::Leaf {
                preferred,
                baselines,
                measure,
                paint: callback,
            } => {
                paint.insert(id, (callback.clone(), None));
                if let Some(measure) = measure {
                    let measure = measure.clone();
                    BoxKind::Measured(Box::new(move |constraints| {
                        (measure.borrow_mut())(constraints)
                    }))
                } else {
                    BoxKind::BaselineLeaf {
                        preferred: *preferred,
                        baselines: *baselines,
                    }
                }
            }
            Kind::Baseline(distance, baseline) => BoxKind::Baseline {
                distance: *distance,
                baseline: *baseline,
                child: Box::new(children.into_iter().next().unwrap()),
            },
            Kind::IgnoreBaseline => BoxKind::IgnoreBaseline {
                child: Box::new(children.into_iter().next().unwrap()),
            },
            Kind::IntrinsicWidth(step_width, step_height) => BoxKind::IntrinsicWidth {
                step_width: *step_width,
                step_height: *step_height,
                child: Box::new(children.into_iter().next().unwrap()),
            },
            Kind::IntrinsicHeight => BoxKind::IntrinsicHeight {
                child: Box::new(children.into_iter().next().unwrap()),
            },
            Kind::Padding(insets) => BoxKind::Padding {
                insets: *insets,
                child: Box::new(children.into_iter().next().unwrap()),
            },
            Kind::Constrained(constraints) => BoxKind::Constrained {
                constraints: *constraints,
                child: Box::new(children.into_iter().next().unwrap()),
            },
            Kind::Align {
                alignment,
                width_factor,
                height_factor,
            } => BoxKind::Align {
                alignment: *alignment,
                width_factor: *width_factor,
                height_factor: *height_factor,
                child: Box::new(children.into_iter().next().unwrap()),
            },
            Kind::Flex(style, sizing) => BoxKind::Flex(layout::Flex {
                axis: style.axis,
                main_size: style.main_size,
                main_alignment: style.main_alignment,
                cross_alignment: style.cross_alignment,
                text_direction: style.text_direction,
                vertical_direction: style.vertical_direction,
                spacing: style.spacing,
                clip: style.clip,
                children: children
                    .into_iter()
                    .zip(sizing)
                    .map(|(node, sizing)| layout::FlexChild {
                        node,
                        sizing: *sizing,
                    })
                    .collect(),
            }),
            Kind::Stack(style, positions) => BoxKind::Stack(layout::Stack {
                alignment: style.alignment,
                fit: style.fit,
                clip: style.clip,
                children: children
                    .into_iter()
                    .zip(positions)
                    .map(|(node, position)| layout::StackChild {
                        node,
                        position: *position,
                    })
                    .collect(),
            }),
            Kind::Component(_)
            | Kind::Inherited(_)
            | Kind::TickerMode(_)
            | Kind::SafeArea(_)
            | Kind::RenderObject(_) => {
                unreachable!()
            }
        };
        BoxNode::new(key, kind)
    }
    fn child_scope(&self, parent: &Scope) -> Scope {
        if let MountedKind::TickerMode(node) = &self.kind {
            return node.child_scope(parent);
        }
        if let MountedKind::SafeArea(node) = &self.kind {
            return node.child_scope(parent);
        }
        let mut scope = parent.clone();
        if let MountedKind::Inherited(provider) = &self.kind {
            let link = provider.link();
            *link.parent.borrow_mut() = scope.get(&link.kind).map(Rc::downgrade);
            scope.insert(link.kind, link);
        }
        scope
    }
    fn find(&self, id: ElementId) -> Option<&Self> {
        if self.id == id {
            Some(self)
        } else {
            self.children.iter().find_map(|c| c.find(id))
        }
    }
    fn snapshots(&self, result: &mut Vec<ElementSnapshot>) {
        result.push(ElementSnapshot {
            id: self.id,
            key: self.key.clone(),
            widget_type: self.name,
            children: self.children.iter().map(|c| c.id).collect(),
        });
        for c in &self.children {
            c.snapshots(result);
        }
    }
}

/// Own one tree per egui context. `update` reconciles a new description; `flush`
/// processes queued state commands once per component in that build pass.
/// Each tree supplies an ambient [`crate::display::ViewMetrics`] model, refreshed
/// from the active viewport before reconciliation and flush. Explicit inherited
/// models of the same type can override it for a subtree.
pub struct ElementTree {
    root: Option<Element>,
    ctx: egui::Context,
    epoch: u64,
    render: Option<BoxNode>,
    paint: HashMap<egui::Id, PaintEntry>,
    view: AmbientModel<crate::display::ViewMetrics>,
    platform_theme: AmbientModel<crate::display::PlatformTheme>,
    platform_text_scale: AmbientModel<crate::text::PlatformTextScale>,
    pipeline: Arc<Pipeline>,
}
impl ElementTree {
    pub fn new(ctx: &egui::Context, root: Widget) -> Result<Self, ElementError> {
        let mut tree = Self {
            root: None,
            ctx: ctx.clone(),
            epoch: 0,
            render: None,
            paint: HashMap::new(),
            pipeline: Pipeline::new(),
            view: AmbientModel::new(
                ElementId(NEXT_ID.fetch_add(1, Ordering::Relaxed)),
                crate::display::view_metrics(ctx),
            ),
            platform_theme: AmbientModel::new(
                ElementId(NEXT_ID.fetch_add(1, Ordering::Relaxed)),
                crate::display::platform_theme(ctx),
            ),
            platform_text_scale: AmbientModel::new(
                ElementId(NEXT_ID.fetch_add(1, Ordering::Relaxed)),
                crate::text::PlatformTextScale(crate::text::platform_text_scale(ctx)),
            ),
        };
        tree.update(root)?;
        Ok(tree)
    }
    pub fn update(&mut self, widget: Widget) -> Result<(), ElementError> {
        // Static descriptor failures leave the previous mounted tree intact.
        validate(&widget, 0)?;
        self.pipeline.invalidate();
        self.render = None;
        self.paint.clear();
        self.epoch = self.epoch.wrapping_add(1);
        self.view.refresh(crate::display::view_metrics(&self.ctx));
        self.platform_theme
            .refresh(crate::display::platform_theme(&self.ctx));
        self.platform_text_scale
            .refresh(crate::text::PlatformTextScale(
                crate::text::platform_text_scale(&self.ctx),
            ));
        self.root = Some(Element::reconcile(
            self.root.take(),
            widget,
            &self.ctx,
            0,
            self.epoch,
            &self.scope(),
            &self.pipeline,
        )?);
        Ok(())
    }
    pub fn flush(&mut self) -> Result<bool, ElementError> {
        self.epoch = self.epoch.wrapping_add(1);
        let mut root = self.root.take().ok_or(ElementError::EmptyTree)?;
        self.view.refresh(crate::display::view_metrics(&self.ctx));
        self.platform_theme
            .refresh(crate::display::platform_theme(&self.ctx));
        self.platform_text_scale
            .refresh(crate::text::PlatformTextScale(
                crate::text::platform_text_scale(&self.ctx),
            ));
        let result = root.flush(&self.ctx, 0, self.epoch, &self.scope());
        match result {
            Ok(changed) => {
                self.root = Some(root);
                if changed {
                    self.pipeline.invalidate();
                    self.render = None;
                    self.paint.clear();
                }
                Ok(changed)
            }
            Err(error) => {
                self.render = None;
                self.paint.clear();
                Err(error)
            }
        }
    }
    pub fn unmount(&mut self) {
        self.render = None;
        self.paint.clear();
        self.root = None;
    }
    pub fn root_id(&self) -> Option<ElementId> {
        self.root.as_ref().map(|root| root.id)
    }
    pub fn elements(&self) -> Vec<ElementSnapshot> {
        let mut result = Vec::new();
        if let Some(root) = &self.root {
            root.snapshots(&mut result);
        }
        result
    }
    pub fn state<S: 'static>(&self, id: ElementId) -> Option<&S> {
        match &self.root.as_ref()?.find(id)?.kind {
            MountedKind::Component(c) => c.state()?.downcast_ref(),
            _ => None,
        }
    }
    pub fn state_handle<S: 'static>(&self, id: ElementId) -> Option<StateHandle<S>> {
        let element = self.root.as_ref()?.find(id)?;
        match &element.kind {
            MountedKind::Component(c) if c.state().is_some_and(|s| s.is::<S>()) => {
                Some(StateHandle::new(element.queue.clone()))
            }
            _ => None,
        }
    }
    pub fn inherited_handle<T: 'static>(&self, id: ElementId) -> Option<InheritedHandle<T>> {
        let element = self.root.as_ref()?.find(id)?;
        match &element.kind {
            MountedKind::Inherited(provider)
                if provider.link().kind == std::any::TypeId::of::<T>() =>
            {
                Some(InheritedHandle::new(element.queue.clone()))
            }
            _ => None,
        }
    }
    /// Flush widget updates and query size without wet layout or painting.
    pub fn dry_layout(&mut self, constraints: BoxConstraints) -> Result<egui::Vec2, ElementError> {
        self.prepare_render()?;
        Ok(self.render.as_ref().unwrap().dry_layout(constraints)?)
    }
    pub fn dry_baseline(
        &mut self,
        constraints: BoxConstraints,
        kind: layout::TextBaseline,
    ) -> Result<Option<f32>, ElementError> {
        self.prepare_render()?;
        Ok(self
            .render
            .as_ref()
            .unwrap()
            .dry_baseline(constraints, kind)?)
    }
    pub fn intrinsic(
        &mut self,
        dimension: layout::IntrinsicDimension,
        extent: f32,
    ) -> Result<f32, ElementError> {
        self.prepare_render()?;
        Ok(self.render.as_ref().unwrap().intrinsic(dimension, extent)?)
    }
    fn prepare_render(&mut self) -> Result<(), ElementError> {
        self.flush()?;
        if self.render.is_none() {
            self.render = Some(
                self.root
                    .as_ref()
                    .ok_or(ElementError::EmptyTree)?
                    .render_node(&mut self.paint),
            );
        }
        Ok(())
    }
    fn scope(&self) -> Scope {
        self.platform_text_scale
            .child_scope(&self.platform_theme.child_scope(&self.view.scope()))
    }
    /// Flush pending builds and compute current box geometry. State errors unmount
    /// the tree; layout errors keep component state mounted for a later retry.
    pub fn layout(&mut self, constraints: BoxConstraints) -> Result<LayoutBox, ElementError> {
        // Resolve layout-driven builds after unwinding all render-object
        // borrows, then lay out the reconciled children before anything paints.
        // Requests coalesce by component, including repeated speculative child
        // measurements in one custom layout. Siblings resolve in one batch.
        for transaction in 0..256 {
            self.prepare_render()?;
            let result = self.render.as_mut().unwrap().layout(constraints);
            if !self.pipeline.resolve_layout_builds(transaction < 255) {
                return Ok(result?);
            }
        }
        Err(ElementError::Layout(LayoutError::InvalidConfiguration(
            "layout builders did not stabilize after 256 transactions",
        )))
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        constraints: BoxConstraints,
    ) -> Result<egui::Response, ElementError> {
        self.show_internal(ui, constraints, None, None)
    }
    /// Layout then dispatch this root's input before painting. Use the same
    /// binding in its GestureDetector descendants. One root/viewport per binding.
    pub fn show_with_gestures(
        &mut self,
        ui: &mut egui::Ui,
        constraints: BoxConstraints,
        binding: &crate::gestures::GestureBinding,
    ) -> Result<egui::Response, ElementError> {
        self.show_internal(ui, constraints, Some(binding), None)
    }
    /// Collect this root in paint order; finish the shared frame after showing
    /// all roots. Gesture callbacks run after collection, rather than here.
    pub fn show_in_gesture_frame(
        &mut self,
        ui: &mut egui::Ui,
        constraints: BoxConstraints,
        frame: &mut crate::gestures::GestureFrame,
    ) -> Result<egui::Response, ElementError> {
        self.show_internal(ui, constraints, None, Some(frame))
    }
    fn show_internal(
        &mut self,
        ui: &mut egui::Ui,
        constraints: BoxConstraints,
        binding: Option<&crate::gestures::GestureBinding>,
        frame: Option<&mut crate::gestures::GestureFrame>,
    ) -> Result<egui::Response, ElementError> {
        if ui.ctx() != &self.ctx {
            return Err(ElementError::ContextChanged);
        }
        let geometry = self.layout(constraints)?;
        let mut input_error = None;
        let response = layout::show_layout_with_input(
            ui,
            &geometry,
            |ui, rect| {
                if let Some(frame) = frame {
                    input_error = frame.add_root(ui, &geometry, rect.min).err();
                }
                if let Some(binding) = binding {
                    match binding.handle_egui(ui, &geometry, rect.min) {
                        Ok(report) => input_error = report.failures.first().map(|f| f.error),
                        Err(error) => input_error = Some(error),
                    }
                }
            },
            |id, phase, ui| {
                if let Some((background, foreground)) = self.paint.get(&id) {
                    let paint = match phase {
                        layout::PaintPhase::Background => Some(background),
                        layout::PaintPhase::Foreground => foreground.as_ref(),
                    };
                    if let Some(paint) = paint {
                        (paint.borrow_mut())(ui);
                    }
                }
            },
        );
        if let Some(error) = input_error {
            Err(ElementError::GestureBinding(error))
        } else {
            Ok(response)
        }
    }
}

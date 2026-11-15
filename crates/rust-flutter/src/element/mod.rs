//! Declarative widgets, keyed element reconciliation and typed component state.
mod inherited;
mod state;
mod widget;
use crate::layout::{self, BoxConstraints, BoxKind, BoxNode, LayoutBox, LayoutError};
use inherited::{Dependencies, Phase, Provider, Scope};
pub use inherited::{DependencyError, InheritedHandle, InheritedModel};
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
static NEXT_ID: AtomicU64 = AtomicU64::new(1);
#[derive(Debug, PartialEq)]
pub enum ElementError {
    DuplicateKey(Key),
    TreeTooDeep,
    EmptyTree,
    ContextChanged,
    Layout(LayoutError),
}
impl std::fmt::Display for ElementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateKey(key) => write!(f, "duplicate sibling key {key:?}"),
            Self::TreeTooDeep => f.write_str("element tree exceeds 256 levels"),
            Self::EmptyTree => f.write_str("element tree is unmounted; update it with a new root"),
            Self::ContextChanged => f.write_str("element tree belongs to a different egui context"),
            Self::Layout(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for ElementError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Layout(e) => Some(e),
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
    ) -> Result<Self, ElementError> {
        validate(&widget, depth)?;
        let mut element = if let Some(old) = old.filter(|old| old.matches(&widget)) {
            old
        } else {
            let id = ElementId(NEXT_ID.fetch_add(1, Ordering::Relaxed));
            let queue = Queue::new(ctx);
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
            vec![component.build()]
        } else {
            if let MountedKind::Inherited(provider) = &mut self.kind {
                provider.apply_pending();
            }
            described
        };
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
            )?);
        }
        self.children = next;
        Ok(())
    }
    fn flush(
        &mut self,
        ctx: &egui::Context,
        depth: usize,
        epoch: u64,
        scope: &Scope,
    ) -> Result<bool, ElementError> {
        let mut changed = false;
        self.dependencies.set_scope(scope);
        if self.last_build != epoch && self.queue.dirty.load(Ordering::Acquire) {
            if matches!(self.kind, MountedKind::Component(_)) {
                self.rebuild(vec![], ctx, depth, epoch, scope)?;
                changed = true;
            } else {
                self.queue.dirty.store(false, Ordering::Release);
                if let MountedKind::Inherited(provider) = &mut self.kind {
                    self.last_build = epoch;
                    provider.apply_pending();
                }
            }
        }
        let child_scope = self.child_scope(scope);
        for child in &mut self.children {
            changed |= child.flush(ctx, depth + 1, epoch, &child_scope)?;
        }
        Ok(changed)
    }
    fn render_node(&self, paint: &mut HashMap<egui::Id, Paint>) -> BoxNode {
        if matches!(
            self.kind,
            MountedKind::Component(_) | MountedKind::Inherited(_)
        ) {
            return self.children[0].render_node(paint);
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
                measure,
                paint: callback,
            } => {
                paint.insert(id, callback.clone());
                if let Some(measure) = measure {
                    let measure = measure.clone();
                    BoxKind::Measured(Box::new(move |constraints| {
                        (measure.borrow_mut())(constraints)
                    }))
                } else {
                    BoxKind::Leaf {
                        preferred: *preferred,
                    }
                }
            }
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
            Kind::Component(_) | Kind::Inherited(_) => unreachable!(),
        };
        BoxNode::new(key, kind)
    }
    fn child_scope(&self, parent: &Scope) -> Scope {
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
pub struct ElementTree {
    root: Option<Element>,
    ctx: egui::Context,
    epoch: u64,
    render: Option<BoxNode>,
    paint: HashMap<egui::Id, Paint>,
}
impl ElementTree {
    pub fn new(ctx: &egui::Context, root: Widget) -> Result<Self, ElementError> {
        let mut tree = Self {
            root: None,
            ctx: ctx.clone(),
            epoch: 0,
            render: None,
            paint: HashMap::new(),
        };
        tree.update(root)?;
        Ok(tree)
    }
    pub fn update(&mut self, widget: Widget) -> Result<(), ElementError> {
        // Static descriptor failures leave the previous mounted tree intact.
        validate(&widget, 0)?;
        self.render = None;
        self.paint.clear();
        self.epoch = self.epoch.wrapping_add(1);
        self.root = Some(Element::reconcile(
            self.root.take(),
            widget,
            &self.ctx,
            0,
            self.epoch,
            &Scope::new(),
        )?);
        Ok(())
    }
    pub fn flush(&mut self) -> Result<bool, ElementError> {
        self.epoch = self.epoch.wrapping_add(1);
        let mut root = self.root.take().ok_or(ElementError::EmptyTree)?;
        let result = root.flush(&self.ctx, 0, self.epoch, &Scope::new());
        match result {
            Ok(changed) => {
                self.root = Some(root);
                if changed {
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
    /// Flush pending builds and compute current box geometry. State errors unmount
    /// the tree; layout errors keep component state mounted for a later retry.
    pub fn layout(&mut self, constraints: BoxConstraints) -> Result<LayoutBox, ElementError> {
        self.flush()?;
        if self.render.is_none() {
            self.render = Some(
                self.root
                    .as_ref()
                    .ok_or(ElementError::EmptyTree)?
                    .render_node(&mut self.paint),
            );
        }
        Ok(self.render.as_mut().unwrap().layout(constraints)?)
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        constraints: BoxConstraints,
    ) -> Result<egui::Response, ElementError> {
        if ui.ctx() != &self.ctx {
            return Err(ElementError::ContextChanged);
        }
        let geometry = self.layout(constraints)?;
        Ok(layout::show_layout(ui, &geometry, |id, ui| {
            if let Some(paint) = self.paint.get(&id) {
                (paint.borrow_mut())(ui);
            }
        }))
    }
}

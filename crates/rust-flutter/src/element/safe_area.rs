use super::{
    ElementId,
    inherited::{AmbientModel, Scope},
};
use crate::{
    display::{ViewEdges, ViewMetrics},
    layout::Insets,
};
use std::any::TypeId;

/// Retained SafeArea configuration. Minimum padding applies even on disabled
/// edges. It must be finite and nonnegative, as for ordinary layout padding.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SafeAreaOptions {
    pub edges: ViewEdges,
    pub minimum: Insets,
    /// Preserve stable bottom bar space when an IME covers it, when measured.
    pub maintain_bottom_view_padding: bool,
}
impl Default for SafeAreaOptions {
    fn default() -> Self {
        Self {
            edges: ViewEdges::ALL,
            minimum: Insets::default(),
            maintain_bottom_view_padding: false,
        }
    }
}
pub(super) struct SafeAreaNode {
    pub options: SafeAreaOptions,
    pub insets: Insets,
    view: AmbientModel<ViewMetrics>,
}
impl SafeAreaNode {
    fn parent_view(scope: &Scope) -> ViewMetrics {
        *scope
            .get(&TypeId::of::<ViewMetrics>())
            .and_then(|link| link.value::<ViewMetrics>())
            .expect("ElementTree supplies inherited view metrics")
    }
    pub fn new(id: ElementId, options: SafeAreaOptions, scope: &Scope) -> Self {
        Self {
            options,
            insets: Insets::default(),
            view: AmbientModel::new(id, Self::parent_view(scope)),
        }
    }
    pub fn refresh(&mut self, parent: &Scope) -> bool {
        let view = Self::parent_view(parent);
        let mut padding = view.padding();
        if self.options.maintain_bottom_view_padding
            && let Some(occlusion) = view.occlusion
        {
            padding.bottom = occlusion.view_padding.bottom;
        }
        let edge = |enabled, value: f32, minimum: f32| {
            // Preserve invalid values for the layout validator. f32::max alone
            // would silently mask NaN or a negative minimum.
            if !minimum.is_finite() || minimum < 0.0 {
                minimum
            } else {
                (if enabled { value } else { 0.0 }).max(minimum)
            }
        };
        let options = self.options;
        let next = Insets {
            left: edge(options.edges.left, padding.left, options.minimum.left),
            top: edge(options.edges.top, padding.top, options.minimum.top),
            right: edge(options.edges.right, padding.right, options.minimum.right),
            bottom: edge(options.edges.bottom, padding.bottom, options.minimum.bottom),
        };
        let changed = next != self.insets;
        self.insets = next;
        self.view.refresh(view.remove_padding(options.edges));
        changed
    }
    pub fn child_scope(&self, parent: &Scope) -> Scope {
        self.view.child_scope(parent)
    }
}

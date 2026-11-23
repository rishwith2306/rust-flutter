//! Retained accessibility metadata and actions for arbitrary child widgets.
use super::{RenderBox, RenderContext, RenderObjectWidget, Widget};
use crate::{
    egui::{self, accesskit},
    layout::{
        BoxChildren, BoxConstraints, BoxGeometry, BoxQueries, IntrinsicDimension, LayoutError,
    },
};
use std::rc::Rc;

/// Accessibility metadata that wraps a single visual child.
#[derive(Clone)]
pub struct Semantics {
    role: accesskit::Role,
    label: Option<String>,
    value: Option<String>,
    enabled: bool,
    selected: Option<bool>,
    toggled: Option<accesskit::Toggled>,
    on_click: Option<Rc<dyn Fn()>>,
}

impl Semantics {
    pub fn new(role: accesskit::Role) -> Self {
        Self {
            role,
            label: None,
            value: None,
            enabled: true,
            selected: None,
            toggled: None,
            on_click: None,
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self
    }

    pub fn toggled(mut self, toggled: accesskit::Toggled) -> Self {
        self.toggled = Some(toggled);
        self
    }

    pub fn on_click(mut self, on_click: impl Fn() + 'static) -> Self {
        self.on_click = Some(Rc::new(on_click));
        self
    }

    pub fn into_widget(self, child: Widget) -> Widget {
        Widget::render_object(self, vec![child])
    }
}

impl RenderObjectWidget for Semantics {
    type Object = SemanticsRenderBox;

    fn create_render_object(&self, context: &RenderContext) -> Self::Object {
        SemanticsRenderBox {
            config: self.clone(),
            id: egui::Id::new(("rust_flutter_semantics", context.element_id().value())),
        }
    }

    fn update_render_object(&self, object: &mut Self::Object, _context: &RenderContext) {
        object.config = self.clone();
    }
}

pub struct SemanticsRenderBox {
    config: Semantics,
    id: egui::Id,
}

impl RenderBox for SemanticsRenderBox {
    fn dry_layout(
        &self,
        constraints: BoxConstraints,
        children: &BoxQueries<'_>,
    ) -> Result<egui::Vec2, LayoutError> {
        if children.len() != 1 {
            return Err(LayoutError::InvalidConfiguration(
                "Semantics requires one child",
            ));
        }
        children.dry_layout(0, constraints)
    }

    fn intrinsic(
        &self,
        dimension: IntrinsicDimension,
        extent: f32,
        children: &BoxQueries<'_>,
    ) -> Result<f32, LayoutError> {
        if children.len() != 1 {
            return Err(LayoutError::InvalidConfiguration(
                "Semantics requires one child",
            ));
        }
        children.intrinsic(0, dimension, extent)
    }

    fn layout(
        &mut self,
        constraints: BoxConstraints,
        children: &mut BoxChildren<'_>,
        _context: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        if children.len() != 1 {
            return Err(LayoutError::InvalidConfiguration(
                "Semantics requires one child",
            ));
        }
        let size = children.layout(0, constraints)?;
        children.place(0, egui::Vec2::ZERO)?;
        Ok(BoxGeometry::new(size))
    }

    fn paint(&mut self, ui: &mut egui::Ui, _context: &RenderContext) {
        if crate::layout::ui_is_offstage(ui) {
            return;
        }
        let response = ui.interact(ui.max_rect(), self.id, egui::Sense::hover());
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_role(self.config.role);
            if let Some(label) = &self.config.label {
                node.set_label(label.clone());
            }
            if let Some(value) = &self.config.value {
                node.set_value(value.clone());
            }
            if !self.config.enabled {
                node.set_disabled();
            }
            if let Some(selected) = self.config.selected {
                node.set_selected(selected);
            }
            if let Some(toggled) = self.config.toggled {
                node.set_toggled(toggled);
            }
            if self.config.enabled && self.config.on_click.is_some() {
                node.add_action(accesskit::Action::Click);
            }
        });
        if !self.config.enabled || self.config.on_click.is_none() {
            return;
        }
        let mut activate = false;
        ui.input_mut(|input| {
            input.consume_accesskit_action_requests(response.id, |request| {
                if request.action == accesskit::Action::Click {
                    activate = true;
                    true
                } else {
                    false
                }
            });
        });
        if activate && let Some(on_click) = &self.config.on_click {
            on_click();
        }
    }
}

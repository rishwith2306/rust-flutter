use super::*;
use crate::{
    egui::{self, Color32, Id, Vec2},
    element::{InheritedNotifier, RenderBox, RenderContext, RenderObjectWidget, Widget},
    focus::FocusNode,
    layout::*,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
type ButtonPaint = Rc<dyn Fn(&egui::Painter, egui::Rect)>;

/// Common semantic roles for arena-backed controls on hosts with AccessKit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GestureSemanticRole {
    Button,
    Switch,
    CheckBox,
    RadioButton,
}

/// Semantic on/off/mixed state for accessibility toggle controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GestureToggleState {
    Off,
    On,
    Mixed,
}

/// An opt-in egui-painted button whose pointer action participates in the shared
/// gesture arena. Ordinary egui buttons remain independent. Keyboard Enter/Space
/// activation uses focus and does not compete with pointer gestures.
#[derive(Clone)]
pub struct GestureButton {
    binding: GestureBinding,
    label: egui::WidgetText,
    size: Vec2,
    fill: Option<Color32>,
    stroke: Option<egui::Stroke>,
    corner_radius: Option<egui::CornerRadius>,
    paint_content: Option<ButtonPaint>,
    enabled: bool,
    accessibility_role: Option<GestureSemanticRole>,
    accessibility_label: Option<String>,
    accessibility_toggled: Option<GestureToggleState>,
    accessibility_selected: Option<bool>,
    activate: Rc<dyn Fn()>,
}
impl GestureButton {
    pub fn new(
        binding: &GestureBinding,
        label: impl Into<egui::WidgetText>,
        activate: impl Fn() + 'static,
    ) -> Self {
        Self {
            binding: binding.clone(),
            label: label.into(),
            size: egui::vec2(80.0, 24.0),
            fill: None,
            stroke: None,
            corner_radius: None,
            paint_content: None,
            enabled: true,
            accessibility_role: None,
            accessibility_label: None,
            accessibility_toggled: None,
            accessibility_selected: None,
            activate: Rc::new(activate),
        }
    }
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = size;
        self
    }
    pub fn fill(mut self, fill: Color32) -> Self {
        self.fill = Some(fill);
        self
    }
    pub fn stroke(mut self, stroke: egui::Stroke) -> Self {
        self.stroke = Some(stroke);
        self
    }
    pub fn corner_radius(mut self, radius: impl Into<egui::CornerRadius>) -> Self {
        self.corner_radius = Some(radius.into());
        self
    }
    /// Paint custom content over the button face while retaining its shared
    /// gesture, focus and keyboard activation behavior.
    pub fn paint_content(mut self, paint: impl Fn(&egui::Painter, egui::Rect) + 'static) -> Self {
        self.paint_content = Some(Rc::new(paint));
        self
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    /// Overrides the AccessKit role for controls painted inside this button.
    pub fn accessibility_role(mut self, role: GestureSemanticRole) -> Self {
        self.accessibility_role = Some(role);
        self
    }
    /// Supplies the accessible name for a custom painted control.
    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
    /// Sets the AccessKit toggled state for switch and checkbox controls.
    pub fn accessibility_toggled(mut self, toggled: bool) -> Self {
        self.accessibility_toggled = Some(if toggled {
            GestureToggleState::On
        } else {
            GestureToggleState::Off
        });
        self
    }
    /// Sets an on, off, or mixed AccessKit state for toggle controls.
    pub fn accessibility_toggle_state(mut self, state: GestureToggleState) -> Self {
        self.accessibility_toggled = Some(state);
        self
    }
    /// Sets the AccessKit selected state for radio-like controls.
    pub fn accessibility_selected(mut self, selected: bool) -> Self {
        self.accessibility_selected = Some(selected);
        self
    }
    pub fn into_widget(self) -> Widget {
        Widget::render_object(self, vec![])
    }
}
struct ButtonState {
    context: egui::Context,
    pressed: Cell<bool>,
    focus: Cell<Option<Id>>,
    non_pointer_frame: Cell<Option<u64>>,
    callback: RefCell<Rc<dyn Fn()>>,
}
impl ButtonState {
    fn pressed(&self, pressed: bool) {
        self.pressed.set(pressed);
        self.context.request_repaint();
    }
    fn activate(&self) {
        self.pressed(false);
        if let Some(id) = self.focus.get() {
            self.context.memory_mut(|m| m.request_focus(id));
        }
        let callback = self.callback.borrow().clone();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback()));
    }
}
pub struct GestureButtonRenderBox {
    config: GestureButton,
    target: Option<GestureTarget>,
    state: Rc<ButtonState>,
    focus_node: Option<FocusNode>,
    focus_scope: Option<crate::focus::FocusScopeNode>,
    scope_target: Option<crate::focus::FocusTarget>,
    error: Option<BindingError>,
}
impl GestureButtonRenderBox {
    fn update_focus_scope(&mut self, ctx: &RenderContext) {
        let next = ctx
            .depend_on::<InheritedNotifier<crate::focus::FocusScopeNode>>()
            .ok()
            .flatten()
            .and_then(|provider| provider.notifier.clone());
        let same = match (&self.focus_scope, &next) {
            (Some(a), Some(b)) => a == b,
            (None, None) => true,
            _ => false,
        };
        if !same {
            self.scope_target = None;
            self.focus_scope = next;
        }
    }
    fn update_focus_node(&mut self, ctx: &RenderContext) {
        let next = ctx
            .depend_on::<InheritedNotifier<FocusNode>>()
            .ok()
            .flatten()
            .and_then(|provider| provider.notifier.clone());
        let same = match (&self.focus_node, &next) {
            (Some(old), Some(new)) => old == new,
            (None, None) => true,
            _ => false,
        };
        if !same {
            if let Some(old) = self.focus_node.take()
                && let Some(id) = self.state.focus.get()
            {
                old.detach(id);
            }
            self.focus_node = next;
        }
    }
    fn configure(&mut self, config: &GestureButton, ctx: &RenderContext) {
        if !self.config.binding.same_source(&config.binding) {
            self.target = None;
        }
        let old = std::mem::replace(
            &mut *self.state.callback.borrow_mut(),
            config.activate.clone(),
        );
        drop(old);
        let (down, tap, cancel) = (self.state.clone(), self.state.clone(), self.state.clone());
        let spec = GestureTargetConfig {
            tap: config.enabled.then_some((
                TapConfig::default(),
                TapCallbacks {
                    on_down: Some(Rc::new(move |_| down.pressed(true))),
                    on_tap: Some(Rc::new(move |_| tap.activate())),
                    on_cancel: Some(Rc::new(move |_| cancel.pressed(false))),
                },
            )),
            ..Default::default()
        };
        let result = if let Some(target) = &self.target {
            target.update(spec)
        } else {
            config
                .binding
                .register_target(Id::new(ctx.element_id()), spec)
                .map(|t| self.target = Some(t))
        };
        self.error = result.err();
        self.config = config.clone();
    }
    fn validate(&self, children: usize) -> Result<(), LayoutError> {
        if children != 0 {
            return Err(LayoutError::InvalidConfiguration(
                "GestureButton has no children",
            ));
        }
        if !self.config.size.is_finite() || self.config.size.x < 0.0 || self.config.size.y < 0.0 {
            return Err(LayoutError::InvalidSize);
        }
        if self.error.is_some() || !self.target.as_ref().is_some_and(GestureTarget::is_active) {
            return Err(LayoutError::InvalidConfiguration(
                "GestureButton target registration is unavailable",
            ));
        }
        Ok(())
    }
}
impl RenderObjectWidget for GestureButton {
    type Object = GestureButtonRenderBox;
    fn create_render_object(&self, ctx: &RenderContext) -> Self::Object {
        let state = Rc::new(ButtonState {
            context: ctx.egui().clone(),
            pressed: Cell::new(false),
            focus: Cell::new(None),
            non_pointer_frame: Cell::new(None),
            callback: RefCell::new(self.activate.clone()),
        });
        let mut object = GestureButtonRenderBox {
            config: self.clone(),
            target: None,
            state,
            focus_node: None,
            focus_scope: None,
            scope_target: None,
            error: None,
        };
        object.configure(self, ctx);
        object
    }
    fn update_render_object(&self, object: &mut Self::Object, ctx: &RenderContext) {
        object.configure(self, ctx);
        object.update_focus_node(ctx);
        object.update_focus_scope(ctx);
    }
}
impl RenderBox for GestureButtonRenderBox {
    fn dry_layout(
        &self,
        c: BoxConstraints,
        children: &BoxQueries<'_>,
    ) -> Result<Vec2, LayoutError> {
        self.validate(children.len())?;
        c.constrain(self.config.size)
    }
    fn intrinsic(
        &self,
        d: IntrinsicDimension,
        _: f32,
        children: &BoxQueries<'_>,
    ) -> Result<f32, LayoutError> {
        self.validate(children.len())?;
        Ok(match d {
            IntrinsicDimension::MinWidth | IntrinsicDimension::MaxWidth => self.config.size.x,
            _ => self.config.size.y,
        })
    }
    fn dry_baseline(
        &self,
        _: BoxConstraints,
        _: TextBaseline,
        children: &BoxQueries<'_>,
    ) -> Result<Option<f32>, LayoutError> {
        self.validate(children.len())?;
        Ok(None)
    }
    fn layout(
        &mut self,
        c: BoxConstraints,
        children: &mut BoxChildren<'_>,
        _: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        self.validate(children.len())?;
        let mut g = BoxGeometry::new(c.constrain(self.config.size)?);
        g.hit_test_behavior = Some(HitTestBehavior::Opaque);
        Ok(g)
    }
    fn paint(&mut self, ui: &mut egui::Ui, ctx: &RenderContext) {
        // Focusable, but deliberately no backend pointer click/drag sense.
        if let Some(scope) = &self.focus_scope {
            scope.prepare_keyboard_navigation(ui.ctx());
        }
        let mut button = egui::Button::new(self.config.label.clone())
            .sense(egui::Sense::focusable_noninteractive());
        if let Some(fill) = self.config.fill {
            button = button.fill(if self.state.pressed.get() {
                fill.gamma_multiply(0.85)
            } else {
                fill
            });
        }
        if let Some(stroke) = self.config.stroke {
            button = button.stroke(stroke);
        }
        if let Some(radius) = self.config.corner_radius {
            button = button.corner_radius(radius);
        }
        let mut scope =
            egui::UiBuilder::new().id(("rust-flutter-gesture-button", ctx.element_id()));
        if !self.config.enabled {
            scope = scope.disabled();
        }
        let response = ui
            .scope_builder(scope, |ui| {
                // Preserve normal button visuals despite the deliberately
                // non-clicking backend sense used for arena participation.
                if ui.is_enabled() {
                    let focused = self
                        .state
                        .focus
                        .get()
                        .is_some_and(|id| ui.memory(|m| m.has_focus(id)));
                    let widgets = &ui.visuals().widgets;
                    let visuals = if self.state.pressed.get() || focused {
                        widgets.active
                    } else if ui.rect_contains_pointer(ui.max_rect()) {
                        widgets.hovered
                    } else {
                        widgets.inactive
                    };
                    ui.visuals_mut().widgets.noninteractive = visuals;
                }
                ui.put(ui.max_rect(), button)
            })
            .inner;
        if let Some(paint_content) = &self.config.paint_content {
            paint_content(ui.painter(), response.rect);
        }
        self.state.focus.set(Some(response.id));
        if let Some(node) = &self.focus_node {
            node.attach(ui.ctx(), response.id);
            node.sync(response.has_focus());
        }
        if let Some(scope) = self.focus_scope.clone() {
            let can_focus = response.enabled()
                && self
                    .focus_node
                    .as_ref()
                    .is_none_or(FocusNode::can_request_focus)
                && !crate::layout::ui_is_offstage(ui);
            if self
                .scope_target
                .as_ref()
                .is_some_and(|target| !target.is_for(&scope, response.id))
            {
                self.scope_target = None;
            }
            if self.scope_target.is_none() {
                self.scope_target = Some(scope.register_target(ui.ctx(), response.id, can_focus));
            } else {
                scope.update_target(ui.ctx(), response.id, can_focus);
            }
            scope.refresh_focus();
        } else {
            self.scope_target = None;
        }
        let frame = ui.ctx().cumulative_frame_nr();
        if !crate::layout::ui_is_offstage(ui) {
            ui.ctx().accesskit_node_builder(response.id, |node| {
                if let Some(role) = self.config.accessibility_role {
                    node.set_role(match role {
                        GestureSemanticRole::Button => egui::accesskit::Role::Button,
                        GestureSemanticRole::Switch => egui::accesskit::Role::Switch,
                        GestureSemanticRole::CheckBox => egui::accesskit::Role::CheckBox,
                        GestureSemanticRole::RadioButton => egui::accesskit::Role::RadioButton,
                    });
                }
                if let Some(label) = self
                    .config
                    .accessibility_label
                    .as_deref()
                    .filter(|label| !label.is_empty())
                {
                    node.set_label(label.to_owned());
                }
                if let Some(toggled) = self.config.accessibility_toggled {
                    node.set_toggled(match toggled {
                        GestureToggleState::Off => egui::accesskit::Toggled::False,
                        GestureToggleState::On => egui::accesskit::Toggled::True,
                        GestureToggleState::Mixed => egui::accesskit::Toggled::Mixed,
                    });
                }
                if let Some(selected) = self.config.accessibility_selected {
                    node.set_selected(selected);
                }
                if response.enabled() {
                    node.add_action(egui::accesskit::Action::Click);
                }
            });
        }
        if response.enabled() && self.state.non_pointer_frame.get() != Some(frame) {
            let mut activate = false;
            if !crate::layout::ui_is_offstage(ui) {
                ui.input_mut(|i| {
                    i.consume_accesskit_action_requests(response.id, |request| {
                        if request.action == egui::accesskit::Action::Click {
                            activate = true;
                            true
                        } else {
                            false
                        }
                    })
                });
            }
            if response.has_focus() {
                activate |= ui.input_mut(|i| {
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                        || i.consume_key(egui::Modifiers::NONE, egui::Key::Space)
                });
            }
            if activate {
                self.state.non_pointer_frame.set(Some(frame));
                self.state.activate();
            }
        }
    }
    fn dispose(&mut self) {
        self.target = None;
        self.state.pressed(false);
        self.scope_target = None;
        if let Some(node) = self.focus_node.take()
            && let Some(id) = self.state.focus.get()
        {
            node.detach(id);
        }
    }
}

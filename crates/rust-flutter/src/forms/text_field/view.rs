use super::*;
use crate::{
    element::{RenderBox, RenderContext, RenderObjectWidget},
    layout::{
        BoxChildren, BoxConstraints, BoxGeometry, BoxQueries, Clip, IntrinsicDimension, LayoutError,
    },
    notifier::Listenable,
};
use egui::{
    Color32, FontId, Id, Rect, Vec2,
    text::{CCursor, CCursorRange},
};
use std::rc::Rc;
type MeasuredField = (
    Vec2,
    Rect,
    Option<std::sync::Arc<egui::Galley>>,
    Option<std::sync::Arc<egui::Galley>>,
    f32,
);
fn text_editor<'a>(
    buffer: &'a mut dyn egui::TextBuffer,
    style: &InputStyle,
    id: Id,
    width: f32,
) -> egui::TextEdit<'a> {
    let edit = if style.multiline {
        egui::TextEdit::multiline(buffer).desired_rows(style.min_lines)
    } else {
        egui::TextEdit::singleline(buffer).password(style.obscure)
    };
    edit.id(id)
        .desired_width(width)
        .hint_text(style.decoration.hint.clone())
        .font(FontId::monospace(14.0))
        .text_color_opt(style.decoration.text_color)
        .background_color(
            style
                .decoration
                .background_color
                .unwrap_or(Color32::from_gray(30)),
        )
        .char_limit(style.max_chars.unwrap_or(usize::MAX))
}

pub(super) struct TextView {
    pub binding: Rc<Binding>,
    pub field: FormFieldController<String>,
    pub style: InputStyle,
    pub focus: Option<crate::focus::FocusNode>,
    pub focus_scope: Option<crate::focus::FocusScopeNode>,
    pub on_changed: Option<TextCallback>,
    pub on_submitted: Option<TextCallback>,
}
pub struct TextInputRender {
    binding: Rc<Binding>,
    field: FormFieldController<String>,
    style: InputStyle,
    focus: Option<crate::focus::FocusNode>,
    focus_target: Option<(crate::focus::FocusNode, Id)>,
    focus_scope: Option<crate::focus::FocusScopeNode>,
    scope_target: Option<crate::focus::FocusTarget>,
    on_changed: Option<TextCallback>,
    on_submitted: Option<TextCallback>,
    context: egui::Context,
    handle: crate::element::RenderHandle,
    subscriptions: Vec<crate::notifier::Subscription>,
    input_rect: Rect,
    label_galley: Option<std::sync::Arc<egui::Galley>>,
    error_galley: Option<std::sync::Arc<egui::Galley>>,
    input_height: f32,
    applied_source: Option<(u64, u64)>,
    last_text: String,
    autofocus_done: bool,
    keep_alive: Option<crate::scrolling::KeepAliveHandle>,
}
impl TextInputRender {
    fn bind(&mut self, ctx: &RenderContext) {
        for subscription in self.subscriptions.drain(..) {
            drop(subscription);
        }
        self.handle = ctx.handle();
        let handle = self.handle.clone();
        if let Ok(s) = self.field.subscribe(Rc::new(move || {
            let _ = handle.mark_needs_layout();
        })) {
            self.subscriptions.push(s);
        }
        let handle = self.handle.clone();
        if let Ok(s) = self.binding.editor().subscribe(Rc::new(move || {
            let _ = handle.mark_needs_paint();
        })) {
            self.subscriptions.push(s);
        }
        let handle = self.handle.clone();
        if let Ok(s) = self.binding.changed.subscribe(Rc::new(move || {
            let _ = handle.mark_needs_layout();
        })) {
            self.subscriptions.push(s);
        }
    }
    fn input_id(&self, ctx: &RenderContext) -> Id {
        Id::new(("rust-flutter-text-field", ctx.element_id().value()))
    }
    fn bind_focus(&mut self, ctx: &RenderContext) {
        if let Some((node, id)) = self.focus_target.take() {
            node.detach(id);
        }
        if let Some(node) = &self.focus {
            let id = self.input_id(ctx);
            node.attach(ctx.egui(), id);
            self.focus_target = Some((node.clone(), id));
        }
    }
    fn measure(&self, c: BoxConstraints) -> Result<MeasuredField, LayoutError> {
        let width = if c.max().x.is_finite() {
            c.max().x
        } else {
            self.style.preferred_width
        }
        .max(c.min().x);
        let label_font = self
            .context
            .style()
            .text_styles
            .get(&egui::TextStyle::Body)
            .cloned()
            .unwrap_or_else(|| FontId::proportional(14.0));
        let input_font = FontId::monospace(label_font.size);
        let (label, _input, error, in_h, total) =
            self.context.fonts_mut(|fonts| {
                let label =
                    self.style.decoration.label.as_ref().map(|s| {
                        fonts.layout_no_wrap(s.clone(), label_font.clone(), Color32::WHITE)
                    });
                let input_width = (width - 8.0).max(0.0);
                let input_text = if self.style.obscure {
                    "•".repeat(self.binding.editor().text().chars().count())
                } else {
                    self.binding.editor().text()
                };
                let input = fonts.layout(
                    input_text,
                    input_font,
                    self.style.decoration.text_color.unwrap_or(Color32::WHITE),
                    input_width,
                );
                let error = self.binding.error().map(|s| {
                    fonts.layout(
                        s,
                        label_font.clone(),
                        self.style.decoration.error_color,
                        width,
                    )
                });
                let row = fonts.row_height(&FontId::monospace(label_font.size));
                let lines = if self.style.multiline {
                    self.style
                        .min_lines
                        .max(input.rows.len())
                        .min(self.style.max_lines.unwrap_or(usize::MAX))
                } else {
                    1
                };
                let ih = (row * lines as f32 + 4.0).max(row + 4.0);
                let h = label.as_ref().map_or(0.0, |g| g.size().y + 3.0)
                    + ih
                    + error.as_ref().map_or(0.0, |g| 3.0 + g.size().y);
                (label, input, error, ih, h)
            });
        let size = c.constrain(Vec2::new(width, total))?;
        let ly = label.as_ref().map_or(0.0, |g| g.size().y + 3.0);
        Ok((
            size,
            Rect::from_min_size(egui::pos2(0.0, ly), Vec2::new(size.x, in_h)),
            label,
            error,
            in_h,
        ))
    }
}
impl RenderObjectWidget for TextView {
    type Object = TextInputRender;
    fn create_render_object(&self, ctx: &RenderContext) -> Self::Object {
        TextInputRender {
            binding: self.binding.clone(),
            field: self.field.clone(),
            style: self.style.clone(),
            focus: self.focus.clone(),
            focus_target: None,
            focus_scope: self.focus_scope.clone(),
            scope_target: None,
            on_changed: self.on_changed.clone(),
            on_submitted: self.on_submitted.clone(),
            context: ctx.egui().clone(),
            handle: ctx.handle(),
            subscriptions: vec![],
            input_rect: Rect::NOTHING,
            label_galley: None,
            error_galley: None,
            input_height: 0.0,
            applied_source: None,
            last_text: String::new(),
            autofocus_done: false,
            keep_alive: None,
        }
    }
    fn update_render_object(&self, o: &mut Self::Object, ctx: &RenderContext) {
        let field_identity = !o.field.same_source(&self.field)
            || !o.binding.editor().same_source(&self.binding.editor());
        let focus_identity = match (&o.focus, &self.focus) {
            (Some(a), Some(b)) => !a.same_source(b),
            (None, None) => false,
            _ => true,
        };
        let scope_identity = match (&o.focus_scope, &self.focus_scope) {
            (Some(a), Some(b)) => a != b,
            (None, None) => false,
            _ => true,
        };
        let layout_changed =
            o.style != self.style || field_identity || focus_identity || scope_identity;
        o.binding = self.binding.clone();
        o.field = self.field.clone();
        o.style = self.style.clone();
        o.focus = self.focus.clone();
        if scope_identity {
            o.scope_target = None;
        }
        o.focus_scope = self.focus_scope.clone();
        o.on_changed = self.on_changed.clone();
        o.on_submitted = self.on_submitted.clone();
        if field_identity {
            o.applied_source = None;
            o.bind(ctx);
        }
        if focus_identity {
            o.bind_focus(ctx);
        }
        if layout_changed {
            let _ = o.handle.mark_needs_layout();
        }
    }
}
impl RenderBox for TextInputRender {
    fn attach(&mut self, ctx: &RenderContext) {
        self.bind(ctx);
        self.bind_focus(ctx);
    }
    fn layout(
        &mut self,
        c: BoxConstraints,
        children: &mut BoxChildren<'_>,
        _ctx: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        if !children.is_empty() {
            return Err(LayoutError::InvalidConfiguration(
                "text form field cannot have children",
            ));
        }
        self.binding.bind(self.field.clone());
        let (size, rect, label, error, h) = self.measure(c)?;
        self.input_rect = rect;
        self.label_galley = label;
        self.error_galley = error;
        self.input_height = h;
        let mut g = BoxGeometry::new(size);
        g.clip = Clip::HardEdge;
        g.hit_test_self = true;
        Ok(g)
    }
    fn dry_layout(&self, c: BoxConstraints, _: &BoxQueries<'_>) -> Result<Vec2, LayoutError> {
        Ok(self.measure(c)?.0)
    }
    fn intrinsic(
        &self,
        d: IntrinsicDimension,
        extent: f32,
        _: &BoxQueries<'_>,
    ) -> Result<f32, LayoutError> {
        let c = match d {
            IntrinsicDimension::MinWidth => {
                BoxConstraints::loose(Vec2::new(self.style.preferred_width, f32::INFINITY))?
            }
            IntrinsicDimension::MaxWidth => {
                BoxConstraints::loose(Vec2::new(f32::INFINITY, f32::INFINITY))?
            }
            IntrinsicDimension::MinHeight | IntrinsicDimension::MaxHeight => {
                BoxConstraints::loose(Vec2::new(extent, f32::INFINITY))?
            }
        };
        let s = self.measure(c)?.0;
        Ok(match d {
            IntrinsicDimension::MinWidth => self.style.preferred_width.min(s.x),
            IntrinsicDimension::MaxWidth => s.x,
            IntrinsicDimension::MinHeight | IntrinsicDimension::MaxHeight => s.y,
        })
    }
    fn paint(&mut self, ui: &mut egui::Ui, ctx: &RenderContext) {
        let id = self.input_id(ctx);
        if let Some(scope) = &self.focus_scope {
            scope.prepare_keyboard_navigation(ui.ctx());
        }
        let origin = ui.max_rect().min;
        let rect = self.input_rect.translate(origin.to_vec2());
        if let Some(g) = &self.label_galley {
            ui.painter().galley(
                origin,
                g.clone(),
                self.style.decoration.text_color.unwrap_or(Color32::WHITE),
            );
        }
        let text = self.binding.editor().text();
        let revision = self.binding.editor().revision();
        let source = (ctx.element_id().value(), revision);
        let source_changed = self.applied_source != Some(source);
        let editor = self.binding.editor();
        let mut state = egui::text_edit::TextEditState::load(ui.ctx(), id).unwrap_or_default();
        if source_changed {
            state.cursor.set_char_range(None);
            if let Some(selection) = editor.value().selection
                && let (Ok(base), Ok(extent)) = (
                    super::super::editing::utf16_to_char(&text, selection.base_offset),
                    super::super::editing::utf16_to_char(&text, selection.extent_offset),
                )
            {
                state.cursor.set_char_range(Some(CCursorRange {
                    primary: CCursor {
                        index: extent,
                        prefer_next_row: selection.affinity == TextAffinity::Downstream,
                    },
                    secondary: CCursor {
                        index: base,
                        prefer_next_row: false,
                    },
                    h_pos: None,
                }));
            }
            if self.last_text != text {
                state.clear_undoer();
            }
            state.store(ui.ctx(), id);
            self.applied_source = Some(source);
            self.last_text = text.clone();
        }
        if self.style.autofocus && !self.autofocus_done {
            ui.memory_mut(|m| m.request_focus(id));
            self.autofocus_done = true;
        }
        let snapshot = self.field.snapshot();
        let enabled = snapshot.enabled && !editor.is_disposed();
        let mut submitted = false;
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).id_salt(id));
        let mut value = text.clone();
        let mut immutable = text.as_str();
        let output = if self.style.multiline {
            if self.style.read_only {
                child
                    .add_enabled_ui(enabled, |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(rect.height())
                            .min_scrolled_height(rect.height())
                            .auto_shrink([false, false])
                            .id_salt((id, "viewport"))
                            .show(ui, |ui| {
                                text_editor(&mut immutable, &self.style, id, rect.width()).show(ui)
                            })
                    })
                    .inner
                    .inner
            } else {
                child
                    .add_enabled_ui(enabled, |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(rect.height())
                            .min_scrolled_height(rect.height())
                            .auto_shrink([false, false])
                            .id_salt((id, "viewport"))
                            .show(ui, |ui| {
                                text_editor(&mut value, &self.style, id, rect.width()).show(ui)
                            })
                    })
                    .inner
                    .inner
            }
        } else if self.style.read_only {
            child
                .add_enabled_ui(enabled, |ui| {
                    text_editor(&mut immutable, &self.style, id, rect.width()).show(ui)
                })
                .inner
        } else {
            child
                .add_enabled_ui(enabled, |ui| {
                    text_editor(&mut value, &self.style, id, rect.width()).show(ui)
                })
                .inner
        };
        let response = output.response.clone();
        if let Some(node) = &self.focus {
            node.sync(response.has_focus());
        }
        if let Some(scope) = self.focus_scope.clone() {
            let can_focus = response.enabled()
                && self
                    .focus
                    .as_ref()
                    .is_none_or(crate::focus::FocusNode::can_request_focus)
                && !crate::layout::ui_is_offstage(ui);
            if self
                .scope_target
                .as_ref()
                .is_some_and(|target| !target.is_for(&scope, id))
            {
                self.scope_target = None;
            }
            if self.scope_target.is_none() {
                self.scope_target = Some(scope.register_target(ui.ctx(), id, can_focus));
            } else {
                scope.update_target(ui.ctx(), id, can_focus);
            }
            scope.refresh_focus();
        } else {
            self.scope_target = None;
        }
        let changed = !self.style.read_only && output.response.changed() && value != text;
        if response.has_focus() {
            if self.keep_alive.is_none() {
                self.keep_alive = crate::scrolling::KeepAliveHandle::request(ctx)
                    .ok()
                    .flatten();
            }
        } else if response.lost_focus() {
            self.keep_alive = None;
            let _ = self.field.did_unfocus();
            submitted = ui.input(|i| i.key_pressed(egui::Key::Enter));
        }
        if changed {
            let cursor = output.cursor_range;
            let selection = cursor.map(|r| TextSelection {
                base_offset: super::super::editing::char_to_utf16(&value, r.secondary.index),
                extent_offset: super::super::editing::char_to_utf16(&value, r.primary.index),
                affinity: if r.primary.prefer_next_row {
                    TextAffinity::Downstream
                } else {
                    TextAffinity::Upstream
                },
            });
            let composing = if value == text {
                editor.value().composing
            } else {
                None
            };
            if let Err(e) = editor.set_value(TextEditingValue {
                text: value.clone(),
                selection,
                composing,
            }) {
                self.binding.fail(Some(e));
            } else {
                self.binding.call("text change", &self.on_changed);
            }
        } else if response.has_focus()
            && let Some(r) = output.cursor_range
        {
            let selection = Some(TextSelection {
                base_offset: super::super::editing::char_to_utf16(&text, r.secondary.index),
                extent_offset: super::super::editing::char_to_utf16(&text, r.primary.index),
                affinity: if r.primary.prefer_next_row {
                    TextAffinity::Downstream
                } else {
                    TextAffinity::Upstream
                },
            });
            if editor.value().selection != selection {
                let _ = editor.set_selection(selection);
            }
        }
        if submitted {
            self.binding.call("text submit", &self.on_submitted);
        }
        if let Some(g) = &self.error_galley {
            let y = rect.bottom() + 3.0;
            ui.painter().galley(
                egui::pos2(origin.x, y),
                g.clone(),
                self.style.decoration.error_color,
            );
        }
    }
    fn detach(&mut self) {
        for s in self.subscriptions.drain(..) {
            drop(s);
        }
        self.keep_alive = None;
        self.scope_target = None;
        if let Some((node, id)) = self.focus_target.take() {
            node.detach(id);
        }
    }
}

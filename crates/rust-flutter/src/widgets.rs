//! Composable layouts and controls; all egui widgets remain available via `egui`.
use egui::{Color32, InnerResponse, Response, RichText, Ui};

pub fn column<R>(ui: &mut Ui, children: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    ui.vertical(children)
}
pub fn row<R>(ui: &mut Ui, children: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    ui.horizontal(children)
}
pub fn wrap<R>(ui: &mut Ui, children: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    ui.horizontal_wrapped(children)
}
pub fn center<R>(ui: &mut Ui, children: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    ui.vertical_centered(children)
}
pub fn scroll<R>(
    ui: &mut Ui,
    id: impl std::hash::Hash,
    children: impl FnOnce(&mut Ui) -> R,
) -> egui::scroll_area::ScrollAreaOutput<R> {
    egui::ScrollArea::vertical().id_salt(id).show(ui, children)
}
pub fn card<R>(ui: &mut Ui, children: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    egui::Frame::group(ui.style())
        .inner_margin(16)
        .show(ui, children)
}
pub fn text(ui: &mut Ui, value: impl Into<egui::WidgetText>) -> Response {
    ui.label(value)
}
pub fn heading(ui: &mut Ui, value: impl Into<RichText>) -> Response {
    ui.heading(value)
}
pub fn button(ui: &mut Ui, label: impl Into<egui::WidgetText>) -> Response {
    ui.add_sized([120.0, 44.0], egui::Button::new(label))
}
pub fn text_field(
    ui: &mut Ui,
    id: impl std::hash::Hash,
    value: &mut String,
    hint: &str,
) -> Response {
    ui.add(
        egui::TextEdit::singleline(value)
            .id_salt(id)
            .hint_text(hint)
            .min_size(egui::vec2(0.0, 44.0)),
    )
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Breakpoint {
    Compact,
    Medium,
    Expanded,
}
impl Breakpoint {
    pub fn for_width(width: f32) -> Self {
        if width < 600.0 {
            Self::Compact
        } else if width < 840.0 {
            Self::Medium
        } else {
            Self::Expanded
        }
    }
}
#[derive(Clone, Copy)]
pub struct Theme {
    pub dark: bool,
    pub accent: Color32,
}
impl Default for Theme {
    fn default() -> Self {
        Self {
            dark: true,
            accent: Color32::from_rgb(105, 180, 255),
        }
    }
}
impl Theme {
    pub fn apply(self, ctx: &egui::Context) {
        let mut visuals = if self.dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        visuals.selection.bg_fill = self.accent;
        ctx.set_visuals(visuals);
    }
}

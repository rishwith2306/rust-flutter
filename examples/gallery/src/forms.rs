use rust_flutter::{
    egui,
    element::{ElementTree, FlexItem, FlexStyle, Widget},
    forms::{AutovalidateMode, Form, FormController, FormField},
    layout::{Axis, BoxConstraints, CrossAxisAlignment, MainAxisSize},
};
use std::{cell::RefCell, rc::Rc};
pub struct FormDemo {
    form: FormController,
    tree: ElementTree,
    saved: Rc<RefCell<String>>,
    status: Option<String>,
}
impl FormDemo {
    pub fn new(ctx: &egui::Context) -> Self {
        let form = FormController::new();
        form.set_autovalidate_mode(AutovalidateMode::OnUserInteraction);
        let saved = Rc::new(RefCell::new(String::new()));
        let result = saved.clone();
        let mut name = FormField::new(String::new(), |_, field| {
            Widget::leaf(egui::vec2(240.0, 76.0), move |ui| {
                ui.label("Name");
                let mut value = field.current_value();
                let response = ui.add(
                    egui::TextEdit::singleline(&mut value)
                        .desired_width(ui.available_width())
                        .background_color(egui::Color32::from_rgb(38, 59, 78))
                        .text_color(egui::Color32::WHITE),
                );
                if response.changed() {
                    let _ = field.did_change(value);
                }
                if response.lost_focus() {
                    let _ = field.did_unfocus();
                }
                if let Some(error) = field.error_text() {
                    ui.colored_label(egui::Color32::LIGHT_RED, error);
                }
                if let Some(error) = field.snapshot().validation_failure {
                    ui.colored_label(egui::Color32::LIGHT_RED, error.to_string());
                }
            })
        })
        .validator(|name| name.trim().is_empty().then(|| "Enter your name".into()))
        .on_saved(move |name| *result.borrow_mut() = name.clone());
        name.autovalidate_mode = AutovalidateMode::OnUserInteraction;
        let terms = FormField::new(false, |_, field| {
            Widget::leaf(egui::vec2(240.0, 56.0), move |ui| {
                ui.painter()
                    .rect_filled(ui.max_rect(), 0.0, egui::Color32::from_rgb(98, 77, 119));
                let mut accepted = field.current_value();
                if ui
                    .checkbox(
                        &mut accepted,
                        egui::RichText::new("Accept the example terms").color(egui::Color32::WHITE),
                    )
                    .changed()
                {
                    let _ = field.did_change(accepted);
                }
                if let Some(error) = field.error_text() {
                    ui.colored_label(egui::Color32::LIGHT_RED, error);
                }
            })
        })
        .validator(|accepted| (!accepted).then(|| "Accept the terms to submit".into()));
        let mut column = FlexStyle::new(Axis::Vertical);
        column.main_size = MainAxisSize::Min;
        column.cross_alignment = CrossAxisAlignment::Stretch;
        column.spacing = 4.0;
        let tree = ElementTree::new(
            ctx,
            Form::new(form.clone()).into_widget(Widget::flex(
                column,
                vec![
                    FlexItem::fixed(name.into_widget()),
                    FlexItem::fixed(terms.into_widget()),
                ],
            )),
        )
        .expect("valid form demo");
        Self {
            form,
            tree,
            saved,
            status: None,
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.label("Editing a field revalidates related fields after interaction.");
        if let Some(error) = self.form.last_auto_failure() {
            ui.colored_label(egui::Color32::LIGHT_RED, error.to_string());
        }
        if let Ok(c) = BoxConstraints::tight_for(Some(ui.available_width().min(480.0)), None)
            && let Err(error) = self.tree.show(ui, c)
        {
            ui.colored_label(egui::Color32::LIGHT_RED, error.to_string());
        }
        ui.horizontal(|ui| {
            if ui
                .add(egui::Button::new("Submit").fill(egui::Color32::from_rgb(66, 107, 88)))
                .clicked()
            {
                self.status = Some(match self.form.validate() {
                    Ok(true) => match self.form.save() {
                        Ok(()) => format!("Saved: {}", self.saved.borrow()),
                        Err(e) => e.to_string(),
                    },
                    Ok(false) => "Please fix the highlighted fields".into(),
                    Err(e) => e.to_string(),
                });
            }
            if ui
                .add(egui::Button::new("Reset form").fill(egui::Color32::from_rgb(147, 100, 59)))
                .clicked()
            {
                self.status = self.form.reset().err().map(|e| e.to_string());
                self.saved.borrow_mut().clear();
            }
        });
        if let Some(status) = &self.status {
            let color = if status.starts_with("Saved: ") {
                egui::Color32::from_rgb(63, 135, 94)
            } else {
                egui::Color32::from_rgb(179, 77, 69)
            };
            egui::Frame::new()
                .fill(color)
                .inner_margin(4.0)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new(status).color(egui::Color32::WHITE));
                });
        }
    }
}

use rust_flutter::{
    egui,
    element::{ElementTree, Widget},
    gestures::GestureBinding,
    layout::BoxConstraints,
    scrolling::{ListView, ScrollPosition, Scrollbar},
};
use std::{cell::RefCell, rc::Rc};
const COUNT: usize = 10_002;
#[derive(Default)]
struct Batch {
    count: usize,
    first: Option<usize>,
    last: Option<usize>,
}
pub struct LazyListDemo {
    tree: ElementTree,
    position: ScrollPosition,
    binding: GestureBinding,
    batch: Rc<RefCell<Batch>>,
    hidden: bool,
    last_toggle_frame: Option<u64>,
}
impl LazyListDemo {
    pub fn new(ctx: &egui::Context) -> Self {
        let position = ScrollPosition::new(ctx, 0.0).expect("valid list position");
        let binding = GestureBinding::new();
        let batch = Rc::new(RefCell::new(Batch::default()));
        let tree = ElementTree::new(ctx, Self::root(&position, &binding, &batch, false))
            .expect("valid list tree");
        Self {
            tree,
            position,
            binding,
            batch,
            hidden: false,
            last_toggle_frame: None,
        }
    }
    fn root(
        position: &ScrollPosition,
        binding: &GestureBinding,
        batch: &Rc<RefCell<Batch>>,
        hidden: bool,
    ) -> Widget {
        let record = batch.clone();
        let list = ListView::builder(position, binding, COUNT, 60.0, move |_, index| {
            let mut batch = record.borrow_mut();
            batch.count += 1;
            batch.first = Some(batch.first.map_or(index, |old| old.min(index)));
            batch.last = Some(batch.last.map_or(index, |old| old.max(index)));
            drop(batch);
            let color = if index == 0 {
                egui::Color32::from_rgb(23, 105, 91)
            } else if index == COUNT - 1 {
                egui::Color32::from_rgb(201, 152, 67)
            } else {
                let (r, g, b) =
                    [(46, 81, 92), (100, 77, 111), (158, 95, 57), (67, 108, 81)][index % 4];
                egui::Color32::from_rgb(r, g, b)
            };
            Widget::leaf(egui::vec2(100.0, 60.0), move |ui| {
                let rect = ui.max_rect();
                ui.painter().rect_filled(rect, 0.0, color);
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("Row {index}"),
                    egui::FontId::proportional(18.0),
                    egui::Color32::WHITE,
                );
            })
        })
        .expect("valid list")
        .cache_extent(120.0)
        .expect("valid cache")
        .max_realized_items(16)
        .expect("valid item budget")
        .into_widget()
        .expect("valid list widget");
        let root = Scrollbar::new(position, binding)
            .expect("valid scrollbar")
            .into_widget(list)
            .expect("valid scrollbar child");
        Widget::offstage(hidden, root)
    }
    pub fn cancel_input(&self) {
        self.binding.cancel_all();
        let _ = self.position.cancel_activity();
    }
    pub fn show(&mut self, ui: &mut egui::Ui) {
        let mut toggle = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F8));
        ui.horizontal_wrapped(|ui| {
            ui.label("10,002 rows · 60 points each");
            if ui
                .add(egui::Button::new("Reset list").fill(egui::Color32::from_rgb(77, 101, 82)))
                .clicked()
            {
                let _ = self.position.jump_to(0.0);
            }
            toggle |= ui
                .add(
                    egui::Button::new(if self.hidden {
                        "Show list (F8)"
                    } else {
                        "Hide list (F8)"
                    })
                    .fill(egui::Color32::from_rgb(91, 74, 117)),
                )
                .clicked();
        });
        let frame_nr = ui.ctx().cumulative_frame_nr();
        if toggle && self.last_toggle_frame != Some(frame_nr) {
            self.last_toggle_frame = Some(frame_nr);
            self.hidden = !self.hidden;
            self.tree
                .update(Self::root(
                    &self.position,
                    &self.binding,
                    &self.batch,
                    self.hidden,
                ))
                .expect("valid offstage update");
            ui.ctx().request_repaint();
        }
        *self.batch.borrow_mut() = Batch::default();
        let mut frame = self
            .binding
            .begin_egui_frame(ui.ctx())
            .expect("one list scene");
        let result = BoxConstraints::loose(egui::vec2(ui.available_width(), 240.0))
            .map_err(rust_flutter::element::ElementError::from)
            .and_then(|c| self.tree.show_in_gesture_frame(ui, c, &mut frame));
        if let Err(error) = result {
            ui.colored_label(egui::Color32::LIGHT_RED, error.to_string());
        }
        let batch = self.batch.borrow();
        if batch.count > 0 {
            println!(
                "rust-flutter gallery lazy-list built={} first={} last={} total={COUNT}",
                batch.count,
                batch.first.unwrap(),
                batch.last.unwrap()
            );
        }
        drop(batch);
        let report = frame.finish().expect("valid list scene");
        assert!(
            report.failures.is_empty(),
            "list input failed: {:?}",
            report.failures
        );
        ui.label(format!("Offset: {:.1}", self.position.pixels()));
    }
}

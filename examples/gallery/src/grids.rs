use rust_flutter::{
    egui,
    element::{ElementTree, Widget},
    gestures::GestureBinding,
    layout::{Axis, BoxConstraints},
    scrolling::{GridView, ScrollPosition, Scrollbar},
};
use std::{cell::Cell, rc::Rc};
const COUNT: usize = 4_000;

pub struct LazyGridDemo {
    tree: ElementTree,
    position: ScrollPosition,
    binding: GestureBinding,
    axis: Axis,
    built: Rc<Cell<usize>>,
}

impl LazyGridDemo {
    pub fn new(ctx: &egui::Context) -> Self {
        let position = ScrollPosition::new(ctx, 0.0).expect("valid grid position");
        let binding = GestureBinding::new();
        let built = Rc::new(Cell::new(0));
        let tree = ElementTree::new(ctx, Self::root(&position, &binding, Axis::Vertical, &built))
            .expect("valid grid tree");
        Self {
            tree,
            position,
            binding,
            axis: Axis::Vertical,
            built,
        }
    }

    fn root(
        position: &ScrollPosition,
        binding: &GestureBinding,
        axis: Axis,
        built: &Rc<Cell<usize>>,
    ) -> Widget {
        let built = built.clone();
        let grid = GridView::builder(position, binding, COUNT, 3, 78.0, move |_, index| {
            built.set(built.get() + 1);
            let colors = [
                egui::Color32::from_rgb(46, 81, 92),
                egui::Color32::from_rgb(100, 77, 111),
                egui::Color32::from_rgb(158, 95, 57),
                egui::Color32::from_rgb(67, 108, 81),
            ];
            let color = colors[index % colors.len()];
            Widget::leaf(egui::vec2(90.0, 78.0), move |ui| {
                let rect = ui.max_rect();
                ui.painter().rect_filled(rect, 6.0, color);
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("Cell {index}"),
                    egui::FontId::proportional(16.0),
                    egui::Color32::WHITE,
                );
            })
        })
        .expect("valid grid")
        .axis(axis)
        .main_axis_spacing(8.0)
        .expect("valid main-axis spacing")
        .cross_axis_spacing(8.0)
        .expect("valid cross-axis spacing")
        .cache_extent(120.0)
        .expect("valid grid cache")
        .max_realized_items(512)
        .expect("valid grid build budget")
        .into_widget()
        .expect("valid grid widget");
        Scrollbar::new(position, binding)
            .expect("valid grid scrollbar")
            .into_widget(grid)
            .expect("valid scrollbar child")
    }

    pub fn cancel_input(&self) {
        self.binding.cancel_all();
        let _ = self.position.cancel_activity();
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("4,000 cells · 3 columns or rows · lazy build");
            for (axis, label) in [
                (Axis::Vertical, "Vertical grid"),
                (Axis::Horizontal, "Horizontal grid"),
            ] {
                if ui.selectable_label(self.axis == axis, label).clicked() && self.axis != axis {
                    self.axis = axis;
                    self.position.jump_to(0.0).expect("valid grid reset");
                    self.tree
                        .update(Self::root(
                            &self.position,
                            &self.binding,
                            self.axis,
                            &self.built,
                        ))
                        .expect("valid grid orientation update");
                }
            }
            if ui.button("Reset").clicked() {
                self.position.jump_to(0.0).expect("valid grid reset");
            }
        });
        self.built.set(0);
        let mut frame = self
            .binding
            .begin_egui_frame(ui.ctx())
            .expect("one grid scene");
        let constraints = BoxConstraints::loose(egui::vec2(ui.available_width(), 260.0));
        let result = constraints
            .map_err(rust_flutter::element::ElementError::from)
            .and_then(|constraints| self.tree.show_in_gesture_frame(ui, constraints, &mut frame));
        if let Err(error) = result {
            ui.colored_label(egui::Color32::LIGHT_RED, error.to_string());
        }
        let report = frame.finish().expect("valid grid input frame");
        assert!(
            report.failures.is_empty(),
            "grid input failed: {:?}",
            report.failures
        );
        ui.label(format!(
            "Built this frame: {} · offset: {:.1}",
            self.built.get(),
            self.position.pixels()
        ));
    }
}

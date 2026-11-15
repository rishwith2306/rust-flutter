use rust_flutter::{
    egui,
    element::*,
    layout::{Axis, BoxConstraints, CrossAxisAlignment},
};
use std::{cell::RefCell, rc::Rc};
#[derive(Default)]
struct Metrics {
    initialized: usize,
    updated: usize,
    disposed: usize,
    dependencies: usize,
}
#[derive(PartialEq)]
struct CounterStep(i32);
struct Counter {
    name: &'static str,
    metrics: Rc<RefCell<Metrics>>,
}
struct CounterState {
    value: i32,
    metrics: Rc<RefCell<Metrics>>,
}
impl StatefulWidget for Counter {
    type State = CounterState;
    fn create_state(&self) -> CounterState {
        CounterState {
            value: 0,
            metrics: self.metrics.clone(),
        }
    }
}
impl State<Counter> for CounterState {
    fn did_change_dependencies(&mut self, _: &Counter, _: &StateContext<Self>) {
        self.metrics.borrow_mut().dependencies += 1;
    }
    fn init_state(&mut self, _: &Counter, _: &StateContext<Self>) {
        self.metrics.borrow_mut().initialized += 1;
    }
    fn did_update_widget(&mut self, _: &Counter, _: &Counter, _: &StateContext<Self>) {
        self.metrics.borrow_mut().updated += 1;
    }
    fn build(&mut self, widget: &Counter, ctx: &StateContext<Self>) -> Widget {
        let step = ctx
            .depend_on::<CounterStep>()
            .expect("lookup while building")
            .map_or(1, |step| step.0);
        let value = self.value;
        let name = widget.name;
        let handle = ctx.handle();
        Widget::leaf(egui::vec2(120.0, 80.0), move |ui| {
            let color = if name == "A" {
                egui::Color32::from_rgb(23, 105, 170)
            } else {
                egui::Color32::from_rgb(25, 125, 85)
            };
            if ui
                .put(
                    ui.max_rect(),
                    egui::Button::new(
                        egui::RichText::new(format!("{name}: {value}"))
                            .size(24.0)
                            .color(egui::Color32::WHITE),
                    )
                    .fill(color),
                )
                .clicked()
            {
                let _ = handle.update(move |state| state.value = state.value.saturating_add(step));
            }
        })
    }
    fn dispose(&mut self) {
        self.metrics.borrow_mut().disposed += 1;
    }
}
struct List {
    reversed: bool,
    show_b: bool,
    metrics: Rc<RefCell<Metrics>>,
}
impl StatelessWidget for List {
    fn build(&self, _: &BuildContext) -> Widget {
        let mut children = vec![
            Widget::stateful(Counter {
                name: "A",
                metrics: self.metrics.clone(),
            })
            .keyed("A"),
        ];
        if self.show_b {
            children.push(
                Widget::stateful(Counter {
                    name: "B",
                    metrics: self.metrics.clone(),
                })
                .keyed("B"),
            );
        }
        if self.reversed {
            children.reverse();
        }
        let mut style = FlexStyle::new(Axis::Horizontal);
        style.spacing = 8.0;
        style.cross_alignment = CrossAxisAlignment::Stretch;
        Widget::flex(
            style,
            children
                .into_iter()
                .map(|child| FlexItem::expanded(child, 1))
                .collect(),
        )
    }
}
pub(super) struct ComponentDemo {
    tree: ElementTree,
    reversed: bool,
    show_b: bool,
    metrics: Rc<RefCell<Metrics>>,
    step: i32,
}
impl ComponentDemo {
    pub fn new(ctx: &egui::Context) -> Self {
        let metrics = Rc::default();
        let root = Widget::inherited(
            CounterStep(1),
            Widget::stateless(List {
                reversed: false,
                show_b: true,
                metrics: Rc::clone(&metrics),
            }),
        );
        Self {
            tree: ElementTree::new(ctx, root).expect("valid component demo"),
            reversed: false,
            show_b: true,
            metrics,
            step: 1,
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.label("Tap A, then swap. Its local state follows its key. Removing B disposes it; adding it again creates fresh state.");
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            if ui.button("Swap keyed children").clicked() {
                self.reversed = !self.reversed;
                changed = true;
            }
            if ui
                .button(if self.show_b { "Remove B" } else { "Add B" })
                .clicked()
            {
                self.show_b = !self.show_b;
                changed = true;
            }
            if ui.button(format!("Step: {}", self.step)).clicked() {
                self.step = if self.step == 1 { 2 } else { 1 };
                if let Some(handle) = self
                    .tree
                    .root_id()
                    .and_then(|id| self.tree.inherited_handle::<CounterStep>(id))
                {
                    let _ = handle.set(CounterStep(self.step));
                }
            }
        });
        if changed
            && let Err(error) = self.tree.update(Widget::inherited(
                CounterStep(self.step),
                Widget::stateless(List {
                    reversed: self.reversed,
                    show_b: self.show_b,
                    metrics: self.metrics.clone(),
                }),
            ))
        {
            ui.colored_label(egui::Color32::LIGHT_RED, error.to_string());
        }
        match BoxConstraints::tight(egui::vec2(ui.available_width(), 80.0)) {
            Ok(constraints) => {
                if let Err(error) = self.tree.show(ui, constraints) {
                    ui.colored_label(egui::Color32::LIGHT_RED, error.to_string());
                }
            }
            Err(error) => {
                ui.colored_label(egui::Color32::LIGHT_RED, error.to_string());
            }
        }
        let metrics = self.metrics.borrow();
        ui.label(format!(
            "init: {} · props updated: {} · disposed: {} · dependencies: {}",
            metrics.initialized, metrics.updated, metrics.disposed, metrics.dependencies
        ));
        ui.label("Step changes notify both counters through an inherited value, preserving their counts.");
        ui.label(
            "These counters belong to mounted components. App restart creates fresh components.",
        );
    }
}

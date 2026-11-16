use rust_flutter::{
    display::{ViewAspect, ViewMetrics},
    egui,
    element::*,
    layout::{
        Axis, BoxChildren, BoxConstraints, BoxGeometry, BoxQueries, CrossAxisAlignment,
        IntrinsicDimension, LayoutError,
    },
    notifier::ValueNotifier,
};
use std::{cell::RefCell, collections::HashSet, rc::Rc};
#[derive(Default)]
struct Metrics {
    initialized: usize,
    updated: usize,
    disposed: usize,
    dependencies: usize,
    builds_a: usize,
    builds_b: usize,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum CounterAspect {
    A,
    B,
}
struct CounterStep {
    a: i32,
    b: i32,
}
impl InheritedModel for CounterStep {
    type Aspect = CounterAspect;
    fn update_should_notify(&self, old: &Self) -> bool {
        self.a != old.a || self.b != old.b
    }
    fn update_should_notify_dependent(&self, old: &Self, aspects: &HashSet<CounterAspect>) -> bool {
        (self.a != old.a && aspects.contains(&CounterAspect::A))
            || (self.b != old.b && aspects.contains(&CounterAspect::B))
    }
}
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
        let aspect = if widget.name == "A" {
            CounterAspect::A
        } else {
            CounterAspect::B
        };
        let step = ctx
            .depend_on_aspect::<CounterStep>(aspect)
            .expect("lookup while building")
            .map_or(1, |step| match aspect {
                CounterAspect::A => step.a,
                CounterAspect::B => step.b,
            });
        {
            let mut metrics = self.metrics.borrow_mut();
            match aspect {
                CounterAspect::A => metrics.builds_a += 1,
                CounterAspect::B => metrics.builds_b += 1,
            }
        }
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
struct ViewReportRender;
struct ViewReportBox {
    layouts: usize,
}
impl RenderObjectWidget for ViewReportRender {
    type Object = ViewReportBox;
    fn create_render_object(&self, _: &RenderContext) -> Self::Object {
        #[cfg(debug_assertions)]
        eprintln!("rust-flutter gallery render-view create");
        ViewReportBox { layouts: 0 }
    }
    fn update_render_object(&self, _: &mut Self::Object, _: &RenderContext) {
        #[cfg(debug_assertions)]
        eprintln!("rust-flutter gallery render-view update");
    }
}
impl RenderBox for ViewReportBox {
    fn dry_layout(
        &self,
        c: BoxConstraints,
        children: &BoxQueries<'_>,
    ) -> Result<egui::Vec2, LayoutError> {
        children.dry_layout(0, c)
    }
    fn intrinsic(
        &self,
        d: IntrinsicDimension,
        extent: f32,
        children: &BoxQueries<'_>,
    ) -> Result<f32, LayoutError> {
        children.intrinsic(0, d, extent)
    }
    fn layout(
        &mut self,
        constraints: BoxConstraints,
        children: &mut BoxChildren<'_>,
        _: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        let size = children.layout(0, constraints)?;
        children.place(0, egui::Vec2::ZERO)?;
        self.layouts += 1;
        #[cfg(debug_assertions)]
        eprintln!(
            "rust-flutter gallery render-view layout={} size={}x{}",
            self.layouts, size.x, size.y
        );
        Ok(BoxGeometry::new(size))
    }
    fn dispose(&mut self) {
        #[cfg(debug_assertions)]
        eprintln!("rust-flutter gallery render-view dispose");
    }
}
struct ViewReport;
#[derive(Default)]
struct ViewReportState {
    builds: usize,
}
impl StatefulWidget for ViewReport {
    type State = ViewReportState;
    fn create_state(&self) -> Self::State {
        ViewReportState::default()
    }
}
impl State<ViewReport> for ViewReportState {
    fn build(&mut self, _: &ViewReport, ctx: &StateContext<Self>) -> Widget {
        let mut view = None;
        for aspect in [
            ViewAspect::Size,
            ViewAspect::Keyboard,
            ViewAspect::ViewInsets,
        ] {
            view = ctx
                .depend_on_aspect::<ViewMetrics>(aspect)
                .expect("mounted view query");
        }
        let view = view.expect("ambient view metrics");
        self.builds += 1;
        let size = view.viewport.size();
        #[cfg(debug_assertions)]
        eprintln!(
            "rust-flutter gallery inherited-view build={} size={}x{} keyboard={:?}",
            self.builds, size.x, size.y, view.keyboard
        );
        let text = format!(
            "View: {:.0} × {:.0} · keyboard: {:?} · view builds: {}",
            size.x, size.y, view.keyboard, self.builds
        );
        Widget::render_object(
            ViewReportRender,
            vec![Widget::leaf(egui::vec2(0.0, 28.0), move |ui| {
                ui.label(&text);
            })],
        )
    }
}

pub(super) struct ComponentDemo {
    tree: ElementTree,
    reversed: bool,
    show_b: bool,
    metrics: Rc<RefCell<Metrics>>,
    step: i32,
    a_step: i32,
    observable: ValueNotifier<i32>,
    observable_tree: ElementTree,
    view_tree: ElementTree,
    last_view_query: Option<egui::Vec2>,
}
impl ComponentDemo {
    pub fn new(ctx: &egui::Context) -> Self {
        let metrics = Rc::default();
        let root = Widget::inherited_model(
            CounterStep { a: 1, b: 1 },
            Widget::stateless(List {
                reversed: false,
                show_b: true,
                metrics: Rc::clone(&metrics),
            }),
        );
        let observable = ValueNotifier::new(0_i32);
        let counter = observable.clone();
        let observable_tree = ElementTree::new(
            ctx,
            Widget::value_listenable_builder(observable.clone(), move |_, value| {
                let counter = counter.clone();
                Widget::leaf(egui::vec2(280.0, 44.0), move |ui| {
                    if ui
                        .put(
                            ui.max_rect(),
                            egui::Button::new(
                                egui::RichText::new(format!("Observable: {value}"))
                                    .size(22.0)
                                    .color(egui::Color32::WHITE),
                            )
                            .fill(egui::Color32::from_rgb(161, 75, 50)),
                        )
                        .clicked()
                    {
                        let _ = counter.update(|value| value.saturating_add(1));
                    }
                })
            }),
        )
        .expect("valid observable demo");
        Self {
            observable,
            observable_tree,
            last_view_query: None,
            view_tree: ElementTree::new(ctx, Widget::stateful(ViewReport))
                .expect("valid view report"),
            tree: ElementTree::new(ctx, root).expect("valid component demo"),
            reversed: false,
            show_b: true,
            metrics,
            step: 1,
            a_step: 1,
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
                self.a_step = self.step;
                if let Some(handle) = self
                    .tree
                    .root_id()
                    .and_then(|id| self.tree.inherited_handle::<CounterStep>(id))
                {
                    let _ = handle.set(CounterStep {
                        a: self.a_step,
                        b: self.step,
                    });
                }
            }
            if ui
                .add(
                    egui::Button::new(format!("A step: {}", self.a_step))
                        .fill(egui::Color32::from_rgb(110, 65, 135)),
                )
                .clicked()
            {
                self.a_step = if self.a_step == 1 { 2 } else { 1 };
                let step = self.a_step;
                if let Some(handle) = self
                    .tree
                    .root_id()
                    .and_then(|id| self.tree.inherited_handle::<CounterStep>(id))
                {
                    let _ = handle.update(move |old| CounterStep { a: step, b: old.b });
                }
            }
        });
        if changed
            && let Err(error) = self.tree.update(Widget::inherited_model(
                CounterStep {
                    a: self.a_step,
                    b: self.step,
                },
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
        ui.label(format!(
            "Builds A: {} · B: {}",
            metrics.builds_a, metrics.builds_b
        ));
        ui.label("Step changes notify both counters. A step changes only A's model aspect; B does not rebuild.");
        ui.label(
            "These counters belong to mounted components. App restart creates fresh components.",
        );
        ui.separator();
        ui.label(
            "Observable state: click the counter, or emit three changes before the next frame.",
        );
        ui.horizontal_wrapped(|ui| {
            if ui
                .add(
                    egui::Button::new("Emit three changes")
                        .fill(egui::Color32::from_rgb(143, 95, 34)),
                )
                .clicked()
            {
                for _ in 0..3 {
                    let _ = self.observable.update(|value| value.saturating_add(1));
                }
            }
            if ui
                .add(
                    egui::Button::new("Reset observable")
                        .fill(egui::Color32::from_rgb(75, 98, 125)),
                )
                .clicked()
            {
                let _ = self.observable.set(0);
            }
        });
        if let Ok(constraints) = BoxConstraints::tight(egui::vec2(ui.available_width(), 44.0))
            && let Err(error) = self.observable_tree.show(ui, constraints)
        {
            ui.colored_label(egui::Color32::LIGHT_RED, error.to_string());
        }
        if let Ok(constraints) = BoxConstraints::tight(egui::vec2(ui.available_width(), 28.0)) {
            let dry = self.view_tree.dry_layout(constraints);
            let shown = self.view_tree.show(ui, constraints);
            match (dry, shown) {
                (Ok(dry), Ok(shown)) => {
                    let actual = shown.rect.size();
                    let matched = (dry - actual).abs().max_elem() < 0.01;
                    if self.last_view_query != Some(dry) {
                        #[cfg(debug_assertions)]
                        eprintln!(
                            "rust-flutter gallery render-view query dry={}x{} actual={}x{} matched={matched}",
                            dry.x, dry.y, actual.x, actual.y
                        );
                        self.last_view_query = Some(dry);
                    }
                    if !matched {
                        ui.colored_label(
                            egui::Color32::LIGHT_RED,
                            "Dry and actual view layout differ",
                        );
                    }
                }
                (Err(error), _) | (_, Err(error)) => {
                    ui.colored_label(egui::Color32::LIGHT_RED, error.to_string());
                }
            }
        }
    }
}

use rust_flutter::{
    display::{ViewAspect, ViewMetrics},
    egui,
    element::*,
    gestures::*,
    layout::{
        Axis, BoxChildren, BoxConstraints, BoxGeometry, BoxQueries, CrossAxisAlignment,
        IntrinsicDimension, LayoutError, TextBaseline,
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

struct AnimatedMarker {
    binding: GestureBinding,
}
struct AnimatedMarkerState {
    expanded: bool,
    dragged: bool,
    released: bool,
}
impl StatefulWidget for AnimatedMarker {
    type State = AnimatedMarkerState;
    fn create_state(&self) -> Self::State {
        AnimatedMarkerState {
            expanded: false,
            dragged: false,
            released: false,
        }
    }
}
impl State<AnimatedMarker> for AnimatedMarkerState {
    fn build(&mut self, widget: &AnimatedMarker, ctx: &StateContext<Self>) -> Widget {
        let tap = ctx.handle();
        let drag = ctx.handle();
        let end = ctx.handle();
        #[cfg(debug_assertions)]
        let tap_binding = widget.binding.downgrade();
        #[cfg(debug_assertions)]
        let drag_binding = widget.binding.downgrade();
        let config = GestureTargetConfig {
            on_scroll: None,
            tap: Some((
                TapConfig::default(),
                TapCallbacks {
                    on_tap: Some(Rc::new(move |_| {
                        #[cfg(debug_assertions)]
                        eprintln!("rust-flutter gallery gesture-arena tap accepted");
                        #[cfg(debug_assertions)]
                        if let Some(binding) = tap_binding.upgrade() {
                            eprintln!(
                                "rust-flutter gallery gesture-binding tap targets={} pointers={}",
                                binding.target_count(),
                                binding.active_pointers()
                            );
                        }
                        let _ = tap.update(|state| state.expanded = !state.expanded);
                    })),
                    on_cancel: Some(Rc::new(|_| {
                        #[cfg(debug_assertions)]
                        eprintln!("rust-flutter gallery gesture-arena tap rejected");
                    })),
                    ..Default::default()
                },
            )),
            pan: Some((
                PanConfig::default(),
                PanCallbacks {
                    on_end: Some(Rc::new(move |_details| {
                        #[cfg(debug_assertions)]
                        if let Some(estimate) = _details.estimate {
                            let velocity = _details.velocity.pixels_per_second();
                            let raw = estimate.velocity.pixels_per_second();
                            eprintln!(
                                "rust-flutter gallery pan-end vx={} vy={} raw_x={} raw_y={} confidence={} samples={} age_ms={} status={:?} fling={}",
                                velocity.x,
                                velocity.y,
                                raw.x,
                                raw.y,
                                estimate.confidence,
                                estimate.samples,
                                estimate.age.as_secs_f64() * 1000.0,
                                estimate.status,
                                _details.is_fling
                            );
                        }
                        let _ = end.update(|state| state.released = true);
                    })),
                    on_start: Some(Rc::new(move |_| {
                        #[cfg(debug_assertions)]
                        eprintln!("rust-flutter gallery gesture-arena drag accepted");
                        #[cfg(debug_assertions)]
                        if let Some(binding) = drag_binding.upgrade() {
                            eprintln!(
                                "rust-flutter gallery gesture-binding pan targets={} pointers={}",
                                binding.target_count(),
                                binding.active_pointers()
                            );
                        }
                        let _ = drag.update(|state| state.dragged = true);
                    })),
                    ..Default::default()
                },
            )),
        };
        let dragged = self.dragged;
        let released = self.released;
        let marker = rust_flutter::animation::TweenAnimationBuilder::new(
            24.0_f32,
            if self.expanded { 48.0 } else { 24.0 },
            std::time::Duration::from_secs(2),
            move |_, width| {
                #[cfg(debug_assertions)]
                eprintln!(
                    "rust-flutter gallery animation value={}",
                    (width - 24.0) / 24.0
                );
                let decorated = rust_flutter::decoration::DecoratedBox {
                    decoration: rust_flutter::decoration::BoxDecoration::solid(
                        egui::Color32::from_rgb(131, 78, 179),
                    ),
                    position: rust_flutter::decoration::DecorationPosition::Foreground,
                }
                .into_widget(Widget::leaf(egui::vec2(width, 24.0), move |ui| {
                    ui.painter().rect_filled(
                        ui.max_rect(),
                        0.0,
                        egui::Color32::from_rgb(227, 189, 66),
                    );
                }));
                Widget::stack(
                    StackStyle::default(),
                    vec![
                        StackItem::new(decorated),
                        StackItem::positioned(
                            Widget::leaf(egui::Vec2::ZERO, move |ui| {
                                if released {
                                    ui.painter().rect_filled(
                                        ui.max_rect(),
                                        0.0,
                                        egui::Color32::from_rgb(72, 199, 189),
                                    );
                                }
                            }),
                            rust_flutter::layout::Positioned {
                                right: Some(2.0),
                                top: Some(2.0),
                                width: Some(4.0),
                                height: Some(4.0),
                                ..Default::default()
                            },
                        ),
                        StackItem::positioned(
                            Widget::leaf(egui::Vec2::ZERO, move |ui| {
                                if dragged {
                                    ui.painter().rect_filled(
                                        ui.max_rect(),
                                        0.0,
                                        egui::Color32::from_rgb(210, 97, 141),
                                    );
                                }
                            }),
                            rust_flutter::layout::Positioned {
                                right: Some(2.0),
                                bottom: Some(2.0),
                                width: Some(4.0),
                                height: Some(4.0),
                                ..Default::default()
                            },
                        ),
                    ],
                )
            },
        )
        .expect("valid implicit marker")
        .on_end(|| {
            #[cfg(debug_assertions)]
            eprintln!("rust-flutter gallery implicit-animation end");
        })
        .into_widget();
        let mut row = FlexStyle::new(Axis::Horizontal);
        row.main_size = rust_flutter::layout::MainAxisSize::Min;
        row.spacing = 8.0;
        GestureDetector::new(&widget.binding, config)
            .expect("valid marker detector")
            .into_widget(Widget::flex(
                row,
                vec![
                    FlexItem::fixed(marker),
                    FlexItem::fixed(Widget::stateful(SpringMarker {
                        expanded: self.expanded,
                    })),
                    FlexItem::fixed(Widget::stateful(CurvedMarker {
                        expanded: self.expanded,
                    })),
                ],
            ))
    }
}
struct SpringMarker {
    expanded: bool,
}
struct SpringMarkerState {
    controller: Option<rust_flutter::animation::AnimationController>,
}
impl StatefulWidget for SpringMarker {
    type State = SpringMarkerState;
    fn create_state(&self) -> Self::State {
        SpringMarkerState { controller: None }
    }
}
impl State<SpringMarker> for SpringMarkerState {
    fn init_state(&mut self, _: &SpringMarker, ctx: &StateContext<Self>) {
        self.controller = Some(
            ctx.animation_controller(rust_flutter::animation::AnimationConfig {
                lower_bound: 12.0,
                upper_bound: 24.0,
                value: Some(12.0),
                ..Default::default()
            })
            .expect("valid spring marker bounds"),
        );
    }
    fn did_update_widget(
        &mut self,
        old: &SpringMarker,
        new: &SpringMarker,
        _: &StateContext<Self>,
    ) {
        if old.expanded != new.expanded {
            let controller = self.controller.as_ref().unwrap();
            let motion = rust_flutter::physics::SpringSimulation::with_options(
                rust_flutter::physics::SpringDescription {
                    mass: 1.0,
                    stiffness: 64.0,
                    damping: 16.0,
                },
                f64::from(controller.value()),
                if new.expanded { 24.0 } else { 12.0 },
                0.0,
                rust_flutter::physics::Tolerance::default(),
                true,
            )
            .expect("valid spring marker motion");
            controller
                .animate_with(Rc::new(motion))
                .expect("mounted spring marker");
        }
    }
    fn build(&mut self, _: &SpringMarker, _: &StateContext<Self>) -> Widget {
        let controller = self.controller.as_ref().unwrap().clone();
        #[cfg(debug_assertions)]
        let weak = controller.downgrade();
        Widget::value_listenable_builder(controller, move |_, width| {
            #[cfg(debug_assertions)]
            eprintln!(
                "rust-flutter gallery spring width={} active={}",
                width,
                weak.upgrade().is_some_and(|c| c.is_animating())
            );
            Widget::leaf(egui::vec2(width, 12.0), |ui| {
                ui.painter()
                    .rect_filled(ui.max_rect(), 0.0, egui::Color32::from_rgb(200, 133, 55));
            })
        })
    }
}
struct CurvedMarker {
    expanded: bool,
}
type MarkerSizeAnimation = rust_flutter::animation::DrivenAnimation<
    rust_flutter::animation::CurvedAnimation<rust_flutter::animation::AnimationController>,
    rust_flutter::animation::ValueTween<egui::Vec2>,
>;
struct CurvedMarkerState {
    curve: Option<
        rust_flutter::animation::CurvedAnimation<rust_flutter::animation::AnimationController>,
    >,
    size: Option<MarkerSizeAnimation>,
}
impl StatefulWidget for CurvedMarker {
    type State = CurvedMarkerState;
    fn create_state(&self) -> Self::State {
        CurvedMarkerState {
            curve: None,
            size: None,
        }
    }
}
impl State<CurvedMarker> for CurvedMarkerState {
    fn init_state(&mut self, _: &CurvedMarker, ctx: &StateContext<Self>) {
        use rust_flutter::animation::{AnimationConfig, Cubic, CurvedAnimation};
        let parent = ctx
            .animation_controller(AnimationConfig {
                duration: std::time::Duration::from_secs(2),
                ..Default::default()
            })
            .expect("valid curved marker parent");
        let curve = CurvedAnimation::new(parent, Cubic::new(0.42, 0.0, 0.58, 1.0).unwrap())
            .expect("valid derived marker");
        curve
            .set_reverse_curve(Some(Rc::new(Cubic::new(0.0, 0.0, 0.58, 1.0).unwrap())))
            .expect("valid reverse curve");
        use rust_flutter::animation::{AnimationDrive, ValueTween};
        self.size = Some(
            curve
                .clone()
                .drive(ValueTween::new(egui::vec2(12.0, 12.0), egui::vec2(24.0, 12.0)).unwrap())
                .expect("valid typed size animation"),
        );
        self.curve = Some(curve);
    }
    fn did_update_widget(
        &mut self,
        old: &CurvedMarker,
        new: &CurvedMarker,
        _: &StateContext<Self>,
    ) {
        if old.expanded != new.expanded {
            let parent = self.curve.as_ref().unwrap().parent();
            if new.expanded {
                parent.forward()
            } else {
                parent.reverse()
            }
            .expect("mounted derived marker");
        }
    }
    fn build(&mut self, _: &CurvedMarker, _: &StateContext<Self>) -> Widget {
        #[cfg(debug_assertions)]
        let weak = self.curve.as_ref().unwrap().downgrade();
        Widget::value_listenable_builder(self.size.as_ref().unwrap().clone(), move |_, size| {
            #[cfg(debug_assertions)]
            if let Some(curve) = weak.upgrade() {
                eprintln!(
                    "rust-flutter gallery curved parent={} value={} status={:?} width={}",
                    curve.parent().value(),
                    curve.value(),
                    curve.status(),
                    size.x
                );
                eprintln!(
                    "rust-flutter gallery typed-tween width={} height={}",
                    size.x, size.y
                );
            }
            Widget::leaf(size, |ui| {
                ui.painter().rect_filled(
                    ui.max_rect(),
                    0.0,
                    egui::Color32::from_rgb(142, 111, 178),
                );
            })
        })
    }
    fn dispose(&mut self) {
        if let Some(size) = self.size.take() {
            let _ = size.dispose();
        }
        if let Some(curve) = self.curve.take() {
            let _ = curve.dispose();
        }
    }
}
struct TickerDemo {
    binding: GestureBinding,
}
struct TickerDemoState {
    enabled: bool,
}
impl StatefulWidget for TickerDemo {
    type State = TickerDemoState;
    fn create_state(&self) -> Self::State {
        TickerDemoState { enabled: true }
    }
}
impl State<TickerDemo> for TickerDemoState {
    fn build(&mut self, widget: &TickerDemo, ctx: &StateContext<Self>) -> Widget {
        let enabled = self.enabled;
        let handle = ctx.handle();
        let mut style = FlexStyle::new(Axis::Horizontal);
        style.main_size = rust_flutter::layout::MainAxisSize::Min;
        style.spacing = 8.0;
        Widget::flex(
            style,
            vec![
                FlexItem::fixed(Widget::ticker_mode(
                    enabled,
                    Widget::stateful(AnimatedMarker {
                        binding: widget.binding.clone(),
                    }),
                )),
                FlexItem::fixed(
                    GestureButton::new(
                        &widget.binding,
                        if enabled { "Ticking" } else { "Muted" },
                        move || {
                            let _ = handle.update(|state| state.enabled = !state.enabled);
                        },
                    )
                    .fill(egui::Color32::from_rgb(62, 108, 114))
                    .into_widget(),
                ),
            ],
        )
    }
}
fn baseline_example(ctx: &egui::Context, binding: &GestureBinding) -> Widget {
    let mut baselines = Vec::new();
    let mut children: Vec<FlexItem> = [
        (16.0, "Ag small", egui::Color32::from_rgb(49, 83, 123)),
        (28.0, "Ag large", egui::Color32::from_rgb(103, 81, 47)),
    ]
    .into_iter()
    .map(|(size, text, color)| {
        let galley = ctx.fonts_mut(|fonts| {
            fonts.layout_no_wrap(
                text.into(),
                egui::FontId::proportional(size),
                egui::Color32::WHITE,
            )
        });
        let distance = galley
            .rows
            .first()
            .and_then(|row| row.glyphs.first().map(|glyph| row.pos.y + glyph.pos.y))
            .expect("ASCII demo has a baseline");
        baselines.push(distance);
        use rust_flutter::text::{InlineSpan, Text, TextStyle};
        let mut label = Text::rich(InlineSpan::group(vec![
            InlineSpan::new("Ag "),
            InlineSpan::new(if size == 16.0 { "small" } else { "large" }).styled(TextStyle {
                // Span inheritance fills unspecified properties even with this flag.
                inherit: false,
                italics: Some(false),
                ..Default::default()
            }),
        ]));
        if size != 16.0 {
            label.style.font_size = Some(size);
        }
        let paragraph = label.into_widget();
        FlexItem::fixed(
            rust_flutter::decoration::DecoratedBox::new(
                rust_flutter::decoration::BoxDecoration::solid(color),
            )
            .into_widget(paragraph),
        )
    })
    .collect();
    children.push(FlexItem::fixed(Widget::stateful(TickerDemo {
        binding: binding.clone(),
    })));
    #[cfg(debug_assertions)]
    eprintln!(
        "rust-flutter gallery baseline-demo small={} large={}",
        baselines[0], baselines[1]
    );
    let mut style = FlexStyle::new(Axis::Horizontal);
    style.cross_alignment = CrossAxisAlignment::Baseline(TextBaseline::Alphabetic);
    style.spacing = 16.0;
    rust_flutter::text::DefaultTextStyle {
        style: rust_flutter::text::TextStyle {
            font_size: Some(16.0),
            color: Some(egui::Color32::WHITE),
            ..Default::default()
        },
        ..Default::default()
    }
    .provide(Widget::flex(style, children))
}
fn scroll_example(
    position: &rust_flutter::scrolling::ScrollPosition,
    binding: &GestureBinding,
) -> Widget {
    let content = Widget::layout_builder(|_, constraints| {
        println!(
            "rust-flutter gallery layout-builder scroll width={} max_height={}",
            constraints.max().x,
            constraints.max().y
        );
        let mut column = FlexStyle::new(Axis::Vertical);
        column.main_size = rust_flutter::layout::MainAxisSize::Min;
        column.cross_alignment = CrossAxisAlignment::Stretch;
        let stripes = [(40, 75, 114), (111, 142, 66), (184, 118, 87)]
            .into_iter()
            .map(|(r, g, b)| {
                FlexItem::fixed(Widget::leaf(
                    egui::vec2(constraints.max().x, 60.0),
                    move |ui| {
                        ui.painter().rect_filled(
                            ui.max_rect(),
                            0.0,
                            egui::Color32::from_rgb(r, g, b),
                        );
                    },
                ))
            })
            .collect();
        Widget::flex(column, stripes)
    });
    let view = rust_flutter::scrolling::SingleChildScrollView::new(position, binding)
        .expect("valid scroll view")
        .into_widget(content)
        .expect("valid scroll child");
    rust_flutter::scrolling::Scrollbar::new(position, binding)
        .expect("valid scrollbar")
        .into_widget(view)
        .expect("valid scrollbar child")
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
    baseline_tree: ElementTree,
    scroll_tree: ElementTree,
    scroll_position: rust_flutter::scrolling::ScrollPosition,
    scroll_controller: rust_flutter::scrolling::ScrollController,
    _scroll_attachment: rust_flutter::scrolling::ScrollAttachment,
    last_scroll_pixels: Option<f32>,
    binding: GestureBinding,
    baseline_scale: f32,
}
impl ComponentDemo {
    pub fn cancel_input(&self) {
        self.binding.cancel_all();
        let _ = self.scroll_position.cancel_activity();
    }
    pub fn new(ctx: &egui::Context) -> Self {
        let binding = GestureBinding::new();
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
        let observable_binding = binding.clone();
        let observable_tree = ElementTree::new(
            ctx,
            Widget::value_listenable_builder(observable.clone(), move |_, value| {
                let counter = counter.clone();
                GestureButton::new(
                    &observable_binding,
                    egui::RichText::new(format!("Observable: {value}"))
                        .size(22.0)
                        .color(egui::Color32::WHITE),
                    move || {
                        let _ = counter.update(|value| value.saturating_add(1));
                    },
                )
                .size(egui::vec2(280.0, 44.0))
                .fill(egui::Color32::from_rgb(161, 75, 50))
                .into_widget()
            }),
        )
        .expect("valid observable demo");
        let scroll_controller = rust_flutter::scrolling::ScrollController::default();
        let scroll_position = scroll_controller
            .create_position(ctx)
            .expect("valid scroll position");
        let scroll_attachment = scroll_controller
            .attach(&scroll_position)
            .expect("single scroll attachment");
        let scroll_tree = ElementTree::new(ctx, scroll_example(&scroll_position, &binding))
            .expect("valid scroll demo");
        Self {
            scroll_tree,
            scroll_position,
            scroll_controller,
            _scroll_attachment: scroll_attachment,
            last_scroll_pixels: None,
            observable,
            observable_tree,
            last_view_query: None,
            baseline_tree: ElementTree::new(ctx, baseline_example(ctx, &binding))
                .expect("valid baseline demo"),
            baseline_scale: ctx.pixels_per_point(),
            binding,
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
        let mut gestures = self
            .binding
            .begin_egui_frame(ui.ctx())
            .expect("one component input frame");
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
            && let Err(error) =
                self.observable_tree
                    .show_in_gesture_frame(ui, constraints, &mut gestures)
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
        if self.baseline_scale != ui.ctx().pixels_per_point() {
            self.baseline_scale = ui.ctx().pixels_per_point();
            if let Err(error) = self
                .baseline_tree
                .update(baseline_example(ui.ctx(), &self.binding))
            {
                ui.colored_label(egui::Color32::LIGHT_RED, error.to_string());
            }
        }
        if let Ok(constraints) = BoxConstraints::tight(egui::vec2(ui.available_width(), 44.0)) {
            let geometry = self.baseline_tree.layout(constraints);
            let response = self
                .baseline_tree
                .show_in_gesture_frame(ui, constraints, &mut gestures);
            if let (Ok(geometry), Ok(response)) = (geometry, response)
                && let Some(distance) = geometry.baseline(TextBaseline::Alphabetic)
            {
                ui.painter().hline(
                    response.rect.x_range(),
                    response.rect.top() + distance,
                    egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(78, 164, 168)),
                );
            }
        }
        ui.horizontal(|ui| {
            ui.label("Retained scroll: drag upward");
            if ui
                .add(egui::Button::new("Reset scroll").fill(egui::Color32::from_rgb(73, 97, 76)))
                .clicked()
            {
                let _ = self.scroll_controller.jump_to(0.0);
            }
        });
        if let Ok(constraints) = BoxConstraints::tight(egui::vec2(ui.available_width(), 60.0))
            && let Err(error) =
                self.scroll_tree
                    .show_in_gesture_frame(ui, constraints, &mut gestures)
        {
            ui.colored_label(egui::Color32::LIGHT_RED, error.to_string());
        }
        let pixels = self.scroll_position.pixels();
        if self.last_scroll_pixels != Some(pixels) {
            self.last_scroll_pixels = Some(pixels);
            #[cfg(debug_assertions)]
            eprintln!(
                "rust-flutter gallery scroll pixels={} max={}",
                pixels,
                self.scroll_position
                    .dimensions()
                    .map_or(0.0, |d| d.max_scroll_extent())
            );
        }
        #[cfg(debug_assertions)]
        let roots = gestures.root_count();
        let report = gestures.finish().expect("valid component input frame");
        #[cfg(debug_assertions)]
        if report.processed > 0 {
            eprintln!(
                "rust-flutter gallery gesture-frame roots={} targets={} processed={}",
                roots,
                self.binding.target_count(),
                report.processed
            );
        }
        #[cfg(debug_assertions)]
        if !report.scroll_claims.is_empty() {
            eprintln!(
                "rust-flutter gallery scroll-signal claimed={}",
                report.scroll_claims.len()
            );
        }
        if let Some(failure) = report.scroll_failures.first() {
            ui.colored_label(egui::Color32::LIGHT_RED, failure.error.to_string());
        }
        if let Some(failure) = report.failures.first() {
            ui.colored_label(egui::Color32::LIGHT_RED, failure.error.to_string());
        }
    }
}

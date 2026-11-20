use rust_flutter::{
    animation::*,
    decoration::*,
    egui::{self, pos2, vec2},
    element::*,
    layout::*,
    notifier::*,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
type ReadHook = Rc<RefCell<Option<Rc<dyn Fn()>>>>;
#[derive(Clone)]
struct Source<T: TweenValue + Copy> {
    data: ValueNotifier<T>,
    closed: Rc<Cell<bool>>,
    error: Rc<Cell<Option<AnimationError>>>,
    live: Rc<Cell<usize>>,
    hook: ReadHook,
    immediate: Rc<Cell<bool>>,
    reads: Rc<Cell<usize>>,
}
impl<T: TweenValue + Copy> Source<T> {
    fn new(value: T) -> Self {
        Self {
            data: ValueNotifier::new(value),
            closed: Rc::default(),
            error: Rc::default(),
            live: Rc::default(),
            hook: Rc::default(),
            immediate: Rc::default(),
            reads: Rc::default(),
        }
    }
    fn set(&self, value: T) {
        self.data.set(value).unwrap();
    }
    fn fail(&self, error: Option<AnimationError>) {
        self.error.set(error);
        self.data.notify_listeners().unwrap();
    }
}
impl<T: TweenValue + Copy> Listenable for Source<T> {
    fn subscribe(&self, listener: Listener) -> Result<Subscription, Disposed> {
        if self.closed.get() {
            return Err(Disposed);
        }
        let callback = listener.clone();
        let subscription = self.data.subscribe(listener)?;
        let live = self.live.clone();
        live.set(live.get() + 1);
        let token = Subscription::new(move || {
            live.set(live.get() - 1);
            drop(subscription);
        });
        if self.immediate.get() {
            callback();
        }
        Ok(token)
    }
    fn same_source(&self, other: &Self) -> bool {
        self.data.same_source(&other.data)
    }
}
impl<T: TweenValue + Copy> ValueListenable for Source<T> {
    type Value = T;
    fn value(&self) -> T {
        self.reads.set(self.reads.get() + 1);
        let value = self.data.get();
        let hook = self.hook.borrow_mut().take();
        if let Some(hook) = hook {
            hook();
        }
        value
    }
}
impl<T: TweenValue + Copy> Animation<T> for Source<T> {
    fn status(&self) -> AnimationStatus {
        AnimationStatus::Forward
    }
    fn subscribe_status(&self, _: Listener) -> Result<Subscription, Disposed> {
        Ok(Subscription::new(|| {}))
    }
    fn is_disposed(&self) -> bool {
        self.closed.get()
    }
    fn try_value(&self) -> Result<T, AnimationError> {
        if self.closed.get() {
            Err(AnimationError::Disposed)
        } else if let Some(error) = self.error.get() {
            Err(error)
        } else {
            Ok(self.value())
        }
    }
}
fn loose() -> BoxConstraints {
    BoxConstraints::loose(vec2(100.0, 100.0)).unwrap()
}
fn child() -> Widget {
    Widget::baseline_leaf(
        vec2(80.0, 40.0),
        Baselines {
            alphabetic: Some(14.0),
            ideographic: None,
        },
        |_| {},
    )
}
fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-4, "{a} != {b}");
}
#[test]
fn size_transition_measures_then_clips_and_aligns_without_scaling_child_geometry() {
    let source = Source::new(0.5_f32);
    let ctx = egui::Context::default();
    let mut tree = ElementTree::new(
        &ctx,
        SizeTransition::new(source.clone()).into_widget(child()),
    )
    .unwrap();
    let g = tree.layout(loose()).unwrap();
    assert_eq!(g.size, vec2(100.0, 20.0));
    assert_eq!(g.clip, Clip::HardEdge);
    assert_eq!(g.children[0].layout.size, vec2(80.0, 40.0));
    assert_eq!(g.children[0].offset, vec2(0.0, -10.0));
    assert!(g.hit_test(pos2(10.0, 10.0)).is_some());
    assert!(g.hit_test(pos2(10.0, 30.0)).is_none());
    source.set(-1.0);
    assert_eq!(tree.layout(loose()).unwrap().size, vec2(100.0, 0.0));
    assert!(
        tree.layout(loose())
            .unwrap()
            .hit_test(pos2(10.0, 0.0))
            .is_none()
    );
    source.set(2.0);
    let g = tree.layout(loose()).unwrap();
    assert_eq!(g.size, vec2(100.0, 80.0));
    assert_eq!(g.children[0].offset, vec2(0.0, 20.0));
}
#[test]
fn axis_alignment_cross_factor_and_tight_unbounded_constraints_follow_align_policy() {
    let ctx = egui::Context::default();
    let source = Source::new(0.5_f32);
    let mut transition = SizeTransition::new(source.clone());
    transition.axis = Axis::Horizontal;
    transition.fixed_cross_axis_size_factor = Some(0.5);
    transition.alignment = Some(Alignment::BOTTOM_RIGHT);
    let mut tree = ElementTree::new(&ctx, transition.into_widget(child())).unwrap();
    let g = tree.layout(loose()).unwrap();
    assert_eq!(g.size, vec2(40.0, 20.0));
    assert_eq!(g.children[0].offset, vec2(-40.0, -20.0));
    let g = tree
        .layout(BoxConstraints::tight(vec2(100.0, 60.0)).unwrap())
        .unwrap();
    assert_eq!(g.size, vec2(100.0, 60.0));
    assert_eq!(g.children[0].offset, vec2(20.0, 20.0));
    let g = tree.layout(BoxConstraints::default()).unwrap();
    assert_eq!(g.size, vec2(40.0, 20.0));
}
#[test]
fn pure_size_queries_match_geometry_and_baselines_without_wet_measurement() {
    let ctx = egui::Context::default();
    let source = Source::new(0.5_f32);
    let mut tree = ElementTree::new(
        &ctx,
        SizeTransition::new(source.clone()).into_widget(child()),
    )
    .unwrap();
    assert_eq!(tree.dry_layout(loose()).unwrap(), vec2(100.0, 20.0));
    assert_eq!(
        tree.intrinsic(IntrinsicDimension::MinHeight, 100.0)
            .unwrap(),
        20.0
    );
    assert_eq!(
        tree.intrinsic(IntrinsicDimension::MaxWidth, 100.0).unwrap(),
        80.0
    );
    assert_eq!(
        tree.dry_baseline(loose(), TextBaseline::Alphabetic)
            .unwrap(),
        Some(4.0)
    );
    assert_eq!(
        tree.layout(loose())
            .unwrap()
            .baseline(TextBaseline::Alphabetic),
        Some(4.0)
    );
    let measured = Rc::new(Cell::new(0));
    let calls = measured.clone();
    tree.update(SizeTransition::new(source).into_widget(Widget::measured(
        move |c| {
            calls.set(calls.get() + 1);
            Ok(c.constrain(vec2(80.0, 40.0)).unwrap())
        },
        |_| {},
    )))
    .unwrap();
    assert!(matches!(
        tree.dry_layout(loose()),
        Err(ElementError::Layout(LayoutError::UnsupportedQuery(_)))
    ));
    assert_eq!(measured.get(), 0);
}
#[derive(Default)]
struct Counts {
    builds: Cell<usize>,
    disposed: Cell<usize>,
    handle: RefCell<Option<StateHandle<ChildState>>>,
}
struct Child(Rc<Counts>);
struct ChildState {
    counts: Rc<Counts>,
    value: usize,
}
impl StatefulWidget for Child {
    type State = ChildState;
    fn create_state(&self) -> Self::State {
        ChildState {
            counts: self.0.clone(),
            value: 0,
        }
    }
}
impl State<Child> for ChildState {
    fn init_state(&mut self, _: &Child, c: &StateContext<Self>) {
        *self.counts.handle.borrow_mut() = Some(c.handle());
    }
    fn build(&mut self, _: &Child, _: &StateContext<Self>) -> Widget {
        self.counts.builds.set(self.counts.builds.get() + 1);
        Widget::leaf(vec2(80.0 + self.value as f32, 40.0), |_| {})
    }
    fn dispose(&mut self) {
        self.counts.disposed.set(self.counts.disposed.get() + 1);
    }
}
#[test]
fn animation_ticks_preserve_child_identity_state_and_do_not_rebuild_child_descriptions() {
    let counts = Rc::new(Counts::default());
    let source = Source::new(0.5_f32);
    let mut tree = ElementTree::new(
        &egui::Context::default(),
        SizeTransition::new(source.clone()).into_widget(Widget::stateful(Child(counts.clone()))),
    )
    .unwrap();
    let original = tree.layout(loose()).unwrap().children[0].layout.id;
    assert_eq!(counts.builds.get(), 1);
    for value in [0.0, 0.25, 0.75, 1.0] {
        source.set(value);
        let g = tree.layout(loose()).unwrap();
        assert_eq!(g.children[0].layout.id, original);
    }
    assert_eq!(counts.builds.get(), 1);
    assert_eq!(counts.disposed.get(), 0);
    counts
        .handle
        .borrow()
        .as_ref()
        .unwrap()
        .update(|s| s.value = 4)
        .unwrap();
    assert_eq!(
        tree.layout(loose()).unwrap().children[0].layout.size.x,
        84.0
    );
    assert_eq!(counts.builds.get(), 2);
    tree.unmount();
    assert_eq!(counts.disposed.get(), 1);
    assert_eq!(source.live.get(), 0);
}
#[test]
fn source_rebinding_removes_old_links_and_updates_geometry_without_replacing_children() {
    let first = Source::new(0.25_f32);
    let next = Source::new(0.75_f32);
    let mut tree = ElementTree::new(
        &egui::Context::default(),
        SizeTransition::new(first.clone()).into_widget(child()),
    )
    .unwrap();
    let old = tree.layout(loose()).unwrap().children[0].layout.id;
    assert_eq!(first.live.get(), 1);
    tree.update(SizeTransition::new(next.clone()).into_widget(child()))
        .unwrap();
    let g = tree.layout(loose()).unwrap();
    assert_eq!(g.size.y, 30.0);
    assert_eq!(g.children[0].layout.id, old);
    assert_eq!(first.live.get(), 0);
    assert_eq!(next.live.get(), 1);
    first.set(1.0);
    assert_eq!(tree.layout(loose()).unwrap().size.y, 30.0);
    tree.unmount();
    assert_eq!(next.live.get(), 0);
}
#[test]
fn alignment_transition_animates_placement_queries_and_unclipped_overflow() {
    let source = Source::new(Alignment::TOP_LEFT);
    let ctx = egui::Context::default();
    let mut tree = ElementTree::new(
        &ctx,
        AlignTransition::new(source.clone()).into_widget(child()),
    )
    .unwrap();
    let g = tree.layout(loose()).unwrap();
    assert_eq!(g.size, vec2(100.0, 100.0));
    assert_eq!(g.children[0].offset, vec2(0.0, 0.0));
    source.set(Alignment::BOTTOM_RIGHT);
    let g = tree.layout(loose()).unwrap();
    assert_eq!(g.children[0].offset, vec2(20.0, 60.0));
    assert_eq!(g.baseline(TextBaseline::Alphabetic), Some(74.0));
    assert_eq!(
        tree.dry_baseline(loose(), TextBaseline::Alphabetic)
            .unwrap(),
        Some(74.0)
    );
    source.set(Alignment { x: 3.0, y: 3.0 });
    let g = tree.layout(loose()).unwrap();
    assert_eq!(g.children[0].offset, vec2(40.0, 120.0));
    assert_eq!(g.clip, Clip::None);
    assert!(g.overflow.bottom > 0.0);
    let mut config = AlignTransition::new(source);
    config.width_factor = Some(1.0);
    config.height_factor = Some(1.0);
    tree.update(config.into_widget(child())).unwrap();
    assert_eq!(tree.layout(loose()).unwrap().size, vec2(80.0, 40.0));
}
fn frame(ctx: &egui::Context, tree: &mut ElementTree) -> Result<egui::FullOutput, ElementError> {
    let mut result = Ok(());
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(300.0, 300.0),
            )),
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                result = tree.show(ui, loose()).map(|_| ());
            });
        },
    );
    result.map(|_| output)
}
#[test]
fn decorated_transition_repaints_without_child_layout_or_build_and_respects_foreground_order() {
    let source = Source::new(BoxDecoration::solid(egui::Color32::RED));
    let ctx = egui::Context::default();
    let layouts = Rc::new(Cell::new(0));
    let measured = layouts.clone();
    let content = Widget::measured(
        move |c| {
            measured.set(measured.get() + 1);
            Ok(c.constrain(vec2(40.0, 40.0)).unwrap())
        },
        |ui| {
            ui.painter()
                .rect_filled(ui.max_rect(), 0.0, egui::Color32::GREEN);
        },
    );
    let mut tree = ElementTree::new(
        &ctx,
        DecoratedBoxTransition::new(source.clone()).into_widget(content),
    )
    .unwrap();
    frame(&ctx, &mut tree).unwrap();
    let count = layouts.get();
    source.set(BoxDecoration::solid(egui::Color32::BLUE));
    let output = frame(&ctx, &mut tree).unwrap();
    assert_eq!(
        layouts.get(),
        count,
        "paint-only source update caused child layout"
    );
    let colors = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::epaint::Shape::Rect(r)
                if r.fill == egui::Color32::BLUE || r.fill == egui::Color32::GREEN =>
            {
                Some(r.fill)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(colors, vec![egui::Color32::BLUE, egui::Color32::GREEN]);
    let mut config = DecoratedBoxTransition::new(source);
    config.position = DecorationPosition::Foreground;
    tree.update(config.into_widget(Widget::leaf(vec2(40.0, 40.0), |ui| {
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, egui::Color32::GREEN);
    })))
    .unwrap();
    let output = frame(&ctx, &mut tree).unwrap();
    let colors = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::epaint::Shape::Rect(r)
                if r.fill == egui::Color32::BLUE || r.fill == egui::Color32::GREEN =>
            {
                Some(r.fill)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(colors, vec![egui::Color32::GREEN, egui::Color32::BLUE]);
}
#[test]
fn animation_errors_and_invalid_paint_values_surface_typed_layout_errors_and_recover() {
    let ctx = egui::Context::default();
    let counts = Rc::new(Counts::default());
    let source = Source::new(BoxDecoration::solid(egui::Color32::RED));
    let mut tree = ElementTree::new(
        &ctx,
        DecoratedBoxTransition::new(source.clone())
            .into_widget(Widget::stateful(Child(counts.clone()))),
    )
    .unwrap();
    tree.layout(loose()).unwrap();
    let mut bad = BoxDecoration::solid(egui::Color32::RED);
    bad.border.width = f32::NAN;
    source.set(bad);
    assert!(matches!(
        tree.layout(loose()),
        Err(ElementError::Layout(LayoutError::Animation(
            AnimationError::InvalidConfiguration(_)
        )))
    ));
    assert!(counts.handle.borrow().as_ref().unwrap().mounted());
    source.set(BoxDecoration::solid(egui::Color32::BLUE));
    tree.layout(loose()).unwrap();
    assert_eq!(counts.builds.get(), 1);
    source.fail(Some(AnimationError::CurvePanicked));
    assert!(matches!(
        tree.layout(loose()),
        Err(ElementError::Layout(LayoutError::Animation(
            AnimationError::CurvePanicked
        )))
    ));
    source.fail(None);
    tree.layout(loose()).unwrap();
    source.closed.set(true);
    assert!(matches!(
        tree.layout(loose()),
        Err(ElementError::Layout(LayoutError::Animation(
            AnimationError::Disposed
        )))
    ));
    tree.unmount();
    assert_eq!(counts.disposed.get(), 1);
    assert_eq!(source.live.get(), 0);
}
#[test]
fn synchronous_subscriptions_and_reentrant_source_reads_keep_the_newest_sample() {
    let ctx = egui::Context::default();
    let source = Source::new(0.25_f32);
    source.immediate.set(true);
    let current = source.clone();
    *source.hook.borrow_mut() = Some(Rc::new(move || current.set(0.75)));
    let mut tree = ElementTree::new(
        &ctx,
        SizeTransition::new(source.clone()).into_widget(child()),
    )
    .unwrap();
    assert_eq!(tree.layout(loose()).unwrap().size.y, 30.0);
    let current = source.clone();
    *source.hook.borrow_mut() = Some(Rc::new(move || current.set(0.5)));
    source.set(1.0);
    assert_eq!(tree.layout(loose()).unwrap().size.y, 20.0);
    tree.unmount();
    assert_eq!(source.live.get(), 0);
}
#[test]
fn invalid_factors_alignments_sources_and_child_counts_return_checked_errors() {
    let ctx = egui::Context::default();
    let source = Source::new(f32::NAN);
    let mut tree = ElementTree::new(
        &ctx,
        SizeTransition::new(source.clone()).into_widget(child()),
    )
    .unwrap();
    assert!(tree.layout(loose()).is_err());
    source.set(0.5);
    tree.layout(loose()).unwrap();
    let mut bad = SizeTransition::new(source.clone());
    bad.fixed_cross_axis_size_factor = Some(-1.0);
    tree.update(bad.into_widget(child())).unwrap();
    assert!(tree.layout(loose()).is_err());
    let mut bad = SizeTransition::new(source.clone());
    bad.alignment = Some(Alignment {
        x: f32::INFINITY,
        y: 0.0,
    });
    tree.update(bad.into_widget(child())).unwrap();
    assert!(tree.layout(loose()).is_err());
    tree.update(Widget::render_object(SizeTransition::new(source), vec![]))
        .unwrap();
    assert!(tree.layout(loose()).is_err());
    let source = Source::new(0.5_f32);
    source.closed.set(true);
    let mut tree =
        ElementTree::new(&ctx, SizeTransition::new(source).into_widget(child())).unwrap();
    assert!(matches!(
        tree.layout(loose()),
        Err(ElementError::Layout(LayoutError::Animation(
            AnimationError::Disposed
        )))
    ));
}
#[test]
fn existing_controller_and_tween_drive_transition_layout_without_rebuilding_child() {
    let ctx = egui::Context::default();
    ctx.begin_pass(egui::RawInput {
        time: Some(0.0),
        ..Default::default()
    });
    let _ = ctx.end_pass();
    let controller = AnimationController::new(
        &ctx,
        AnimationConfig {
            duration: Duration::from_secs(1),
            ..Default::default()
        },
    )
    .unwrap();
    let factor = controller
        .clone()
        .drive(ValueTween::new(0.5_f32, 1.0).unwrap())
        .unwrap();
    let counts = Rc::new(Counts::default());
    let mut tree = ElementTree::new(
        &ctx,
        SizeTransition::new(factor.clone()).into_widget(Widget::stateful(Child(counts.clone()))),
    )
    .unwrap();
    near(tree.layout(loose()).unwrap().size.y, 20.0);
    controller.forward().unwrap();
    ctx.begin_pass(egui::RawInput {
        time: Some(0.5),
        ..Default::default()
    });
    let _ = ctx.end_pass();
    near(tree.layout(loose()).unwrap().size.y, 30.0);
    ctx.begin_pass(egui::RawInput {
        time: Some(1.0),
        ..Default::default()
    });
    let _ = ctx.end_pass();
    near(tree.layout(loose()).unwrap().size.y, 40.0);
    assert_eq!(counts.builds.get(), 1);
    tree.unmount();
    factor.dispose().unwrap();
    controller.dispose().unwrap();
}

#[test]
fn rebinding_same_source_retries_a_failed_subscription_and_preserves_child_state() {
    let source = Source::new(0.5_f32);
    source.closed.set(true);
    let counts = Rc::new(Counts::default());
    let mut tree = ElementTree::new(
        &egui::Context::default(),
        SizeTransition::new(source.clone()).into_widget(Widget::stateful(Child(counts.clone()))),
    )
    .unwrap();
    assert!(matches!(
        tree.layout(loose()),
        Err(ElementError::Layout(LayoutError::Animation(
            AnimationError::Disposed
        )))
    ));
    assert_eq!(source.live.get(), 0);
    source.closed.set(false);
    tree.update(
        SizeTransition::new(source.clone()).into_widget(Widget::stateful(Child(counts.clone()))),
    )
    .unwrap();
    assert_eq!(tree.layout(loose()).unwrap().size.y, 20.0);
    assert_eq!(source.live.get(), 1);
    source.set(0.75);
    assert_eq!(tree.layout(loose()).unwrap().size.y, 30.0);
    assert_eq!(counts.disposed.get(), 0);
    tree.unmount();
    assert_eq!(counts.disposed.get(), 1);
    assert_eq!(source.live.get(), 0);
}

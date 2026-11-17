use rust_flutter::{
    animation::*,
    decoration::BoxDecoration,
    egui::{self, Color32, vec2},
    element::*,
    layout::{BoxConstraints, Insets},
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
fn ctx() -> egui::Context {
    let ctx = egui::Context::default();
    ctx.begin_pass(egui::RawInput {
        time: Some(0.0),
        ..Default::default()
    });
    ctx
}
fn begin(ctx: &egui::Context, time: f64) {
    ctx.begin_pass(egui::RawInput {
        time: Some(time),
        ..Default::default()
    });
}
fn end(ctx: &egui::Context) {
    let _ = ctx.end_pass();
}
fn constraints() -> BoxConstraints {
    BoxConstraints::loose(vec2(1000.0, 1000.0)).unwrap()
}
fn widget(from: f32, target: f32, seconds: u64) -> TweenAnimationBuilder<f32> {
    TweenAnimationBuilder::new(from, target, Duration::from_secs(seconds), |_, value| {
        Widget::leaf(vec2(value, 10.0), |_| {})
    })
    .unwrap()
}
fn width(tree: &mut ElementTree) -> f32 {
    tree.layout(constraints()).unwrap().size.x
}
fn ids(tree: &ElementTree) -> Vec<ElementId> {
    tree.elements().iter().map(|e| e.id).collect()
}
#[test]
fn initial_tween_runs_automatically_and_completion_fires_after_ui_pass() {
    let ctx = ctx();
    let calls = Rc::new(Cell::new(0));
    let log = calls.clone();
    let mut tree = ElementTree::new(
        &ctx,
        widget(0.0, 100.0, 1)
            .on_end(move || log.set(log.get() + 1))
            .into_widget(),
    )
    .unwrap();
    let original = ids(&tree);
    assert_eq!(width(&mut tree), 0.0);
    end(&ctx);
    begin(&ctx, 0.5);
    assert_eq!(width(&mut tree), 50.0);
    assert_eq!(calls.get(), 0);
    end(&ctx);
    begin(&ctx, 1.0);
    assert_eq!(width(&mut tree), 100.0);
    assert_eq!(calls.get(), 0);
    end(&ctx);
    assert_eq!(calls.get(), 1);
    assert_eq!(ids(&tree), original);
    begin(&ctx, 2.0);
    assert_eq!(width(&mut tree), 100.0);
    end(&ctx);
    assert_eq!(calls.get(), 1);
}
#[test]
fn midflight_retarget_is_continuous_and_new_begin_is_ignored() {
    let ctx = ctx();
    let mut tree = ElementTree::new(&ctx, widget(0.0, 100.0, 1).into_widget()).unwrap();
    end(&ctx);
    begin(&ctx, 0.4);
    assert_eq!(width(&mut tree), 40.0);
    tree.update(widget(999.0, 200.0, 1).into_widget()).unwrap();
    assert_eq!(width(&mut tree), 40.0);
    end(&ctx);
    begin(&ctx, 0.9);
    assert_eq!(width(&mut tree), 120.0);
    end(&ctx);
    begin(&ctx, 1.4);
    assert_eq!(width(&mut tree), 200.0);
    end(&ctx);
}
#[test]
fn unchanged_target_does_not_restart_and_builder_replacement_keeps_identity() {
    let ctx = ctx();
    let mut tree = ElementTree::new(&ctx, widget(0.0, 100.0, 1).into_widget()).unwrap();
    let original = ids(&tree);
    end(&ctx);
    begin(&ctx, 0.4);
    assert_eq!(width(&mut tree), 40.0);
    tree.update(
        TweenAnimationBuilder::new(900.0, 100.0, Duration::from_secs(1), |_, value| {
            Widget::leaf(vec2(value, 25.0), |_| {})
        })
        .unwrap()
        .into_widget(),
    )
    .unwrap();
    assert_eq!(tree.layout(constraints()).unwrap().size, vec2(40.0, 25.0));
    assert_eq!(ids(&tree), original);
    end(&ctx);
    begin(&ctx, 1.0);
    assert_eq!(width(&mut tree), 100.0);
    end(&ctx);
}
#[test]
fn changing_active_curve_or_duration_retargets_from_current_sample() {
    let ctx = ctx();
    let calls = Rc::new(Cell::new(0));
    let first = calls.clone();
    let mut tree = ElementTree::new(
        &ctx,
        widget(0.0, 100.0, 1)
            .on_end(move || first.set(first.get() + 100))
            .into_widget(),
    )
    .unwrap();
    end(&ctx);
    begin(&ctx, 0.5);
    assert_eq!(width(&mut tree), 50.0);
    let second = calls.clone();
    tree.update(
        widget(0.0, 100.0, 2)
            .with_curve(Curve::EaseOut)
            .on_end(move || second.set(second.get() + 1))
            .into_widget(),
    )
    .unwrap();
    assert_eq!(width(&mut tree), 50.0);
    end(&ctx);
    begin(&ctx, 1.5);
    assert_eq!(width(&mut tree), 93.75);
    end(&ctx);
    assert_eq!(calls.get(), 0);
    begin(&ctx, 2.5);
    assert_eq!(width(&mut tree), 100.0);
    end(&ctx);
    assert_eq!(calls.get(), 1);
}
#[test]
fn implicit_playback_obeys_scoped_muting_and_unmount_cancels_pending_callbacks() {
    let ctx = ctx();
    let calls = Rc::new(Cell::new(0));
    let make = |enabled| {
        let log = calls.clone();
        Widget::ticker_mode(
            enabled,
            widget(0.0, 100.0, 1)
                .on_end(move || log.set(log.get() + 1))
                .into_widget(),
        )
    };
    let mut tree = ElementTree::new(&ctx, make(false)).unwrap();
    let original = ids(&tree);
    end(&ctx);
    begin(&ctx, 0.5);
    assert_eq!(width(&mut tree), 0.0);
    tree.update(make(true)).unwrap();
    end(&ctx);
    begin(&ctx, 0.75);
    assert_eq!(width(&mut tree), 75.0);
    assert_eq!(ids(&tree), original);
    end(&ctx);
    begin(&ctx, 1.0);
    assert_eq!(width(&mut tree), 100.0);
    tree.unmount();
    end(&ctx);
    assert_eq!(calls.get(), 0);
}
#[test]
fn zero_duration_displays_target_immediately_but_callback_waits_for_mount_and_end_pass() {
    let ctx = ctx();
    let holder = Rc::new(RefCell::new(None::<ElementTree>));
    let weak = Rc::downgrade(&holder);
    let calls = Rc::new(Cell::new(0));
    let log = calls.clone();
    let w = widget(24.0, 48.0, 0).on_end(move || {
        assert!(
            weak.upgrade()
                .unwrap()
                .borrow()
                .as_ref()
                .unwrap()
                .root_id()
                .is_some()
        );
        log.set(log.get() + 1);
    });
    *holder.borrow_mut() = Some(ElementTree::new(&ctx, w.into_widget()).unwrap());
    assert_eq!(width(holder.borrow_mut().as_mut().unwrap()), 48.0);
    assert_eq!(calls.get(), 0);
    end(&ctx);
    assert_eq!(calls.get(), 1);
    let log = calls.clone();
    let mut equal = ElementTree::new(
        &ctx,
        widget(10.0, 10.0, 0)
            .on_end(move || log.set(log.get() + 1))
            .into_widget(),
    )
    .unwrap();
    assert_eq!(width(&mut equal), 10.0);
    begin(&ctx, 1.0);
    end(&ctx);
    assert_eq!(calls.get(), 1);
}
#[test]
fn completed_callbacks_keep_their_snapshot_and_unmount_drops_undelivered_later_completion() {
    let ctx = ctx();
    let calls = Rc::new(RefCell::new(vec![]));
    let a = calls.clone();
    let mut tree = ElementTree::new(
        &ctx,
        widget(0.0, 100.0, 1)
            .on_end(move || a.borrow_mut().push("A"))
            .into_widget(),
    )
    .unwrap();
    end(&ctx);
    begin(&ctx, 1.0);
    let b = calls.clone();
    tree.update(
        widget(0.0, 200.0, 1)
            .on_end(move || b.borrow_mut().push("B"))
            .into_widget(),
    )
    .unwrap();
    end(&ctx);
    assert_eq!(*calls.borrow(), vec!["A"]);
    begin(&ctx, 2.0);
    tree.unmount();
    end(&ctx);
    assert_eq!(*calls.borrow(), vec!["A"]);
}
#[test]
fn completion_callback_can_unmount_its_owner_and_suppress_remaining_callbacks() {
    let ctx = ctx();
    let holder = Rc::new(RefCell::new(None::<ElementTree>));
    let weak = Rc::downgrade(&holder);
    let calls = Rc::new(Cell::new(0));
    let a = calls.clone();
    let initial = widget(0.0, 10.0, 0).on_end(move || {
        a.set(a.get() + 1);
        weak.upgrade()
            .unwrap()
            .borrow_mut()
            .as_mut()
            .unwrap()
            .unmount();
    });
    *holder.borrow_mut() = Some(ElementTree::new(&ctx, initial.into_widget()).unwrap());
    let b = calls.clone();
    holder
        .borrow_mut()
        .as_mut()
        .unwrap()
        .update(
            widget(0.0, 20.0, 0)
                .on_end(move || b.set(b.get() + 10))
                .into_widget(),
        )
        .unwrap();
    end(&ctx);
    assert_eq!(calls.get(), 1);
    assert!(holder.borrow().as_ref().unwrap().root_id().is_none());
}
#[test]
fn builtin_values_validate_and_interpolate_geometry_color_and_decoration() {
    assert!(
        TweenAnimationBuilder::new(f32::NAN, 1.0, Duration::ZERO, |_, _| Widget::empty()).is_err()
    );
    assert!(
        TweenAnimationBuilder::new(
            vec2(0.0, f32::INFINITY),
            vec2(1.0, 1.0),
            Duration::ZERO,
            |_, _| Widget::empty()
        )
        .is_err()
    );
    assert!(
        TweenAnimationBuilder::new(
            Insets::all(-1.0),
            Insets::all(2.0),
            Duration::ZERO,
            |_, _| Widget::empty()
        )
        .is_err()
    );
    assert_eq!((-f32::MAX).interpolate(&f32::MAX, 0.5), 0.0);
    assert_eq!((-f64::MAX).interpolate(&f64::MAX, 0.5), 0.0);
    assert!(
        TweenAnimationBuilder::new(f64::NAN, 1.0_f64, Duration::ZERO, |_, _| Widget::empty())
            .is_err()
    );
    let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(10.0, 20.0));
    assert_eq!(
        rect.interpolate(&rect.translate(vec2(20.0, 10.0)), 0.5),
        rect.translate(vec2(10.0, 5.0))
    );
    assert!(
        TweenAnimationBuilder::new(egui::Rect::NOTHING, rect, Duration::ZERO, |_, _| {
            Widget::empty()
        })
        .is_err()
    );

    assert_eq!(
        vec2(-10.0, 20.0).interpolate(&vec2(10.0, 40.0), 0.5),
        vec2(0.0, 30.0)
    );
    assert_eq!(
        Insets::all(2.0).interpolate(&Insets::all(8.0), 0.5),
        Insets::all(5.0)
    );
    assert_eq!(
        Color32::TRANSPARENT
            .interpolate(&Color32::RED, 0.5)
            .to_array(),
        [128, 0, 0, 128]
    );
    let a = BoxDecoration::solid(Color32::BLACK);
    let b = BoxDecoration {
        color: Color32::WHITE,
        border: egui::Stroke::new(4.0_f32, Color32::RED),
        corner_radius: egui::CornerRadius::same(10),
    };
    let mid = a.interpolate(&b, 0.5);
    assert_eq!(mid.border.width, 2.0);
    assert_eq!(mid.corner_radius, egui::CornerRadius::same(5));
    assert_eq!(mid.color.to_array(), [128, 128, 128, 255]);
}
struct Child {
    width: f32,
    inits: Rc<Cell<usize>>,
}
struct ChildState;
impl StatefulWidget for Child {
    type State = ChildState;
    fn create_state(&self) -> ChildState {
        ChildState
    }
}
impl State<Child> for ChildState {
    fn init_state(&mut self, w: &Child, _: &StateContext<Self>) {
        w.inits.set(w.inits.get() + 1);
    }
    fn build(&mut self, w: &Child, _: &StateContext<Self>) -> Widget {
        Widget::leaf(vec2(w.width, 10.0), |_| {})
    }
}
#[test]
fn child_component_state_survives_ticks_and_retargeted_builder_configurations() {
    let ctx = ctx();
    let inits = Rc::new(Cell::new(0));
    let make = |target| {
        let inits = inits.clone();
        TweenAnimationBuilder::new(10.0, target, Duration::from_secs(1), move |_, value| {
            Widget::stateful(Child {
                width: value,
                inits: inits.clone(),
            })
            .keyed("child")
        })
        .unwrap()
        .into_widget()
    };
    let mut tree = ElementTree::new(&ctx, make(100.0)).unwrap();
    let original = ids(&tree);
    end(&ctx);
    begin(&ctx, 0.5);
    assert_eq!(width(&mut tree), 55.0);
    tree.update(make(200.0)).unwrap();
    end(&ctx);
    begin(&ctx, 1.0);
    assert_eq!(width(&mut tree), 127.5);
    end(&ctx);
    assert_eq!(inits.get(), 1);
    assert_eq!(ids(&tree), original);
}
#[derive(Clone, PartialEq)]
struct Pair(f32, f32);
impl TweenValue for Pair {
    fn interpolate(&self, b: &Self, t: f32) -> Self {
        assert!(
            0.0 < t && t < 1.0,
            "custom interpolation only receives interior progress"
        );
        Self(self.0.interpolate(&b.0, t), self.1.interpolate(&b.1, t))
    }
}
#[test]
fn application_value_types_can_animate_and_completion_panics_are_isolated() {
    let ctx = ctx();
    let mut tree = ElementTree::new(
        &ctx,
        TweenAnimationBuilder::new(
            Pair(10.0, 20.0),
            Pair(30.0, 40.0),
            Duration::from_secs(1),
            |_, p| Widget::leaf(vec2(p.0, p.1), |_| {}),
        )
        .unwrap()
        .on_end(|| panic!("isolated completion"))
        .into_widget(),
    )
    .unwrap();
    let calls = Rc::new(Cell::new(0));
    let log = calls.clone();
    let _other = ElementTree::new(
        &ctx,
        widget(10.0, 20.0, 1)
            .on_end(move || log.set(log.get() + 1))
            .into_widget(),
    )
    .unwrap();
    end(&ctx);
    begin(&ctx, 0.5);
    assert_eq!(tree.layout(constraints()).unwrap().size, vec2(20.0, 30.0));
    end(&ctx);
    begin(&ctx, 1.0);
    end(&ctx);
    assert_eq!(calls.get(), 1);
}

#[test]
fn completion_can_retarget_the_same_tree_and_new_completion_waits_for_next_pass() {
    let ctx = ctx();
    let holder = Rc::new(RefCell::new(None::<ElementTree>));
    let weak = Rc::downgrade(&holder);
    let calls = Rc::new(Cell::new(0));
    let first = calls.clone();
    let second = calls.clone();
    let initial = widget(0.0, 10.0, 0).on_end(move || {
        first.set(first.get() + 1);
        let log = second.clone();
        weak.upgrade()
            .unwrap()
            .borrow_mut()
            .as_mut()
            .unwrap()
            .update(
                widget(0.0, 20.0, 0)
                    .on_end(move || log.set(log.get() + 1))
                    .into_widget(),
            )
            .unwrap();
    });
    *holder.borrow_mut() = Some(ElementTree::new(&ctx, initial.into_widget()).unwrap());
    end(&ctx);
    assert_eq!(calls.get(), 1);
    assert_eq!(width(holder.borrow_mut().as_mut().unwrap()), 20.0);
    begin(&ctx, 1.0);
    end(&ctx);
    assert_eq!(calls.get(), 2);
}

#[test]
fn rounded_curve_endpoints_bypass_custom_interpolation() {
    let ctx = ctx();
    let mut tree = ElementTree::new(
        &ctx,
        TweenAnimationBuilder::new(
            Pair(0.0, 0.0),
            Pair(10.0, 20.0),
            Duration::from_secs(1),
            |_, p| Widget::leaf(vec2(p.0, p.1), |_| {}),
        )
        .unwrap()
        .with_curve(Curve::EaseOut)
        .into_widget(),
    )
    .unwrap();
    end(&ctx);
    begin(&ctx, 0.99999);
    assert_eq!(tree.layout(constraints()).unwrap().size, vec2(10.0, 20.0));
    end(&ctx);
}

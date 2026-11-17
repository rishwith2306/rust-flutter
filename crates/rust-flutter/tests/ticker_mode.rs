use rust_flutter::{animation::*, egui, element::*, layout::Axis, notifier::Listenable};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
#[derive(Default)]
struct Log {
    controllers: Vec<AnimationController>,
    contexts: Vec<BuildContext>,
    builds: usize,
    disposed: usize,
    already_disposed: bool,
}
struct Tracked {
    log: Rc<RefCell<Log>>,
    invalid: bool,
    multiple: bool,
}
struct TrackedState {
    log: Rc<RefCell<Log>>,
}
impl StatefulWidget for Tracked {
    type State = TrackedState;
    fn create_state(&self) -> Self::State {
        TrackedState {
            log: self.log.clone(),
        }
    }
}
impl State<Tracked> for TrackedState {
    fn init_state(&mut self, w: &Tracked, ctx: &StateContext<Self>) {
        for _ in 0..if w.multiple { 2 } else { 1 } {
            let controller = ctx
                .animation_controller(AnimationConfig {
                    duration: Duration::from_secs(1),
                    ..Default::default()
                })
                .unwrap();
            self.log.borrow_mut().controllers.push(controller);
        }
        self.log.borrow_mut().contexts.push((**ctx).clone());
    }
    fn build(&mut self, w: &Tracked, _: &StateContext<Self>) -> Widget {
        self.log.borrow_mut().builds += 1;
        if w.invalid {
            Widget::flex(
                FlexStyle::new(Axis::Horizontal),
                vec![
                    FlexItem::fixed(Widget::empty().keyed("dup")),
                    FlexItem::fixed(Widget::empty().keyed("dup")),
                ],
            )
        } else {
            Widget::leaf(egui::vec2(30.0, 20.0), |_| {})
        }
    }
    fn dispose(&mut self) {
        let mut log = self.log.borrow_mut();
        log.disposed += 1;
        log.already_disposed = log.controllers.iter().all(|c| c.is_disposed());
    }
}
fn tracked(log: &Rc<RefCell<Log>>) -> Widget {
    Widget::stateful(Tracked {
        log: log.clone(),
        invalid: false,
        multiple: false,
    })
}
fn frame(ctx: &egui::Context, time: f64) {
    ctx.begin_pass(egui::RawInput {
        time: Some(time),
        ..Default::default()
    });
    let _ = ctx.end_pass();
}
fn setup() -> (egui::Context, Rc<RefCell<Log>>) {
    let ctx = egui::Context::default();
    frame(&ctx, 0.0);
    (ctx, Rc::default())
}
#[test]
fn nested_modes_combine_and_retained_owners_catch_up_after_unmuting() {
    let (ctx, log) = setup();
    let widget =
        |outer, inner| Widget::ticker_mode(outer, Widget::ticker_mode(inner, tracked(&log)));
    let mut tree = ElementTree::new(&ctx, widget(false, true)).unwrap();
    let ids = tree.elements().iter().map(|e| e.id).collect::<Vec<_>>();
    let c = log.borrow().controllers[0].clone();
    assert!(c.is_muted());
    c.set_muted(false).unwrap();
    assert!(c.is_muted());
    let run = c.forward().unwrap();
    frame(&ctx, 0.2);
    assert_eq!(c.value(), 0.0);
    tree.update(widget(true, true)).unwrap();
    assert!(!c.is_muted());
    frame(&ctx, 0.5);
    assert_eq!(c.value(), 0.5);
    tree.update(widget(true, false)).unwrap();
    frame(&ctx, 2.0);
    assert_eq!(c.value(), 0.5);
    assert!(c.is_animating());
    assert_eq!(run.outcome(), None);
    tree.update(widget(false, true)).unwrap();
    assert!(c.is_muted());
    tree.update(widget(true, true)).unwrap();
    frame(&ctx, 2.1);
    assert_eq!(c.value(), 1.0);
    assert_eq!(run.outcome(), Some(AnimationOutcome::Completed));
    assert_eq!(log.borrow().controllers.len(), 1);
    assert_eq!(
        tree.elements().iter().map(|e| e.id).collect::<Vec<_>>(),
        ids
    );
}
#[test]
fn inherited_mode_changes_refresh_owned_controllers_without_rebuilding_owner() {
    let (ctx, log) = setup();
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited(
            TickerMode { enabled: true },
            Widget::ticker_mode(true, tracked(&log)),
        ),
    )
    .unwrap();
    let c = log.borrow().controllers[0].clone();
    let handle = tree
        .inherited_handle::<TickerMode>(tree.root_id().unwrap())
        .unwrap();
    c.forward().unwrap();
    frame(&ctx, 0.25);
    assert_eq!(c.value(), 0.25);
    assert_eq!(log.borrow().builds, 1);
    handle.set(TickerMode { enabled: false }).unwrap();
    tree.flush().unwrap();
    assert!(c.is_muted());
    assert_eq!(log.borrow().builds, 1);
    frame(&ctx, 2.0);
    assert_eq!(c.value(), 0.25);
    handle.set(TickerMode { enabled: true }).unwrap();
    tree.flush().unwrap();
    frame(&ctx, 2.1);
    assert_eq!(c.value(), 1.0);
    assert_eq!(log.borrow().builds, 1);
}
#[test]
fn explicit_muting_combines_with_scope_while_unmanaged_controllers_are_independent() {
    let (ctx, log) = setup();
    let mut tree = ElementTree::new(&ctx, Widget::ticker_mode(false, tracked(&log))).unwrap();
    let c = log.borrow().controllers[0].clone();
    let raw = AnimationController::new(
        &ctx,
        AnimationConfig {
            duration: Duration::from_secs(1),
            ..Default::default()
        },
    )
    .unwrap();
    c.forward().unwrap();
    raw.forward().unwrap();
    frame(&ctx, 0.25);
    assert_eq!(c.value(), 0.0);
    assert_eq!(raw.value(), 0.25);
    c.set_muted(true).unwrap();
    tree.update(Widget::ticker_mode(true, tracked(&log)))
        .unwrap();
    assert!(c.is_muted());
    frame(&ctx, 0.5);
    assert_eq!(c.value(), 0.0);
    c.set_muted(false).unwrap();
    frame(&ctx, 0.75);
    assert_eq!(c.value(), 0.75);
}
#[test]
fn unmount_cancels_every_owned_controller_even_with_external_clones_and_contexts() {
    let (ctx, log) = setup();
    let mut tree = ElementTree::new(
        &ctx,
        Widget::stateful(Tracked {
            log: log.clone(),
            invalid: false,
            multiple: true,
        }),
    )
    .unwrap();
    let controllers = log.borrow().controllers.clone();
    let context = log.borrow().contexts[0].clone();
    controllers[0].dispose().unwrap();
    let run = controllers[1].forward().unwrap();
    frame(&ctx, 0.2);
    tree.unmount();
    assert!(controllers.iter().all(|c| c.is_disposed()));
    assert_eq!(run.outcome(), Some(AnimationOutcome::Canceled));
    assert!(!context.mounted());
    assert_eq!(
        context
            .animation_controller(AnimationConfig::default())
            .err(),
        Some(AnimationError::Unmounted)
    );
    assert!(log.borrow().already_disposed);
    assert_eq!(log.borrow().disposed, 1);
}
#[test]
fn unmount_from_value_listener_cancels_immediately_and_defers_listener_cleanup_safely() {
    let (ctx, log) = setup();
    let tree = Rc::new(RefCell::new(ElementTree::new(&ctx, tracked(&log)).unwrap()));
    let c = log.borrow().controllers[0].clone();
    let weak = Rc::downgrade(&tree);
    let weak_controller = c.downgrade();
    let calls = Rc::new(Cell::new(0));
    let seen = calls.clone();
    let _subscription = c
        .subscribe(Rc::new(move || {
            seen.set(seen.get() + 1);
            weak.upgrade().unwrap().borrow_mut().unmount();
            let c = weak_controller.upgrade().unwrap();
            assert!(c.is_disposed());
            assert_eq!(c.forward().err(), Some(AnimationError::Disposed));
            assert!(c.subscribe(Rc::new(|| {})).is_err());
        }))
        .unwrap();
    let run = c.forward().unwrap();
    frame(&ctx, 0.2);
    assert_eq!(calls.get(), 1);
    assert!(c.is_disposed());
    assert_eq!(run.outcome(), Some(AnimationOutcome::Canceled));
    assert!(log.borrow().already_disposed);
    frame(&ctx, 0.4);
    assert_eq!(calls.get(), 1);
    assert!(c.subscribe_status(Rc::new(|| {})).is_err());
}
#[test]
fn failed_initial_mount_retires_controllers_created_during_initialization() {
    let (ctx, log) = setup();
    assert!(
        ElementTree::new(
            &ctx,
            Widget::stateful(Tracked {
                log: log.clone(),
                invalid: true,
                multiple: false
            })
        )
        .is_err()
    );
    assert!(log.borrow().controllers[0].is_disposed());
    assert!(log.borrow().already_disposed);
    assert_eq!(log.borrow().disposed, 1);
}

#[test]
fn mode_is_read_only_and_does_not_change_geometry_or_pure_queries() {
    use rust_flutter::layout::*;
    let (ctx, _) = setup();
    let mut tree = ElementTree::new(
        &ctx,
        Widget::ticker_mode(
            false,
            Widget::baseline_leaf(
                egui::vec2(30.0, 20.0),
                Baselines {
                    alphabetic: Some(12.0),
                    ideographic: None,
                },
                |_| {},
            ),
        ),
    )
    .unwrap();
    assert!(
        tree.inherited_handle::<TickerMode>(tree.root_id().unwrap())
            .is_none()
    );
    let constraints = BoxConstraints::loose(egui::vec2(100.0, 100.0)).unwrap();
    assert_eq!(
        tree.dry_layout(constraints).unwrap(),
        egui::vec2(30.0, 20.0)
    );
    assert_eq!(
        tree.layout(constraints).unwrap().size,
        egui::vec2(30.0, 20.0)
    );
    assert_eq!(
        tree.dry_baseline(constraints, TextBaseline::Alphabetic)
            .unwrap(),
        Some(12.0)
    );
    assert_eq!(
        tree.intrinsic(IntrinsicDimension::MaxWidth, 0.0).unwrap(),
        30.0
    );
}
#[test]
fn unmount_from_status_listener_retires_the_just_started_run() {
    let (ctx, log) = setup();
    let tree = Rc::new(RefCell::new(ElementTree::new(&ctx, tracked(&log)).unwrap()));
    let c = log.borrow().controllers[0].clone();
    let weak = Rc::downgrade(&tree);
    let calls = Rc::new(Cell::new(0));
    let seen = calls.clone();
    let _sub = c
        .subscribe_status(Rc::new(move || {
            seen.set(seen.get() + 1);
            weak.upgrade().unwrap().borrow_mut().unmount();
        }))
        .unwrap();
    let run = c.forward().unwrap();
    assert_eq!(run.outcome(), Some(AnimationOutcome::Canceled));
    assert!(c.is_disposed());
    assert_eq!(calls.get(), 1);
    assert!(log.borrow().already_disposed);
}

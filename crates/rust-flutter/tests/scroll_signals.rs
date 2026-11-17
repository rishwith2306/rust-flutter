use rust_flutter::{
    egui::{self, Id, pos2, vec2},
    element::{ElementTree, FlexItem, FlexStyle, Widget},
    gestures::*,
    layout::*,
    scrolling::*,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
fn event(delta: f32) -> PointerScrollEvent {
    PointerScrollEvent {
        position: pos2(20.0, 20.0),
        scroll_delta: vec2(0.0, delta),
        unit: ScrollSignalUnit::Points,
        time: Duration::ZERO,
        modifiers: Default::default(),
    }
}
fn config(callback: impl Fn(PointerScrollEvent) -> bool + 'static) -> GestureTargetConfig {
    GestureTargetConfig {
        on_scroll: Some(Rc::new(callback)),
        ..Default::default()
    }
}
#[test]
fn first_willing_handler_wins_once_without_pointer_arena_or_capture() {
    let b = GestureBinding::new();
    let log = Rc::new(RefCell::new(vec![]));
    let l = log.clone();
    let a = Id::new("a");
    let p = Id::new("p");
    let _a = b
        .register_target(
            a,
            config(move |_| {
                l.borrow_mut().push("a");
                true
            }),
        )
        .unwrap();
    let l = log.clone();
    let _p = b
        .register_target(
            p,
            config(move |_| {
                l.borrow_mut().push("p");
                true
            }),
        )
        .unwrap();
    let r = b.dispatch_scroll(event(10.0), [a, a, p]);
    assert_eq!(*log.borrow(), vec!["a"]);
    assert_eq!(r.scroll_claims[0].target, a);
    assert_eq!(r.processed, 1);
    assert_eq!(b.active_pointers(), 0);
    assert_eq!(b.arena().active_pointers(), 0);
    assert_eq!(b.router().route_count(), 0);
}
#[test]
fn declining_child_allows_parent_and_invalid_events_do_not_call_handlers() {
    let b = GestureBinding::new();
    let calls = Rc::new(Cell::new(0));
    let c = calls.clone();
    let a = Id::new("a");
    let p = Id::new("p");
    let _a = b.register_target(a, config(|_| false)).unwrap();
    let _p = b
        .register_target(
            p,
            config(move |_| {
                c.set(c.get() + 1);
                true
            }),
        )
        .unwrap();
    assert_eq!(
        b.dispatch_scroll(event(10.0), [a, p]).scroll_claims[0].target,
        p
    );
    for e in [
        PointerScrollEvent {
            position: pos2(f32::NAN, 0.0),
            ..event(1.0)
        },
        event(f32::INFINITY),
    ] {
        let r = b.dispatch_scroll(e, [p]);
        assert_eq!(r.scroll_failures.len(), 1);
    }
    assert!(b.dispatch_scroll(event(0.0), [p]).scroll_claims.is_empty());
    assert_eq!(calls.get(), 1);
}
#[test]
fn panic_aborts_event_instead_of_falling_through_after_possible_side_effects() {
    let b = GestureBinding::new();
    let calls = Rc::new(Cell::new(0));
    let c = calls.clone();
    let a = Id::new("a");
    let p = Id::new("p");
    let _a = b
        .register_target(a, config(|_| panic!("signal callback")))
        .unwrap();
    let _p = b
        .register_target(
            p,
            config(move |_| {
                c.set(1);
                true
            }),
        )
        .unwrap();
    let r = b.dispatch_scroll(event(10.0), [a, p]);
    assert_eq!(
        r.scroll_failures[0].error,
        BindingError::ScrollCallbackPanicked(a)
    );
    assert_eq!(calls.get(), 0);
}
#[test]
fn reentrant_pointer_and_signal_packets_wait_until_current_signal_finishes() {
    let b = GestureBinding::new();
    let a = Id::new("a");
    let p = Id::new("p");
    let tap = Id::new("tap");
    let log = Rc::new(RefCell::new(vec![]));
    let l = log.clone();
    let _p = b
        .register_target(
            p,
            config(move |_| {
                l.borrow_mut().push("p");
                true
            }),
        )
        .unwrap();
    let l = log.clone();
    let _tap = b
        .register_target(
            tap,
            GestureTargetConfig {
                tap: Some((
                    TapConfig::default(),
                    TapCallbacks {
                        on_down: Some(Rc::new(move |_| l.borrow_mut().push("down"))),
                        ..Default::default()
                    },
                )),
                ..Default::default()
            },
        )
        .unwrap();
    let weak = b.downgrade();
    let l = log.clone();
    let _a = b
        .register_target(
            a,
            config(move |_| {
                l.borrow_mut().push("a");
                let b = weak.upgrade().unwrap();
                assert!(
                    b.dispatch(
                        PointerEvent {
                            pointer: PointerId(7),
                            phase: PointerPhase::Down,
                            position: pos2(0.0, 0.0),
                            time: Duration::ZERO,
                            device: PointerDeviceKind::Mouse,
                            buttons: PointerButtons::PRIMARY
                        },
                        [tap]
                    )
                    .queued
                );
                assert!(b.dispatch_scroll(event(5.0), [p]).queued);
                false
            }),
        )
        .unwrap();
    let r = b.dispatch_scroll(event(10.0), [a, p]);
    assert_eq!(*log.borrow(), vec!["a", "p", "down", "p"]);
    assert_eq!(r.processed, 3);
    assert_eq!(r.scroll_claims.len(), 2);
}
#[test]
fn removed_and_replaced_targets_do_not_receive_old_hit_snapshots() {
    let b = GestureBinding::new();
    let a = Id::new("a");
    let p = Id::new("p");
    let calls = Rc::new(Cell::new(0));
    let c = calls.clone();
    let token = Rc::new(RefCell::new(Some(
        b.register_target(
            p,
            config(move |_| {
                c.set(c.get() + 1);
                true
            }),
        )
        .unwrap(),
    )));
    let old = token.clone();
    let weak = b.downgrade();
    let c = calls.clone();
    let _a = b
        .register_target(
            a,
            config(move |_| {
                old.borrow_mut().take();
                let c = c.clone();
                *old.borrow_mut() = Some(
                    weak.upgrade()
                        .unwrap()
                        .register_target(
                            p,
                            config(move |_| {
                                c.set(c.get() + 1);
                                true
                            }),
                        )
                        .unwrap(),
                );
                false
            }),
        )
        .unwrap();
    assert!(
        b.dispatch_scroll(event(10.0), [a, p])
            .scroll_claims
            .is_empty()
    );
    assert_eq!(calls.get(), 0);
    assert_eq!(b.dispatch_scroll(event(10.0), [p]).scroll_claims.len(), 1);
}
#[test]
fn callback_updates_and_disposal_are_reentrant_without_config_borrows() {
    let b = GestureBinding::new();
    let a = Id::new("a");
    let token = Rc::new(RefCell::new(None::<GestureTarget>));
    let slot = token.clone();
    *token.borrow_mut() = Some(
        b.register_target(
            a,
            config(move |_| {
                slot.borrow()
                    .as_ref()
                    .unwrap()
                    .update(config(|_| true))
                    .unwrap();
                false
            }),
        )
        .unwrap(),
    );
    assert!(b.dispatch_scroll(event(10.0), [a]).scroll_claims.is_empty());
    assert_eq!(b.dispatch_scroll(event(10.0), [a]).scroll_claims.len(), 1);
    let weak = b.downgrade();
    token
        .borrow()
        .as_ref()
        .unwrap()
        .update(config(move |_| {
            weak.upgrade().unwrap().dispose();
            false
        }))
        .unwrap();
    b.dispatch_scroll(event(1.0), [a]);
    assert!(b.is_disposed());
    assert_eq!(
        b.dispatch_scroll(event(1.0), [a]).scroll_failures[0].error,
        BindingError::Disposed
    );
}
fn setup(
    axis: Axis,
    reverse: bool,
) -> (egui::Context, ScrollPosition, GestureBinding, ElementTree) {
    let ctx = egui::Context::default();
    ctx.begin_pass(egui::RawInput {
        time: Some(0.0),
        ..Default::default()
    });
    let p = ScrollPosition::new(&ctx, 0.0).unwrap();
    let b = GestureBinding::new();
    let tree = ElementTree::new(
        &ctx,
        SingleChildScrollView::new(&p, &b)
            .unwrap()
            .axis(axis)
            .reverse(reverse)
            .into_widget(Widget::leaf(vec2(500.0, 500.0), |_| {}))
            .unwrap(),
    )
    .unwrap();
    (ctx, p, b, tree)
}
fn constraints() -> BoxConstraints {
    BoxConstraints::tight(vec2(80.0, 100.0)).unwrap()
}
fn wheel(unit: egui::MouseWheelUnit, y: f32) -> egui::Event {
    egui::Event::MouseWheel {
        unit,
        delta: vec2(0.0, y),
        modifiers: Default::default(),
    }
}
fn scene(
    ctx: &egui::Context,
    b: &GestureBinding,
    tree: &mut ElementTree,
    events: Vec<egui::Event>,
    t: f64,
    repeat: bool,
) -> Vec<DispatchReport> {
    let mut reports = vec![];
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(600.0, 400.0),
            )),
            time: Some(t),
            events,
            ..Default::default()
        },
        |ctx| {
            let mut f = b.begin_egui_frame(ctx).unwrap();
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(
                    egui::Rect::from_min_size(pos2(10.0, 10.0), vec2(80.0, 100.0)),
                ));
                tree.show_in_gesture_frame(&mut child, constraints(), &mut f)
                    .unwrap();
            });
            reports.push(f.finish().unwrap());
            if repeat && ctx.current_pass_index() == 0 {
                ctx.request_discard("test wheel replay");
            }
        },
    );
    reports
}
#[test]
fn viewport_converts_points_lines_pages_and_modifier_policy() {
    let (ctx, p, b, mut tree) = setup(Axis::Vertical, false);
    let g = tree.layout(constraints()).unwrap();
    let _ = ctx.end_pass();
    for (unit, delta, expected) in [
        (ScrollSignalUnit::Points, 30.0, 30.0),
        (ScrollSignalUnit::Lines, 2.0, 70.0),
        (ScrollSignalUnit::Pages, 0.5, 120.0),
    ] {
        let r = b.dispatch_scroll_layout(
            PointerScrollEvent {
                unit,
                ..event(delta)
            },
            &g,
            pos2(0.0, 0.0),
        );
        assert_eq!(r.scroll_claims.len(), 1);
        assert_eq!(p.pixels(), expected);
    }
    assert!(
        b.dispatch_scroll_layout(
            PointerScrollEvent {
                modifiers: egui::Modifiers::CTRL,
                ..event(20.0)
            },
            &g,
            pos2(0.0, 0.0)
        )
        .scroll_claims
        .is_empty()
    );
    assert_eq!(p.pixels(), 120.0);
}
#[test]
fn horizontal_shift_and_reverse_map_unit_delta() {
    for reverse in [false, true] {
        let (ctx, p, b, mut tree) = setup(Axis::Horizontal, reverse);
        let g = tree.layout(constraints()).unwrap();
        let _ = ctx.end_pass();
        let e = PointerScrollEvent {
            unit: ScrollSignalUnit::Lines,
            modifiers: egui::Modifiers::SHIFT,
            ..event(if reverse { -2.0 } else { 2.0 })
        };
        assert_eq!(
            b.dispatch_scroll_layout(e, &g, pos2(0.0, 0.0))
                .scroll_claims
                .len(),
            1
        );
        assert_eq!(p.pixels(), 40.0);
    }
}
#[test]
fn shared_frame_routes_raw_wheel_once_and_tracks_stationary_cursor() {
    let (ctx, p, b, mut tree) = setup(Axis::Vertical, false);
    let _ = ctx.end_pass();
    let r = scene(
        &ctx,
        &b,
        &mut tree,
        vec![
            egui::Event::PointerMoved(pos2(30.0, 30.0)),
            wheel(egui::MouseWheelUnit::Point, -30.0),
        ],
        1.0,
        true,
    );
    assert_eq!(p.pixels(), 30.0);
    assert_eq!(r.iter().map(|r| r.scroll_claims.len()).sum::<usize>(), 1);
    scene(
        &ctx,
        &b,
        &mut tree,
        vec![wheel(egui::MouseWheelUnit::Point, -20.0)],
        2.0,
        false,
    );
    assert_eq!(p.pixels(), 50.0);
}
#[test]
fn ordered_cursor_events_do_not_route_wheels_to_a_future_pointer_location() {
    let (ctx, p, b, mut tree) = setup(Axis::Vertical, false);
    let _ = ctx.end_pass();
    scene(
        &ctx,
        &b,
        &mut tree,
        vec![
            wheel(egui::MouseWheelUnit::Point, -50.0),
            egui::Event::PointerMoved(pos2(30.0, 30.0)),
            wheel(egui::MouseWheelUnit::Point, -20.0),
            egui::Event::PointerGone,
            wheel(egui::MouseWheelUnit::Point, -50.0),
        ],
        1.0,
        false,
    );
    assert_eq!(p.pixels(), 20.0);
}
#[test]
fn points_use_layer_scale_but_lines_and_pages_are_local_units() {
    let (ctx, p, b, mut tree) = setup(Axis::Vertical, false);
    let _ = ctx.end_pass();
    let layer = egui::LayerId::background();
    ctx.set_transform_layer(
        layer,
        egui::emath::TSTransform {
            scaling: 2.0,
            translation: vec2(0.0, 0.0),
        },
    );
    scene(
        &ctx,
        &b,
        &mut tree,
        vec![
            egui::Event::PointerMoved(pos2(60.0, 60.0)),
            wheel(egui::MouseWheelUnit::Point, -80.0),
        ],
        1.0,
        false,
    );
    assert_eq!(p.pixels(), 40.0);
    scene(
        &ctx,
        &b,
        &mut tree,
        vec![wheel(egui::MouseWheelUnit::Line, -2.0)],
        2.0,
        false,
    );
    assert_eq!(p.pixels(), 80.0);
    scene(
        &ctx,
        &b,
        &mut tree,
        vec![wheel(egui::MouseWheelUnit::Page, -0.5)],
        3.0,
        false,
    );
    assert_eq!(p.pixels(), 130.0);
}
#[test]
fn nested_viewport_at_edge_hands_signal_to_ancestor_without_residual_split() {
    let ctx = egui::Context::default();
    ctx.begin_pass(egui::RawInput::default());
    let b = GestureBinding::new();
    let outer = ScrollPosition::new(&ctx, 0.0).unwrap();
    let inner = ScrollPosition::new(&ctx, 0.0).unwrap();
    let child = SingleChildScrollView::new(&inner, &b)
        .unwrap()
        .into_widget(Widget::leaf(vec2(80.0, 500.0), |_| {}))
        .unwrap();
    let mut style = FlexStyle::new(Axis::Vertical);
    style.main_size = MainAxisSize::Min;
    let column = Widget::flex(
        style,
        vec![
            FlexItem::fixed(Widget::constrained(constraints(), child)),
            FlexItem::fixed(Widget::leaf(vec2(80.0, 500.0), |_| {})),
        ],
    );
    let mut tree = ElementTree::new(
        &ctx,
        SingleChildScrollView::new(&outer, &b)
            .unwrap()
            .into_widget(column)
            .unwrap(),
    )
    .unwrap();
    let g = tree.layout(constraints()).unwrap();
    let _ = ctx.end_pass();
    b.dispatch_scroll_layout(event(30.0), &g, pos2(0.0, 0.0));
    assert_eq!((inner.pixels(), outer.pixels()), (30.0, 0.0));
    inner.jump_to(390.0).unwrap();
    b.dispatch_scroll_layout(event(30.0), &g, pos2(0.0, 0.0));
    assert_eq!((inner.pixels(), outer.pixels()), (400.0, 0.0));
    b.dispatch_scroll_layout(event(30.0), &g, pos2(0.0, 0.0));
    assert_eq!((inner.pixels(), outer.pixels()), (400.0, 30.0));
}
#[test]
fn single_root_adapter_also_routes_wheel_input() {
    let (ctx, p, b, mut tree) = setup(Axis::Vertical, false);
    let _ = ctx.end_pass();
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(300.0, 200.0),
            )),
            time: Some(1.0),
            events: vec![
                egui::Event::PointerMoved(pos2(30.0, 30.0)),
                wheel(egui::MouseWheelUnit::Point, -30.0),
            ],
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                tree.show_with_gestures(ui, constraints(), &b).unwrap();
            });
        },
    );
    assert_eq!(p.pixels(), 30.0);
}

#[test]
fn wheel_cancels_programmatic_motion_and_external_service_disposal_is_terminal() {
    let (ctx, p, b, mut tree) = setup(Axis::Vertical, false);
    let g = tree.layout(constraints()).unwrap();
    let _ = ctx.end_pass();
    let run = p
        .animate_to(
            300.0,
            Duration::from_secs(1),
            rust_flutter::animation::Curve::Linear,
        )
        .unwrap();
    b.dispatch_scroll_layout(event(20.0), &g, pos2(0.0, 0.0));
    assert_eq!(
        run.outcome(),
        Some(rust_flutter::animation::AnimationOutcome::Canceled)
    );
    assert_eq!(p.pixels(), 20.0);
    assert!(!p.is_scrolling());
    b.arena().dispose().unwrap();
    assert_eq!(
        b.dispatch_scroll_layout(event(20.0), &g, pos2(0.0, 0.0))
            .scroll_failures[0]
            .error,
        BindingError::Disposed
    );
    assert!(b.is_disposed());
}

#[test]
fn front_root_receives_signal_and_disabled_opaque_root_blocks_the_back_root() {
    let (ctx, p, b, mut back) = setup(Axis::Vertical, false);
    let q = ScrollPosition::new(&ctx, 0.0).unwrap();
    let mut front = ElementTree::new(
        &ctx,
        SingleChildScrollView::new(&q, &b)
            .unwrap()
            .into_widget(Widget::leaf(vec2(80.0, 500.0), |_| {}))
            .unwrap(),
    )
    .unwrap();
    let _ = ctx.end_pass();
    for (enabled, time) in [(true, 1.0), (false, 2.0)] {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    pos2(0.0, 0.0),
                    vec2(400.0, 300.0),
                )),
                time: Some(time),
                events: vec![
                    egui::Event::PointerMoved(pos2(30.0, 30.0)),
                    wheel(egui::MouseWheelUnit::Point, -30.0),
                ],
                ..Default::default()
            },
            |ctx| {
                let mut f = b.begin_egui_frame(ctx).unwrap();
                egui::CentralPanel::default().show(ctx, |ui| {
                    for (index, tree) in [&mut back, &mut front].into_iter().enumerate() {
                        let mut child =
                            ui.new_child(egui::UiBuilder::new().id_salt(index).max_rect(
                                egui::Rect::from_min_size(pos2(10.0, 10.0), vec2(80.0, 100.0)),
                            ));
                        if index == 1 && !enabled {
                            child.disable();
                        }
                        tree.show_in_gesture_frame(&mut child, constraints(), &mut f)
                            .unwrap();
                    }
                });
                let r = f.finish().unwrap();
                assert_eq!(r.scroll_claims.len(), usize::from(enabled));
            },
        );
        assert_eq!((p.pixels(), q.pixels()), (0.0, 30.0));
    }
}

#[test]
fn single_root_wheel_uses_cursor_at_event_time_instead_of_final_frame_position() {
    let (ctx, p, b, mut tree) = setup(Axis::Vertical, false);
    let _ = ctx.end_pass();
    for (t, events) in [
        (1.0, vec![egui::Event::PointerMoved(pos2(30.0, 30.0))]),
        (
            2.0,
            vec![
                wheel(egui::MouseWheelUnit::Point, -30.0),
                egui::Event::PointerMoved(pos2(200.0, 150.0)),
            ],
        ),
    ] {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    pos2(0.0, 0.0),
                    vec2(300.0, 200.0),
                )),
                time: Some(t),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    tree.show_with_gestures(ui, constraints(), &b).unwrap();
                });
            },
        );
    }
    assert_eq!(p.pixels(), 30.0);
}

#[test]
fn unfinished_frame_does_not_leave_a_stale_wheel_cursor() {
    let (ctx, p, b, mut tree) = setup(Axis::Vertical, false);
    let _ = ctx.end_pass();
    scene(
        &ctx,
        &b,
        &mut tree,
        vec![egui::Event::PointerMoved(pos2(30.0, 30.0))],
        1.0,
        false,
    );
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(600.0, 400.0),
            )),
            time: Some(2.0),
            events: vec![egui::Event::PointerMoved(pos2(200.0, 200.0))],
            ..Default::default()
        },
        |ctx| {
            let mut f = b.begin_egui_frame(ctx).unwrap();
            egui::CentralPanel::default().show(ctx, |ui| {
                tree.show_in_gesture_frame(ui, constraints(), &mut f)
                    .unwrap();
            });
            drop(f);
        },
    );
    scene(
        &ctx,
        &b,
        &mut tree,
        vec![wheel(egui::MouseWheelUnit::Point, -30.0)],
        3.0,
        false,
    );
    assert_eq!(p.pixels(), 0.0);
}

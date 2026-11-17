use rust_flutter::{
    egui::{self, pos2, vec2},
    element::*,
    gestures::*,
    layout::*,
    scrolling::*,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
fn setup() -> (egui::Context, ScrollPosition, GestureBinding) {
    let ctx = egui::Context::default();
    ctx.begin_pass(egui::RawInput {
        time: Some(0.0),
        ..Default::default()
    });
    let p = ScrollPosition::new(&ctx, 0.0).unwrap();
    (ctx, p, GestureBinding::new())
}
fn widget(
    p: &ScrollPosition,
    b: &GestureBinding,
    axis: Axis,
    reverse: bool,
    size: egui::Vec2,
) -> Widget {
    SingleChildScrollView::new(p, b)
        .unwrap()
        .axis(axis)
        .reverse(reverse)
        .into_widget(Widget::leaf(size, |_| {}))
        .unwrap()
}
fn constraints() -> BoxConstraints {
    BoxConstraints::tight(vec2(80.0, 100.0)).unwrap()
}
fn event(phase: PointerPhase, x: f32, y: f32, ms: u64) -> PointerEvent {
    PointerEvent {
        pointer: PointerId(1),
        phase,
        position: pos2(x, y),
        time: Duration::from_millis(ms),
        device: PointerDeviceKind::Touch,
        buttons: if matches!(phase, PointerPhase::Up | PointerPhase::Cancel) {
            PointerButtons::NONE
        } else {
            PointerButtons::PRIMARY
        },
    }
}
fn send(b: &GestureBinding, g: &LayoutBox, e: PointerEvent) {
    let report = b.dispatch_layout(e, g, pos2(0.0, 0.0));
    assert!(report.failures.is_empty(), "{:?}", report.failures);
}
#[test]
fn viewport_unbounds_scroll_axis_clips_and_publishes_dimensions_after_pass() {
    let (ctx, p, b) = setup();
    let mut tree = ElementTree::new(
        &ctx,
        widget(&p, &b, Axis::Vertical, false, vec2(80.0, 300.0)),
    )
    .unwrap();
    let g = tree.layout(constraints()).unwrap();
    let viewport = &g.children[0].layout;
    assert_eq!(g.size, vec2(80.0, 100.0));
    assert_eq!(viewport.clip, Clip::HardEdge);
    assert_eq!(viewport.children[0].layout.size, vec2(80.0, 300.0));
    assert_eq!(viewport.paint_bounds.size(), viewport.size);
    assert!(g.hit_test_path(pos2(20.0, 120.0)).is_empty());
    assert!(p.dimensions().is_none());
    let _ = ctx.end_pass();
    let d = p.dimensions().unwrap();
    assert_eq!(d.max_scroll_extent(), 200.0);
    assert_eq!(d.viewport_dimension(), 100.0);
}
#[test]
fn dry_queries_are_pure_and_intrinsic_protocol_is_explicitly_unsupported() {
    let (ctx, p, b) = setup();
    let mut tree = ElementTree::new(
        &ctx,
        widget(&p, &b, Axis::Vertical, false, vec2(80.0, 300.0)),
    )
    .unwrap();
    assert_eq!(tree.dry_layout(constraints()).unwrap(), vec2(80.0, 100.0));
    assert_eq!(
        tree.dry_baseline(constraints(), TextBaseline::Alphabetic)
            .unwrap(),
        None
    );
    assert!(tree.intrinsic(IntrinsicDimension::MaxWidth, 100.0).is_err());
    let _ = ctx.end_pass();
    assert!(p.dimensions().is_none());
}
#[test]
fn offsets_relayout_cached_geometry_and_keep_element_ids() {
    let (ctx, p, b) = setup();
    let mut tree = ElementTree::new(
        &ctx,
        widget(&p, &b, Axis::Vertical, false, vec2(80.0, 300.0)),
    )
    .unwrap();
    tree.layout(constraints()).unwrap();
    let _ = ctx.end_pass();
    let ids = tree.elements().iter().map(|e| e.id).collect::<Vec<_>>();
    p.jump_to(75.0).unwrap();
    let g = tree.layout(constraints()).unwrap();
    assert_eq!(g.children[0].layout.children[0].offset, vec2(0.0, -75.0));
    assert_eq!(
        tree.elements().iter().map(|e| e.id).collect::<Vec<_>>(),
        ids
    );
}
#[test]
fn horizontal_reverse_and_short_content_alignment_work() {
    let (ctx, p, b) = setup();
    let mut tree = ElementTree::new(
        &ctx,
        widget(&p, &b, Axis::Horizontal, true, vec2(300.0, 100.0)),
    )
    .unwrap();
    let g = tree.layout(constraints()).unwrap();
    assert_eq!(g.children[0].layout.children[0].offset, vec2(-220.0, 0.0));
    let _ = ctx.end_pass();
    p.jump_to(50.0).unwrap();
    assert_eq!(
        tree.layout(constraints()).unwrap().children[0]
            .layout
            .children[0]
            .offset,
        vec2(-170.0, 0.0)
    );
    tree.update(widget(&p, &b, Axis::Horizontal, true, vec2(30.0, 100.0)))
        .unwrap();
    let g = tree.layout(constraints()).unwrap();
    assert_eq!(g.children[0].layout.children[0].offset, vec2(50.0, 0.0));
}
#[test]
fn loose_and_unbounded_constraints_shrink_wrap_finite_content() {
    let (ctx, p, b) = setup();
    let mut tree = ElementTree::new(
        &ctx,
        widget(&p, &b, Axis::Vertical, false, vec2(30.0, 40.0)),
    )
    .unwrap();
    assert_eq!(
        tree.layout(BoxConstraints::default()).unwrap().size,
        vec2(30.0, 40.0)
    );
    let _ = ctx.end_pass();
    assert_eq!(p.dimensions().unwrap().max_scroll_extent(), 0.0);
}
#[test]
fn viewport_resize_corrects_offset_after_pass_without_layout_callbacks() {
    let (ctx, p, b) = setup();
    let mut tree = ElementTree::new(
        &ctx,
        widget(&p, &b, Axis::Vertical, false, vec2(80.0, 300.0)),
    )
    .unwrap();
    tree.layout(constraints()).unwrap();
    let _ = ctx.end_pass();
    p.jump_to(200.0).unwrap();
    let called = Rc::new(Cell::new(0));
    let c = called.clone();
    let _s = p
        .subscribe_metrics(Rc::new(move || c.set(c.get() + 1)))
        .unwrap();
    ctx.begin_pass(egui::RawInput {
        time: Some(1.0),
        ..Default::default()
    });
    let g = tree
        .layout(BoxConstraints::tight(vec2(80.0, 250.0)).unwrap())
        .unwrap();
    assert_eq!(g.children[0].layout.children[0].offset.y, -50.0);
    assert_eq!(p.pixels(), 200.0);
    assert_eq!(called.get(), 0);
    let _ = ctx.end_pass();
    assert_eq!(p.pixels(), 50.0);
    assert_eq!(called.get(), 1);
}
#[test]
fn unmount_suppresses_pending_metrics_and_releases_viewport_lease() {
    let (ctx, p, b) = setup();
    let mut tree = ElementTree::new(
        &ctx,
        widget(&p, &b, Axis::Vertical, false, vec2(80.0, 300.0)),
    )
    .unwrap();
    tree.layout(constraints()).unwrap();
    tree.update(Widget::empty()).unwrap();
    let _ = ctx.end_pass();
    assert!(p.dimensions().is_none());
    let mut other = ElementTree::new(
        &ctx,
        widget(&p, &b, Axis::Vertical, false, vec2(80.0, 200.0)),
    )
    .unwrap();
    assert!(other.layout(constraints()).is_ok());
    assert!(!p.is_disposed());
}
#[test]
fn duplicate_live_viewports_fail_without_disturbing_the_original() {
    let (ctx, p, b) = setup();
    let mut first = ElementTree::new(
        &ctx,
        widget(&p, &b, Axis::Vertical, false, vec2(80.0, 300.0)),
    )
    .unwrap();
    first.layout(constraints()).unwrap();
    let mut second = ElementTree::new(
        &ctx,
        widget(&p, &b, Axis::Vertical, false, vec2(80.0, 500.0)),
    )
    .unwrap();
    assert!(second.layout(constraints()).is_err());
    assert!(first.layout(constraints()).is_ok());
    let _ = ctx.end_pass();
    assert_eq!(p.dimensions().unwrap().max_scroll_extent(), 200.0);
}
#[test]
fn pan_drives_offset_with_capture_outside_and_stationary_release_stops() {
    let (ctx, p, b) = setup();
    let mut tree = ElementTree::new(
        &ctx,
        widget(&p, &b, Axis::Vertical, false, vec2(80.0, 300.0)),
    )
    .unwrap();
    let g = tree.layout(constraints()).unwrap();
    let _ = ctx.end_pass();
    send(&b, &g, event(PointerPhase::Down, 40.0, 80.0, 0));
    send(&b, &g, event(PointerPhase::Move, 40.0, 40.0, 10));
    assert_eq!(p.pixels(), 40.0);
    send(&b, &g, event(PointerPhase::Move, 40.0, -20.0, 20));
    assert_eq!(p.pixels(), 100.0);
    send(&b, &g, event(PointerPhase::Up, 40.0, -20.0, 100));
    assert_eq!(p.activity(), ScrollActivity::Idle);
    assert_eq!(b.active_pointers(), 0);
}
#[test]
fn horizontal_reverse_pan_maps_axis_and_sign() {
    let (ctx, p, b) = setup();
    let mut tree = ElementTree::new(
        &ctx,
        widget(&p, &b, Axis::Horizontal, true, vec2(300.0, 100.0)),
    )
    .unwrap();
    let g = tree.layout(constraints()).unwrap();
    let _ = ctx.end_pass();
    send(&b, &g, event(PointerPhase::Down, 10.0, 40.0, 0));
    send(&b, &g, event(PointerPhase::Move, 50.0, 40.0, 10));
    assert_eq!(p.pixels(), 40.0);
    send(&b, &g, event(PointerPhase::Cancel, 50.0, 40.0, 20));
    assert_eq!(p.activity(), ScrollActivity::Idle);
}
#[test]
fn child_tap_competes_with_scroll_pan_and_still_works_after_drag() {
    let (ctx, p, b) = setup();
    let taps = Rc::new(Cell::new(0));
    let log = taps.clone();
    let child = GestureDetector::new(
        &b,
        GestureTargetConfig {
            tap: Some((
                TapConfig::default(),
                TapCallbacks {
                    on_tap: Some(Rc::new(move |_| log.set(log.get() + 1))),
                    ..Default::default()
                },
            )),
            ..Default::default()
        },
    )
    .unwrap()
    .into_widget(Widget::leaf(vec2(80.0, 300.0), |_| {}));
    let mut tree = ElementTree::new(
        &ctx,
        SingleChildScrollView::new(&p, &b)
            .unwrap()
            .into_widget(child)
            .unwrap(),
    )
    .unwrap();
    let g = tree.layout(constraints()).unwrap();
    let _ = ctx.end_pass();
    send(&b, &g, event(PointerPhase::Down, 40.0, 80.0, 0));
    send(&b, &g, event(PointerPhase::Move, 40.0, 40.0, 10));
    send(&b, &g, event(PointerPhase::Up, 40.0, 40.0, 100));
    assert_eq!(taps.get(), 0);
    assert_eq!(p.pixels(), 40.0);
    let g = tree.layout(constraints()).unwrap();
    send(&b, &g, event(PointerPhase::Down, 40.0, 40.0, 200));
    send(&b, &g, event(PointerPhase::Up, 40.0, 40.0, 220));
    assert_eq!(taps.get(), 1);
    assert_eq!(p.pixels(), 40.0);
}
#[test]
fn changing_position_cancels_old_capture_and_preserves_child_identity() {
    let (ctx, p, b) = setup();
    let q = ScrollPosition::new(&ctx, 0.0).unwrap();
    let mut tree = ElementTree::new(
        &ctx,
        widget(&p, &b, Axis::Vertical, false, vec2(80.0, 300.0)),
    )
    .unwrap();
    let g = tree.layout(constraints()).unwrap();
    let _ = ctx.end_pass();
    let ids = tree.elements().iter().map(|e| e.id).collect::<Vec<_>>();
    send(&b, &g, event(PointerPhase::Down, 40.0, 80.0, 0));
    send(&b, &g, event(PointerPhase::Move, 40.0, 40.0, 10));
    assert_eq!(p.activity(), ScrollActivity::Drag);
    tree.update(widget(&q, &b, Axis::Vertical, false, vec2(80.0, 300.0)))
        .unwrap();
    tree.layout(constraints()).unwrap();
    assert_eq!(p.activity(), ScrollActivity::Idle);
    assert_eq!(q.pixels(), 0.0);
    assert_eq!(
        tree.elements().iter().map(|e| e.id).collect::<Vec<_>>(),
        ids
    );
    send(&b, &g, event(PointerPhase::Up, 40.0, 10.0, 100));
    assert_eq!(q.pixels(), 0.0);
}
#[test]
fn metrics_callbacks_can_query_the_tree_after_render_borrows_are_released() {
    let (ctx, p, b) = setup();
    let tree = Rc::new(RefCell::new(
        ElementTree::new(
            &ctx,
            widget(&p, &b, Axis::Vertical, false, vec2(80.0, 300.0)),
        )
        .unwrap(),
    ));
    let weak = Rc::downgrade(&tree);
    let result = Rc::new(Cell::new(false));
    let got = result.clone();
    let _s = p
        .subscribe_metrics(Rc::new(move || {
            let tree = weak.upgrade().unwrap();
            got.set(tree.borrow_mut().dry_layout(constraints()).is_ok());
        }))
        .unwrap();
    tree.borrow_mut().layout(constraints()).unwrap();
    let _ = ctx.end_pass();
    assert!(result.get());
}

#[test]
fn transformed_scene_converts_global_pan_distance_after_widget_update() {
    let (ctx, p, b) = setup();
    let _ = ctx.end_pass();
    let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("scroll-scale"));
    ctx.set_transform_layer(
        layer,
        egui::emath::TSTransform {
            scaling: 2.0,
            translation: vec2(20.0, 10.0),
        },
    );
    let mut tree = ElementTree::new(
        &ctx,
        widget(&p, &b, Axis::Vertical, false, vec2(80.0, 300.0)),
    )
    .unwrap();
    for (index, events) in [
        vec![],
        vec![egui::Event::PointerButton {
            pos: pos2(100.0, 190.0),
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Default::default(),
        }],
        vec![egui::Event::PointerMoved(pos2(100.0, 110.0))],
    ]
    .into_iter()
    .enumerate()
    {
        if index == 2 {
            tree.update(widget(&p, &b, Axis::Vertical, false, vec2(80.0, 300.0)))
                .unwrap();
        }
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    pos2(0.0, 0.0),
                    vec2(600.0, 400.0),
                )),
                time: Some(1.0 + index as f64 * 0.1),
                events,
                ..Default::default()
            },
            |ctx| {
                let mut frame = b.begin_egui_frame(ctx).unwrap();
                egui::CentralPanel::default().show(ctx, |ui| {
                    let mut child = ui.new_child(egui::UiBuilder::new().layer_id(layer).max_rect(
                        egui::Rect::from_min_size(pos2(10.0, 10.0), vec2(80.0, 100.0)),
                    ));
                    tree.show_in_gesture_frame(&mut child, constraints(), &mut frame)
                        .unwrap();
                });
                assert!(frame.finish().unwrap().failures.is_empty());
            },
        );
    }
    assert_eq!(p.pixels(), 40.0);
}

#[test]
fn axis_or_reverse_changes_cancel_capture_without_replacing_elements() {
    let (ctx, p, b) = setup();
    let mut tree = ElementTree::new(
        &ctx,
        widget(&p, &b, Axis::Vertical, false, vec2(300.0, 300.0)),
    )
    .unwrap();
    let g = tree.layout(constraints()).unwrap();
    let _ = ctx.end_pass();
    let ids = tree.elements().iter().map(|e| e.id).collect::<Vec<_>>();
    send(&b, &g, event(PointerPhase::Down, 40.0, 80.0, 0));
    send(&b, &g, event(PointerPhase::Move, 40.0, 40.0, 10));
    assert_eq!(p.activity(), ScrollActivity::Drag);
    tree.update(widget(&p, &b, Axis::Horizontal, true, vec2(300.0, 300.0)))
        .unwrap();
    tree.layout(constraints()).unwrap();
    assert_eq!(p.activity(), ScrollActivity::Idle);
    assert_eq!(
        tree.elements().iter().map(|e| e.id).collect::<Vec<_>>(),
        ids
    );
}

#[test]
fn latest_layout_wins_and_a_later_unmounted_delivery_is_skipped() {
    let (ctx, p, b) = setup();
    let q = ScrollPosition::new(&ctx, 0.0).unwrap();
    let mut first = ElementTree::new(
        &ctx,
        widget(&p, &b, Axis::Vertical, false, vec2(80.0, 300.0)),
    )
    .unwrap();
    let second = Rc::new(RefCell::new(
        ElementTree::new(
            &ctx,
            widget(&q, &b, Axis::Vertical, false, vec2(80.0, 500.0)),
        )
        .unwrap(),
    ));
    first.layout(constraints()).unwrap();
    first
        .layout(BoxConstraints::tight(vec2(80.0, 200.0)).unwrap())
        .unwrap();
    second.borrow_mut().layout(constraints()).unwrap();
    let other = Rc::downgrade(&second);
    let _s = p
        .subscribe_metrics(Rc::new(move || {
            other
                .upgrade()
                .unwrap()
                .borrow_mut()
                .update(Widget::empty())
                .unwrap()
        }))
        .unwrap();
    let _ = ctx.end_pass();
    assert_eq!(p.dimensions().unwrap().max_scroll_extent(), 100.0);
    assert!(q.dimensions().is_none());
}

#[test]
fn first_pass_pan_uses_measured_dimensions_before_end_pass() {
    let (ctx, p, b) = setup();
    let mut tree = ElementTree::new(
        &ctx,
        widget(&p, &b, Axis::Vertical, false, vec2(80.0, 300.0)),
    )
    .unwrap();
    let g = tree.layout(constraints()).unwrap();
    assert!(p.dimensions().is_none());
    send(&b, &g, event(PointerPhase::Down, 40.0, 80.0, 0));
    assert_eq!(p.dimensions().unwrap().max_scroll_extent(), 200.0);
    send(&b, &g, event(PointerPhase::Move, 40.0, 40.0, 10));
    assert_eq!(p.pixels(), 40.0);
    let _ = ctx.end_pass();
    assert_eq!(p.activity(), ScrollActivity::Drag);
}

#[test]
fn failed_second_viewport_can_acquire_after_the_original_unmounts() {
    let (ctx, p, b) = setup();
    let mut first = ElementTree::new(
        &ctx,
        widget(&p, &b, Axis::Vertical, false, vec2(80.0, 300.0)),
    )
    .unwrap();
    first.layout(constraints()).unwrap();
    let mut second = ElementTree::new(
        &ctx,
        widget(&p, &b, Axis::Vertical, false, vec2(80.0, 500.0)),
    )
    .unwrap();
    assert!(second.layout(constraints()).is_err());
    first.update(Widget::empty()).unwrap();
    assert!(second.layout(constraints()).is_ok());
    let _ = ctx.end_pass();
    assert_eq!(p.dimensions().unwrap().max_scroll_extent(), 400.0);
}

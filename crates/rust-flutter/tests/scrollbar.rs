use rust_flutter::{
    animation::{AnimationOutcome, Curve},
    egui::{self, pos2, vec2},
    element::{ElementTree, Widget},
    gestures::*,
    layout::*,
    notifier::Listenable,
    scrolling::*,
};
use std::{cell::Cell, rc::Rc, time::Duration};
fn widget(
    p: &ScrollPosition,
    b: &GestureBinding,
    axis: Axis,
    reverse: bool,
    interactive: bool,
) -> Widget {
    let size = if axis == Axis::Vertical {
        vec2(80.0, 300.0)
    } else {
        vec2(300.0, 80.0)
    };
    let view = SingleChildScrollView::new(p, b)
        .unwrap()
        .axis(axis)
        .reverse(reverse)
        .into_widget(Widget::leaf(size, |_| {}))
        .unwrap();
    Scrollbar::new(p, b)
        .unwrap()
        .reverse(reverse)
        .axis(axis)
        .interactive(interactive)
        .into_widget(view)
        .unwrap()
}
fn setup(
    axis: Axis,
    reverse: bool,
    interactive: bool,
) -> (egui::Context, ScrollPosition, GestureBinding, ElementTree) {
    let ctx = egui::Context::default();
    let p = ScrollPosition::new(&ctx, 0.0).unwrap();
    let b = GestureBinding::new();
    let t = ElementTree::new(&ctx, widget(&p, &b, axis, reverse, interactive)).unwrap();
    (ctx, p, b, t)
}
fn bounds(axis: Axis) -> BoxConstraints {
    BoxConstraints::tight(if axis == Axis::Vertical {
        vec2(80.0, 100.0)
    } else {
        vec2(100.0, 80.0)
    })
    .unwrap()
}
fn frame(
    ctx: &egui::Context,
    b: &GestureBinding,
    t: &mut ElementTree,
    axis: Axis,
    events: Vec<egui::Event>,
    enabled: bool,
) {
    let time = ctx.input(|i| i.time) + 0.05;
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(400.0, 400.0),
            )),
            time: Some(time),
            events,
            ..Default::default()
        },
        |ctx| {
            let mut f = b.begin_egui_frame(ctx).unwrap();
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut ui = ui.new_child(egui::UiBuilder::new().id_salt("bar-test").max_rect(
                    egui::Rect::from_min_size(pos2(10.0, 10.0), vec2(200.0, 200.0)),
                ));
                if !enabled {
                    ui.disable();
                }
                t.show_in_gesture_frame(&mut ui, bounds(axis), &mut f)
                    .unwrap();
            });
            let r = f.finish().unwrap();
            assert!(r.failures.is_empty(), "{:?}", r.failures);
        },
    );
}
fn warm(ctx: &egui::Context, b: &GestureBinding, t: &mut ElementTree, axis: Axis) {
    frame(ctx, b, t, axis, vec![], true);
    frame(ctx, b, t, axis, vec![], true);
}
fn button(x: f32, y: f32, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos: pos2(x, y),
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    }
}
fn click(x: f32, y: f32) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(pos2(x, y)),
        button(x, y, true),
        button(x, y, false),
    ]
}
fn thumb(g: &LayoutBox, interactive: bool) -> &LayoutBox {
    let track = &g.children[1].layout;
    let part = if interactive {
        &track.children[0].layout
    } else {
        track
    };
    let thumb = &part.children[0].layout;
    if interactive {
        &thumb.children[0].layout
    } else {
        thumb
    }
}
#[test]
fn proportional_geometry_tracks_offsets_and_retains_content() {
    let (ctx, p, b, mut t) = setup(Axis::Vertical, false, true);
    warm(&ctx, &b, &mut t, Axis::Vertical);
    let g = t.layout(bounds(Axis::Vertical)).unwrap();
    let id = g.children[0].layout.id;
    assert_eq!(g.size, vec2(80.0, 100.0));
    assert_eq!(g.clip, Clip::HardEdge);
    assert_eq!(g.children[1].offset, vec2(70.0, 2.0));
    assert_eq!(g.children[1].layout.size, vec2(8.0, 96.0));
    assert_eq!(thumb(&g, true).size, vec2(8.0, 32.0));
    p.jump_to(100.0).unwrap();
    let g = t.layout(bounds(Axis::Vertical)).unwrap();
    assert_eq!(g.children[0].layout.id, id);
    assert_eq!(
        g.children[1].layout.children[0].layout.children[0].offset,
        vec2(0.0, 32.0)
    );
    p.jump_to(200.0).unwrap();
    let g = t.layout(bounds(Axis::Vertical)).unwrap();
    assert_eq!(
        g.children[1].layout.children[0].layout.children[0].offset,
        vec2(0.0, 64.0)
    );
}
#[test]
fn thumb_drag_wins_against_content_pan_and_captures_outside() {
    let (ctx, p, b, mut t) = setup(Axis::Vertical, false, true);
    warm(&ctx, &b, &mut t, Axis::Vertical);
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Vertical,
        vec![
            egui::Event::PointerMoved(pos2(84.0, 28.0)),
            button(84.0, 28.0, true),
        ],
        true,
    );
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Vertical,
        vec![egui::Event::PointerMoved(pos2(84.0, 60.0))],
        true,
    );
    assert!((p.pixels() - 100.0).abs() < 0.001, "{}", p.pixels());
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Vertical,
        vec![egui::Event::PointerMoved(pos2(200.0, 160.0))],
        true,
    );
    assert_eq!(p.pixels(), 200.0);
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Vertical,
        vec![button(200.0, 160.0, false)],
        true,
    );
    assert_eq!(p.activity(), ScrollActivity::Idle);
}
#[test]
fn track_pages_both_ways_without_jumping_to_pointer() {
    let (ctx, p, b, mut t) = setup(Axis::Vertical, false, true);
    warm(&ctx, &b, &mut t, Axis::Vertical);
    frame(&ctx, &b, &mut t, Axis::Vertical, click(84.0, 90.0), true);
    assert_eq!(p.pixels(), 80.0);
    frame(&ctx, &b, &mut t, Axis::Vertical, click(84.0, 14.0), true);
    assert_eq!(p.pixels(), 0.0);
    frame(&ctx, &b, &mut t, Axis::Vertical, click(84.0, 28.0), true);
    assert_eq!(p.pixels(), 0.0);
}
#[test]
fn wheel_over_thumb_reaches_underlying_viewport() {
    let (ctx, p, b, mut t) = setup(Axis::Vertical, false, true);
    warm(&ctx, &b, &mut t, Axis::Vertical);
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Vertical,
        vec![
            egui::Event::PointerMoved(pos2(84.0, 28.0)),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0.0, -20.0),
                modifiers: Default::default(),
            },
        ],
        true,
    );
    assert_eq!(p.pixels(), 20.0);
}
#[test]
fn horizontal_reverse_thumb_drag_and_track_direction() {
    let (ctx, p, b, mut t) = setup(Axis::Horizontal, true, true);
    warm(&ctx, &b, &mut t, Axis::Horizontal);
    let g = t.layout(bounds(Axis::Horizontal)).unwrap();
    assert_eq!(thumb(&g, true).size, vec2(32.0, 8.0));
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Horizontal,
        vec![
            egui::Event::PointerMoved(pos2(92.0, 84.0)),
            button(92.0, 84.0, true),
        ],
        true,
    );
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Horizontal,
        vec![egui::Event::PointerMoved(pos2(60.0, 84.0))],
        true,
    );
    assert_eq!(p.pixels(), 100.0);
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Horizontal,
        vec![button(60.0, 84.0, false)],
        true,
    );
    frame(&ctx, &b, &mut t, Axis::Horizontal, click(14.0, 84.0), true);
    assert_eq!(p.pixels(), 180.0);
}
#[test]
fn noninteractive_bar_leaves_content_gestures_available() {
    let (ctx, p, b, mut t) = setup(Axis::Vertical, false, false);
    warm(&ctx, &b, &mut t, Axis::Vertical);
    frame(&ctx, &b, &mut t, Axis::Vertical, click(84.0, 90.0), true);
    assert_eq!(p.pixels(), 0.0);
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Vertical,
        vec![
            egui::Event::PointerMoved(pos2(84.0, 80.0)),
            button(84.0, 80.0, true),
        ],
        true,
    );
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Vertical,
        vec![egui::Event::PointerMoved(pos2(84.0, 40.0))],
        true,
    );
    assert_eq!(p.pixels(), 40.0);
}
#[test]
fn missing_zero_and_disposed_metrics_hide_the_bar() {
    let (ctx, p, b, mut t) = setup(Axis::Vertical, false, true);
    let g = t.layout(bounds(Axis::Vertical)).unwrap();
    assert_eq!(g.children[1].layout.size, egui::Vec2::ZERO);
    let _ = ctx.end_pass();
    warm(&ctx, &b, &mut t, Axis::Vertical);
    p.dispose().unwrap();
    assert!(t.layout(bounds(Axis::Vertical)).is_err());
    let ctx = egui::Context::default();
    let p = ScrollPosition::new(&ctx, 0.0).unwrap();
    let b = GestureBinding::new();
    p.apply_dimensions(ScrollDimensions::new(0.0, 0.0, 100.0, Axis::Vertical).unwrap())
        .unwrap();
    let mut t = ElementTree::new(
        &ctx,
        Scrollbar::new(&p, &b)
            .unwrap()
            .into_widget(Widget::leaf(vec2(80.0, 100.0), |_| {}))
            .unwrap(),
    )
    .unwrap();
    let g = t.layout(bounds(Axis::Vertical)).unwrap();
    assert_eq!(g.children[1].layout.size, egui::Vec2::ZERO);
    p.apply_dimensions(ScrollDimensions::new(0.0, 200.0, 100.0, Axis::Vertical).unwrap())
        .unwrap();
    let g = t.layout(bounds(Axis::Vertical)).unwrap();
    assert!(g.children[1].layout.size.y > 0.0);
    p.dispose().unwrap();
    let g = t.layout(bounds(Axis::Vertical)).unwrap();
    assert_eq!(g.children[1].layout.size, egui::Vec2::ZERO);
}
#[test]
fn invalid_style_is_rejected_and_extreme_finite_metrics_stay_finite() {
    let ctx = egui::Context::default();
    let p = ScrollPosition::new(&ctx, 0.0).unwrap();
    let b = GestureBinding::new();
    for style in [
        ScrollbarStyle {
            thickness: f32::NAN,
            ..Default::default()
        },
        ScrollbarStyle {
            margin: -1.0,
            ..Default::default()
        },
        ScrollbarStyle {
            min_thumb_length: 0.0,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            Scrollbar::new(&p, &b).unwrap().style(style),
            Err(ScrollError::InvalidScrollbarStyle)
        ));
    }
    p.apply_dimensions(
        ScrollDimensions::new(f32::MIN, f32::MAX, f32::MAX, Axis::Vertical).unwrap(),
    )
    .unwrap();
    let mut t = ElementTree::new(
        &ctx,
        Scrollbar::new(&p, &b)
            .unwrap()
            .into_widget(Widget::leaf(vec2(80.0, 100.0), |_| {}))
            .unwrap(),
    )
    .unwrap();
    let g = t.layout(bounds(Axis::Vertical)).unwrap();
    assert!(thumb(&g, true).size.is_finite());
    assert_eq!(thumb(&g, true).size.y, 32.0);
}
#[test]
fn thumb_down_cancels_programmatic_motion_and_unmount_releases_capture() {
    let (ctx, p, b, mut t) = setup(Axis::Vertical, false, true);
    warm(&ctx, &b, &mut t, Axis::Vertical);
    let run = p
        .animate_to(200.0, Duration::from_secs(10), Curve::Linear)
        .unwrap();
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Vertical,
        vec![
            egui::Event::PointerMoved(pos2(84.0, 28.0)),
            button(84.0, 28.0, true),
        ],
        true,
    );
    assert_eq!(run.outcome(), Some(AnimationOutcome::Canceled));
    drop(t);
    assert!(!p.is_disposed());
    assert_eq!(b.target_count(), 0);
}
#[test]
fn pixel_callbacks_during_thumb_drag_are_outside_render_borrows() {
    let (ctx, p, b, mut t) = setup(Axis::Vertical, false, true);
    warm(&ctx, &b, &mut t, Axis::Vertical);
    let called = Rc::new(Cell::new(0));
    let seen = called.clone();
    let copy = p.clone();
    let _sub = p
        .subscribe(Rc::new(move || {
            assert!(copy.metrics().is_some());
            seen.set(seen.get() + 1);
        }))
        .unwrap();
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Vertical,
        vec![
            egui::Event::PointerMoved(pos2(84.0, 28.0)),
            button(84.0, 28.0, true),
        ],
        true,
    );
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Vertical,
        vec![egui::Event::PointerMoved(pos2(84.0, 60.0))],
        true,
    );
    assert_eq!(called.get(), 1);
}

#[test]
fn rebuild_keeps_a_captured_thumb_and_noninteractive_update_cancels_it() {
    let (ctx, p, b, mut t) = setup(Axis::Vertical, false, true);
    warm(&ctx, &b, &mut t, Axis::Vertical);
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Vertical,
        vec![
            egui::Event::PointerMoved(pos2(84.0, 28.0)),
            button(84.0, 28.0, true),
        ],
        true,
    );
    t.update(widget(&p, &b, Axis::Vertical, false, true))
        .unwrap();
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Vertical,
        vec![egui::Event::PointerMoved(pos2(84.0, 60.0))],
        true,
    );
    assert_eq!(p.pixels(), 100.0);
    t.update(widget(&p, &b, Axis::Vertical, false, false))
        .unwrap();
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Vertical,
        vec![egui::Event::PointerMoved(pos2(84.0, 100.0))],
        true,
    );
    assert_eq!(p.pixels(), 100.0);
}
#[test]
fn dry_queries_do_not_publish_metrics_or_show_a_bar() {
    let (ctx, p, _b, mut t) = setup(Axis::Vertical, false, true);
    assert_eq!(
        t.dry_layout(bounds(Axis::Vertical)).unwrap(),
        vec2(80.0, 100.0)
    );
    assert_eq!(
        t.dry_baseline(bounds(Axis::Vertical), TextBaseline::Alphabetic)
            .unwrap(),
        None
    );
    assert!(p.dimensions().is_none());
    let _ = ctx;
}
#[test]
fn resize_and_style_update_recompute_thumb_and_keep_content() {
    let (ctx, p, b, mut t) = setup(Axis::Vertical, false, true);
    warm(&ctx, &b, &mut t, Axis::Vertical);
    let id = t.layout(bounds(Axis::Vertical)).unwrap().children[0]
        .layout
        .id;
    let c = BoxConstraints::tight(vec2(80.0, 150.0)).unwrap();
    ctx.begin_pass(egui::RawInput::default());
    t.layout(c).unwrap();
    let _ = ctx.end_pass();
    let g = t.layout(c).unwrap();
    assert_eq!(thumb(&g, true).size.y, 73.0);
    assert_eq!(g.children[0].layout.id, id);
    let child = SingleChildScrollView::new(&p, &b)
        .unwrap()
        .into_widget(Widget::leaf(vec2(80.0, 300.0), |_| {}))
        .unwrap();
    t.update(
        Scrollbar::new(&p, &b)
            .unwrap()
            .style(ScrollbarStyle {
                min_thumb_length: 100.0,
                ..Default::default()
            })
            .unwrap()
            .into_widget(child)
            .unwrap(),
    )
    .unwrap();
    let g = t.layout(c).unwrap();
    assert_eq!(thumb(&g, true).size.y, 100.0);
    assert_eq!(g.children[0].layout.id, id);
}
#[test]
fn disabled_ui_and_cross_axis_motion_do_not_move_thumb() {
    let (ctx, p, b, mut t) = setup(Axis::Vertical, false, true);
    warm(&ctx, &b, &mut t, Axis::Vertical);
    frame(&ctx, &b, &mut t, Axis::Vertical, click(84.0, 90.0), false);
    assert_eq!(p.pixels(), 0.0);
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Vertical,
        vec![
            egui::Event::PointerMoved(pos2(84.0, 28.0)),
            button(84.0, 28.0, true),
        ],
        true,
    );
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Vertical,
        vec![egui::Event::PointerMoved(pos2(140.0, 28.0))],
        true,
    );
    assert_eq!(p.pixels(), 0.0);
}
#[test]
fn scaled_layer_converts_thumb_and_track_input() {
    let (ctx, p, b, mut t) = setup(Axis::Vertical, false, true);
    let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("bar-scale"));
    ctx.set_transform_layer(
        layer,
        egui::emath::TSTransform {
            scaling: 2.0,
            translation: vec2(20.0, 10.0),
        },
    );
    for (i, events) in [
        vec![],
        vec![],
        vec![
            egui::Event::PointerMoved(pos2(188.0, 66.0)),
            button(188.0, 66.0, true),
        ],
        vec![egui::Event::PointerMoved(pos2(188.0, 130.0))],
        vec![button(188.0, 130.0, false)],
        click(188.0, 38.0),
    ]
    .into_iter()
    .enumerate()
    {
        if i == 3 {
            t.update(widget(&p, &b, Axis::Vertical, false, true))
                .unwrap();
        }
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    pos2(0.0, 0.0),
                    vec2(600.0, 400.0),
                )),
                time: Some(i as f64 * 0.1),
                events,
                ..Default::default()
            },
            |ctx| {
                let mut f = b.begin_egui_frame(ctx).unwrap();
                egui::CentralPanel::default().show(ctx, |ui| {
                    let mut ui = ui.new_child(egui::UiBuilder::new().layer_id(layer).max_rect(
                        egui::Rect::from_min_size(pos2(10.0, 10.0), vec2(80.0, 100.0)),
                    ));
                    t.show_in_gesture_frame(&mut ui, bounds(Axis::Vertical), &mut f)
                        .unwrap();
                });
                assert!(f.finish().unwrap().failures.is_empty());
            },
        );
        if i == 3 {
            assert_eq!(p.pixels(), 100.0);
        }
    }
    assert_eq!(p.pixels(), 20.0);
}
#[test]
fn reentrant_pixel_listener_can_query_layout_during_thumb_dispatch() {
    use std::cell::RefCell;
    let (ctx, p, b, mut t) = setup(Axis::Vertical, false, true);
    warm(&ctx, &b, &mut t, Axis::Vertical);
    let t = Rc::new(RefCell::new(t));
    let query = t.clone();
    let calls = Rc::new(Cell::new(0));
    let called = calls.clone();
    let _sub = p
        .subscribe(Rc::new(move || {
            query.borrow_mut().layout(bounds(Axis::Vertical)).unwrap();
            called.set(called.get() + 1);
        }))
        .unwrap();
    for events in [
        vec![
            egui::Event::PointerMoved(pos2(84.0, 28.0)),
            button(84.0, 28.0, true),
        ],
        vec![egui::Event::PointerMoved(pos2(84.0, 60.0))],
    ] {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    pos2(0.0, 0.0),
                    vec2(400.0, 400.0),
                )),
                time: Some(ctx.input(|i| i.time) + 0.05),
                events,
                ..Default::default()
            },
            |ctx| {
                let mut f = b.begin_egui_frame(ctx).unwrap();
                egui::CentralPanel::default().show(ctx, |ui| {
                    let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(
                        egui::Rect::from_min_size(pos2(10.0, 10.0), vec2(80.0, 100.0)),
                    ));
                    t.borrow_mut()
                        .show_in_gesture_frame(&mut ui, bounds(Axis::Vertical), &mut f)
                        .unwrap();
                });
                assert!(f.finish().unwrap().failures.is_empty());
            },
        );
    }
    assert_eq!(calls.get(), 1);
    assert_eq!(p.pixels(), 100.0);
}

fn shared_frame(
    ctx: &egui::Context,
    b: &GestureBinding,
    t: &std::cell::RefCell<ElementTree>,
    events: Vec<egui::Event>,
) {
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(400.0, 400.0),
            )),
            time: Some(ctx.input(|i| i.time) + 0.05),
            events,
            ..Default::default()
        },
        |ctx| {
            let mut f = b.begin_egui_frame(ctx).unwrap();
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(
                    egui::Rect::from_min_size(pos2(10.0, 10.0), vec2(80.0, 100.0)),
                ));
                t.borrow_mut()
                    .show_in_gesture_frame(&mut ui, bounds(Axis::Vertical), &mut f)
                    .unwrap();
            });
            assert!(f.finish().unwrap().failures.is_empty());
        },
    );
}
#[test]
fn application_command_supersedes_drag_and_release_preserves_new_motion() {
    let (ctx, p, b, mut t) = setup(Axis::Vertical, false, true);
    warm(&ctx, &b, &mut t, Axis::Vertical);
    let p2 = p.clone();
    let once = Rc::new(Cell::new(false));
    let done = once.clone();
    let _sub = p
        .subscribe(Rc::new(move || {
            if !done.replace(true) {
                p2.animate_to(180.0, Duration::from_secs(10), Curve::Linear)
                    .unwrap();
            }
        }))
        .unwrap();
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Vertical,
        vec![
            egui::Event::PointerMoved(pos2(84.0, 28.0)),
            button(84.0, 28.0, true),
        ],
        true,
    );
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Vertical,
        vec![egui::Event::PointerMoved(pos2(84.0, 60.0))],
        true,
    );
    assert_eq!(p.activity(), ScrollActivity::Driven);
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Vertical,
        vec![egui::Event::PointerMoved(pos2(84.0, 95.0))],
        true,
    );
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Vertical,
        vec![button(84.0, 95.0, false)],
        true,
    );
    assert_eq!(p.activity(), ScrollActivity::Driven);
    assert!(p.pixels() < 110.0);
}
#[test]
fn unmount_during_drag_start_does_not_leave_position_dragging() {
    use std::cell::RefCell;
    let (ctx, p, b, mut t) = setup(Axis::Vertical, false, true);
    warm(&ctx, &b, &mut t, Axis::Vertical);
    let t = Rc::new(RefCell::new(t));
    let weak = Rc::downgrade(&t);
    let p2 = p.clone();
    let _sub = p
        .subscribe_activity(Rc::new(move || {
            if p2.activity() == ScrollActivity::Drag {
                weak.upgrade()
                    .unwrap()
                    .borrow_mut()
                    .update(Widget::leaf(vec2(80.0, 100.0), |_| {}))
                    .unwrap();
            }
        }))
        .unwrap();
    shared_frame(
        &ctx,
        &b,
        &t,
        vec![
            egui::Event::PointerMoved(pos2(84.0, 28.0)),
            button(84.0, 28.0, true),
        ],
    );
    shared_frame(
        &ctx,
        &b,
        &t,
        vec![egui::Event::PointerMoved(pos2(84.0, 60.0))],
    );
    assert_eq!(p.activity(), ScrollActivity::Idle);
    assert_eq!(p.pixels(), 0.0);
    assert_eq!(b.target_count(), 0);
}
#[test]
fn rebuild_during_drag_start_preserves_owned_drag() {
    use std::cell::RefCell;
    let (ctx, p, b, mut t) = setup(Axis::Vertical, false, true);
    warm(&ctx, &b, &mut t, Axis::Vertical);
    let t = Rc::new(RefCell::new(t));
    let weak = Rc::downgrade(&t);
    let p2 = p.clone();
    let binding = b.clone();
    let once = Rc::new(Cell::new(false));
    let done = once.clone();
    let _sub = p
        .subscribe_activity(Rc::new(move || {
            if p2.activity() == ScrollActivity::Drag && !done.replace(true) {
                weak.upgrade()
                    .unwrap()
                    .borrow_mut()
                    .update(widget(&p2, &binding, Axis::Vertical, false, true))
                    .unwrap();
            }
        }))
        .unwrap();
    shared_frame(
        &ctx,
        &b,
        &t,
        vec![
            egui::Event::PointerMoved(pos2(84.0, 28.0)),
            button(84.0, 28.0, true),
        ],
    );
    shared_frame(
        &ctx,
        &b,
        &t,
        vec![egui::Event::PointerMoved(pos2(84.0, 60.0))],
    );
    assert_eq!(p.activity(), ScrollActivity::Drag);
    assert_eq!(p.pixels(), 100.0);
    shared_frame(&ctx, &b, &t, vec![button(84.0, 60.0, false)]);
    assert_eq!(p.activity(), ScrollActivity::Idle);
}

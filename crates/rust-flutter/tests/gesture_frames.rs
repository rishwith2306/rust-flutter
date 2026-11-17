use rust_flutter::{
    egui::{self, Pos2, Rect, pos2, vec2},
    element::{ElementTree, RenderBox, RenderContext, RenderObjectWidget, Widget},
    gestures::*,
    layout::*,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
type Log = Rc<RefCell<Vec<String>>>;
fn spec(log: &Log, name: &str, pan: bool) -> GestureTargetConfig {
    let (a, b, c, d) = (log.clone(), log.clone(), log.clone(), log.clone());
    let (n, m, p, q) = (
        name.to_string(),
        name.to_string(),
        name.to_string(),
        name.to_string(),
    );
    if pan {
        GestureTargetConfig {
            pan: Some((
                PanConfig::default(),
                PanCallbacks {
                    on_down: Some(Rc::new(move |_| a.borrow_mut().push(format!("{n} down")))),
                    on_start: Some(Rc::new(move |_| b.borrow_mut().push(format!("{m} pan")))),
                    on_end: Some(Rc::new(move |_| c.borrow_mut().push(format!("{p} end")))),
                    on_cancel: Some(Rc::new(move |_| d.borrow_mut().push(format!("{q} cancel")))),
                    ..Default::default()
                },
            )),
            ..Default::default()
        }
    } else {
        GestureTargetConfig {
            tap: Some((
                TapConfig::default(),
                TapCallbacks {
                    on_down: Some(Rc::new(move |_| a.borrow_mut().push(format!("{n} down")))),
                    on_tap: Some(Rc::new(move |_| b.borrow_mut().push(format!("{m} tap")))),
                    on_cancel: Some(Rc::new(move |_| c.borrow_mut().push(format!("{p} cancel")))),
                },
            )),
            ..Default::default()
        }
    }
}
fn detector(b: &GestureBinding, log: &Log, name: &str, pan: bool, transparent: bool) -> Widget {
    let child = if transparent {
        Widget::render_object(Transparent, vec![])
    } else {
        Widget::leaf(vec2(50.0, 30.0), |_| {})
    };
    GestureDetector::new(b, spec(log, name, pan))
        .unwrap()
        .behavior(if transparent {
            HitTestBehavior::Translucent
        } else {
            HitTestBehavior::Opaque
        })
        .into_widget(child)
}
#[derive(Clone, Copy)]
struct Transparent;
impl RenderObjectWidget for Transparent {
    type Object = Self;
    fn create_render_object(&self, _: &RenderContext) -> Self {
        *self
    }
    fn update_render_object(&self, _: &mut Self, _: &RenderContext) {}
}
impl RenderBox for Transparent {
    fn layout(
        &mut self,
        c: BoxConstraints,
        _: &mut BoxChildren<'_>,
        _: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        let mut g = BoxGeometry::new(c.constrain(vec2(50.0, 30.0))?);
        g.hit_test_behavior = Some(HitTestBehavior::Translucent);
        Ok(g)
    }
}
fn mouse(x: f32, y: f32, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos: pos2(x, y),
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    }
}
#[derive(Clone, Copy)]
struct Place {
    index: usize,
    origin: Pos2,
    enabled: bool,
}
fn place(index: usize, x: f32) -> Place {
    Place {
        index,
        origin: pos2(x, 10.0),
        enabled: true,
    }
}
fn frame(
    ctx: &egui::Context,
    b: &GestureBinding,
    trees: &mut [ElementTree],
    places: &[Place],
    events: Vec<egui::Event>,
    time: f64,
) -> Vec<DispatchReport> {
    let mut reports = Vec::new();
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 240.0))),
            time: Some(time),
            events,
            ..Default::default()
        },
        |ctx| {
            let mut gestures = b.begin_egui_frame(ctx).unwrap();
            egui::CentralPanel::default().show(ctx, |ui| {
                for p in places {
                    let mut child = ui.new_child(
                        egui::UiBuilder::new()
                            .id_salt(p.index)
                            .max_rect(Rect::from_min_size(p.origin, vec2(100.0, 60.0))),
                    );
                    if !p.enabled {
                        child.disable();
                    }
                    trees[p.index]
                        .show_in_gesture_frame(
                            &mut child,
                            BoxConstraints::loose(vec2(100.0, 60.0)).unwrap(),
                            &mut gestures,
                        )
                        .unwrap();
                }
            });
            let report = gestures.finish().unwrap();
            assert!(report.failures.is_empty(), "{:?}", report.failures);
            reports.push(report);
            if ctx.current_pass_index() == 0 {
                ctx.request_discard("test multipass collection");
            }
        },
    );
    assert_eq!(reports.len(), 2);
    reports
}
#[test]
fn roots_dispatch_once_after_all_collection_and_later_painted_root_is_frontmost() {
    let ctx = egui::Context::default();
    let b = GestureBinding::new();
    let log = Log::default();
    let mut trees = [
        ElementTree::new(&ctx, detector(&b, &log, "back", false, false)).unwrap(),
        ElementTree::new(&ctx, detector(&b, &log, "front", false, false)).unwrap(),
    ];
    let reports = frame(
        &ctx,
        &b,
        &mut trees,
        &[place(0, 10.0), place(1, 10.0)],
        vec![mouse(20.0, 20.0, true), mouse(20.0, 20.0, false)],
        0.0,
    );
    assert_eq!(*log.borrow(), vec!["front down", "front tap"]);
    assert_eq!(reports[0].processed, 2);
    assert_eq!(reports[1].processed, 0);
    log.borrow_mut().clear();
    frame(
        &ctx,
        &b,
        &mut trees,
        &[place(1, 10.0), place(0, 10.0)],
        vec![mouse(20.0, 20.0, true), mouse(20.0, 20.0, false)],
        0.1,
    );
    assert_eq!(*log.borrow(), vec!["back down", "back tap"]);
}
#[test]
fn translucent_front_root_and_root_behind_compete_in_one_arena() {
    let ctx = egui::Context::default();
    let b = GestureBinding::new();
    let log = Log::default();
    let mut trees = [
        ElementTree::new(&ctx, detector(&b, &log, "back", false, false)).unwrap(),
        ElementTree::new(&ctx, detector(&b, &log, "front", true, true)).unwrap(),
    ];
    frame(
        &ctx,
        &b,
        &mut trees,
        &[place(0, 10.0), place(1, 10.0)],
        vec![mouse(20.0, 20.0, true), mouse(20.0, 20.0, false)],
        0.0,
    );
    assert_eq!(
        *log.borrow(),
        vec!["front down", "back down", "front cancel", "back tap"]
    );
    assert_eq!(b.active_pointers(), 0);
}
#[test]
fn disjoint_roots_receive_their_own_clicks_and_capture_does_not_retarget() {
    let ctx = egui::Context::default();
    let b = GestureBinding::new();
    let log = Log::default();
    let mut trees = [
        ElementTree::new(&ctx, detector(&b, &log, "a", true, false)).unwrap(),
        ElementTree::new(&ctx, detector(&b, &log, "b", false, false)).unwrap(),
    ];
    let places = [place(0, 10.0), place(1, 110.0)];
    frame(
        &ctx,
        &b,
        &mut trees,
        &places,
        vec![mouse(20.0, 20.0, true)],
        0.0,
    );
    frame(
        &ctx,
        &b,
        &mut trees,
        &places,
        vec![
            egui::Event::PointerMoved(pos2(120.0, 20.0)),
            mouse(120.0, 20.0, false),
        ],
        0.1,
    );
    assert_eq!(*log.borrow(), vec!["a down", "a pan", "a end"]);
    frame(
        &ctx,
        &b,
        &mut trees,
        &places,
        vec![mouse(120.0, 20.0, true), mouse(120.0, 20.0, false)],
        0.2,
    );
    assert_eq!(log.borrow().last(), Some(&"b tap".into()));
}
#[test]
fn omitted_and_disabled_roots_cancel_tracking_and_disabled_top_root_blocks_behind() {
    for omit in [true, false] {
        let ctx = egui::Context::default();
        let b = GestureBinding::new();
        let log = Log::default();
        let mut trees = [
            ElementTree::new(&ctx, detector(&b, &log, "back", false, false)).unwrap(),
            ElementTree::new(&ctx, detector(&b, &log, "front", false, false)).unwrap(),
        ];
        frame(
            &ctx,
            &b,
            &mut trees,
            &[place(0, 10.0), place(1, 10.0)],
            vec![mouse(20.0, 20.0, true)],
            0.0,
        );
        let mut places = vec![place(0, 10.0)];
        if !omit {
            places.push(Place {
                enabled: false,
                ..place(1, 10.0)
            });
        }
        frame(
            &ctx,
            &b,
            &mut trees,
            &places,
            vec![mouse(20.0, 20.0, false)],
            0.1,
        );
        assert_eq!(*log.borrow(), vec!["front down", "front cancel"]);
        assert_eq!(b.router().route_count(), 0);
        if !omit {
            log.borrow_mut().clear();
            frame(
                &ctx,
                &b,
                &mut trees,
                &places,
                vec![mouse(20.0, 20.0, true), mouse(20.0, 20.0, false)],
                0.2,
            );
            assert!(log.borrow().is_empty());
            assert_eq!(b.active_pointers(), 0);
        }
    }
}
#[test]
fn unfinished_collector_cancels_capture_and_releases_exclusive_lease() {
    let ctx = egui::Context::default();
    let b = GestureBinding::new();
    let log = Log::default();
    let mut trees = [ElementTree::new(&ctx, detector(&b, &log, "a", false, false)).unwrap()];
    frame(
        &ctx,
        &b,
        &mut trees,
        &[place(0, 10.0)],
        vec![mouse(20.0, 20.0, true)],
        0.0,
    );
    let _ = ctx.run(
        egui::RawInput {
            time: Some(0.1),
            ..Default::default()
        },
        |ctx| {
            let collector = b.begin_egui_frame(ctx).unwrap();
            assert!(matches!(
                b.begin_egui_frame(ctx),
                Err(BindingError::FrameAlreadyOpen)
            ));
            drop(collector);
            let collector = b.begin_egui_frame(ctx).unwrap();
            drop(collector);
        },
    );
    assert_eq!(*log.borrow(), vec!["a down", "a cancel"]);
    assert_eq!(b.active_pointers(), 0);
}
fn button(b: &GestureBinding, count: &Rc<Cell<usize>>, enabled: bool) -> Widget {
    let count = count.clone();
    GestureButton::new(b, "Action", move || count.set(count.get() + 1))
        .size(vec2(50.0, 30.0))
        .enabled(enabled)
        .into_widget()
}
#[test]
fn arena_button_pointer_action_is_not_duplicated_by_parent_tap_or_egui() {
    let ctx = egui::Context::default();
    let b = GestureBinding::new();
    let count = Rc::new(Cell::new(0));
    let log = Log::default();
    let root = GestureDetector::new(&b, spec(&log, "parent", false))
        .unwrap()
        .into_widget(button(&b, &count, true));
    let mut trees = [ElementTree::new(&ctx, root).unwrap()];
    frame(
        &ctx,
        &b,
        &mut trees,
        &[place(0, 10.0)],
        vec![mouse(20.0, 20.0, true), mouse(20.0, 20.0, false)],
        0.0,
    );
    assert_eq!(count.get(), 1);
    assert_eq!(*log.borrow(), vec!["parent down", "parent cancel"]);
    assert_eq!(b.router().route_count(), 0);
}
#[test]
fn arena_button_loses_pointer_action_to_parent_pan() {
    let ctx = egui::Context::default();
    let b = GestureBinding::new();
    let count = Rc::new(Cell::new(0));
    let log = Log::default();
    let root = GestureDetector::new(&b, spec(&log, "parent", true))
        .unwrap()
        .into_widget(button(&b, &count, true));
    let mut trees = [ElementTree::new(&ctx, root).unwrap()];
    frame(
        &ctx,
        &b,
        &mut trees,
        &[place(0, 10.0)],
        vec![mouse(20.0, 20.0, true)],
        0.0,
    );
    frame(
        &ctx,
        &b,
        &mut trees,
        &[place(0, 10.0)],
        vec![
            egui::Event::PointerMoved(pos2(150.0, 20.0)),
            mouse(150.0, 20.0, false),
        ],
        0.1,
    );
    assert_eq!(count.get(), 0);
    assert_eq!(
        *log.borrow(),
        vec!["parent down", "parent pan", "parent end"]
    );
}
fn key(key: egui::Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Default::default(),
    }
}
#[test]
fn arena_button_keyboard_activation_works_once_in_multipass_after_pointer_focus() {
    let ctx = egui::Context::default();
    let b = GestureBinding::new();
    let count = Rc::new(Cell::new(0));
    let mut trees = [ElementTree::new(&ctx, button(&b, &count, true)).unwrap()];
    frame(
        &ctx,
        &b,
        &mut trees,
        &[place(0, 10.0)],
        vec![mouse(20.0, 20.0, true), mouse(20.0, 20.0, false)],
        0.0,
    );
    assert_eq!(count.get(), 1);
    frame(
        &ctx,
        &b,
        &mut trees,
        &[place(0, 10.0)],
        vec![key(egui::Key::Enter)],
        0.1,
    );
    assert_eq!(count.get(), 2);
    frame(
        &ctx,
        &b,
        &mut trees,
        &[place(0, 10.0)],
        vec![key(egui::Key::Space)],
        0.2,
    );
    assert_eq!(count.get(), 3);
}
#[test]
fn arena_button_callback_updates_preserve_tap_and_disabling_cancels_it() {
    let ctx = egui::Context::default();
    let b = GestureBinding::new();
    let old = Rc::new(Cell::new(0));
    let new = Rc::new(Cell::new(0));
    let mut trees = [ElementTree::new(&ctx, button(&b, &old, true)).unwrap()];
    frame(
        &ctx,
        &b,
        &mut trees,
        &[place(0, 10.0)],
        vec![mouse(20.0, 20.0, true)],
        0.0,
    );
    trees[0].update(button(&b, &new, true)).unwrap();
    frame(
        &ctx,
        &b,
        &mut trees,
        &[place(0, 10.0)],
        vec![mouse(20.0, 20.0, false)],
        0.1,
    );
    assert_eq!(old.get(), 0);
    assert_eq!(new.get(), 1);
    frame(
        &ctx,
        &b,
        &mut trees,
        &[place(0, 10.0)],
        vec![mouse(20.0, 20.0, true)],
        0.2,
    );
    trees[0].update(button(&b, &new, false)).unwrap();
    frame(
        &ctx,
        &b,
        &mut trees,
        &[place(0, 10.0)],
        vec![mouse(20.0, 20.0, false), key(egui::Key::Enter)],
        0.3,
    );
    assert_eq!(new.get(), 1);
}
#[test]
fn bare_foreground_layer_uses_uniform_transform_for_hit_testing() {
    let ctx = egui::Context::default();
    let b = GestureBinding::new();
    let log = Log::default();
    let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("transformed"));
    ctx.set_transform_layer(
        layer,
        egui::emath::TSTransform {
            scaling: 2.0,
            translation: vec2(20.0, 10.0),
        },
    );
    let mut tree = ElementTree::new(&ctx, detector(&b, &log, "scaled", false, false)).unwrap();
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 240.0))),
            events: vec![mouse(60.0, 50.0, true), mouse(60.0, 50.0, false)],
            ..Default::default()
        },
        |ctx| {
            let mut f = b.begin_egui_frame(ctx).unwrap();
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .layer_id(layer)
                        .max_rect(Rect::from_min_size(pos2(10.0, 10.0), vec2(100.0, 60.0))),
                );
                tree.show_in_gesture_frame(
                    &mut child,
                    BoxConstraints::loose(vec2(100.0, 60.0)).unwrap(),
                    &mut f,
                )
                .unwrap();
            });
            assert!(f.finish().unwrap().failures.is_empty());
        },
    );
    assert_eq!(*log.borrow(), vec!["scaled down", "scaled tap"]);
}
#[test]
fn unrelated_backend_area_occludes_roots_beneath_it() {
    let ctx = egui::Context::default();
    let b = GestureBinding::new();
    let log = Log::default();
    let clicks = Cell::new(0);
    let mut tree = ElementTree::new(&ctx, detector(&b, &log, "under", false, false)).unwrap();
    for (time, events) in [
        (0.0, vec![]),
        (0.1, vec![]),
        (
            0.2,
            vec![
                egui::Event::PointerMoved(pos2(20.0, 20.0)),
                mouse(20.0, 20.0, true),
            ],
        ),
        (0.3, vec![mouse(20.0, 20.0, false)]),
    ] {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 240.0))),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ctx| {
                let mut f = b.begin_egui_frame(ctx).unwrap();
                egui::CentralPanel::default().show(ctx, |ui| {
                    tree.show_in_gesture_frame(
                        ui,
                        BoxConstraints::loose(vec2(100.0, 60.0)).unwrap(),
                        &mut f,
                    )
                    .unwrap();
                });
                egui::Area::new(egui::Id::new("backend-area"))
                    .order(egui::Order::Foreground)
                    .fixed_pos(pos2(10.0, 10.0))
                    .movable(false)
                    .show(ctx, |ui| {
                        if ui
                            .add_sized(vec2(80.0, 40.0), egui::Button::new("Backend"))
                            .clicked()
                        {
                            clicks.set(clicks.get() + 1);
                        }
                    });
                assert!(f.finish().unwrap().failures.is_empty());
            },
        );
    }
    assert!(log.borrow().is_empty());
    assert!(clicks.get() > 0);
    assert_eq!(b.active_pointers(), 0);
}
#[test]
fn collector_respects_half_open_clip_edges_and_duplicate_root_errors() {
    let ctx = egui::Context::default();
    let b = GestureBinding::new();
    let log = Log::default();
    let mut tree = ElementTree::new(&ctx, detector(&b, &log, "clipped", false, false)).unwrap();
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 240.0))),
            events: vec![mouse(20.0, 15.0, true)],
            ..Default::default()
        },
        |ctx| {
            let mut f = b.begin_egui_frame(ctx).unwrap();
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.set_clip_rect(Rect::from_min_max(Pos2::ZERO, pos2(20.0, 20.0)));
                let g = tree
                    .layout(BoxConstraints::loose(vec2(100.0, 60.0)).unwrap())
                    .unwrap();
                f.add_root(ui, &g, pos2(10.0, 10.0)).unwrap();
                assert_eq!(
                    f.add_root(ui, &g, pos2(10.0, 10.0)),
                    Err(BindingError::DuplicateRoot(g.id))
                );
            });
            assert!(f.finish().unwrap().failures.is_empty());
        },
    );
    assert_eq!(b.active_pointers(), 0);
    assert!(log.borrow().is_empty());
}
#[test]
fn expired_collectors_release_lease_and_invalid_layer_transform_is_rejected() {
    let ctx = egui::Context::default();
    let b = GestureBinding::new();
    let mut pending = None;
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        pending = Some(b.begin_egui_frame(ctx).unwrap());
    });
    assert_eq!(
        pending.take().unwrap().finish().err(),
        Some(BindingError::FrameExpired)
    );
    let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("bad-scale"));
    for scaling in [0.0, -1.0, f32::from_bits(1)] {
        ctx.set_transform_layer(
            layer,
            egui::emath::TSTransform {
                scaling,
                translation: egui::Vec2::ZERO,
            },
        );
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let mut f = b.begin_egui_frame(ctx).unwrap();
            egui::CentralPanel::default().show(ctx, |ui| {
                let child = ui.new_child(egui::UiBuilder::new().layer_id(layer));
                let g = BoxNode::leaf("manual", vec2(20.0, 20.0))
                    .layout(BoxConstraints::loose(vec2(50.0, 50.0)).unwrap())
                    .unwrap();
                f.add_root(&child, &g, pos2(10.0, 10.0)).unwrap();
            });
            assert_eq!(f.finish().err(), Some(BindingError::InvalidTransform));
            assert!(b.begin_egui_frame(ctx).is_ok()); // temporary collector drops and releases
        });
    }
}
#[test]
fn switching_between_single_root_and_collected_input_cancels_old_capture() {
    for first_single in [true, false] {
        let ctx = egui::Context::default();
        let b = GestureBinding::new();
        let log = Log::default();
        let mut tree = ElementTree::new(&ctx, detector(&b, &log, "a", false, false)).unwrap();
        for i in 0..2 {
            let single = if i == 0 { first_single } else { !first_single };
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 240.0))),
                    time: Some(i as f64),
                    events: vec![mouse(20.0, 20.0, i == 0)],
                    ..Default::default()
                },
                |ctx| {
                    let mut f = if single {
                        None
                    } else {
                        Some(b.begin_egui_frame(ctx).unwrap())
                    };
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let c = BoxConstraints::loose(vec2(100.0, 60.0)).unwrap();
                        if let Some(f) = f.as_mut() {
                            tree.show_in_gesture_frame(ui, c, f).unwrap();
                        } else {
                            tree.show_with_gestures(ui, c, &b).unwrap();
                        }
                    });
                    if let Some(f) = f {
                        assert!(f.finish().unwrap().failures.is_empty());
                    }
                },
            );
        }
        assert_eq!(*log.borrow(), vec!["a down", "a cancel"]);
        assert_eq!(b.active_pointers(), 0);
    }
}
#[test]
fn button_queries_are_pure_and_invalid_sizes_can_be_corrected() {
    let ctx = egui::Context::default();
    let b = GestureBinding::new();
    let count = Rc::new(Cell::new(0));
    let mut tree = ElementTree::new(&ctx, button(&b, &count, true)).unwrap();
    let c = BoxConstraints::loose(vec2(100.0, 100.0)).unwrap();
    assert_eq!(tree.dry_layout(c).unwrap(), vec2(50.0, 30.0));
    assert_eq!(
        tree.intrinsic(IntrinsicDimension::MinWidth, 100.0).unwrap(),
        50.0
    );
    assert_eq!(
        tree.intrinsic(IntrinsicDimension::MaxHeight, 100.0)
            .unwrap(),
        30.0
    );
    assert_eq!(
        tree.dry_baseline(c, TextBaseline::Alphabetic).unwrap(),
        None
    );
    assert_eq!(count.get(), 0);
    tree.update(
        GestureButton::new(&b, "Bad", || {})
            .size(vec2(f32::NAN, 20.0))
            .into_widget(),
    )
    .unwrap();
    assert!(matches!(
        tree.layout(c),
        Err(rust_flutter::element::ElementError::Layout(
            LayoutError::InvalidSize
        ))
    ));
    tree.update(button(&b, &count, true)).unwrap();
    assert_eq!(tree.layout(c).unwrap().size, vec2(50.0, 30.0));
}
#[cfg(not(target_os = "android"))]
#[test]
fn arena_button_exports_and_handles_desktop_accesskit_click_once() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let b = GestureBinding::new();
    let count = Rc::new(Cell::new(0));
    let mut trees = [ElementTree::new(&ctx, button(&b, &count, true)).unwrap()];
    let output = ctx.run(egui::RawInput::default(), |ctx| {
        let mut f = b.begin_egui_frame(ctx).unwrap();
        egui::CentralPanel::default().show(ctx, |ui| {
            trees[0]
                .show_in_gesture_frame(
                    ui,
                    BoxConstraints::loose(vec2(100.0, 60.0)).unwrap(),
                    &mut f,
                )
                .unwrap();
        });
        f.finish().unwrap();
    });
    let update = output.platform_output.accesskit_update.unwrap();
    let (id, node) = update
        .nodes
        .iter()
        .find(|(_, n)| n.role() == egui::accesskit::Role::Button)
        .unwrap();
    assert!(node.supports_action(egui::accesskit::Action::Click));
    let event = egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
        action: egui::accesskit::Action::Click,
        target: *id,
        data: None,
    });
    frame(&ctx, &b, &mut trees, &[place(0, 10.0)], vec![event], 0.1);
    assert_eq!(count.get(), 1);
    assert_eq!(b.active_pointers(), 0);
}
#[test]
fn backend_modal_gate_cancels_captured_targets_below_it() {
    let ctx = egui::Context::default();
    let b = GestureBinding::new();
    let log = Log::default();
    let mut trees = [ElementTree::new(&ctx, detector(&b, &log, "under", false, false)).unwrap()];
    frame(
        &ctx,
        &b,
        &mut trees,
        &[place(0, 10.0)],
        vec![mouse(20.0, 20.0, true)],
        0.0,
    );
    for i in 0..2 {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 240.0))),
                time: Some(0.1 + i as f64 * 0.1),
                events: if i == 0 {
                    vec![]
                } else {
                    vec![mouse(20.0, 20.0, false)]
                },
                ..Default::default()
            },
            |ctx| {
                let mut f = b.begin_egui_frame(ctx).unwrap();
                egui::CentralPanel::default().show(ctx, |ui| {
                    trees[0]
                        .show_in_gesture_frame(
                            ui,
                            BoxConstraints::loose(vec2(100.0, 60.0)).unwrap(),
                            &mut f,
                        )
                        .unwrap();
                });
                ctx.memory_mut(|m| {
                    m.set_modal_layer(egui::LayerId::new(
                        egui::Order::Foreground,
                        egui::Id::new("modal"),
                    ))
                });
                assert!(f.finish().unwrap().failures.is_empty());
            },
        );
    }
    assert_eq!(*log.borrow(), vec!["under down", "under cancel"]);
    assert_eq!(b.active_pointers(), 0);
}

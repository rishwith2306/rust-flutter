use rust_flutter::{
    animation::{AnimationOutcome, Curve},
    egui::{self, Key, Modifiers, pos2, vec2},
    element::{ElementTree, Widget},
    gestures::*,
    layout::*,
    notifier::*,
    scrolling::*,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
fn key(k: Key, m: Modifiers) -> egui::Event {
    egui::Event::Key {
        key: k,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: m,
    }
}
fn click() -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(pos2(30.0, 30.0)),
        egui::Event::PointerButton {
            pos: pos2(30.0, 30.0),
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Default::default(),
        },
        egui::Event::PointerButton {
            pos: pos2(30.0, 30.0),
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        },
    ]
}
fn view(
    p: &ScrollPosition,
    b: &GestureBinding,
    axis: Axis,
    reverse: bool,
    enabled: bool,
    child: Widget,
) -> Widget {
    SingleChildScrollView::new(p, b)
        .unwrap()
        .axis(axis)
        .reverse(reverse)
        .keyboard_scrolling(enabled)
        .into_widget(child)
        .unwrap()
}
fn widget(
    p: &ScrollPosition,
    b: &GestureBinding,
    axis: Axis,
    reverse: bool,
    enabled: bool,
) -> Widget {
    view(
        p,
        b,
        axis,
        reverse,
        enabled,
        Widget::leaf(vec2(500.0, 500.0), |_| {}),
    )
}
fn setup(
    axis: Axis,
    reverse: bool,
) -> (egui::Context, ScrollPosition, GestureBinding, ElementTree) {
    let ctx = egui::Context::default();
    let p = ScrollPosition::new(&ctx, 0.0).unwrap();
    let b = GestureBinding::new();
    let tree = ElementTree::new(&ctx, widget(&p, &b, axis, reverse, true)).unwrap();
    (ctx, p, b, tree)
}
fn frame(
    ctx: &egui::Context,
    b: &GestureBinding,
    tree: &mut ElementTree,
    events: Vec<egui::Event>,
    time: f64,
    repeat: bool,
    enabled: bool,
) {
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(600.0, 400.0),
            )),
            time: Some(time),
            events,
            ..Default::default()
        },
        |ctx| {
            let mut f = b.begin_egui_frame(ctx).unwrap();
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut child =
                    ui.new_child(egui::UiBuilder::new().id_salt(time.to_bits()).max_rect(
                        egui::Rect::from_min_size(pos2(10.0, 10.0), vec2(80.0, 100.0)),
                    ));
                if !enabled {
                    child.disable();
                }
                tree.show_in_gesture_frame(
                    &mut child,
                    BoxConstraints::tight(vec2(80.0, 100.0)).unwrap(),
                    &mut f,
                )
                .unwrap();
            });
            assert!(f.finish().unwrap().failures.is_empty());
            if repeat && ctx.current_pass_index() == 0 {
                ctx.request_discard("keyboard replay test");
            }
        },
    );
}
fn focus(ctx: &egui::Context, b: &GestureBinding, t: &mut ElementTree) {
    let base = ctx.input(|i| i.time);
    frame(ctx, b, t, vec![], base + 0.01, false, true);
    frame(ctx, b, t, click(), base + 0.1, false, true);
    assert!(ctx.memory(|m| m.focused()).is_some());
    frame(ctx, b, t, vec![], base + 0.2, false, true);
}
#[test]
fn blank_tap_focuses_and_line_page_home_end_move_with_stable_focus() {
    let (ctx, p, b, mut t) = setup(Axis::Vertical, false);
    focus(&ctx, &b, &mut t);
    let id = ctx.memory(|m| m.focused()).unwrap();
    for (i, k, expected) in [
        (1, Key::ArrowDown, 50.0),
        (2, Key::PageDown, 130.0),
        (3, Key::End, 400.0),
        (4, Key::Home, 0.0),
    ] {
        frame(
            &ctx,
            &b,
            &mut t,
            vec![key(k, Modifiers::NONE)],
            i as f64,
            false,
            true,
        );
        assert_eq!(p.pixels(), expected);
        assert_eq!(ctx.memory(|m| m.focused()), Some(id));
    }
}
#[test]
fn duplicate_events_keep_order_and_repeat_pass_does_not_replay() {
    let (ctx, p, b, mut t) = setup(Axis::Vertical, false);
    focus(&ctx, &b, &mut t);
    frame(
        &ctx,
        &b,
        &mut t,
        vec![
            key(Key::ArrowDown, Modifiers::NONE),
            key(Key::ArrowDown, Modifiers::NONE),
        ],
        1.0,
        true,
        true,
    );
    assert_eq!(p.pixels(), 100.0);
    frame(
        &ctx,
        &b,
        &mut t,
        vec![
            key(Key::Home, Modifiers::NONE),
            key(Key::ArrowDown, Modifiers::NONE),
            key(Key::Home, Modifiers::NONE),
        ],
        2.0,
        false,
        true,
    );
    assert_eq!(p.pixels(), 0.0);
}
#[test]
fn space_shift_space_and_reverse_horizontal_direction_work() {
    let (ctx, p, b, mut t) = setup(Axis::Horizontal, true);
    focus(&ctx, &b, &mut t);
    frame(
        &ctx,
        &b,
        &mut t,
        vec![key(Key::ArrowLeft, Modifiers::NONE)],
        1.0,
        false,
        true,
    );
    assert_eq!(p.pixels(), 50.0);
    frame(
        &ctx,
        &b,
        &mut t,
        vec![key(Key::Space, Modifiers::SHIFT)],
        2.0,
        false,
        true,
    );
    assert_eq!(p.pixels(), 114.0);
    frame(
        &ctx,
        &b,
        &mut t,
        vec![key(Key::Space, Modifiers::NONE)],
        3.0,
        false,
        true,
    );
    assert_eq!(p.pixels(), 50.0);
    frame(
        &ctx,
        &b,
        &mut t,
        vec![key(Key::End, Modifiers::NONE)],
        4.0,
        false,
        true,
    );
    assert_eq!(p.pixels(), 420.0);
}
#[test]
fn unfocused_cross_axis_modified_and_released_keys_do_not_scroll() {
    let (ctx, p, b, mut t) = setup(Axis::Vertical, false);
    frame(
        &ctx,
        &b,
        &mut t,
        vec![key(Key::End, Modifiers::NONE)],
        0.0,
        false,
        true,
    );
    assert_eq!(p.pixels(), 0.0);
    focus(&ctx, &b, &mut t);
    let release = egui::Event::Key {
        key: Key::End,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    frame(
        &ctx,
        &b,
        &mut t,
        vec![
            key(Key::ArrowRight, Modifiers::NONE),
            key(Key::End, Modifiers::CTRL),
            release,
        ],
        1.0,
        false,
        true,
    );
    assert_eq!(p.pixels(), 0.0);
    assert!(ctx.input(|i| {
        i.events
            .iter()
            .any(|e| matches!(e,egui::Event::Key{key:Key::End,modifiers,..}if modifiers.ctrl))
    }));
}
#[test]
fn keyboard_can_be_disabled_without_losing_pointer_scroll() {
    let (ctx, p, b, mut t) = setup(Axis::Vertical, false);
    focus(&ctx, &b, &mut t);
    t.update(widget(&p, &b, Axis::Vertical, false, false))
        .unwrap();
    frame(
        &ctx,
        &b,
        &mut t,
        vec![key(Key::End, Modifiers::NONE)],
        1.0,
        false,
        true,
    );
    assert_eq!(p.pixels(), 0.0);
    assert!(ctx.memory(|m| m.focused()).is_none());
    frame(
        &ctx,
        &b,
        &mut t,
        vec![
            egui::Event::PointerMoved(pos2(30.0, 30.0)),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0.0, -30.0),
                modifiers: Modifiers::NONE,
            },
        ],
        2.0,
        false,
        true,
    );
    assert_eq!(p.pixels(), 30.0);
}
#[test]
fn disabled_and_hidden_viewports_do_not_process_keys() {
    let (ctx, p, b, mut t) = setup(Axis::Vertical, false);
    focus(&ctx, &b, &mut t);
    frame(
        &ctx,
        &b,
        &mut t,
        vec![key(Key::End, Modifiers::NONE)],
        1.0,
        false,
        false,
    );
    assert_eq!(p.pixels(), 0.0);
    focus(&ctx, &b, &mut t);
    let _ = ctx.run(
        egui::RawInput {
            time: Some(2.0),
            events: vec![key(Key::End, Modifiers::NONE)],
            ..Default::default()
        },
        |ctx| {
            b.begin_egui_frame(ctx).unwrap().finish().unwrap();
        },
    );
    assert_eq!(p.pixels(), 0.0);
}
#[test]
fn descendant_text_edit_focus_is_not_stolen_by_parent_tap() {
    let (ctx, p, b, mut t) = setup(Axis::Vertical, false);
    let text = Rc::new(RefCell::new(String::from("hello")));
    let id = Rc::new(Cell::new(None));
    let log = id.clone();
    t.update(view(
        &p,
        &b,
        Axis::Vertical,
        false,
        true,
        Widget::leaf(vec2(80.0, 500.0), move |ui| {
            let response = ui.put(
                egui::Rect::from_min_size(ui.max_rect().min, vec2(80.0, 30.0)),
                egui::TextEdit::singleline(&mut *text.borrow_mut())
                    .id(egui::Id::new("keyboard-child-edit")),
            );
            log.set(Some(response.id));
        }),
    ))
    .unwrap();
    frame(&ctx, &b, &mut t, vec![], 0.0, false, true);
    frame(&ctx, &b, &mut t, click(), 0.1, false, true);
    assert_eq!(ctx.memory(|m| m.focused()), id.get());
    frame(
        &ctx,
        &b,
        &mut t,
        vec![key(Key::End, Modifiers::NONE)],
        1.0,
        false,
        true,
    );
    assert_eq!(p.pixels(), 0.0);
    assert_eq!(ctx.memory(|m| m.focused()), id.get());
}
#[test]
fn descendant_arena_button_keeps_its_keyboard_action() {
    let (ctx, p, b, mut t) = setup(Axis::Vertical, false);
    let count = Rc::new(Cell::new(0));
    let log = count.clone();
    let button = GestureButton::new(&b, "child", move || log.set(log.get() + 1))
        .size(vec2(80.0, 30.0))
        .into_widget();
    t.update(view(
        &p,
        &b,
        Axis::Vertical,
        false,
        true,
        Widget::constrained(BoxConstraints::tight(vec2(80.0, 500.0)).unwrap(), button),
    ))
    .unwrap();
    frame(&ctx, &b, &mut t, vec![], 0.0, false, true);
    frame(&ctx, &b, &mut t, click(), 0.1, false, true);
    assert_eq!(count.get(), 1);
    frame(
        &ctx,
        &b,
        &mut t,
        vec![key(Key::Space, Modifiers::NONE)],
        1.0,
        false,
        true,
    );
    assert_eq!(count.get(), 2);
    assert_eq!(p.pixels(), 0.0);
}
#[test]
fn keyboard_interrupts_motion_and_retains_element_identity() {
    let (ctx, p, b, mut t) = setup(Axis::Vertical, false);
    focus(&ctx, &b, &mut t);
    let ids = t.elements().iter().map(|e| e.id).collect::<Vec<_>>();
    let run = p
        .animate_to(300.0, Duration::from_secs(10), Curve::Linear)
        .unwrap();
    frame(
        &ctx,
        &b,
        &mut t,
        vec![key(Key::Home, Modifiers::NONE)],
        1.0,
        false,
        true,
    );
    assert_eq!(run.outcome(), Some(AnimationOutcome::Canceled));
    assert_eq!(p.pixels(), 0.0);
    assert_eq!(t.elements().iter().map(|e| e.id).collect::<Vec<_>>(), ids);
}
#[test]
fn position_listener_can_query_layout_during_keyboard_dispatch() {
    let (ctx, p, b, mut t) = setup(Axis::Vertical, false);
    focus(&ctx, &b, &mut t);
    let tree = Rc::new(RefCell::new(t));
    let weak = Rc::downgrade(&tree);
    let success = Rc::new(Cell::new(false));
    let got = success.clone();
    let _s = p
        .subscribe(Rc::new(move || {
            got.set(
                weak.upgrade()
                    .unwrap()
                    .borrow_mut()
                    .dry_layout(BoxConstraints::tight(vec2(80.0, 100.0)).unwrap())
                    .is_ok(),
            );
        }))
        .unwrap();
    let _ = ctx.run(
        egui::RawInput {
            time: Some(1.0),
            events: vec![key(Key::ArrowDown, Modifiers::NONE)],
            ..Default::default()
        },
        |ctx| {
            let mut f = b.begin_egui_frame(ctx).unwrap();
            egui::CentralPanel::default().show(ctx, |ui| {
                tree.borrow_mut()
                    .show_in_gesture_frame(
                        ui,
                        BoxConstraints::tight(vec2(80.0, 100.0)).unwrap(),
                        &mut f,
                    )
                    .unwrap();
            });
            f.finish().unwrap();
        },
    );
    assert_eq!(p.pixels(), 50.0);
    assert!(success.get());
}

#[test]
fn tab_enters_and_leaves_the_scroll_focus_target() {
    let (ctx, p, b, mut t) = setup(Axis::Vertical, false);
    let button = Cell::new(None);
    let mut viewport_focus = None;
    for (time, events) in [
        (0.0, vec![]),
        (1.0, vec![key(Key::Tab, Modifiers::NONE)]),
        (2.0, vec![key(Key::End, Modifiers::NONE)]),
        (3.0, vec![key(Key::Tab, Modifiers::NONE)]),
        (4.0, vec![key(Key::Home, Modifiers::NONE)]),
    ] {
        let _ = ctx.run(
            egui::RawInput {
                time: Some(time),
                events,
                screen_rect: Some(egui::Rect::from_min_size(
                    pos2(0.0, 0.0),
                    vec2(600.0, 400.0),
                )),
                ..Default::default()
            },
            |ctx| {
                let mut f = b.begin_egui_frame(ctx).unwrap();
                egui::CentralPanel::default().show(ctx, |ui| {
                    t.show_in_gesture_frame(
                        ui,
                        BoxConstraints::tight(vec2(80.0, 100.0)).unwrap(),
                        &mut f,
                    )
                    .unwrap();
                    button.set(Some(ui.button("next").id));
                });
                f.finish().unwrap();
            },
        );
        if time == 1.0 {
            viewport_focus = ctx.memory(|m| m.focused());
            assert!(viewport_focus.is_some() && viewport_focus != button.get());
        }
        if time >= 2.0 {
            assert_eq!(p.pixels(), 400.0);
        }
        if time >= 3.0 {
            assert_eq!(ctx.memory(|m| m.focused()), button.get());
            assert_ne!(button.get(), viewport_focus);
        }
    }
}

#[test]
fn focus_loss_during_a_key_stops_remaining_commands_without_consuming_them() {
    let (ctx, p, b, mut t) = setup(Axis::Vertical, false);
    focus(&ctx, &b, &mut t);
    let id = ctx.memory(|m| m.focused()).unwrap();
    let context = ctx.clone();
    let _s = p
        .subscribe(Rc::new(move || {
            context.memory_mut(|m| m.surrender_focus(id))
        }))
        .unwrap();
    frame(
        &ctx,
        &b,
        &mut t,
        vec![
            key(Key::ArrowDown, Modifiers::NONE),
            key(Key::End, Modifiers::NONE),
        ],
        1.0,
        false,
        true,
    );
    assert_eq!(p.pixels(), 50.0);
    assert!(ctx.input(|i| {
        i.events
            .iter()
            .any(|e| matches!(e, egui::Event::Key { key: Key::End, .. }))
    }));
}

#[test]
fn resize_publishes_dimensions_before_boundary_key_action() {
    let (ctx, p, b, mut t) = setup(Axis::Vertical, false);
    focus(&ctx, &b, &mut t);
    let _ = ctx.run(
        egui::RawInput {
            time: Some(1.0),
            events: vec![key(Key::End, Modifiers::NONE)],
            ..Default::default()
        },
        |ctx| {
            let mut f = b.begin_egui_frame(ctx).unwrap();
            egui::CentralPanel::default().show(ctx, |ui| {
                t.show_in_gesture_frame(
                    ui,
                    BoxConstraints::tight(vec2(80.0, 200.0)).unwrap(),
                    &mut f,
                )
                .unwrap();
            });
            f.finish().unwrap();
        },
    );
    assert_eq!(p.dimensions().unwrap().max_scroll_extent(), 300.0);
    assert_eq!(p.pixels(), 300.0);
}

#[test]
fn unmount_after_paint_suppresses_deferred_keyboard_work() {
    let (ctx, p, b, mut t) = setup(Axis::Vertical, false);
    focus(&ctx, &b, &mut t);
    ctx.begin_pass(egui::RawInput {
        time: Some(1.0),
        events: vec![key(Key::End, Modifiers::NONE)],
        ..Default::default()
    });
    let mut f = b.begin_egui_frame(&ctx).unwrap();
    egui::CentralPanel::default().show(&ctx, |ui| {
        t.show_in_gesture_frame(
            ui,
            BoxConstraints::tight(vec2(80.0, 100.0)).unwrap(),
            &mut f,
        )
        .unwrap();
    });
    f.finish().unwrap();
    t.update(Widget::empty()).unwrap();
    let _ = ctx.end_pass();
    assert_eq!(p.pixels(), 0.0);
}

#[test]
fn rejected_duplicate_viewport_does_not_disable_the_live_focus_node() {
    let ctx = egui::Context::default();
    let p = ScrollPosition::new(&ctx, 0.0).unwrap();
    let b = GestureBinding::new();
    let view = SingleChildScrollView::new(&p, &b).unwrap();
    let mut first = ElementTree::new(
        &ctx,
        view.clone()
            .into_widget(Widget::leaf(vec2(80.0, 500.0), |_| {}))
            .unwrap(),
    )
    .unwrap();
    focus(&ctx, &b, &mut first);
    let id = ctx.memory(|m| m.focused());
    let mut second = ElementTree::new(
        &ctx,
        view.keyboard_scrolling(false)
            .into_widget(Widget::leaf(vec2(80.0, 500.0), |_| {}))
            .unwrap(),
    )
    .unwrap();
    assert!(
        second
            .layout(BoxConstraints::tight(vec2(80.0, 100.0)).unwrap())
            .is_err()
    );
    assert_eq!(ctx.memory(|m| m.focused()), id);
    frame(
        &ctx,
        &b,
        &mut first,
        vec![key(Key::End, Modifiers::NONE)],
        1.0,
        false,
        true,
    );
    assert_eq!(p.pixels(), 400.0);
}

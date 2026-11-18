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
};
fn loose() -> BoxConstraints {
    BoxConstraints::loose(vec2(100.0, 100.0)).unwrap()
}
fn frame(
    ctx: &egui::Context,
    b: &GestureBinding,
    t: &mut ElementTree,
    events: Vec<egui::Event>,
    legacy: bool,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(400.0, 300.0),
            )),
            time: Some(ctx.input(|i| i.time) + 0.05),
            events,
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut ui =
                    ui.new_child(egui::UiBuilder::new().id_salt("offstage-test").max_rect(
                        egui::Rect::from_min_size(pos2(10.0, 10.0), vec2(100.0, 100.0)),
                    ));
                if legacy {
                    t.show_with_gestures(&mut ui, loose(), b).unwrap();
                } else {
                    let mut f = b.begin_egui_frame(ctx).unwrap();
                    t.show_in_gesture_frame(&mut ui, loose(), &mut f).unwrap();
                    assert!(f.finish().unwrap().failures.is_empty());
                }
            });
        },
    )
}
fn button(pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos: pos2(20.0, 20.0),
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    }
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
fn hidden_child_is_measured_but_reports_minimum_size_no_overflow_or_hits() {
    let ctx = egui::Context::default();
    let mut t = ElementTree::new(
        &ctx,
        Offstage::default().into_widget(Widget::baseline_leaf(
            vec2(80.0, 30.0),
            Baselines {
                alphabetic: Some(12.0),
                ideographic: Some(14.0),
            },
            |_| {},
        )),
    )
    .unwrap();
    let g = t.layout(loose()).unwrap();
    assert!(g.is_offstage());
    assert_eq!(g.size, egui::Vec2::ZERO);
    assert_eq!(g.children[0].layout.size, vec2(80.0, 30.0));
    assert_eq!(g.overflow, Insets::default());
    assert_eq!(g.paint_bounds.size(), egui::Vec2::ZERO);
    assert_eq!(g.baselines, Baselines::NONE);
    assert!(g.hit_test_path(pos2(5.0, 5.0)).is_empty());
    let g = t
        .layout(BoxConstraints::tight(vec2(100.0, 50.0)).unwrap())
        .unwrap();
    assert_eq!(g.size, vec2(100.0, 50.0));
    assert!(g.hit_test_path(pos2(5.0, 5.0)).is_empty());
}
#[test]
fn hidden_dry_queries_do_not_measure_child_and_visible_baselines_forward() {
    let ctx = egui::Context::default();
    let calls = Rc::new(Cell::new(0));
    let counted = calls.clone();
    let mut t = ElementTree::new(
        &ctx,
        Widget::offstage(
            true,
            Widget::measured(
                move |c| {
                    counted.set(counted.get() + 1);
                    c.constrain(vec2(80.0, 30.0))
                },
                |_| {},
            ),
        ),
    )
    .unwrap();
    assert_eq!(t.dry_layout(loose()).unwrap(), egui::Vec2::ZERO);
    assert_eq!(
        t.intrinsic(IntrinsicDimension::MaxWidth, 100.0).unwrap(),
        0.0
    );
    assert_eq!(
        t.dry_baseline(loose(), TextBaseline::Alphabetic).unwrap(),
        None
    );
    assert_eq!(calls.get(), 0);
    t.layout(loose()).unwrap();
    assert_eq!(calls.get(), 1);
    t.update(Widget::offstage(
        false,
        Widget::baseline_leaf(
            vec2(80.0, 30.0),
            Baselines {
                alphabetic: Some(12.0),
                ideographic: Some(14.0),
            },
            |_| {},
        ),
    ))
    .unwrap();
    assert_eq!(t.dry_layout(loose()).unwrap(), vec2(80.0, 30.0));
    assert_eq!(
        t.dry_baseline(loose(), TextBaseline::Alphabetic).unwrap(),
        Some(12.0)
    );
    let g = t.layout(loose()).unwrap();
    assert_eq!(g.baselines.alphabetic, Some(12.0));
    assert!(!g.is_offstage());
}
#[derive(Default)]
struct Log {
    id: Option<ElementId>,
    init: usize,
    dispose: usize,
}
struct Counter(Rc<RefCell<Log>>);
struct CounterState {
    log: Rc<RefCell<Log>>,
    value: usize,
}
impl StatefulWidget for Counter {
    type State = CounterState;
    fn create_state(&self) -> CounterState {
        CounterState {
            log: self.0.clone(),
            value: 0,
        }
    }
}
impl State<Counter> for CounterState {
    fn init_state(&mut self, _: &Counter, cx: &StateContext<Self>) {
        let mut log = self.log.borrow_mut();
        log.init += 1;
        log.id = Some(cx.element_id());
    }
    fn build(&mut self, _: &Counter, _: &StateContext<Self>) -> Widget {
        Widget::leaf(vec2(80.0, 30.0), |_| {})
    }
    fn dispose(&mut self) {
        self.log.borrow_mut().dispose += 1;
    }
}
#[test]
fn toggling_keeps_keyed_element_state_and_hidden_updates_alive() {
    let ctx = egui::Context::default();
    let log = Rc::new(RefCell::new(Log::default()));
    let child = || Widget::stateful(Counter(log.clone())).keyed("counter");
    let mut t = ElementTree::new(&ctx, Widget::offstage(false, child())).unwrap();
    let id = log.borrow().id.unwrap();
    let handle = t.state_handle::<CounterState>(id).unwrap();
    t.update(Widget::offstage(true, child())).unwrap();
    handle.update(|s| s.value = 7).unwrap();
    t.layout(loose()).unwrap();
    assert_eq!(t.state::<CounterState>(id).unwrap().value, 7);
    assert_eq!(log.borrow().init, 1);
    assert_eq!(log.borrow().dispose, 0);
    t.update(Widget::offstage(false, child())).unwrap();
    assert_eq!(t.state::<CounterState>(id).unwrap().value, 7);
    drop(t);
    assert_eq!(log.borrow().dispose, 1);
    assert!(!handle.mounted());
}
#[test]
fn hidden_backend_callbacks_run_with_empty_clip_and_never_emit_visible_red() {
    let ctx = egui::Context::default();
    let b = GestureBinding::new();
    let calls = Rc::new(Cell::new(0));
    let paint = calls.clone();
    let child = Widget::leaf(vec2(80.0, 30.0), move |ui| {
        paint.set(paint.get() + 1);
        assert!(!ui.clip_rect().is_positive());
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, egui::Color32::RED);
    });
    let mut t = ElementTree::new(&ctx, Widget::offstage(true, child)).unwrap();
    let output = frame(&ctx, &b, &mut t, vec![], false);
    assert_eq!(calls.get(), 1);
    for shape in output.shapes {
        if let egui::epaint::Shape::Rect(rect) = shape.shape
            && rect.fill == egui::Color32::RED
        {
            assert!(!shape.clip_rect.is_positive());
        }
    }
}
fn tap_widget(
    b: &GestureBinding,
    hidden: bool,
    taps: Rc<Cell<usize>>,
    cancels: Rc<Cell<usize>>,
) -> Widget {
    let detector = GestureDetector::new(
        b,
        GestureTargetConfig {
            tap: Some((
                TapConfig::default(),
                TapCallbacks {
                    on_tap: Some(Rc::new(move |_| taps.set(taps.get() + 1))),
                    on_cancel: Some(Rc::new(move |_| cancels.set(cancels.get() + 1))),
                    ..Default::default()
                },
            )),
            ..Default::default()
        },
    )
    .unwrap()
    .behavior(HitTestBehavior::Opaque)
    .into_widget(Widget::leaf(vec2(80.0, 30.0), |_| {}));
    Widget::offstage(hidden, detector)
}
#[test]
fn hiding_cancels_captured_tap_in_shared_and_legacy_adapters() {
    for legacy in [false, true] {
        let ctx = egui::Context::default();
        let b = GestureBinding::new();
        let taps = Rc::new(Cell::new(0));
        let cancels = Rc::new(Cell::new(0));
        let mut t =
            ElementTree::new(&ctx, tap_widget(&b, false, taps.clone(), cancels.clone())).unwrap();
        frame(
            &ctx,
            &b,
            &mut t,
            vec![egui::Event::PointerMoved(pos2(20.0, 20.0)), button(true)],
            legacy,
        );
        t.update(tap_widget(&b, true, taps.clone(), cancels.clone()))
            .unwrap();
        frame(&ctx, &b, &mut t, vec![button(false)], legacy);
        assert_eq!(taps.get(), 0);
        assert_eq!(cancels.get(), 1);
        t.update(tap_widget(&b, false, taps.clone(), cancels.clone()))
            .unwrap();
        frame(&ctx, &b, &mut t, vec![button(true), button(false)], legacy);
        assert_eq!(taps.get(), 1);
    }
}
fn editor(hidden: bool, text: Rc<RefCell<String>>, id: egui::Id) -> Widget {
    Widget::offstage(
        hidden,
        Widget::leaf(vec2(80.0, 30.0), move |ui| {
            ui.put(
                ui.max_rect(),
                egui::TextEdit::singleline(&mut *text.borrow_mut()).id(id),
            );
        }),
    )
}
#[test]
fn focused_text_editor_keeps_keyboard_input_while_offstage() {
    let ctx = egui::Context::default();
    let b = GestureBinding::new();
    let text = Rc::new(RefCell::new(String::new()));
    let id = egui::Id::new("offstage-editor");
    let mut t = ElementTree::new(&ctx, editor(false, text.clone(), id)).unwrap();
    frame(&ctx, &b, &mut t, vec![], false);
    ctx.memory_mut(|m| m.request_focus(id));
    t.update(editor(true, text.clone(), id)).unwrap();
    frame(
        &ctx,
        &b,
        &mut t,
        vec![egui::Event::Text("typed".into())],
        false,
    );
    assert_eq!(&*text.borrow(), "typed");
    assert!(ctx.memory(|m| m.has_focus(id)));
}
fn arena_button(b: &GestureBinding, hidden: bool, count: Rc<Cell<usize>>) -> Widget {
    Widget::offstage(
        hidden,
        GestureButton::new(b, "Button", move || count.set(count.get() + 1))
            .size(vec2(80.0, 30.0))
            .into_widget(),
    )
}
#[test]
fn focused_gesture_button_keeps_keyboard_activation_but_not_pointer_activation() {
    let ctx = egui::Context::default();
    let b = GestureBinding::new();
    let count = Rc::new(Cell::new(0));
    let mut t = ElementTree::new(&ctx, arena_button(&b, false, count.clone())).unwrap();
    frame(
        &ctx,
        &b,
        &mut t,
        vec![
            egui::Event::PointerMoved(pos2(20.0, 20.0)),
            button(true),
            button(false),
        ],
        false,
    );
    assert_eq!(count.get(), 1);
    let focus = ctx.memory(|m| m.focused()).unwrap();
    t.update(arena_button(&b, true, count.clone())).unwrap();
    frame(&ctx, &b, &mut t, vec![key(egui::Key::Space)], false);
    assert_eq!(count.get(), 2);
    assert!(ctx.memory(|m| m.has_focus(focus)));
    frame(&ctx, &b, &mut t, vec![button(true), button(false)], false);
    assert_eq!(count.get(), 2);
}
fn scroll_widget(p: &ScrollPosition, b: &GestureBinding, hidden: bool) -> Widget {
    Widget::offstage(
        hidden,
        ListView::builder(p, b, 100, 20.0, |_, _| Widget::empty())
            .unwrap()
            .cache_extent(0.0)
            .unwrap()
            .into_widget()
            .unwrap(),
    )
}
#[test]
fn focused_hidden_scroll_view_keeps_keyboard_but_declines_wheel() {
    let ctx = egui::Context::default();
    let b = GestureBinding::new();
    let p = ScrollPosition::new(&ctx, 0.0).unwrap();
    let mut t = ElementTree::new(&ctx, scroll_widget(&p, &b, false)).unwrap();
    frame(&ctx, &b, &mut t, vec![], false);
    frame(
        &ctx,
        &b,
        &mut t,
        vec![
            egui::Event::PointerMoved(pos2(20.0, 20.0)),
            button(true),
            button(false),
        ],
        false,
    );
    t.update(scroll_widget(&p, &b, true)).unwrap();
    frame(&ctx, &b, &mut t, vec![key(egui::Key::End)], false);
    assert_eq!(p.pixels(), 1900.0);
    frame(
        &ctx,
        &b,
        &mut t,
        vec![
            egui::Event::PointerMoved(pos2(20.0, 20.0)),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0.0, 20.0),
                modifiers: Default::default(),
            },
        ],
        false,
    );
    assert_eq!(p.pixels(), 1900.0);
}

#[test]
fn nested_hidden_child_cancels_legacy_capture_without_hiding_the_root() {
    let ctx = egui::Context::default();
    let b = GestureBinding::new();
    let taps = Rc::new(Cell::new(0));
    let cancels = Rc::new(Cell::new(0));
    let make = |hidden| {
        Widget::padding(
            Insets::all(0.0),
            tap_widget(&b, hidden, taps.clone(), cancels.clone()),
        )
    };
    let mut t = ElementTree::new(&ctx, make(false)).unwrap();
    frame(
        &ctx,
        &b,
        &mut t,
        vec![egui::Event::PointerMoved(pos2(20.0, 20.0)), button(true)],
        true,
    );
    t.update(make(true)).unwrap();
    assert!(!t.layout(loose()).unwrap().is_offstage());
    frame(&ctx, &b, &mut t, vec![button(false)], true);
    assert_eq!(taps.get(), 0);
    assert_eq!(cancels.get(), 1);
}
#[test]
fn hidden_top_root_does_not_occlude_an_interactive_root_beneath_it() {
    let ctx = egui::Context::default();
    let b = GestureBinding::new();
    let under = Rc::new(Cell::new(0));
    let over = Rc::new(Cell::new(0));
    let mut bottom = ElementTree::new(
        &ctx,
        tap_widget(&b, false, under.clone(), Rc::new(Cell::new(0))),
    )
    .unwrap();
    let mut top = ElementTree::new(
        &ctx,
        tap_widget(&b, true, over.clone(), Rc::new(Cell::new(0))),
    )
    .unwrap();
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                vec2(400.0, 300.0),
            )),
            events: vec![
                egui::Event::PointerMoved(pos2(20.0, 20.0)),
                button(true),
                button(false),
            ],
            ..Default::default()
        },
        |ctx| {
            let mut f = b.begin_egui_frame(ctx).unwrap();
            egui::CentralPanel::default().show(ctx, |ui| {
                for (index, t) in [&mut bottom, &mut top].into_iter().enumerate() {
                    let mut ui = ui.new_child(egui::UiBuilder::new().id_salt(index).max_rect(
                        egui::Rect::from_min_size(pos2(10.0, 10.0), vec2(80.0, 30.0)),
                    ));
                    t.show_in_gesture_frame(
                        &mut ui,
                        BoxConstraints::tight(vec2(80.0, 30.0)).unwrap(),
                        &mut f,
                    )
                    .unwrap();
                }
            });
            assert!(f.finish().unwrap().failures.is_empty());
        },
    );
    assert_eq!(under.get(), 1);
    assert_eq!(over.get(), 0);
}
#[derive(PartialEq)]
struct Theme(f32);
struct Consumer(Rc<RefCell<Vec<f32>>>);
impl StatelessWidget for Consumer {
    fn build(&self, cx: &BuildContext) -> Widget {
        let height = cx.depend_on::<Theme>().unwrap().unwrap().0;
        self.0.borrow_mut().push(height);
        Widget::leaf(vec2(40.0, height), |_| {})
    }
}
#[test]
fn inherited_updates_and_layout_builders_remain_active_while_hidden() {
    let ctx = egui::Context::default();
    let seen = Rc::new(RefCell::new(vec![]));
    let seen2 = seen.clone();
    let mut t = ElementTree::new(
        &ctx,
        Widget::inherited(
            Theme(10.0),
            Widget::offstage(
                true,
                Widget::layout_builder(move |_, c| {
                    assert_eq!(c, loose());
                    Widget::stateless(Consumer(seen2.clone()))
                }),
            ),
        ),
    )
    .unwrap();
    let g = t.layout(loose()).unwrap();
    assert_eq!(g.size, egui::Vec2::ZERO);
    assert_eq!(*seen.borrow(), vec![10.0]);
    let handle = t.inherited_handle::<Theme>(t.root_id().unwrap()).unwrap();
    handle.update(|_| Theme(35.0)).unwrap();
    let g = t.layout(loose()).unwrap();
    assert_eq!(*seen.borrow(), vec![10.0, 35.0]);
    assert_eq!(g.children[0].layout.size, vec2(40.0, 35.0));
}
#[cfg(not(target_os = "android"))]
#[test]
fn hidden_accesskit_container_suppresses_click_but_showing_restores_it() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let b = GestureBinding::new();
    let count = Rc::new(Cell::new(0));
    let mut t = ElementTree::new(&ctx, arena_button(&b, false, count.clone())).unwrap();
    let output = frame(&ctx, &b, &mut t, vec![], false);
    let update = output.platform_output.accesskit_update.unwrap();
    let id = update
        .nodes
        .iter()
        .find(|(_, n)| n.role() == egui::accesskit::Role::Button)
        .unwrap()
        .0;
    let event = || {
        egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Click,
            target: id,
            data: None,
        })
    };
    t.update(arena_button(&b, true, count.clone())).unwrap();
    let output = frame(&ctx, &b, &mut t, vec![event()], false);
    assert_eq!(count.get(), 0);
    let update = output.platform_output.accesskit_update.unwrap();
    let mut current = id;
    let mut hidden = false;
    for _ in 0..update.nodes.len() {
        if let Some((_, node)) = update.nodes.iter().find(|(node_id, _)| *node_id == current)
            && node.is_hidden()
        {
            hidden = true;
            break;
        }
        if let Some((parent, _)) = update
            .nodes
            .iter()
            .find(|(_, node)| node.children().contains(&current))
        {
            current = *parent;
        } else {
            break;
        }
    }
    assert!(
        hidden,
        "button must have an explicitly hidden accessibility ancestor"
    );
    t.update(arena_button(&b, false, count.clone())).unwrap();
    frame(&ctx, &b, &mut t, vec![event()], false);
    assert_eq!(count.get(), 1);
}
struct Animated(Rc<RefCell<Option<rust_flutter::animation::AnimationController>>>);
struct AnimatedState;
impl StatefulWidget for Animated {
    type State = AnimatedState;
    fn create_state(&self) -> Self::State {
        AnimatedState
    }
}
impl State<Animated> for AnimatedState {
    fn init_state(&mut self, w: &Animated, cx: &StateContext<Self>) {
        *w.0.borrow_mut() = Some(
            cx.animation_controller(rust_flutter::animation::AnimationConfig {
                duration: std::time::Duration::from_secs(1),
                ..Default::default()
            })
            .unwrap(),
        );
    }
    fn build(&mut self, _: &Animated, _: &StateContext<Self>) -> Widget {
        Widget::empty()
    }
}
#[test]
fn offstage_does_not_implicitly_mute_element_owned_animation() {
    let ctx = egui::Context::default();
    let b = GestureBinding::new();
    let saved = Rc::new(RefCell::new(None));
    let mut t = ElementTree::new(
        &ctx,
        Widget::offstage(true, Widget::stateful(Animated(saved.clone()))),
    )
    .unwrap();
    frame(&ctx, &b, &mut t, vec![], false);
    let controller = saved.borrow().clone().unwrap();
    controller.forward().unwrap();
    for _ in 0..5 {
        frame(&ctx, &b, &mut t, vec![], false);
    }
    assert!(!controller.is_muted());
    assert!(controller.value() > 0.0);
    drop(t);
    assert!(controller.is_disposed());
}

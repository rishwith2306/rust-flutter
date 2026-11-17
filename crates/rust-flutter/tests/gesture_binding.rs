use rust_flutter::{
    egui::{self, Id, Vec2, pos2, vec2},
    element::{ElementTree, Widget},
    gestures::*,
    layout::*,
};
use std::{cell::RefCell, rc::Rc, time::Duration};
type Log = Rc<RefCell<Vec<String>>>;
fn sample(pointer: u64, phase: PointerPhase, x: f32, time: u64) -> PointerEvent {
    PointerEvent {
        pointer: PointerId(pointer),
        phase,
        position: pos2(x, 5.0),
        time: Duration::from_millis(time),
        device: PointerDeviceKind::Touch,
        buttons: if matches!(phase, PointerPhase::Up | PointerPhase::Cancel) {
            PointerButtons::NONE
        } else {
            PointerButtons::PRIMARY
        },
    }
}
fn config(log: &Log, name: &str, pan: bool) -> GestureTargetConfig {
    let (a, b, c, d) = (log.clone(), log.clone(), log.clone(), log.clone());
    let (n, m, p, q) = (
        name.to_string(),
        name.to_string(),
        name.to_string(),
        name.to_string(),
    );
    GestureTargetConfig {
        tap: Some((
            TapConfig::default(),
            TapCallbacks {
                on_tap: Some(Rc::new(move |_| a.borrow_mut().push(format!("{n} tap")))),
                on_cancel: Some(Rc::new(move |_| b.borrow_mut().push(format!("{m} cancel")))),
                ..Default::default()
            },
        )),
        pan: pan.then_some((
            PanConfig::default(),
            PanCallbacks {
                on_start: Some(Rc::new(move |_| c.borrow_mut().push(format!("{p} pan")))),
                on_end: Some(Rc::new(move |_| d.borrow_mut().push(format!("{q} end")))),
                ..Default::default()
            },
        )),
    }
}
fn send(b: &GestureBinding, e: PointerEvent, path: Vec<Id>) -> DispatchReport {
    let report = b.dispatch(e, path);
    assert!(report.failures.is_empty(), "{:?}", report.failures);
    report
}
#[test]
fn down_registers_entire_path_before_resolution_and_deepest_tap_wins() {
    let b = GestureBinding::new();
    let log = Log::default();
    let child = Id::new("child");
    let parent = Id::new("parent");
    let _parent = b
        .register_target(parent, config(&log, "parent", false))
        .unwrap();
    let _child = b
        .register_target(child, config(&log, "child", false))
        .unwrap();
    send(
        &b,
        sample(1, PointerPhase::Down, 5.0, 0),
        vec![child, parent, child],
    );
    assert!(log.borrow().is_empty());
    assert_eq!(b.router().route_count(), 2);
    send(&b, sample(1, PointerPhase::Up, 300.0, 10), vec![]); // exceeds tap slop, so neither completes
    assert_eq!(log.borrow().len(), 2);
    assert_eq!(b.active_pointers(), 0);
    assert_eq!(b.router().route_count(), 0);
    log.borrow_mut().clear();
    send(
        &b,
        sample(1, PointerPhase::Down, 5.0, 20),
        vec![child, parent],
    );
    send(&b, sample(1, PointerPhase::Up, 5.0, 30), vec![]);
    assert_eq!(*log.borrow(), vec!["parent cancel", "child tap"]);
}
#[test]
fn capture_keeps_pan_outside_hit_region_and_releases_on_up() {
    let b = GestureBinding::new();
    let log = Log::default();
    let id = Id::new(1);
    let _target = b.register_target(id, config(&log, "marker", true)).unwrap();
    send(&b, sample(1, PointerPhase::Down, 5.0, 0), vec![id]);
    send(&b, sample(1, PointerPhase::Move, 200.0, 10), vec![]);
    send(&b, sample(1, PointerPhase::Up, 300.0, 20), vec![]);
    assert_eq!(
        *log.borrow(),
        vec!["marker cancel", "marker pan", "marker end"]
    );
    assert_eq!(b.active_pointers(), 0);
    assert_eq!(b.arena().active_pointers(), 0);
    assert_eq!(b.router().route_count(), 0);
}
#[test]
fn callback_update_preserves_active_tap_and_settings_update_cancels_it() {
    let b = GestureBinding::new();
    let log = Log::default();
    let id = Id::new(1);
    let target = b.register_target(id, config(&log, "old", false)).unwrap();
    send(&b, sample(1, PointerPhase::Down, 5.0, 0), vec![id]);
    target.update(config(&log, "new", false)).unwrap();
    send(&b, sample(1, PointerPhase::Up, 5.0, 10), vec![]);
    assert_eq!(*log.borrow(), vec!["new tap"]);
    send(&b, sample(1, PointerPhase::Down, 5.0, 20), vec![id]);
    let mut changed = config(&log, "changed", false);
    changed.tap.as_mut().unwrap().0.post_accept_slop = 20.0;
    target.update(changed).unwrap();
    assert_eq!(*log.borrow(), vec!["new tap", "new cancel"]);
    assert_eq!(b.router().route_count(), 0);
    send(&b, sample(1, PointerPhase::Up, 5.0, 30), vec![]);
    assert_eq!(log.borrow().len(), 2);
}
#[test]
fn removing_child_target_leaves_parent_candidate_and_reused_id_safe() {
    let b = GestureBinding::new();
    let log = Log::default();
    let a = Id::new(1);
    let p = Id::new(2);
    let child = b.register_target(a, config(&log, "child", false)).unwrap();
    let _parent = b.register_target(p, config(&log, "parent", false)).unwrap();
    send(&b, sample(1, PointerPhase::Down, 5.0, 0), vec![a, p]);
    assert!(child.cancel());
    let replacement = b
        .register_target(a, config(&log, "replacement", false))
        .unwrap();
    assert!(!child.cancel());
    assert!(replacement.is_active());
    send(&b, sample(1, PointerPhase::Up, 5.0, 10), vec![]);
    assert_eq!(*log.borrow(), vec!["child cancel", "parent tap"]);
}
#[test]
fn reentrant_same_pointer_down_is_delivered_after_old_up_cleanup() {
    let b = GestureBinding::new();
    let log = Log::default();
    let id = Id::new(1);
    let weak = b.downgrade();
    let l = log.clone();
    let mut c = config(&log, "marker", false);
    c.tap.as_mut().unwrap().1.on_tap = Some(Rc::new(move |d| {
        l.borrow_mut().push("tap".into());
        let b = weak.upgrade().unwrap();
        let next = PointerEvent {
            phase: PointerPhase::Down,
            time: Duration::from_millis(20),
            buttons: PointerButtons::PRIMARY,
            ..d.up
        };
        assert!(b.dispatch(next, [id]).queued);
    }));
    let _target = b.register_target(id, c).unwrap();
    send(&b, sample(1, PointerPhase::Down, 5.0, 0), vec![id]);
    let r = send(&b, sample(1, PointerPhase::Up, 5.0, 10), vec![]);
    assert_eq!(r.processed, 2);
    assert_eq!(b.active_pointers(), 1);
    assert_eq!(b.router().route_count(), 1);
    assert_eq!(b.arena().active_pointers(), 0); // sole tap won, still awaiting up
    b.cancel_all();
    assert_eq!(b.router().route_count(), 0);
    assert_eq!(b.active_pointers(), 0);
}
#[test]
fn unregistering_during_tap_down_does_not_leave_an_orphan_pan() {
    let b = GestureBinding::new();
    let log = Log::default();
    let id = Id::new(1);
    let token = Rc::new(RefCell::new(None::<GestureTarget>));
    let t = token.clone();
    let mut c = config(&log, "marker", true);
    c.tap.as_mut().unwrap().1.on_down = Some(Rc::new(move |_| {
        t.borrow_mut().take();
    }));
    *token.borrow_mut() = Some(b.register_target(id, c).unwrap());
    send(&b, sample(1, PointerPhase::Down, 5.0, 0), vec![id]);
    assert_eq!(b.router().route_count(), 0);
    assert_eq!(b.target_count(), 0);
    assert_eq!(*log.borrow(), vec!["marker cancel"]);
    send(&b, sample(1, PointerPhase::Up, 5.0, 10), vec![]);
}
#[test]
fn global_observers_receive_each_phase_once_including_cancel() {
    let b = GestureBinding::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    let l = log.clone();
    let _observer = b
        .router()
        .add_global_route(move |_, e| l.borrow_mut().push(e.phase))
        .unwrap();
    send(&b, sample(1, PointerPhase::Down, 5.0, 0), vec![]);
    send(&b, sample(1, PointerPhase::Move, 6.0, 1), vec![]);
    b.cancel_all();
    assert_eq!(
        *log.borrow(),
        vec![PointerPhase::Down, PointerPhase::Move, PointerPhase::Cancel]
    );
    assert_eq!(b.active_pointers(), 0);
}
#[test]
fn independent_pointers_invalid_protocol_and_terminal_disposal() {
    let b = GestureBinding::new();
    send(&b, sample(1, PointerPhase::Down, 5.0, 0), vec![]);
    send(&b, sample(2, PointerPhase::Down, 5.0, 0), vec![]);
    assert_eq!(
        b.dispatch(sample(1, PointerPhase::Down, 5.0, 1), [])
            .failures[0]
            .error,
        BindingError::DuplicateDown(PointerId(1))
    );
    send(&b, sample(1, PointerPhase::Move, 6.0, 20), vec![]);
    assert_eq!(
        b.dispatch(sample(1, PointerPhase::Move, 6.0, 10), [])
            .failures[0]
            .error,
        BindingError::OldTimestamp(PointerId(1))
    );
    assert_eq!(
        b.dispatch(sample(3, PointerPhase::Up, 5.0, 1), []).failures[0].error,
        BindingError::MissingDown(PointerId(3))
    );
    send(&b, sample(1, PointerPhase::Up, 5.0, 30), vec![]);
    assert_eq!(b.active_pointers(), 1);
    b.dispose();
    b.dispose();
    assert_eq!(b.active_pointers(), 0);
    assert_eq!(
        b.dispatch(sample(1, PointerPhase::Down, 5.0, 40), [])
            .failures[0]
            .error,
        BindingError::Disposed
    );
}
#[test]
fn disposal_during_down_prevents_later_candidates_and_success_callbacks() {
    let b = GestureBinding::new();
    let log = Log::default();
    let weak = b.downgrade();
    let mut c = config(&log, "a", true);
    c.tap.as_mut().unwrap().1.on_down = Some(Rc::new(move |_| weak.upgrade().unwrap().dispose()));
    let _a = b.register_target(Id::new(1), c).unwrap();
    let _other = b
        .register_target(Id::new(2), config(&log, "b", false))
        .unwrap();
    send(
        &b,
        sample(1, PointerPhase::Down, 5.0, 0),
        vec![Id::new(1), Id::new(2)],
    );
    assert!(b.is_disposed());
    assert_eq!(b.router().route_count(), 0);
    assert_eq!(*log.borrow(), vec!["a cancel"]);
}
#[test]
fn dropped_binding_cancels_and_weak_tokens_do_not_retain_it() {
    let b = GestureBinding::new();
    let weak = b.downgrade();
    let log = Log::default();
    let target = b
        .register_target(Id::new(1), config(&log, "a", false))
        .unwrap();
    send(&b, sample(1, PointerPhase::Down, 5.0, 0), vec![Id::new(1)]);
    drop(b);
    assert!(weak.upgrade().is_none());
    assert!(!target.is_active());
    assert_eq!(*log.borrow(), vec!["a cancel"]);
}
#[test]
fn external_service_disposal_terminates_binding_and_cancels_already_winning_pan() {
    for dispose_router in [true, false] {
        let b = GestureBinding::new();
        let log = Log::default();
        let mut c = config(&log, "marker", true);
        let l = log.clone();
        c.pan.as_mut().unwrap().1.on_cancel =
            Some(Rc::new(move |_| l.borrow_mut().push("pan cancel".into())));
        let target = b.register_target(Id::new(1), c).unwrap();
        send(&b, sample(1, PointerPhase::Down, 5.0, 0), vec![Id::new(1)]);
        send(&b, sample(1, PointerPhase::Move, 40.0, 10), vec![]);
        if dispose_router {
            b.router().dispose();
        } else {
            b.arena().dispose().unwrap();
        }
        assert_eq!(
            b.dispatch(sample(1, PointerPhase::Up, 40.0, 20), [])
                .failures[0]
                .error,
            BindingError::Disposed
        );
        assert_eq!(
            *log.borrow(),
            vec!["marker cancel", "marker pan", "pan cancel"]
        );
        assert!(b.is_disposed());
        assert!(!target.is_active());
        assert_eq!(b.active_pointers(), 0);
        assert_eq!(b.router().route_count(), 0);
    }
}

struct Surface {
    behavior: HitTestBehavior,
}
impl CustomBoxLayout for Surface {
    fn layout(
        &mut self,
        c: BoxConstraints,
        children: &mut BoxChildren<'_>,
    ) -> Result<BoxGeometry, LayoutError> {
        for i in 0..children.len() {
            children.layout(i, c)?;
            children.place(i, Vec2::ZERO)?;
        }
        let mut g = BoxGeometry::new(c.constrain(vec2(30.0, 30.0))?);
        g.hit_test_behavior = Some(self.behavior);
        Ok(g)
    }
}
fn surface(id: &str, behavior: HitTestBehavior, children: Vec<BoxNode>) -> BoxNode {
    BoxNode::new(
        id,
        BoxKind::Custom {
            layout: CustomBox::new(Surface { behavior }),
            children,
        },
    )
}
#[test]
fn hit_paths_preserve_offsets_and_deepest_to_ancestor_order() {
    let mut node = BoxNode::padding(
        "padding",
        Insets::all(5.0),
        BoxNode::constrained(
            "sized",
            BoxConstraints::tight(vec2(20.0, 10.0)).unwrap(),
            BoxNode::leaf("leaf", vec2(1.0, 1.0)),
        ),
    );
    let g = node
        .layout(BoxConstraints::loose(vec2(100.0, 100.0)).unwrap())
        .unwrap();
    let path = g.hit_test_path(pos2(8.0, 7.0));
    assert_eq!(
        path.iter().map(|e| e.id).collect::<Vec<_>>(),
        vec![Id::new("leaf"), Id::new("sized"), Id::new("padding")]
    );
    assert_eq!(path[0].local_position, pos2(3.0, 2.0));
    assert_eq!(path[0].offset, vec2(5.0, 5.0));
    assert_eq!(path[2].offset, Vec2::ZERO);
    assert!(g.hit_test_path(pos2(2.0, 2.0)).is_empty());
    assert!(g.hit_test_path(pos2(f32::NAN, 2.0)).is_empty());
    assert!(g.hit_test_path(pos2(g.size.x, 2.0)).is_empty());
}
#[test]
fn translucent_surface_allows_behind_while_opaque_descendant_blocks_it() {
    for child in [false, true] {
        let top = surface(
            "top",
            HitTestBehavior::Translucent,
            if child {
                vec![BoxNode::leaf("top-child", vec2(30.0, 30.0))]
            } else {
                vec![]
            },
        );
        let mut n = BoxNode::stack(
            "stack",
            Stack::new(vec![
                StackChild::new(BoxNode::leaf("behind", vec2(30.0, 30.0))),
                StackChild::new(top),
            ]),
        );
        let g = n
            .layout(BoxConstraints::tight(vec2(30.0, 30.0)).unwrap())
            .unwrap();
        let ids = g
            .hit_test_path(pos2(5.0, 5.0))
            .iter()
            .map(|e| e.id)
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            if child {
                vec![Id::new("top-child"), Id::new("top"), Id::new("stack")]
            } else {
                vec![Id::new("top"), Id::new("behind"), Id::new("stack")]
            }
        );
    }
}
#[test]
fn empty_deferred_surface_misses_and_opaque_surface_blocks_behind() {
    for behavior in [HitTestBehavior::DeferToChild, HitTestBehavior::Opaque] {
        let mut n = BoxNode::stack(
            "stack",
            Stack::new(vec![
                StackChild::new(BoxNode::leaf("behind", vec2(30.0, 30.0))),
                StackChild::new(surface("top", behavior, vec![])),
            ]),
        );
        let g = n
            .layout(BoxConstraints::tight(vec2(30.0, 30.0)).unwrap())
            .unwrap();
        assert_eq!(
            g.hit_test(pos2(5.0, 5.0)),
            Some(Id::new(if behavior == HitTestBehavior::Opaque {
                "top"
            } else {
                "behind"
            }))
        );
    }
}
#[test]
fn retained_detector_preserves_queries_callbacks_and_unmount_cancellation() {
    let ctx = egui::Context::default();
    let b = GestureBinding::new();
    let log = Log::default();
    let widget = |name: &str| {
        GestureDetector::new(&b, config(&log, name, false))
            .unwrap()
            .into_widget(Widget::baseline_leaf(
                vec2(30.0, 20.0),
                Baselines {
                    alphabetic: Some(12.0),
                    ideographic: None,
                },
                |_| {},
            ))
    };
    let mut tree = ElementTree::new(&ctx, widget("old")).unwrap();
    let c = BoxConstraints::loose(vec2(100.0, 100.0)).unwrap();
    let g = tree.layout(c).unwrap();
    assert_eq!(g.size, vec2(30.0, 20.0));
    assert_eq!(tree.dry_layout(c).unwrap(), g.size);
    assert_eq!(
        tree.dry_baseline(c, TextBaseline::Alphabetic).unwrap(),
        Some(12.0)
    );
    let e = sample(1, PointerPhase::Down, 105.0, 0);
    assert!(
        b.dispatch_layout(e, &g, pos2(100.0, 0.0))
            .failures
            .is_empty()
    );
    tree.update(widget("new")).unwrap();
    tree.layout(c).unwrap();
    send(
        &b,
        PointerEvent {
            phase: PointerPhase::Up,
            buttons: PointerButtons::NONE,
            time: Duration::from_millis(10),
            ..e
        },
        vec![],
    );
    assert_eq!(*log.borrow(), vec!["new tap"]);
    assert!(
        b.dispatch_layout(
            PointerEvent {
                time: Duration::from_millis(20),
                ..e
            },
            &g,
            pos2(100.0, 0.0)
        )
        .failures
        .is_empty()
    );
    tree.unmount();
    assert_eq!(b.target_count(), 0);
    assert_eq!(b.router().route_count(), 0);
    assert_eq!(*log.borrow(), vec!["new tap", "new cancel"]);
}

#[test]
fn queued_down_does_not_join_a_replaced_target_with_the_same_id() {
    let b = GestureBinding::new();
    let log = Log::default();
    let id = Id::new(1);
    let token = Rc::new(RefCell::new(None::<GestureTarget>));
    let t = token.clone();
    let weak = b.downgrade();
    let l = log.clone();
    let mut c = config(&log, "old", false);
    c.tap.as_mut().unwrap().1.on_tap = Some(Rc::new(move |_| {
        let b = weak.upgrade().unwrap();
        b.dispatch(sample(2, PointerPhase::Down, 5.0, 20), [id]);
        let mut changed = config(&l, "new", false);
        changed.tap.as_mut().unwrap().0.pre_accept_slop = 12.0;
        t.borrow().as_ref().unwrap().update(changed).unwrap();
    }));
    *token.borrow_mut() = Some(b.register_target(id, c).unwrap());
    send(&b, sample(1, PointerPhase::Down, 5.0, 0), vec![id]);
    let r = send(&b, sample(1, PointerPhase::Up, 5.0, 10), vec![]);
    assert_eq!(r.processed, 2);
    assert_eq!(b.active_pointers(), 1);
    assert_eq!(b.router().route_count(), 0);
    send(&b, sample(2, PointerPhase::Up, 5.0, 30), vec![]);
    assert!(log.borrow().is_empty());
}
#[test]
fn repeated_reentrant_cancel_all_is_idempotent_and_invalid_move_terminates_capture() {
    let b = GestureBinding::new();
    let weak = b.downgrade();
    let phases = Rc::new(RefCell::new(Vec::new()));
    let p = phases.clone();
    let _observer = b
        .router()
        .add_global_route(move |_, e| {
            p.borrow_mut().push(e.phase);
            if matches!(e.phase, PointerPhase::Move | PointerPhase::Cancel) {
                let b = weak.upgrade().unwrap();
                b.cancel_all();
                b.cancel_all();
            }
        })
        .unwrap();
    send(&b, sample(1, PointerPhase::Down, 5.0, 0), vec![]);
    let r = send(&b, sample(1, PointerPhase::Move, 6.0, 10), vec![]);
    assert_eq!(r.processed, 2);
    assert_eq!(
        *phases.borrow(),
        vec![PointerPhase::Down, PointerPhase::Move, PointerPhase::Cancel]
    );
    assert_eq!(b.active_pointers(), 0);
    send(&b, sample(1, PointerPhase::Down, 5.0, 20), vec![]);
    assert_eq!(
        b.dispatch(sample(1, PointerPhase::Move, f32::NAN, 30), [])
            .failures[0]
            .error,
        BindingError::InvalidPosition
    );
    assert_eq!(b.active_pointers(), 0);
    assert_eq!(phases.borrow().last(), Some(&PointerPhase::Cancel));
}
fn mouse(pos: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    }
}
fn gui_frame(
    ctx: &egui::Context,
    tree: &mut ElementTree,
    b: &GestureBinding,
    events: Vec<egui::Event>,
    time: f64,
    flags: [bool; 3],
) {
    let [focused, disabled, clipped] = flags;
    let mut passes = 0;
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                vec2(300.0, 200.0),
            )),
            time: Some(time),
            focused,
            events,
            ..Default::default()
        },
        |ctx| {
            passes += 1;
            egui::CentralPanel::default().show(ctx, |ui| {
                if disabled {
                    ui.disable();
                }
                if clipped {
                    ui.set_clip_rect(egui::Rect::from_min_max(egui::Pos2::ZERO, pos2(10.0, 10.0)));
                }
                tree.show_with_gestures(ui, BoxConstraints::loose(vec2(200.0, 100.0)).unwrap(), b)
                    .unwrap();
            });
            if ctx.current_pass_index() == 0 {
                ctx.request_discard("exercise input deduplication");
            }
        },
    );
    assert_eq!(passes, 2);
}
#[test]
fn retained_egui_input_is_dispatched_once_in_multipass_and_captures_outside_root() {
    let ctx = egui::Context::default();
    let b = GestureBinding::new();
    let log = Log::default();
    let root = GestureDetector::new(&b, config(&log, "marker", true))
        .unwrap()
        .into_widget(Widget::leaf(vec2(30.0, 24.0), |_| {}));
    let mut tree = ElementTree::new(&ctx, root).unwrap();
    gui_frame(
        &ctx,
        &mut tree,
        &b,
        vec![mouse(pos2(15.0, 15.0), true)],
        0.0,
        [true, false, false],
    );
    gui_frame(
        &ctx,
        &mut tree,
        &b,
        vec![egui::Event::PointerMoved(pos2(180.0, 15.0))],
        0.1,
        [true, false, false],
    );
    gui_frame(
        &ctx,
        &mut tree,
        &b,
        vec![mouse(pos2(220.0, 15.0), false)],
        0.2,
        [true, false, false],
    );
    assert_eq!(
        *log.borrow(),
        vec!["marker cancel", "marker pan", "marker end"]
    );
    assert_eq!(b.active_pointers(), 0);
    gui_frame(
        &ctx,
        &mut tree,
        &b,
        vec![
            mouse(pos2(15.0, 15.0), true),
            mouse(pos2(15.0, 15.0), false),
        ],
        0.3,
        [true, false, false],
    );
    assert_eq!(log.borrow().last(), Some(&"marker tap".into()));
}
#[test]
fn retained_egui_input_respects_parent_clip_focus_and_disabled_root() {
    let ctx = egui::Context::default();
    let b = GestureBinding::new();
    let log = Log::default();
    let root = GestureDetector::new(&b, config(&log, "marker", false))
        .unwrap()
        .into_widget(Widget::leaf(vec2(30.0, 24.0), |_| {}));
    let mut tree = ElementTree::new(&ctx, root).unwrap();
    gui_frame(
        &ctx,
        &mut tree,
        &b,
        vec![mouse(pos2(15.0, 15.0), true)],
        0.0,
        [true, false, true],
    );
    assert_eq!(b.active_pointers(), 0);
    assert!(log.borrow().is_empty());
    gui_frame(
        &ctx,
        &mut tree,
        &b,
        vec![mouse(pos2(15.0, 15.0), true)],
        0.1,
        [true, false, false],
    );
    gui_frame(&ctx, &mut tree, &b, vec![], 0.2, [false, false, false]);
    assert_eq!(*log.borrow(), vec!["marker cancel"]);
    assert_eq!(b.active_pointers(), 0);
    gui_frame(
        &ctx,
        &mut tree,
        &b,
        vec![mouse(pos2(15.0, 15.0), true)],
        0.3,
        [true, false, false],
    );
    gui_frame(&ctx, &mut tree, &b, vec![], 0.4, [true, true, false]);
    assert_eq!(*log.borrow(), vec!["marker cancel", "marker cancel"]);
    assert_eq!(b.router().route_count(), 0);
}
#[test]
fn retained_egui_input_excludes_parent_clip_max_edges() {
    for point in [pos2(10.0, 9.0), pos2(9.0, 10.0)] {
        let ctx = egui::Context::default();
        let binding = GestureBinding::new();
        let log = Log::default();
        let root = GestureDetector::new(&binding, config(&log, "marker", false))
            .unwrap()
            .into_widget(Widget::leaf(vec2(30.0, 24.0), |_| {}));
        let mut tree = ElementTree::new(&ctx, root).unwrap();
        gui_frame(
            &ctx,
            &mut tree,
            &binding,
            vec![mouse(point, true)],
            0.0,
            [true, false, true],
        );
        assert_eq!(
            binding.active_pointers(),
            0,
            "clipped edge {point:?} captured input"
        );
        assert_eq!(binding.router().route_count(), 0);
        assert!(log.borrow().is_empty());
    }
}

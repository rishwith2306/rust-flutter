use rust_flutter::{
    egui::{self, vec2},
    element::*,
    layout::{BoxChildren, BoxConstraints, BoxGeometry, Clip, LayoutError},
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[derive(Default)]
struct Log {
    events: Vec<&'static str>,
    layouts: usize,
    updates: usize,
    slots: Vec<(Vec<egui::Id>, Vec<egui::Id>)>,
    handle: Option<RenderHandle>,
    identities: Vec<usize>,
    paint: Vec<&'static str>,
}
struct Strip {
    gap: f32,
    log: Rc<RefCell<Log>>,
    fail_layout: bool,
    dirty_during_layout: Rc<Cell<bool>>,
}
impl Strip {
    fn new(gap: f32, log: &Rc<RefCell<Log>>) -> Self {
        Self {
            gap,
            log: log.clone(),
            fail_layout: false,
            dirty_during_layout: Rc::new(Cell::new(false)),
        }
    }
}
struct StripBox {
    gap: f32,
    log: Rc<RefCell<Log>>,
    identity: Rc<()>,
    fail: bool,
    dirty: Rc<Cell<bool>>,
}
#[derive(Clone, Debug, PartialEq)]
struct Spacing(f32);
impl RenderObjectWidget for Strip {
    type Object = StripBox;
    fn create_render_object(&self, ctx: &RenderContext) -> StripBox {
        assert_eq!(
            ctx.depend_on::<Spacing>().unwrap_err(),
            DependencyError::InvalidPhase
        );
        self.log.borrow_mut().events.push("create");
        self.log.borrow_mut().handle = Some(ctx.handle());
        StripBox {
            gap: self.gap,
            log: self.log.clone(),
            identity: Rc::new(()),
            fail: self.fail_layout,
            dirty: self.dirty_during_layout.clone(),
        }
    }
    fn update_render_object(&self, object: &mut StripBox, ctx: &RenderContext) {
        object.gap = ctx
            .depend_on::<Spacing>()
            .unwrap()
            .map_or(self.gap, |s| s.0);
        object.fail = self.fail_layout;
        let mut log = self.log.borrow_mut();
        log.updates += 1;
        log.identities.push(Rc::as_ptr(&object.identity) as usize);
    }
    fn did_unmount_render_object(&self, _: &mut StripBox) {
        self.log.borrow_mut().events.push("unmount");
    }
}
impl RenderBox for StripBox {
    fn attach(&mut self, _: &RenderContext) {
        self.log.borrow_mut().events.push("attach");
    }
    fn children_changed(&mut self, old: &[egui::Id], new: &[egui::Id], _: &RenderContext) {
        self.log
            .borrow_mut()
            .slots
            .push((old.to_vec(), new.to_vec()));
    }
    fn layout(
        &mut self,
        c: BoxConstraints,
        children: &mut BoxChildren<'_>,
        ctx: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        self.log.borrow_mut().layouts += 1;
        if self.dirty.replace(false) {
            ctx.handle().mark_needs_layout().unwrap();
        }
        if self.fail {
            return Ok(BoxGeometry::new(vec2(100_000.0, 100_000.0)));
        }
        let mut x = 0.0_f32;
        let mut h = 0.0_f32;
        for i in 0..children.len() {
            let size = children.layout(i, c.loosen())?;
            children.place(i, vec2(x, 0.0))?;
            x += size.x;
            if i + 1 < children.len() {
                x += self.gap;
            }
            h = h.max(size.y);
        }
        let size = if children.is_empty() {
            c.constrain(vec2(20.0, 20.0))?
        } else {
            c.constrain(vec2(x, h))?
        };
        Ok(BoxGeometry {
            size,
            clip: Clip::HardEdge,
            hit_test_self: true,
        })
    }
    fn paint(&mut self, ui: &mut egui::Ui, _: &RenderContext) {
        self.log.borrow_mut().paint.push("parent");
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, egui::Color32::DARK_RED);
    }
    fn detach(&mut self) {
        self.log.borrow_mut().events.push("detach");
    }
    fn dispose(&mut self) {
        self.log.borrow_mut().events.push("dispose");
    }
}
fn leaf(key: &str) -> Widget {
    Widget::leaf(vec2(50.0, 20.0), |_| {}).keyed(key)
}
fn constraints() -> BoxConstraints {
    BoxConstraints::loose(vec2(500.0, 300.0)).unwrap()
}
fn id(tree: &ElementTree, key: &str) -> ElementId {
    tree.elements()
        .iter()
        .find(|e| e.key == Some(Key::from(key)))
        .unwrap()
        .id
}

#[test]
fn persistent_object_updates_keyed_slots_and_disposes_once() {
    let ctx = egui::Context::default();
    let log = Rc::new(RefCell::new(Log::default()));
    let mut tree = ElementTree::new(
        &ctx,
        Widget::render_object(Strip::new(5.0, &log), vec![leaf("A"), leaf("B")]),
    )
    .unwrap();
    let root = tree.root_id();
    let a = id(&tree, "A");
    let b = id(&tree, "B");
    assert_eq!(tree.layout(constraints()).unwrap().size, vec2(105.0, 20.0));
    tree.layout(constraints()).unwrap();
    assert_eq!(log.borrow().layouts, 1);
    assert_eq!(
        log.borrow().slots[0].1,
        vec![egui::Id::new(a), egui::Id::new(b)]
    );
    tree.update(Widget::render_object(
        Strip::new(10.0, &log),
        vec![leaf("B"), leaf("A")],
    ))
    .unwrap();
    let geometry = tree.layout(constraints()).unwrap();
    assert_eq!(
        geometry.rect_for(egui::Id::new(a)).unwrap().min,
        egui::pos2(60.0, 0.0)
    );
    assert_eq!(tree.root_id(), root);
    assert_eq!(id(&tree, "A"), a);
    assert_eq!(id(&tree, "B"), b);
    assert_eq!(log.borrow().layouts, 2);
    assert_eq!(log.borrow().updates, 2);
    assert!(log.borrow().identities.windows(2).all(|p| p[0] == p[1]));
    assert_eq!(
        log.borrow().slots[1],
        (
            vec![egui::Id::new(a), egui::Id::new(b)],
            vec![egui::Id::new(b), egui::Id::new(a)]
        )
    );
    assert!(matches!(
        tree.update(Widget::render_object(
            Strip::new(0.0, &log),
            vec![leaf("dup"), leaf("dup")]
        )),
        Err(ElementError::DuplicateKey(_))
    ));
    assert_eq!(tree.root_id(), root);
    tree.unmount();
    tree.unmount();
    assert_eq!(
        log.borrow().events,
        vec!["create", "attach", "detach", "unmount", "dispose"]
    );
    let handle = log.borrow().handle.clone().unwrap();
    assert!(!handle.mounted());
    assert_eq!(handle.mark_needs_layout(), Err(Unmounted));
    assert_eq!(handle.mark_needs_paint(), Err(Unmounted));
}

#[test]
fn paint_only_requests_reuse_layout_and_worker_invalidations_relayout() {
    let ctx = egui::Context::default();
    let log = Rc::new(RefCell::new(Log::default()));
    let mut tree =
        ElementTree::new(&ctx, Widget::render_object(Strip::new(0.0, &log), vec![])).unwrap();
    tree.layout(constraints()).unwrap();
    let handle = log.borrow().handle.clone().unwrap();
    handle.mark_needs_paint().unwrap();
    tree.layout(constraints()).unwrap();
    assert_eq!(log.borrow().layouts, 1);
    handle.mark_needs_layout().unwrap();
    tree.layout(constraints()).unwrap();
    assert_eq!(log.borrow().layouts, 2);
    std::thread::spawn(move || handle.mark_needs_layout().unwrap())
        .join()
        .unwrap();
    tree.layout(constraints()).unwrap();
    assert_eq!(log.borrow().layouts, 3);
    tree.layout(BoxConstraints::tight(vec2(80.0, 90.0)).unwrap())
        .unwrap();
    assert_eq!(log.borrow().layouts, 4);
}

#[test]
fn layout_requests_made_during_layout_are_preserved_for_the_next_pass() {
    let ctx = egui::Context::default();
    let log = Rc::new(RefCell::new(Log::default()));
    let widget = Strip::new(0.0, &log);
    widget.dirty_during_layout.set(true);
    let mut tree = ElementTree::new(&ctx, Widget::render_object(widget, vec![])).unwrap();
    tree.layout(constraints()).unwrap();
    tree.layout(constraints()).unwrap();
    tree.layout(constraints()).unwrap();
    assert_eq!(log.borrow().layouts, 2);
}

#[test]
fn inherited_updates_reconfigure_the_same_object_and_layout_errors_preserve_it() {
    let ctx = egui::Context::default();
    let log = Rc::new(RefCell::new(Log::default()));
    let mut widget = Strip::new(1.0, &log);
    widget.fail_layout = true;
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited(
            Spacing(4.0),
            Widget::render_object(widget, vec![leaf("A"), leaf("B")]),
        ),
    )
    .unwrap();
    let handle = tree
        .inherited_handle::<Spacing>(tree.root_id().unwrap())
        .unwrap();
    assert!(matches!(
        tree.layout(constraints()),
        Err(ElementError::Layout(LayoutError::ChildViolatedConstraints(
            _
        )))
    ));
    assert_eq!(log.borrow().events, vec!["create", "attach"]);
    tree.update(Widget::inherited(
        Spacing(4.0),
        Widget::render_object(Strip::new(1.0, &log), vec![leaf("A"), leaf("B")]),
    ))
    .unwrap();
    assert_eq!(tree.layout(constraints()).unwrap().size.x, 104.0);
    handle.set(Spacing(14.0)).unwrap();
    assert_eq!(tree.layout(constraints()).unwrap().size.x, 114.0);
    assert_eq!(log.borrow().updates, 3);
    assert!(log.borrow().identities.windows(2).all(|p| p[0] == p[1]));
}

struct Switch;
#[derive(Default)]
struct SwitchState {
    alternate: bool,
    fail: bool,
}
impl StatefulWidget for Switch {
    type State = SwitchState;
    fn create_state(&self) -> SwitchState {
        SwitchState::default()
    }
}
impl State<Switch> for SwitchState {
    fn build(&mut self, _: &Switch, _: &StateContext<Self>) -> Widget {
        if self.fail {
            Widget::flex(
                FlexStyle::new(rust_flutter::layout::Axis::Horizontal),
                vec![FlexItem::fixed(leaf("dup")), FlexItem::fixed(leaf("dup"))],
            )
        } else {
            leaf(if self.alternate { "second" } else { "first" })
        }
    }
}
#[test]
fn slots_follow_real_render_descendants_and_failed_builds_dispose_the_parent() {
    let ctx = egui::Context::default();
    let log = Rc::new(RefCell::new(Log::default()));
    let mut tree = ElementTree::new(
        &ctx,
        Widget::render_object(
            Strip::new(0.0, &log),
            vec![Widget::stateful(Switch).keyed("switch")],
        ),
    )
    .unwrap();
    let switch = id(&tree, "switch");
    let first = id(&tree, "first");
    let handle = tree.state_handle::<SwitchState>(switch).unwrap();
    tree.layout(constraints()).unwrap();
    handle.update(|s| s.alternate = true).unwrap();
    tree.layout(constraints()).unwrap();
    let second = id(&tree, "second");
    assert_ne!(first, second);
    assert_eq!(switch, id(&tree, "switch"));
    assert_eq!(
        log.borrow().slots.last().unwrap(),
        &(vec![egui::Id::new(first)], vec![egui::Id::new(second)])
    );
    handle.update(|s| s.fail = true).unwrap();
    assert!(tree.flush().is_err());
    assert!(tree.root_id().is_none());
    assert_eq!(
        log.borrow().events,
        vec!["create", "attach", "detach", "unmount", "dispose"]
    );
}

#[test]
fn custom_containers_paint_behind_children_and_preserve_clipped_child_hit_regions() {
    let ctx = egui::Context::default();
    let log = Rc::new(RefCell::new(Log::default()));
    let clicks = Rc::new(Cell::new(0));
    let child_log = log.clone();
    let count = clicks.clone();
    let bounds = Rc::new(Cell::new(egui::Rect::NOTHING));
    let captured = bounds.clone();
    let mut tree = ElementTree::new(
        &ctx,
        Widget::render_object(
            Strip::new(0.0, &log),
            vec![Widget::leaf(vec2(50.0, 20.0), move |ui| {
                child_log.borrow_mut().paint.push("child");
                captured.set(ui.max_rect());
                if ui.put(ui.max_rect(), egui::Button::new("Child")).clicked() {
                    count.set(count.get() + 1);
                }
            })],
        ),
    )
    .unwrap();
    let mut frame = |events| {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    vec2(500.0, 300.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::Area::new(egui::Id::new("scene"))
                    .fixed_pos(egui::Pos2::ZERO)
                    .movable(false)
                    .show(ctx, |ui| {
                        tree.show(ui, constraints()).unwrap();
                    });
            },
        );
    };
    frame(vec![]);
    frame(vec![]); // Area's initial sizing pass precedes interactive hit registration.
    let pos = bounds.get().center();
    frame(vec![
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        },
    ]);
    frame(vec![egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    }]);
    assert_eq!(
        clicks.get(),
        1,
        "child bounds: {:?}; paints {:?}",
        bounds.get(),
        log.borrow().paint
    );
    {
        let log = log.borrow();
        let (pairs, remainder) = log.paint.as_chunks::<2>();
        assert!(!pairs.is_empty() && remainder.is_empty());
        assert!(pairs.iter().all(|p| p == &["parent", "child"]));
    }
    assert_eq!(log.borrow().layouts, 1);
}

struct Alternate(Strip);
impl RenderObjectWidget for Alternate {
    type Object = StripBox;
    fn create_render_object(&self, ctx: &RenderContext) -> StripBox {
        self.0.create_render_object(ctx)
    }
    fn update_render_object(&self, object: &mut StripBox, ctx: &RenderContext) {
        self.0.update_render_object(object, ctx);
    }
    fn did_unmount_render_object(&self, object: &mut StripBox) {
        self.0.did_unmount_render_object(object);
    }
}
#[test]
fn different_widget_types_replace_objects_even_when_the_object_type_and_key_match() {
    let ctx = egui::Context::default();
    let log = Rc::new(RefCell::new(Log::default()));
    let mut tree = ElementTree::new(
        &ctx,
        Widget::render_object(Strip::new(0.0, &log), vec![]).keyed("render"),
    )
    .unwrap();
    let old = tree.root_id();
    let old_handle = log.borrow().handle.clone().unwrap();
    tree.update(Widget::render_object(Alternate(Strip::new(0.0, &log)), vec![]).keyed("render"))
        .unwrap();
    assert_ne!(tree.root_id(), old);
    assert!(!old_handle.mounted());
    assert_eq!(
        log.borrow().events,
        vec![
            "create", "attach", "detach", "unmount", "dispose", "create", "attach"
        ]
    );
    tree.unmount();
    assert_eq!(
        log.borrow()
            .events
            .iter()
            .filter(|&&event| event == "dispose")
            .count(),
        2
    );
}
struct BadBuild;
impl StatelessWidget for BadBuild {
    fn build(&self, _: &BuildContext) -> Widget {
        Widget::flex(
            FlexStyle::new(rust_flutter::layout::Axis::Horizontal),
            vec![FlexItem::fixed(leaf("dup")), FlexItem::fixed(leaf("dup"))],
        )
    }
}
#[test]
fn failure_while_mounting_children_cleans_up_the_new_object_and_its_handle() {
    let ctx = egui::Context::default();
    let log = Rc::new(RefCell::new(Log::default()));
    assert!(
        ElementTree::new(
            &ctx,
            Widget::render_object(Strip::new(0.0, &log), vec![Widget::stateless(BadBuild)])
        )
        .is_err()
    );
    assert_eq!(
        log.borrow().events,
        vec!["create", "attach", "detach", "unmount", "dispose"]
    );
    assert!(!log.borrow().handle.as_ref().unwrap().mounted());
}

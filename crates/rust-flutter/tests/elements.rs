use rust_flutter::{
    egui::{self, vec2},
    element::*,
    layout::{Axis, BoxConstraints, CrossAxisAlignment, LayoutError},
};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
#[derive(Default)]
struct Counts {
    init: usize,
    build: usize,
    update: Vec<(i32, i32)>,
    dispose: usize,
}
struct Counter {
    initial: i32,
    counts: Rc<RefCell<Counts>>,
}
struct CounterState {
    value: i32,
    counts: Rc<RefCell<Counts>>,
}
impl StatefulWidget for Counter {
    type State = CounterState;
    fn create_state(&self) -> CounterState {
        CounterState {
            value: self.initial,
            counts: self.counts.clone(),
        }
    }
}
impl State<Counter> for CounterState {
    fn init_state(&mut self, _: &Counter, ctx: &StateContext<Self>) {
        assert!(ctx.mounted());
        self.counts.borrow_mut().init += 1;
    }
    fn did_update_widget(&mut self, old: &Counter, new: &Counter, _: &StateContext<Self>) {
        self.counts
            .borrow_mut()
            .update
            .push((old.initial, new.initial));
    }
    fn build(&mut self, _: &Counter, ctx: &StateContext<Self>) -> Widget {
        self.counts.borrow_mut().build += 1;
        let value = self.value;
        let handle = ctx.handle();
        Widget::leaf(vec2(100.0, 44.0), move |ui| {
            if ui
                .put(ui.max_rect(), egui::Button::new(format!("{value}")))
                .clicked()
            {
                handle.update(|state| state.value += 1).unwrap();
            }
        })
    }
    fn dispose(&mut self) {
        self.counts.borrow_mut().dispose += 1;
    }
}
fn counter(initial: i32, counts: &Rc<RefCell<Counts>>, key: &str) -> Widget {
    Widget::stateful(Counter {
        initial,
        counts: counts.clone(),
    })
    .keyed(key)
}
fn row(children: Vec<Widget>) -> Widget {
    let mut style = FlexStyle::new(Axis::Horizontal);
    style.cross_alignment = CrossAxisAlignment::Stretch;
    Widget::flex(style, children.into_iter().map(FlexItem::fixed).collect())
}
fn keyed_id(tree: &ElementTree, key: &str) -> ElementId {
    tree.elements()
        .into_iter()
        .find(|e| e.key == Some(Key::from(key)))
        .unwrap()
        .id
}

#[test]
fn state_initializes_once_updates_properties_without_reset_and_disposes_once() {
    let ctx = egui::Context::default();
    let counts = Rc::default();
    let mut tree = ElementTree::new(&ctx, counter(2, &counts, "root")).unwrap();
    let id = tree.root_id().unwrap();
    let handle = tree.state_handle::<CounterState>(id).unwrap();
    handle.update(|s| s.value += 3).unwrap();
    tree.flush().unwrap();
    assert_eq!(tree.state::<CounterState>(id).unwrap().value, 5);
    tree.update(counter(99, &counts, "root")).unwrap();
    assert_eq!(tree.root_id(), Some(id));
    assert_eq!(tree.state::<CounterState>(id).unwrap().value, 5);
    assert_eq!(counts.borrow().init, 1);
    assert_eq!(counts.borrow().update, [(2, 99)]);
    tree.unmount();
    tree.unmount();
    drop(tree);
    assert_eq!(counts.borrow().dispose, 1);
    assert!(!handle.mounted());
    assert_eq!(handle.update(|_| {}), Err(Unmounted));
}
#[test]
fn keyed_reordering_insertion_and_removal_keep_state_with_its_key() {
    let ctx = egui::Context::default();
    let a: Rc<RefCell<Counts>> = Rc::default();
    let b: Rc<RefCell<Counts>> = Rc::default();
    let c = Rc::default();
    let mut tree =
        ElementTree::new(&ctx, row(vec![counter(1, &a, "a"), counter(2, &b, "b")])).unwrap();
    let aid = keyed_id(&tree, "a");
    let bid = keyed_id(&tree, "b");
    let ah = tree.state_handle::<CounterState>(aid).unwrap();
    let bh = tree.state_handle::<CounterState>(bid).unwrap();
    ah.update(|s| s.value = 41).unwrap();
    bh.update(|s| s.value = 82).unwrap();
    tree.flush().unwrap();
    tree.update(row(vec![
        counter(200, &b, "b"),
        counter(3, &c, "c"),
        counter(100, &a, "a"),
    ]))
    .unwrap();
    assert_eq!(keyed_id(&tree, "a"), aid);
    assert_eq!(keyed_id(&tree, "b"), bid);
    assert_eq!(tree.state::<CounterState>(aid).unwrap().value, 41);
    assert_eq!(tree.state::<CounterState>(bid).unwrap().value, 82);
    tree.update(row(vec![counter(0, &a, "a"), counter(0, &c, "c")]))
        .unwrap();
    assert_eq!(b.borrow().dispose, 1);
    assert!(!bh.mounted());
    assert!(ah.mounted());
}
#[test]
fn unkeyed_children_follow_slots_and_key_changes_create_fresh_state() {
    let ctx = egui::Context::default();
    let a: Rc<RefCell<Counts>> = Rc::default();
    let b: Rc<RefCell<Counts>> = Rc::default();
    let unkeyed = |n, counts: &Rc<RefCell<Counts>>| {
        Widget::stateful(Counter {
            initial: n,
            counts: counts.clone(),
        })
    };
    let mut tree = ElementTree::new(&ctx, row(vec![unkeyed(1, &a), unkeyed(2, &b)])).unwrap();
    let root = tree.root_id().unwrap();
    let first = tree
        .elements()
        .into_iter()
        .find(|e| e.id == root)
        .unwrap()
        .children[0];
    tree.update(row(vec![unkeyed(20, &b), unkeyed(10, &a)]))
        .unwrap();
    assert_eq!(tree.state::<CounterState>(first).unwrap().value, 1);
    tree.update(counter(7, &a, "old")).unwrap();
    let old = tree.root_id().unwrap();
    let h = tree.state_handle::<CounterState>(old).unwrap();
    tree.update(counter(9, &a, "new")).unwrap();
    assert_ne!(tree.root_id(), Some(old));
    assert!(!h.mounted());
    assert_eq!(
        tree.state::<CounterState>(tree.root_id().unwrap())
            .unwrap()
            .value,
        9
    );
}
#[test]
fn same_key_with_another_component_type_replaces_the_element() {
    struct Other;
    impl StatelessWidget for Other {
        fn build(&self, _: &BuildContext) -> Widget {
            Widget::empty()
        }
    }
    let ctx = egui::Context::default();
    let counts = Rc::default();
    let mut tree = ElementTree::new(&ctx, counter(1, &counts, "x")).unwrap();
    let old = tree.root_id().unwrap();
    tree.update(Widget::stateless(Other).keyed("x")).unwrap();
    assert_ne!(tree.root_id(), Some(old));
    assert_eq!(counts.borrow().dispose, 1);
}
#[test]
fn duplicate_sibling_keys_are_rejected_before_the_existing_tree_is_changed() {
    let ctx = egui::Context::default();
    let counts = Rc::default();
    let mut tree = ElementTree::new(&ctx, counter(1, &counts, "root")).unwrap();
    let old = tree.root_id();
    assert!(matches!(
        tree.update(row(vec![
            counter(1, &counts, "x"),
            counter(2, &counts, "x")
        ])),
        Err(ElementError::DuplicateKey(_))
    ));
    assert_eq!(tree.root_id(), old);
    assert_eq!(counts.borrow().init, 1);
    assert_eq!(counts.borrow().dispose, 0);
    // Local keys may be repeated under distinct parents.
    tree.update(row(vec![
        Widget::padding(Default::default(), counter(1, &counts, "local")),
        Widget::padding(Default::default(), counter(2, &counts, "local")),
    ]))
    .unwrap();
}
#[test]
fn worker_updates_non_send_state_on_the_ui_thread_and_wakes_the_context() {
    let ctx = egui::Context::default();
    let counts = Rc::default();
    let mut tree = ElementTree::new(&ctx, counter(1, &counts, "root")).unwrap();
    let id = tree.root_id().unwrap();
    for _ in 0..4 {
        let _ = ctx.run(Default::default(), |_| {});
    }
    let wake = Arc::new(AtomicUsize::new(0));
    let seen = wake.clone();
    ctx.set_request_repaint_callback(move |_| {
        seen.fetch_add(1, Ordering::SeqCst);
    });
    let handle = tree.state_handle::<CounterState>(id).unwrap();
    std::thread::spawn(move || handle.update(|s| s.value = 45).unwrap())
        .join()
        .unwrap();
    assert!(wake.load(Ordering::SeqCst) > 0);
    assert_eq!(tree.state::<CounterState>(id).unwrap().value, 1);
    assert!(tree.flush().unwrap());
    assert_eq!(tree.state::<CounterState>(id).unwrap().value, 45);
    assert!(!tree.flush().unwrap());
}
#[test]
fn updates_requested_during_build_wait_for_a_later_pass_without_spinning() {
    struct SelfUpdating;
    struct Value(i32);
    impl StatefulWidget for SelfUpdating {
        type State = Value;
        fn create_state(&self) -> Value {
            Value(0)
        }
    }
    impl State<SelfUpdating> for Value {
        fn build(&mut self, _: &SelfUpdating, ctx: &StateContext<Self>) -> Widget {
            if self.0 < 3 {
                ctx.handle().update(|s| s.0 += 1).unwrap();
            }
            Widget::empty()
        }
    }
    let ctx = egui::Context::default();
    let mut tree = ElementTree::new(&ctx, Widget::stateful(SelfUpdating)).unwrap();
    let id = tree.root_id().unwrap();
    assert_eq!(tree.state::<Value>(id).unwrap().0, 0);
    for expected in 1..=3 {
        tree.flush().unwrap();
        assert_eq!(tree.state::<Value>(id).unwrap().0, expected);
    }
    assert!(!tree.flush().unwrap());
}
#[test]
fn dynamic_build_failure_unmounts_all_state_and_accepts_a_fresh_root() {
    struct Bad {
        counts: Rc<RefCell<Counts>>,
    }
    struct BadState(Rc<RefCell<Counts>>);
    impl StatefulWidget for Bad {
        type State = BadState;
        fn create_state(&self) -> BadState {
            BadState(self.counts.clone())
        }
    }
    impl State<Bad> for BadState {
        fn build(&mut self, _: &Bad, _: &StateContext<Self>) -> Widget {
            row(vec![
                Widget::empty().keyed("dup"),
                Widget::empty().keyed("dup"),
            ])
        }
        fn dispose(&mut self) {
            self.0.borrow_mut().dispose += 1;
        }
    }
    let ctx = egui::Context::default();
    let counts = Rc::default();
    let mut tree = ElementTree::new(&ctx, counter(1, &counts, "old")).unwrap();
    let handle = tree
        .state_handle::<CounterState>(tree.root_id().unwrap())
        .unwrap();
    assert!(
        tree.update(Widget::stateful(Bad {
            counts: counts.clone()
        }))
        .is_err()
    );
    assert!(tree.root_id().is_none());
    assert!(!handle.mounted());
    assert_eq!(counts.borrow().dispose, 2);
    tree.update(Widget::empty()).unwrap();
    assert!(tree.root_id().is_some());
}
#[test]
fn context_changes_are_rejected_and_layout_errors_preserve_component_state() {
    let ctx = egui::Context::default();
    let counts = Rc::default();
    let mut tree = ElementTree::new(
        &ctx,
        Widget::flex(
            FlexStyle::new(Axis::Horizontal),
            vec![FlexItem::expanded(counter(1, &counts, "x"), 1)],
        ),
    )
    .unwrap();
    assert!(matches!(
        tree.layout(BoxConstraints::default()),
        Err(ElementError::Layout(LayoutError::UnboundedFlex))
    ));
    assert_eq!(counts.borrow().dispose, 0);
    tree.layout(BoxConstraints::tight(vec2(200.0, 50.0)).unwrap())
        .unwrap();
    let other = egui::Context::default();
    let _ = other.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            assert!(matches!(
                tree.show(ui, BoxConstraints::default()),
                Err(ElementError::ContextChanged)
            ));
        });
    });
}
#[test]
fn real_pointer_click_updates_component_state_and_reuses_render_identity() {
    let ctx = egui::Context::default();
    let counts = Rc::default();
    let mut tree = ElementTree::new(&ctx, counter(0, &counts, "counter")).unwrap();
    let id = tree.root_id().unwrap();
    let mut paint = |events| {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    vec2(300.0, 150.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    tree.show(ui, BoxConstraints::tight(vec2(100.0, 44.0)).unwrap())
                        .unwrap();
                });
            },
        )
    };
    let first = paint(vec![]);
    assert!(!first.shapes.is_empty());
    let _ = paint(vec![]);
    let point = egui::pos2(50.0, 30.0);
    for pressed in [true, false] {
        let _ = paint(vec![
            egui::Event::PointerMoved(point),
            egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            },
        ]);
    }
    let _ = paint(vec![]);
    assert_eq!(tree.state::<CounterState>(id).unwrap().value, 1);
    let geometry = tree
        .layout(BoxConstraints::tight(vec2(100.0, 44.0)).unwrap())
        .unwrap();
    let render = geometry.id;
    tree.update(counter(100, &counts, "counter")).unwrap();
    assert_eq!(
        tree.layout(BoxConstraints::tight(vec2(100.0, 44.0)).unwrap())
            .unwrap()
            .id,
        render
    );
}
#[test]
fn unmount_discards_pending_commands_and_nested_components_dispose_child_first() {
    struct Parent {
        log: Arc<Mutex<Vec<&'static str>>>,
    }
    struct ParentState(Arc<Mutex<Vec<&'static str>>>);
    struct Child {
        log: Arc<Mutex<Vec<&'static str>>>,
    }
    struct ChildState(Arc<Mutex<Vec<&'static str>>>);
    impl StatefulWidget for Parent {
        type State = ParentState;
        fn create_state(&self) -> ParentState {
            ParentState(self.log.clone())
        }
    }
    impl StatefulWidget for Child {
        type State = ChildState;
        fn create_state(&self) -> ChildState {
            ChildState(self.log.clone())
        }
    }
    impl State<Parent> for ParentState {
        fn build(&mut self, _: &Parent, _: &StateContext<Self>) -> Widget {
            Widget::stateful(Child {
                log: self.0.clone(),
            })
        }
        fn dispose(&mut self) {
            self.0.lock().unwrap().push("parent");
        }
    }
    impl State<Child> for ChildState {
        fn build(&mut self, _: &Child, _: &StateContext<Self>) -> Widget {
            Widget::empty()
        }
        fn dispose(&mut self) {
            self.0.lock().unwrap().push("child");
        }
    }
    let ctx = egui::Context::default();
    let log: Arc<Mutex<Vec<&'static str>>> = Arc::default();
    let mut tree = ElementTree::new(&ctx, Widget::stateful(Parent { log: log.clone() })).unwrap();
    let handle = tree
        .state_handle::<ParentState>(tree.root_id().unwrap())
        .unwrap();
    handle
        .update(|_| panic!("discard pending command"))
        .unwrap();
    tree.unmount();
    assert_eq!(*log.lock().unwrap(), ["child", "parent"]);
    assert!(!handle.mounted());
}

#[test]
fn dirty_child_rebuild_does_not_rebuild_parent_or_unchanged_sibling() {
    struct Parent {
        builds: Arc<AtomicUsize>,
        a: Rc<RefCell<Counts>>,
        b: Rc<RefCell<Counts>>,
    }
    impl StatelessWidget for Parent {
        fn build(&self, _: &BuildContext) -> Widget {
            self.builds.fetch_add(1, Ordering::SeqCst);
            row(vec![counter(0, &self.a, "a"), counter(0, &self.b, "b")])
        }
    }
    let ctx = egui::Context::default();
    let a: Rc<RefCell<Counts>> = Rc::default();
    let b: Rc<RefCell<Counts>> = Rc::default();
    let builds = Arc::new(AtomicUsize::new(0));
    let mut tree = ElementTree::new(
        &ctx,
        Widget::stateless(Parent {
            builds: builds.clone(),
            a: a.clone(),
            b: b.clone(),
        }),
    )
    .unwrap();
    tree.state_handle::<CounterState>(keyed_id(&tree, "a"))
        .unwrap()
        .update(|s| s.value += 1)
        .unwrap();
    tree.flush().unwrap();
    assert_eq!(builds.load(Ordering::SeqCst), 1);
    assert_eq!(a.borrow().build, 2);
    assert_eq!(b.borrow().build, 1);
}
#[test]
fn updates_arriving_while_another_command_executes_are_not_lost() {
    let ctx = egui::Context::default();
    let counts = Rc::default();
    let mut tree = ElementTree::new(&ctx, counter(0, &counts, "root")).unwrap();
    let id = tree.root_id().unwrap();
    let handle = tree.state_handle::<CounterState>(id).unwrap();
    let second = handle.clone();
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let worker_barrier = barrier.clone();
    handle
        .update(move |s| {
            barrier.wait();
            barrier.wait();
            s.value = 10;
        })
        .unwrap();
    let worker = std::thread::spawn(move || {
        worker_barrier.wait();
        second.update(|s| s.value = 20).unwrap();
        worker_barrier.wait();
    });
    tree.flush().unwrap();
    worker.join().unwrap();
    assert_eq!(tree.state::<CounterState>(id).unwrap().value, 10);
    assert!(tree.flush().unwrap());
    assert_eq!(tree.state::<CounterState>(id).unwrap().value, 20);
}
#[test]
fn command_capture_destructors_can_inspect_a_handle_during_unmount() {
    struct InspectOnDrop {
        handle: StateHandle<CounterState>,
        result: Arc<Mutex<Option<Result<(), Unmounted>>>>,
    }
    impl Drop for InspectOnDrop {
        fn drop(&mut self) {
            *self.result.lock().unwrap() = Some(self.handle.update(|_| {}));
        }
    }
    let ctx = egui::Context::default();
    let counts = Rc::default();
    let mut tree = ElementTree::new(&ctx, counter(0, &counts, "root")).unwrap();
    let handle = tree
        .state_handle::<CounterState>(tree.root_id().unwrap())
        .unwrap();
    let result = Arc::new(Mutex::new(None));
    let capture = InspectOnDrop {
        handle: handle.clone(),
        result: result.clone(),
    };
    handle.update(move |_| drop(capture)).unwrap();
    tree.unmount();
    assert_eq!(*result.lock().unwrap(), Some(Err(Unmounted)));
}
#[test]
fn failure_during_queued_rebuild_disposes_the_existing_component() {
    struct Flaky {
        disposed: Arc<AtomicUsize>,
    }
    struct FlakyState {
        invalid: bool,
        disposed: Arc<AtomicUsize>,
    }
    impl StatefulWidget for Flaky {
        type State = FlakyState;
        fn create_state(&self) -> FlakyState {
            FlakyState {
                invalid: false,
                disposed: self.disposed.clone(),
            }
        }
    }
    impl State<Flaky> for FlakyState {
        fn build(&mut self, _: &Flaky, _: &StateContext<Self>) -> Widget {
            if self.invalid {
                row(vec![Widget::empty().keyed("x"), Widget::empty().keyed("x")])
            } else {
                Widget::empty()
            }
        }
        fn dispose(&mut self) {
            self.disposed.fetch_add(1, Ordering::SeqCst);
        }
    }
    let ctx = egui::Context::default();
    let disposed = Arc::new(AtomicUsize::new(0));
    let mut tree = ElementTree::new(
        &ctx,
        Widget::stateful(Flaky {
            disposed: disposed.clone(),
        }),
    )
    .unwrap();
    let handle = tree
        .state_handle::<FlakyState>(tree.root_id().unwrap())
        .unwrap();
    handle.update(|s| s.invalid = true).unwrap();
    assert!(matches!(tree.flush(), Err(ElementError::DuplicateKey(_))));
    assert_eq!(disposed.load(Ordering::SeqCst), 1);
    assert!(!handle.mounted());
    assert!(tree.root_id().is_none());
}

#[test]
fn measured_widget_uses_parent_constraints_and_preserves_identity_on_configuration_updates() {
    let ctx = egui::Context::default();
    let measured = || {
        Widget::measured(
            |constraints| constraints.constrain(vec2(constraints.max().x, 20.0)),
            |_| {},
        )
        .keyed("measure")
    };
    let mut tree = ElementTree::new(&ctx, measured()).unwrap();
    let id = tree.root_id().unwrap();
    assert_eq!(
        tree.layout(BoxConstraints::loose(vec2(140.0, 80.0)).unwrap())
            .unwrap()
            .size,
        vec2(140.0, 20.0)
    );
    tree.update(measured()).unwrap();
    assert_eq!(tree.root_id(), Some(id));
    assert_eq!(
        tree.layout(BoxConstraints::loose(vec2(70.0, 80.0)).unwrap())
            .unwrap()
            .size,
        vec2(70.0, 20.0)
    );
}

#[test]
fn recursive_component_builds_fail_at_depth_limit_and_dispose_all_created_states() {
    struct Recursive {
        created: Arc<AtomicUsize>,
        disposed: Arc<AtomicUsize>,
    }
    struct RecursiveState {
        created: Arc<AtomicUsize>,
        disposed: Arc<AtomicUsize>,
    }
    impl StatefulWidget for Recursive {
        type State = RecursiveState;
        fn create_state(&self) -> RecursiveState {
            self.created.fetch_add(1, Ordering::SeqCst);
            RecursiveState {
                created: self.created.clone(),
                disposed: self.disposed.clone(),
            }
        }
    }
    impl State<Recursive> for RecursiveState {
        fn build(&mut self, _: &Recursive, _: &StateContext<Self>) -> Widget {
            Widget::stateful(Recursive {
                created: self.created.clone(),
                disposed: self.disposed.clone(),
            })
        }
        fn dispose(&mut self) {
            self.disposed.fetch_add(1, Ordering::SeqCst);
        }
    }
    let ctx = egui::Context::default();
    let created = Arc::new(AtomicUsize::new(0));
    let disposed = Arc::new(AtomicUsize::new(0));
    let result = ElementTree::new(
        &ctx,
        Widget::stateful(Recursive {
            created: created.clone(),
            disposed: disposed.clone(),
        }),
    );
    assert!(matches!(result, Err(ElementError::TreeTooDeep)));
    assert!(created.load(Ordering::SeqCst) > 0 && created.load(Ordering::SeqCst) <= 256);
    assert_eq!(
        created.load(Ordering::SeqCst),
        disposed.load(Ordering::SeqCst)
    );
}

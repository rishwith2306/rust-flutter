use rust_flutter::{
    egui,
    element::*,
    layout::{Axis, BoxConstraints},
};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
#[derive(PartialEq, Debug)]
struct Theme(u32);
#[derive(Default)]
struct Log {
    init: usize,
    build: usize,
    dependencies: usize,
    dispose: usize,
}
struct Consumer {
    listen: bool,
    hook_only: bool,
    log: Rc<RefCell<Log>>,
    context: Rc<RefCell<Option<BuildContext>>>,
}
struct ConsumerState {
    listen: bool,
    hook_only: bool,
    seen: Option<u32>,
    local: u32,
    log: Rc<RefCell<Log>>,
}
impl StatefulWidget for Consumer {
    type State = ConsumerState;
    fn create_state(&self) -> ConsumerState {
        ConsumerState {
            listen: self.listen,
            hook_only: self.hook_only,
            seen: None,
            local: 0,
            log: self.log.clone(),
        }
    }
}
impl State<Consumer> for ConsumerState {
    fn init_state(&mut self, widget: &Consumer, ctx: &StateContext<Self>) {
        self.log.borrow_mut().init += 1;
        assert_eq!(
            ctx.depend_on::<Theme>().unwrap_err(),
            DependencyError::InvalidPhase
        );
        *widget.context.borrow_mut() = Some((**ctx).clone());
    }
    fn did_change_dependencies(&mut self, _: &Consumer, ctx: &StateContext<Self>) {
        self.log.borrow_mut().dependencies += 1;
        if self.hook_only {
            self.seen = ctx.depend_on::<Theme>().unwrap().map(|value| value.0);
        }
    }
    fn build(&mut self, _: &Consumer, ctx: &StateContext<Self>) -> Widget {
        self.log.borrow_mut().build += 1;
        if !self.hook_only {
            self.seen = if self.listen {
                ctx.depend_on::<Theme>().unwrap()
            } else {
                ctx.read::<Theme>().unwrap()
            }
            .map(|value| value.0);
        }
        Widget::leaf(egui::vec2(20.0, 20.0), |_| {})
    }
    fn dispose(&mut self) {
        self.log.borrow_mut().dispose += 1;
    }
}
fn consumer(
    key: &str,
    listen: bool,
    hook_only: bool,
    log: &Rc<RefCell<Log>>,
    context: &Rc<RefCell<Option<BuildContext>>>,
) -> Widget {
    Widget::stateful(Consumer {
        listen,
        hook_only,
        log: log.clone(),
        context: context.clone(),
    })
    .keyed(key)
}
fn id(tree: &ElementTree, key: &str) -> ElementId {
    tree.elements()
        .into_iter()
        .find(|e| e.key == Some(Key::from(key)))
        .unwrap()
        .id
}
fn row(children: Vec<Widget>) -> Widget {
    Widget::flex(
        FlexStyle::new(Axis::Horizontal),
        children.into_iter().map(FlexItem::fixed).collect(),
    )
}
#[test]
fn nearest_provider_notifies_subscribers_without_rebuilding_nonlisteners_or_resetting_state() {
    let ctx = egui::Context::default();
    let listen = Rc::default();
    let read = Rc::default();
    let lc = Rc::default();
    let rc = Rc::default();
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited(
            Theme(1),
            row(vec![
                consumer("listen", true, false, &listen, &lc),
                consumer("read", false, false, &read, &rc),
            ]),
        )
        .keyed("theme"),
    )
    .unwrap();
    let listener = id(&tree, "listen");
    let reader = id(&tree, "read");
    tree.state_handle::<ConsumerState>(listener)
        .unwrap()
        .update(|s| s.local = 45)
        .unwrap();
    tree.flush().unwrap();
    let provider = tree.inherited_handle::<Theme>(id(&tree, "theme")).unwrap();
    provider.set(Theme(2)).unwrap();
    tree.flush().unwrap();
    assert_eq!(tree.state::<ConsumerState>(listener).unwrap().seen, Some(2));
    assert_eq!(tree.state::<ConsumerState>(listener).unwrap().local, 45);
    assert_eq!(tree.state::<ConsumerState>(reader).unwrap().seen, Some(1));
    assert_eq!(read.borrow().build, 1);
    assert_eq!(listen.borrow().init, 1);
    assert_eq!(listen.borrow().dependencies, 2);
    let context = lc.borrow();
    assert_eq!(
        context
            .as_ref()
            .unwrap()
            .depend_on::<Theme>()
            .unwrap()
            .unwrap()
            .0,
        2
    );
    assert_eq!(
        context
            .as_ref()
            .unwrap()
            .read::<Theme>()
            .unwrap()
            .unwrap()
            .0,
        2
    );
}
#[test]
fn nested_same_type_providers_shadow_outer_values_and_notifications() {
    let ctx = egui::Context::default();
    let outer_log = Rc::default();
    let inner_log = Rc::default();
    let oc = Rc::default();
    let ic = Rc::default();
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited(
            Theme(10),
            row(vec![
                consumer("outer-consumer", true, false, &outer_log, &oc),
                Widget::inherited(
                    Theme(20),
                    consumer("inner-consumer", true, false, &inner_log, &ic),
                )
                .keyed("inner"),
            ]),
        )
        .keyed("outer"),
    )
    .unwrap();
    tree.inherited_handle::<Theme>(id(&tree, "outer"))
        .unwrap()
        .set(Theme(11))
        .unwrap();
    tree.flush().unwrap();
    assert_eq!(outer_log.borrow().build, 2);
    assert_eq!(inner_log.borrow().build, 1);
    assert_eq!(
        tree.state::<ConsumerState>(id(&tree, "inner-consumer"))
            .unwrap()
            .seen,
        Some(20)
    );
    tree.inherited_handle::<Theme>(id(&tree, "inner"))
        .unwrap()
        .set(Theme(21))
        .unwrap();
    tree.flush().unwrap();
    assert_eq!(inner_log.borrow().build, 2);
    assert_eq!(outer_log.borrow().build, 2);
}
#[test]
fn equality_and_custom_predicate_suppress_notifications_but_reads_see_new_snapshots() {
    let ctx = egui::Context::default();
    let log = Rc::default();
    let context = Rc::default();
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited_with(
            Theme(1),
            |old, new| old.0 / 10 != new.0 / 10,
            consumer("consumer", true, false, &log, &context),
        ),
    )
    .unwrap();
    let provider = tree
        .inherited_handle::<Theme>(tree.root_id().unwrap())
        .unwrap();
    let old = context
        .borrow()
        .as_ref()
        .unwrap()
        .read::<Theme>()
        .unwrap()
        .unwrap();
    provider.set(Theme(2)).unwrap();
    assert!(!tree.flush().unwrap());
    assert_eq!(log.borrow().build, 1);
    assert_eq!(old.0, 1);
    assert_eq!(
        context
            .borrow()
            .as_ref()
            .unwrap()
            .read::<Theme>()
            .unwrap()
            .unwrap()
            .0,
        2
    );
    provider.set(Theme(10)).unwrap();
    tree.flush().unwrap();
    assert_eq!(log.borrow().build, 2);
    assert_eq!(log.borrow().dependencies, 2);
    provider.notify().unwrap();
    tree.flush().unwrap();
    assert_eq!(log.borrow().dependencies, 3);
}
#[test]
fn equality_provider_coalesces_updates_and_notifies_once_per_flush() {
    let ctx = egui::Context::default();
    let log = Rc::default();
    let context = Rc::default();
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited(Theme(1), consumer("consumer", true, false, &log, &context)),
    )
    .unwrap();
    let provider = tree
        .inherited_handle::<Theme>(tree.root_id().unwrap())
        .unwrap();
    provider.set(Theme(1)).unwrap();
    assert!(!tree.flush().unwrap());
    assert_eq!(log.borrow().dependencies, 1);
    provider.set(Theme(2)).unwrap();
    provider.set(Theme(3)).unwrap();
    provider.set(Theme(4)).unwrap();
    tree.flush().unwrap();
    assert_eq!(log.borrow().dependencies, 2);
    assert_eq!(log.borrow().build, 2);
    assert_eq!(
        tree.state::<ConsumerState>(id(&tree, "consumer"))
            .unwrap()
            .seen,
        Some(4)
    );
}
#[test]
fn previous_dependencies_remain_subscribed_when_a_later_build_stops_reading() {
    let ctx = egui::Context::default();
    let log = Rc::default();
    let context = Rc::default();
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited(Theme(1), consumer("consumer", true, false, &log, &context)),
    )
    .unwrap();
    let state = tree
        .state_handle::<ConsumerState>(id(&tree, "consumer"))
        .unwrap();
    state.update(|s| s.listen = false).unwrap();
    tree.flush().unwrap();
    tree.inherited_handle::<Theme>(tree.root_id().unwrap())
        .unwrap()
        .set(Theme(2))
        .unwrap();
    assert!(tree.flush().unwrap());
    assert_eq!(log.borrow().build, 3);
    assert_eq!(log.borrow().dependencies, 2);
}
#[test]
fn hook_only_dependencies_survive_unrelated_local_state_rebuilds() {
    let ctx = egui::Context::default();
    let log = Rc::default();
    let context = Rc::default();
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited(Theme(1), consumer("consumer", true, true, &log, &context)),
    )
    .unwrap();
    let consumer_id = id(&tree, "consumer");
    tree.state_handle::<ConsumerState>(consumer_id)
        .unwrap()
        .update(|s| s.local += 1)
        .unwrap();
    tree.flush().unwrap();
    assert_eq!(log.borrow().dependencies, 1);
    tree.inherited_handle::<Theme>(tree.root_id().unwrap())
        .unwrap()
        .set(Theme(5))
        .unwrap();
    tree.flush().unwrap();
    assert_eq!(
        tree.state::<ConsumerState>(consumer_id).unwrap().seen,
        Some(5)
    );
    assert_eq!(log.borrow().dependencies, 2);
    assert_eq!(log.borrow().build, 3);
}
#[test]
fn inherited_lookups_do_not_see_sibling_providers_or_descendant_providers() {
    struct Parent {
        seen: Rc<RefCell<Vec<Option<u32>>>>,
    }
    impl StatelessWidget for Parent {
        fn build(&self, ctx: &BuildContext) -> Widget {
            self.seen
                .borrow_mut()
                .push(ctx.depend_on::<Theme>().unwrap().map(|v| v.0));
            Widget::inherited(Theme(99), Widget::empty())
        }
    }
    let ctx = egui::Context::default();
    let log = Rc::default();
    let context = Rc::default();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let tree = ElementTree::new(
        &ctx,
        row(vec![
            Widget::stateless(Parent { seen: seen.clone() }),
            consumer("sibling", true, false, &log, &context),
        ]),
    )
    .unwrap();
    assert_eq!(*seen.borrow(), [None]);
    assert_eq!(
        tree.state::<ConsumerState>(id(&tree, "sibling"))
            .unwrap()
            .seen,
        None
    );
}
#[test]
fn declarative_provider_updates_use_new_predicate_and_keep_consumer_identity() {
    let ctx = egui::Context::default();
    let log = Rc::default();
    let context = Rc::default();
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited_with(
            Theme(1),
            |_, _| false,
            consumer("consumer", true, false, &log, &context),
        ),
    )
    .unwrap();
    let consumer_id = id(&tree, "consumer");
    tree.update(Widget::inherited_with(
        Theme(2),
        |_, _| true,
        consumer("consumer", true, false, &log, &context),
    ))
    .unwrap();
    assert_eq!(id(&tree, "consumer"), consumer_id);
    assert_eq!(log.borrow().dependencies, 2);
    assert_eq!(log.borrow().init, 1);
}
#[test]
fn inherited_handles_send_updates_to_ui_owned_non_send_values_and_request_repaint() {
    struct Local(Rc<u32>);
    struct Reader(Rc<RefCell<Vec<u32>>>);
    impl StatelessWidget for Reader {
        fn build(&self, ctx: &BuildContext) -> Widget {
            self.0
                .borrow_mut()
                .push(*ctx.depend_on::<Local>().unwrap().unwrap().0);
            Widget::empty()
        }
    }
    let observed = Rc::new(RefCell::new(Vec::new()));
    let ctx = egui::Context::default();
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited_with(
            Local(Rc::new(1)),
            |old, new| old.0 != new.0,
            Widget::stateless(Reader(observed.clone())),
        ),
    )
    .unwrap();
    let provider = tree
        .inherited_handle::<Local>(tree.root_id().unwrap())
        .unwrap();
    for _ in 0..4 {
        let _ = ctx.run(Default::default(), |_| {});
    }
    let wakes = Arc::new(AtomicUsize::new(0));
    let count = wakes.clone();
    ctx.set_request_repaint_callback(move |_| {
        count.fetch_add(1, Ordering::SeqCst);
    });
    let thread_provider = provider.clone();
    std::thread::spawn(move || {
        thread_provider
            .update(|old| Local(Rc::new(*old.0 + 1)))
            .unwrap()
    })
    .join()
    .unwrap();
    assert!(wakes.load(Ordering::SeqCst) > 0);
    tree.flush().unwrap();
    assert_eq!(*observed.borrow(), [1, 2]);
    tree.unmount();
    assert!(!provider.mounted());
    assert_eq!(provider.notify(), Err(Unmounted));
}
#[test]
fn contexts_release_scopes_on_unmount_and_stale_lookups_and_handles_fail() {
    let ctx = egui::Context::default();
    let log = Rc::default();
    let context = Rc::default();
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited(Theme(1), consumer("consumer", true, false, &log, &context)),
    )
    .unwrap();
    let handle = context
        .borrow()
        .as_ref()
        .unwrap()
        .inherited_handle::<Theme>()
        .unwrap()
        .unwrap();
    tree.unmount();
    assert!(!handle.mounted());
    assert_eq!(handle.set(Theme(3)), Err(Unmounted));
    assert_eq!(
        context
            .borrow()
            .as_ref()
            .unwrap()
            .read::<Theme>()
            .unwrap_err(),
        DependencyError::Unmounted
    );
    assert_eq!(log.borrow().dispose, 1);
}
#[test]
fn independent_trees_have_isolated_inherited_scopes() {
    let ctx = egui::Context::default();
    let a = Rc::default();
    let b = Rc::default();
    let ac = Rc::default();
    let bc = Rc::default();
    let mut first = ElementTree::new(
        &ctx,
        Widget::inherited(Theme(1), consumer("a", true, false, &a, &ac)),
    )
    .unwrap();
    let second = ElementTree::new(
        &ctx,
        Widget::inherited(Theme(2), consumer("b", true, false, &b, &bc)),
    )
    .unwrap();
    first
        .inherited_handle::<Theme>(first.root_id().unwrap())
        .unwrap()
        .set(Theme(3))
        .unwrap();
    first.flush().unwrap();
    assert_eq!(
        second
            .state::<ConsumerState>(id(&second, "b"))
            .unwrap()
            .seen,
        Some(2)
    );
    assert_eq!(b.borrow().build, 1);
}
#[test]
fn provider_changes_preserve_render_identity_and_geometry_constraints() {
    let ctx = egui::Context::default();
    let log = Rc::default();
    let context = Rc::default();
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited(Theme(1), consumer("consumer", true, false, &log, &context)),
    )
    .unwrap();
    let before = tree
        .layout(BoxConstraints::tight(egui::vec2(100.0, 40.0)).unwrap())
        .unwrap();
    tree.inherited_handle::<Theme>(tree.root_id().unwrap())
        .unwrap()
        .set(Theme(2))
        .unwrap();
    let after = tree
        .layout(BoxConstraints::tight(egui::vec2(100.0, 40.0)).unwrap())
        .unwrap();
    assert_eq!(before.id, after.id);
    assert_eq!(before.size, after.size);
}

#[test]
fn different_inherited_types_coalesce_the_dependency_hook_and_use_both_latest_values() {
    #[derive(PartialEq)]
    struct Locale(&'static str);
    struct Pair;
    struct PairState {
        seen: (u32, &'static str),
        hooks: usize,
        builds: usize,
    }
    impl StatefulWidget for Pair {
        type State = PairState;
        fn create_state(&self) -> PairState {
            PairState {
                seen: (0, ""),
                hooks: 0,
                builds: 0,
            }
        }
    }
    impl State<Pair> for PairState {
        fn did_change_dependencies(&mut self, _: &Pair, _: &StateContext<Self>) {
            self.hooks += 1;
        }
        fn build(&mut self, _: &Pair, ctx: &StateContext<Self>) -> Widget {
            self.builds += 1;
            self.seen = (
                ctx.depend_on::<Theme>().unwrap().unwrap().0,
                ctx.depend_on::<Locale>().unwrap().unwrap().0,
            );
            Widget::empty()
        }
    }
    let ctx = egui::Context::default();
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited(
            Theme(1),
            Widget::inherited(Locale("en"), Widget::stateful(Pair).keyed("pair")).keyed("locale"),
        )
        .keyed("theme"),
    )
    .unwrap();
    tree.inherited_handle::<Theme>(id(&tree, "theme"))
        .unwrap()
        .set(Theme(2))
        .unwrap();
    tree.inherited_handle::<Locale>(id(&tree, "locale"))
        .unwrap()
        .set(Locale("es"))
        .unwrap();
    tree.flush().unwrap();
    let state = tree.state::<PairState>(id(&tree, "pair")).unwrap();
    assert_eq!(state.seen, (2, "es"));
    assert_eq!((state.hooks, state.builds), (2, 2));
}
#[test]
fn dependency_hook_runs_after_init_and_before_build_then_only_for_notifications() {
    struct Ordered(Rc<RefCell<Vec<&'static str>>>);
    struct OrderedState(Rc<RefCell<Vec<&'static str>>>);
    impl StatefulWidget for Ordered {
        type State = OrderedState;
        fn create_state(&self) -> OrderedState {
            OrderedState(self.0.clone())
        }
    }
    impl State<Ordered> for OrderedState {
        fn init_state(&mut self, _: &Ordered, _: &StateContext<Self>) {
            self.0.borrow_mut().push("init");
        }
        fn did_change_dependencies(&mut self, _: &Ordered, ctx: &StateContext<Self>) {
            self.0.borrow_mut().push("dependencies");
            ctx.depend_on::<Theme>().unwrap();
        }
        fn build(&mut self, _: &Ordered, _: &StateContext<Self>) -> Widget {
            self.0.borrow_mut().push("build");
            Widget::empty()
        }
    }
    let ctx = egui::Context::default();
    let order: Rc<RefCell<Vec<&'static str>>> = Rc::default();
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited(
            Theme(1),
            Widget::stateful(Ordered(order.clone())).keyed("consumer"),
        ),
    )
    .unwrap();
    assert_eq!(*order.borrow(), ["init", "dependencies", "build"]);
    tree.inherited_handle::<Theme>(tree.root_id().unwrap())
        .unwrap()
        .set(Theme(2))
        .unwrap();
    tree.flush().unwrap();
    tree.state_handle::<OrderedState>(id(&tree, "consumer"))
        .unwrap()
        .request_rebuild()
        .unwrap();
    tree.flush().unwrap();
    assert_eq!(
        *order.borrow(),
        [
            "init",
            "dependencies",
            "build",
            "dependencies",
            "build",
            "build"
        ]
    );
}
#[test]
fn unmount_releases_provider_values_even_if_a_context_is_retained() {
    struct Tracked(Arc<AtomicUsize>);
    impl Drop for Tracked {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    struct Reader(Rc<RefCell<Option<BuildContext>>>);
    impl StatelessWidget for Reader {
        fn build(&self, ctx: &BuildContext) -> Widget {
            ctx.depend_on::<Tracked>().unwrap();
            *self.0.borrow_mut() = Some(ctx.clone());
            Widget::empty()
        }
    }
    let ctx = egui::Context::default();
    let dropped = Arc::new(AtomicUsize::new(0));
    let saved: Rc<RefCell<Option<BuildContext>>> = Rc::default();
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited_with(
            Tracked(dropped.clone()),
            |_, _| true,
            Widget::stateless(Reader(saved.clone())),
        ),
    )
    .unwrap();
    tree.unmount();
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
    assert!(matches!(
        saved.borrow().as_ref().unwrap().read::<Tracked>(),
        Err(DependencyError::Unmounted)
    ));
}

#[test]
fn measurement_paint_and_idle_event_lookups_register_the_owning_component() {
    #[derive(PartialEq)]
    struct Accent(u32);
    #[derive(PartialEq)]
    struct EventValue(u32);
    struct Reader {
        builds: Rc<RefCell<usize>>,
        saved: Rc<RefCell<Option<BuildContext>>>,
    }
    impl StatelessWidget for Reader {
        fn build(&self, ctx: &BuildContext) -> Widget {
            *self.builds.borrow_mut() += 1;
            *self.saved.borrow_mut() = Some(ctx.clone());
            let measure = ctx.clone();
            let paint = ctx.clone();
            Widget::measured(
                move |constraints| {
                    assert!(measure.depend_on::<Theme>().unwrap().is_some());
                    constraints.constrain(egui::vec2(40.0, 20.0))
                },
                move |ui| {
                    let color = paint.depend_on::<Accent>().unwrap().unwrap();
                    ui.label(color.0.to_string());
                },
            )
        }
    }
    let ctx = egui::Context::default();
    let builds: Rc<RefCell<usize>> = Rc::default();
    let saved: Rc<RefCell<Option<BuildContext>>> = Rc::default();
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited(
            Theme(1),
            Widget::inherited(
                Accent(2),
                Widget::inherited(
                    EventValue(3),
                    Widget::stateless(Reader {
                        builds: builds.clone(),
                        saved: saved.clone(),
                    }),
                )
                .keyed("event"),
            )
            .keyed("accent"),
        )
        .keyed("theme"),
    )
    .unwrap();
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            tree.show(ui, BoxConstraints::tight(egui::vec2(100.0, 40.0)).unwrap())
                .unwrap();
        });
    });
    assert!(
        saved
            .borrow()
            .as_ref()
            .unwrap()
            .depend_on::<EventValue>()
            .unwrap()
            .is_some()
    );
    for key in ["theme", "accent", "event"] {
        match key {
            "theme" => tree
                .inherited_handle::<Theme>(id(&tree, key))
                .unwrap()
                .set(Theme(4))
                .unwrap(),
            "accent" => tree
                .inherited_handle::<Accent>(id(&tree, key))
                .unwrap()
                .set(Accent(5))
                .unwrap(),
            _ => tree
                .inherited_handle::<EventValue>(id(&tree, key))
                .unwrap()
                .set(EventValue(6))
                .unwrap(),
        }
        assert!(tree.flush().unwrap());
    }
    assert_eq!(*builds.borrow(), 4);
}

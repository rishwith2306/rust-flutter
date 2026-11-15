use rust_flutter::{egui, element::*, layout::Axis, notifier::*};
use std::{cell::Cell, rc::Rc};
type Model = InheritedNotifier<ValueNotifier<i32>>;
struct Consumer {
    listen: bool,
}
#[derive(Default)]
struct ConsumerState {
    builds: usize,
    hooks: usize,
    local: usize,
    seen: Option<i32>,
    context: Option<BuildContext>,
}
impl StatefulWidget for Consumer {
    type State = ConsumerState;
    fn create_state(&self) -> Self::State {
        ConsumerState::default()
    }
}
impl State<Consumer> for ConsumerState {
    fn init_state(&mut self, _: &Consumer, ctx: &StateContext<Self>) {
        self.context = Some((**ctx).clone());
    }
    fn did_change_dependencies(&mut self, _: &Consumer, _: &StateContext<Self>) {
        self.hooks += 1;
    }
    fn build(&mut self, widget: &Consumer, ctx: &StateContext<Self>) -> Widget {
        self.builds += 1;
        let model = if widget.listen {
            ctx.depend_on::<Model>()
        } else {
            ctx.read::<Model>()
        }
        .unwrap();
        self.seen = model.and_then(|model| model.notifier.as_ref().map(ValueNotifier::get));
        Widget::empty()
    }
}
fn consumer(key: &str, listen: bool) -> Widget {
    Widget::stateful(Consumer { listen }).keyed(key)
}
fn row(children: Vec<Widget>) -> Widget {
    Widget::flex(
        FlexStyle::new(Axis::Horizontal),
        children.into_iter().map(FlexItem::fixed).collect(),
    )
}
fn id(tree: &ElementTree, key: &str) -> ElementId {
    tree.elements()
        .into_iter()
        .find(|e| e.key == Some(Key::from(key)))
        .unwrap()
        .id
}
fn state<'a>(tree: &'a ElementTree, key: &str) -> &'a ConsumerState {
    tree.state(id(tree, key)).unwrap()
}
fn source(tree: &ElementTree, key: &str) -> ValueNotifier<i32> {
    state(tree, key)
        .context
        .as_ref()
        .unwrap()
        .read::<Model>()
        .unwrap()
        .unwrap()
        .notifier
        .clone()
        .unwrap()
}

#[test]
fn source_events_coalesce_hooks_and_preserve_state_without_rebuilding_nonlisteners() {
    let ctx = egui::Context::default();
    let notifier = ValueNotifier::new(1);
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited_notifier(
            Some(notifier.clone()),
            row(vec![consumer("listen", true), consumer("read", false)]),
        )
        .keyed("provider"),
    )
    .unwrap();
    assert!(notifier.has_listeners());
    let listener = id(&tree, "listen");
    tree.state_handle::<ConsumerState>(listener)
        .unwrap()
        .update(|state| state.local = 99)
        .unwrap();
    tree.flush().unwrap();
    for _ in 0..4 {
        let _ = ctx.run(Default::default(), |_| {});
    }
    let wakes = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    ctx.set_request_repaint_callback({
        let wakes = wakes.clone();
        move |_| {
            wakes.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    });
    notifier.set(2).unwrap();
    notifier.set(3).unwrap();
    notifier.set(4).unwrap();
    assert!(wakes.load(std::sync::atomic::Ordering::SeqCst) > 0);
    assert_eq!(state(&tree, "listen").seen, Some(1));
    tree.flush().unwrap();
    assert_eq!(
        (state(&tree, "listen").builds, state(&tree, "listen").hooks),
        (3, 2)
    );
    assert_eq!(state(&tree, "listen").seen, Some(4));
    assert_eq!(state(&tree, "listen").local, 99);
    assert_eq!(state(&tree, "read").builds, 1);
    assert_eq!(state(&tree, "read").seen, Some(1));
    assert_eq!(id(&tree, "listen"), listener);
    tree.unmount();
    assert!(!notifier.has_listeners());
}

#[test]
fn replacing_and_removing_sources_detaches_old_listeners_and_notifies_identity_changes() {
    let ctx = egui::Context::default();
    let old = ValueNotifier::new(1);
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited_notifier(Some(old.clone()), consumer("listen", true)).keyed("provider"),
    )
    .unwrap();
    let handle = tree
        .inherited_handle::<Model>(id(&tree, "provider"))
        .unwrap();
    // Non-Send notifiers are constructed on the UI thread by a sendable command.
    handle
        .update(|_| Model {
            notifier: Some(ValueNotifier::new(20)),
        })
        .unwrap();
    tree.flush().unwrap();
    let new = source(&tree, "listen");
    assert!(!old.has_listeners());
    assert!(new.has_listeners());
    assert_eq!(
        (state(&tree, "listen").seen, state(&tree, "listen").hooks),
        (Some(20), 2)
    );
    old.set(2).unwrap();
    tree.flush().unwrap();
    assert_eq!(state(&tree, "listen").hooks, 2);
    new.set(30).unwrap();
    tree.flush().unwrap();
    assert_eq!(state(&tree, "listen").seen, Some(30));
    handle.update(|value| value.clone()).unwrap();
    tree.flush().unwrap();
    assert_eq!(state(&tree, "listen").hooks, 3);
    handle.update(|_| Model { notifier: None }).unwrap();
    tree.flush().unwrap();
    assert!(!new.has_listeners());
    assert_eq!(
        (state(&tree, "listen").seen, state(&tree, "listen").hooks),
        (None, 4)
    );
}

#[derive(Clone)]
struct Spy {
    changes: ChangeNotifier,
    added: Rc<Cell<usize>>,
    removed: Rc<Cell<usize>>,
}
impl Spy {
    fn new() -> Self {
        Self {
            changes: ChangeNotifier::new(),
            added: Rc::default(),
            removed: Rc::default(),
        }
    }
}
impl Listenable for Spy {
    fn subscribe(&self, listener: Listener) -> Result<Subscription, Disposed> {
        let subscription = self.changes.subscribe(listener)?;
        self.added.set(self.added.get() + 1);
        let removed = self.removed.clone();
        Ok(Subscription::new(move || {
            drop(subscription);
            removed.set(removed.get() + 1);
        }))
    }
    fn same_source(&self, other: &Self) -> bool {
        self.changes.same_source(&other.changes)
    }
}
struct SpyConsumer {
    hooks: Rc<Cell<usize>>,
}
struct SpyState;
impl StatefulWidget for SpyConsumer {
    type State = SpyState;
    fn create_state(&self) -> SpyState {
        SpyState
    }
}
impl State<SpyConsumer> for SpyState {
    fn did_change_dependencies(&mut self, widget: &SpyConsumer, _: &StateContext<Self>) {
        widget.hooks.set(widget.hooks.get() + 1);
    }
    fn build(&mut self, _: &SpyConsumer, ctx: &StateContext<Self>) -> Widget {
        ctx.depend_on::<InheritedNotifier<Spy>>().unwrap();
        Widget::empty()
    }
}
#[test]
fn custom_identity_gate_does_not_suppress_source_events_or_listener_rebinding() {
    let ctx = egui::Context::default();
    let a = Spy::new();
    let b = Spy::new();
    let hooks = Rc::new(Cell::new(0));
    let root = |source: Spy| {
        Widget::inherited_notifier_with(
            Some(source),
            |_, _| false,
            Widget::stateful(SpyConsumer {
                hooks: hooks.clone(),
            }),
        )
    };
    let mut tree = ElementTree::new(&ctx, root(a.clone())).unwrap();
    tree.update(root(a.clone())).unwrap();
    assert_eq!((a.added.get(), a.removed.get()), (1, 0));
    assert_eq!(hooks.get(), 1);
    tree.update(root(b.clone())).unwrap();
    assert_eq!((a.added.get(), a.removed.get()), (1, 1));
    assert_eq!(b.added.get(), 1);
    assert_eq!(hooks.get(), 1);
    a.changes.notify_listeners().unwrap();
    tree.flush().unwrap();
    assert_eq!(hooks.get(), 1);
    b.changes.notify_listeners().unwrap();
    tree.flush().unwrap();
    assert_eq!(hooks.get(), 2);
    tree.unmount();
    assert_eq!(b.removed.get(), 1);
}

#[test]
fn disposed_source_mount_and_queued_replacement_fail_cleanly() {
    let ctx = egui::Context::default();
    let dead = ValueNotifier::new(1);
    dead.dispose().unwrap();
    assert!(matches!(
        ElementTree::new(
            &ctx,
            Widget::inherited_notifier(Some(dead), Widget::empty())
        ),
        Err(ElementError::DisposedNotifier)
    ));
    let live = ValueNotifier::new(2);
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited_notifier(Some(live.clone()), consumer("listen", true)).keyed("provider"),
    )
    .unwrap();
    let context = state(&tree, "listen").context.clone().unwrap();
    let handle = tree
        .inherited_handle::<Model>(id(&tree, "provider"))
        .unwrap();
    handle
        .update(|_| {
            let dead = ValueNotifier::new(3);
            dead.dispose().unwrap();
            Model {
                notifier: Some(dead),
            }
        })
        .unwrap();
    assert_eq!(tree.flush(), Err(ElementError::DisposedNotifier));
    assert!(tree.root_id().is_none());
    assert!(!context.mounted());
    assert!(!live.has_listeners());
    assert!(!handle.mounted());
}

#[test]
fn nested_notifiers_shadow_outer_events_and_release_sources_on_drop() {
    let ctx = egui::Context::default();
    let outer = ValueNotifier::new(1);
    let inner = ValueNotifier::new(10);
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited_notifier(
            Some(outer.clone()),
            row(vec![
                consumer("outer", true),
                Widget::inherited_notifier(Some(inner.clone()), consumer("inner", true)),
            ]),
        ),
    )
    .unwrap();
    outer.set(2).unwrap();
    tree.flush().unwrap();
    assert_eq!(state(&tree, "outer").seen, Some(2));
    assert_eq!(state(&tree, "inner").hooks, 1);
    inner.set(11).unwrap();
    tree.flush().unwrap();
    assert_eq!(state(&tree, "inner").seen, Some(11));
    let context = state(&tree, "inner").context.clone().unwrap();
    drop(tree);
    assert!(!outer.has_listeners());
    assert!(!inner.has_listeners());
    assert!(!context.mounted());
    inner.set(12).unwrap();
    assert!(matches!(
        context.read::<Model>(),
        Err(DependencyError::Unmounted)
    ));
}

#[test]
fn enabling_a_watch_binds_the_incoming_value_instead_of_the_old_disposed_source() {
    let ctx = egui::Context::default();
    let old = ValueNotifier::new(1);
    old.dispose().unwrap();
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited_with(
            Model {
                notifier: Some(old),
            },
            |_, _| true,
            consumer("listen", true),
        ),
    )
    .unwrap();
    let listener = id(&tree, "listen");
    let live = ValueNotifier::new(2);
    tree.update(Widget::inherited_notifier(
        Some(live.clone()),
        consumer("listen", true),
    ))
    .unwrap();
    assert!(live.has_listeners());
    assert_eq!(id(&tree, "listen"), listener);
    live.set(3).unwrap();
    tree.flush().unwrap();
    assert_eq!(state(&tree, "listen").seen, Some(3));
}

#[test]
fn pending_old_events_and_source_replacement_coalesce_against_the_latest_source() {
    let ctx = egui::Context::default();
    let old = ValueNotifier::new(1);
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited_notifier(Some(old.clone()), consumer("listen", true)).keyed("provider"),
    )
    .unwrap();
    old.set(2).unwrap();
    tree.inherited_handle::<Model>(id(&tree, "provider"))
        .unwrap()
        .update(|_| Model {
            notifier: Some(ValueNotifier::new(3)),
        })
        .unwrap();
    tree.flush().unwrap();
    assert_eq!(
        (state(&tree, "listen").seen, state(&tree, "listen").hooks),
        (Some(3), 2)
    );
    assert!(!old.has_listeners());
}

#[test]
fn value_builder_uses_the_latest_source_and_cancels_replaced_and_unmounted_sources() {
    use std::cell::RefCell;
    fn root(source: ValueNotifier<i32>, values: Rc<RefCell<Vec<i32>>>) -> Widget {
        Widget::value_listenable_builder(source, move |_, value| {
            values.borrow_mut().push(value);
            Widget::empty()
        })
    }
    let ctx = egui::Context::default();
    let old = ValueNotifier::new(1);
    let values = Rc::default();
    let mut tree = ElementTree::new(&ctx, root(old.clone(), Rc::clone(&values))).unwrap();
    old.set(2).unwrap();
    old.set(3).unwrap();
    tree.flush().unwrap();
    assert_eq!(*values.borrow(), [1, 3]);
    let new = ValueNotifier::new(4);
    tree.update(root(new.clone(), values.clone())).unwrap();
    assert!(!old.has_listeners());
    assert!(new.has_listeners());
    assert_eq!(*values.borrow(), [1, 3, 4]);
    old.set(5).unwrap();
    tree.flush().unwrap();
    assert_eq!(values.borrow().len(), 3);
    new.set(6).unwrap();
    tree.flush().unwrap();
    assert_eq!(*values.borrow(), [1, 3, 4, 6]);
    tree.unmount();
    assert!(!new.has_listeners());
}

#[test]
fn listenable_builder_observes_model_notifications_without_a_value_trait() {
    let ctx = egui::Context::default();
    let source = ChangeNotifier::new();
    let data = Rc::new(Cell::new(1));
    let seen = Rc::new(Cell::new(0));
    let root = Widget::listenable_builder(source.clone(), {
        let data = data.clone();
        let seen = seen.clone();
        move |_, _| {
            seen.set(data.get());
            Widget::empty()
        }
    });
    let mut tree = ElementTree::new(&ctx, root).unwrap();
    assert_eq!(seen.get(), 1);
    data.set(2);
    source.notify_listeners().unwrap();
    tree.flush().unwrap();
    assert_eq!(seen.get(), 2);
    drop(tree);
    assert!(!source.has_listeners());
}

#[test]
fn value_builders_accept_shared_type_erased_value_listenables() {
    let ctx = egui::Context::default();
    let source = ValueNotifier::new(1);
    let erased: Rc<dyn ValueListenable<Value = i32>> = Rc::new(source.clone());
    let seen = Rc::new(Cell::new(0));
    let root = Widget::value_listenable_builder(erased, {
        let seen = seen.clone();
        move |_, value| {
            seen.set(value);
            Widget::empty()
        }
    });
    let mut tree = ElementTree::new(&ctx, root).unwrap();
    assert_eq!(seen.get(), 1);
    source.set(2).unwrap();
    tree.flush().unwrap();
    assert_eq!(seen.get(), 2);
    drop(tree);
    assert!(!source.has_listeners());
}

#[test]
fn clearing_a_builders_optional_provider_temporarily_removes_its_output() {
    use std::cell::RefCell;
    let ctx = egui::Context::default();
    let values = Rc::new(RefCell::new(Vec::new()));
    let old = ValueNotifier::new(1);
    let root = Widget::value_listenable_builder(old.clone(), {
        let values = values.clone();
        move |_, value| {
            values.borrow_mut().push(value);
            Widget::empty()
        }
    });
    let mut tree = ElementTree::new(&ctx, root).unwrap();
    let provider = tree
        .inherited_handle::<Model>(tree.root_id().unwrap())
        .unwrap();
    provider.update(|_| Model { notifier: None }).unwrap();
    tree.flush().unwrap();
    assert!(!old.has_listeners());
    assert_eq!(*values.borrow(), [1]);
    provider
        .update(|_| Model {
            notifier: Some(ValueNotifier::new(3)),
        })
        .unwrap();
    tree.flush().unwrap();
    assert_eq!(*values.borrow(), [1, 3]);
}

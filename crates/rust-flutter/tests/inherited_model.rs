use rust_flutter::{egui, element::*, layout::Axis};
use std::{cell::RefCell, collections::HashSet, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Aspect {
    A,
    B,
}
#[derive(Clone, Debug)]
struct Model {
    a: i32,
    b: i32,
    supports: HashSet<Aspect>,
    notify: bool,
    observed: Rc<RefCell<Vec<HashSet<Aspect>>>>,
}
impl Model {
    fn new(a: i32, b: i32) -> Self {
        Self {
            a,
            b,
            supports: HashSet::from([Aspect::A, Aspect::B]),
            notify: true,
            observed: Rc::default(),
        }
    }
}
impl InheritedModel for Model {
    type Aspect = Aspect;
    fn update_should_notify(&self, old: &Self) -> bool {
        self.notify && (self.a != old.a || self.b != old.b || self.supports != old.supports)
    }
    fn update_should_notify_dependent(&self, old: &Self, aspects: &HashSet<Aspect>) -> bool {
        self.observed.borrow_mut().push(aspects.clone());
        aspects.iter().any(|aspect| {
            self.supports.contains(aspect) != old.supports.contains(aspect)
                || match aspect {
                    Aspect::A => self.a != old.a,
                    Aspect::B => self.b != old.b,
                }
        })
    }
    fn is_supported_aspect(&self, aspect: &Aspect) -> bool {
        self.supports.contains(aspect)
    }
}
struct Observer {
    aspects: Vec<Aspect>,
    all: bool,
    listen: bool,
    hook: bool,
}
#[derive(Default)]
struct ObserverState {
    builds: usize,
    hooks: usize,
    seen: Vec<(i32, i32)>,
    local: usize,
    context: Option<BuildContext>,
}
impl StatefulWidget for Observer {
    type State = ObserverState;
    fn create_state(&self) -> Self::State {
        ObserverState::default()
    }
}
impl ObserverState {
    fn observe(&mut self, widget: &Observer, ctx: &BuildContext) {
        self.seen.clear();
        for aspect in &widget.aspects {
            if let Some(value) = ctx.depend_on_aspect::<Model>(*aspect).unwrap() {
                self.seen.push((value.a, value.b));
            }
        }
        if widget.all
            && let Some(value) = ctx.depend_on::<Model>().unwrap()
        {
            self.seen.push((value.a, value.b));
        }
        if !widget.listen
            && let Some(value) = ctx.read::<Model>().unwrap()
        {
            self.seen.push((value.a, value.b));
        }
    }
}
impl State<Observer> for ObserverState {
    fn init_state(&mut self, _: &Observer, ctx: &StateContext<Self>) {
        assert_eq!(
            ctx.depend_on_aspect::<Model>(Aspect::A).unwrap_err(),
            DependencyError::InvalidPhase
        );
        self.context = Some((**ctx).clone());
    }
    fn did_change_dependencies(&mut self, widget: &Observer, ctx: &StateContext<Self>) {
        self.hooks += 1;
        if widget.hook {
            self.observe(widget, ctx);
        }
    }
    fn build(&mut self, widget: &Observer, ctx: &StateContext<Self>) -> Widget {
        self.builds += 1;
        if !widget.hook {
            self.observe(widget, ctx);
        }
        Widget::empty()
    }
}
fn observer(name: &str, aspects: &[Aspect], all: bool) -> Widget {
    Widget::stateful(Observer {
        aspects: aspects.to_vec(),
        all,
        listen: true,
        hook: false,
    })
    .keyed(name)
}
fn row(children: Vec<Widget>) -> Widget {
    Widget::flex(
        FlexStyle::new(Axis::Horizontal),
        children.into_iter().map(FlexItem::fixed).collect(),
    )
}
fn id(tree: &ElementTree, name: &str) -> ElementId {
    tree.elements()
        .into_iter()
        .find(|e| e.key == Some(Key::from(name)))
        .unwrap()
        .id
}
fn state<'a>(tree: &'a ElementTree, name: &str) -> &'a ObserverState {
    tree.state(id(tree, name)).unwrap()
}
fn handle(tree: &ElementTree, name: &str) -> InheritedHandle<Model> {
    tree.inherited_handle(id(tree, name)).unwrap()
}
fn set_a(handle: &InheritedHandle<Model>, a: i32) {
    handle
        .update(move |old| Model { a, ..old.clone() })
        .unwrap();
}
fn set_b(handle: &InheritedHandle<Model>, b: i32) {
    handle
        .update(move |old| Model { b, ..old.clone() })
        .unwrap();
}

#[test]
fn aspects_selectively_rebuild_state_and_hooks_without_resetting_local_state() {
    let ctx = egui::Context::default();
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited_model(
            Model::new(1, 2),
            row(vec![
                observer("a", &[Aspect::A], false),
                observer("b", &[Aspect::B], false),
                observer("all", &[], true),
                Widget::stateful(Observer {
                    aspects: vec![],
                    all: false,
                    listen: false,
                    hook: false,
                })
                .keyed("read"),
            ]),
        )
        .keyed("model"),
    )
    .unwrap();
    let a_id = id(&tree, "a");
    tree.state_handle::<ObserverState>(a_id)
        .unwrap()
        .update(|s| s.local = 99)
        .unwrap();
    tree.flush().unwrap();
    let provider = handle(&tree, "model");
    set_a(&provider, 3);
    tree.flush().unwrap();
    assert_eq!((state(&tree, "a").builds, state(&tree, "a").hooks), (3, 2));
    assert_eq!(state(&tree, "a").local, 99);
    assert_eq!(state(&tree, "a").seen, [(3, 2)]);
    assert_eq!((state(&tree, "b").builds, state(&tree, "b").hooks), (1, 1));
    assert_eq!(state(&tree, "all").builds, 2);
    assert_eq!(state(&tree, "read").builds, 1);
    set_b(&provider, 4);
    tree.flush().unwrap();
    assert_eq!(state(&tree, "a").builds, 3);
    assert_eq!(state(&tree, "b").seen, [(3, 4)]);
    assert_eq!(state(&tree, "all").builds, 3);
    assert_eq!(id(&tree, "a"), a_id);
}

#[test]
fn repeated_aspects_form_a_set_and_unconditional_subscription_dominates() {
    let model = Model::new(1, 2);
    let calls = model.observed.clone();
    let ctx = egui::Context::default();
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited_model(
            model,
            row(vec![
                observer("union", &[Aspect::A, Aspect::A, Aspect::B], false),
                observer("all", &[Aspect::A], true),
            ]),
        )
        .keyed("model"),
    )
    .unwrap();
    // Also test subscribing to all first, then adding aspects from an idle event.
    let all = state(&tree, "all").context.clone().unwrap();
    all.depend_on_aspect::<Model>(Aspect::B).unwrap();
    set_b(&handle(&tree, "model"), 7);
    tree.flush().unwrap();
    assert_eq!(state(&tree, "union").builds, 2);
    assert_eq!(state(&tree, "all").builds, 2);
    assert_eq!(*calls.borrow(), [HashSet::from([Aspect::A, Aspect::B])]);
}

#[test]
fn global_gate_suppresses_aspect_predicates_and_force_notify_reaches_every_listener() {
    let model = Model::new(1, 2);
    let calls = model.observed.clone();
    let ctx = egui::Context::default();
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited_model(model, observer("a", &[Aspect::A], false)).keyed("model"),
    )
    .unwrap();
    let provider = handle(&tree, "model");
    provider
        .update(|old| Model {
            a: 8,
            notify: false,
            ..old.clone()
        })
        .unwrap();
    tree.flush().unwrap();
    assert_eq!(state(&tree, "a").builds, 1);
    assert!(calls.borrow().is_empty());
    let context = state(&tree, "a").context.clone().unwrap();
    assert_eq!(context.read::<Model>().unwrap().unwrap().a, 8);
    provider.notify().unwrap();
    tree.flush().unwrap();
    assert_eq!(state(&tree, "a").seen, [(8, 2)]);
    assert_eq!(state(&tree, "a").hooks, 2);
    assert!(calls.borrow().is_empty());
}

#[test]
fn unsupported_aspects_subscribe_through_nested_models_and_stop_at_supporting_ancestor() {
    let mut inner = Model::new(10, 20);
    inner.supports = HashSet::from([Aspect::B]);
    let mut middle = Model::new(100, 200);
    middle.supports.clear();
    let ctx = egui::Context::default();
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited_model(
            Model::new(1, 2),
            Widget::inherited_model(
                middle,
                Widget::inherited_model(
                    inner,
                    row(vec![
                        observer("a", &[Aspect::A], false),
                        observer("b", &[Aspect::B], false),
                        observer("all", &[], true),
                    ]),
                )
                .keyed("inner"),
            )
            .keyed("middle"),
        )
        .keyed("outer"),
    )
    .unwrap();
    assert_eq!(state(&tree, "a").seen, [(1, 2)]);
    assert_eq!(state(&tree, "b").seen, [(10, 20)]);
    assert_eq!(state(&tree, "all").seen, [(10, 20)]);
    set_a(&handle(&tree, "outer"), 3);
    tree.flush().unwrap();
    assert_eq!(state(&tree, "a").seen, [(3, 2)]);
    assert_eq!(state(&tree, "b").builds, 1);
    assert_eq!(state(&tree, "all").builds, 1);
    set_a(&handle(&tree, "middle"), 101);
    tree.flush().unwrap();
    assert_eq!(
        state(&tree, "a").builds,
        3,
        "unsupported intermediate model is still a dependency"
    );
    handle(&tree, "inner")
        .update(|old| Model {
            supports: HashSet::from([Aspect::A, Aspect::B]),
            ..old.clone()
        })
        .unwrap();
    tree.flush().unwrap();
    assert_eq!(state(&tree, "a").seen, [(10, 20)]);
    set_a(&handle(&tree, "outer"), 4);
    tree.flush().unwrap();
    assert_eq!(
        state(&tree, "a").builds,
        5,
        "registered ancestor dependencies persist until unmount"
    );
    assert_eq!(state(&tree, "a").seen, [(10, 20)]);
}

#[test]
fn no_support_returns_outermost_model_and_no_model_returns_none() {
    let mut outer = Model::new(1, 2);
    outer.supports.clear();
    let mut inner = Model::new(3, 4);
    inner.supports.clear();
    let ctx = egui::Context::default();
    let tree = ElementTree::new(
        &ctx,
        Widget::inherited_model(
            outer,
            Widget::inherited_model(inner, observer("a", &[Aspect::A], false)),
        ),
    )
    .unwrap();
    assert_eq!(state(&tree, "a").seen, [(1, 2)]);
    let empty = ElementTree::new(&ctx, observer("a", &[Aspect::A], false)).unwrap();
    assert!(state(&empty, "a").seen.is_empty());
}

#[test]
fn queued_transitions_coalesce_hooks_and_preserve_each_relevant_change() {
    let ctx = egui::Context::default();
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited_model(
            Model::new(1, 2),
            row(vec![
                observer("a", &[Aspect::A], false),
                observer("b", &[Aspect::B], false),
            ]),
        )
        .keyed("model"),
    )
    .unwrap();
    let provider = handle(&tree, "model");
    std::thread::spawn(move || {
        set_a(&provider, 9);
        set_b(&provider, 8);
        set_a(&provider, 1);
    })
    .join()
    .unwrap();
    tree.flush().unwrap();
    for name in ["a", "b"] {
        assert_eq!(
            (state(&tree, name).builds, state(&tree, name).hooks),
            (2, 2)
        );
        assert_eq!(state(&tree, name).seen, [(1, 8)]);
    }
}

#[test]
fn hook_only_subscriptions_survive_local_builds_and_stale_contexts_fail() {
    let ctx = egui::Context::default();
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited_model(
            Model::new(1, 2),
            Widget::stateful(Observer {
                aspects: vec![Aspect::A],
                all: false,
                listen: true,
                hook: true,
            })
            .keyed("a"),
        )
        .keyed("model"),
    )
    .unwrap();
    let provider = handle(&tree, "model");
    let context = state(&tree, "a").context.clone().unwrap();
    context.request_rebuild().unwrap();
    tree.flush().unwrap();
    set_a(&provider, 5);
    tree.flush().unwrap();
    assert_eq!(state(&tree, "a").seen, [(5, 2)]);
    assert_eq!(state(&tree, "a").hooks, 2);
    tree.unmount();
    assert!(matches!(
        context.depend_on_aspect::<Model>(Aspect::A),
        Err(DependencyError::Unmounted)
    ));
    assert!(!provider.mounted());
}

#[test]
fn ordinary_provider_keeps_unconditional_semantics_even_with_aspect_lookups() {
    let ctx = egui::Context::default();
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited_with(
            Model::new(1, 2),
            |_, _| true,
            observer("a", &[Aspect::A], false),
        )
        .keyed("model"),
    )
    .unwrap();
    set_b(&handle(&tree, "model"), 9);
    tree.flush().unwrap();
    assert_eq!(state(&tree, "a").seen, [(1, 9)]);
}

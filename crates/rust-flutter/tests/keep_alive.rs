use rust_flutter::{
    egui::{self, vec2},
    element::*,
    gestures::GestureBinding,
    layout::*,
    scrolling::*,
};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};
#[derive(Default)]
struct Data {
    ids: RefCell<Vec<usize>>,
    wanted: RefCell<HashSet<usize>>,
    request_counts: RefCell<HashMap<usize, usize>>,
    requests: RefCell<HashMap<usize, Vec<KeepAliveHandle>>>,
    handles: RefCell<HashMap<usize, StateHandle<RowState>>>,
    values: RefCell<HashMap<usize, (usize, ElementId)>>,
    disposed: RefCell<Vec<usize>>,
    builds: RefCell<Vec<usize>>,
}
struct Row {
    id: usize,
    data: Rc<Data>,
}
struct RowState {
    id: usize,
    data: Rc<Data>,
    value: usize,
    requested: bool,
}
impl StatefulWidget for Row {
    type State = RowState;
    fn create_state(&self) -> Self::State {
        RowState {
            id: self.id,
            data: self.data.clone(),
            value: 0,
            requested: false,
        }
    }
}
impl State<Row> for RowState {
    fn init_state(&mut self, _: &Row, context: &StateContext<Self>) {
        self.data
            .handles
            .borrow_mut()
            .insert(self.id, context.handle());
    }
    fn build(&mut self, _: &Row, context: &StateContext<Self>) -> Widget {
        if !self.requested {
            self.requested = true;
            let count = self
                .data
                .request_counts
                .borrow()
                .get(&self.id)
                .copied()
                .unwrap_or(0);
            for _ in 0..count {
                if let Some(handle) = KeepAliveHandle::request(context).unwrap() {
                    self.data
                        .requests
                        .borrow_mut()
                        .entry(self.id)
                        .or_default()
                        .push(handle);
                }
            }
        }
        self.data
            .values
            .borrow_mut()
            .insert(self.id, (self.value, context.element_id()));
        Widget::leaf(vec2(40.0, 20.0), |_| {})
    }
    fn dispose(&mut self) {
        self.data.disposed.borrow_mut().push(self.id);
        self.data.values.borrow_mut().remove(&self.id);
    }
}
fn config(
    p: &ScrollPosition,
    b: &GestureBinding,
    data: &Rc<Data>,
    automatic: bool,
    map: bool,
) -> Widget {
    let d = data.clone();
    let mut list = ListView::builder(p, b, data.ids.borrow().len(), 20.0, move |_, index| {
        let id = d.ids.borrow()[index];
        d.builds.borrow_mut().push(id);
        Widget::keep_alive(
            d.wanted.borrow().contains(&id),
            Widget::stateful(Row {
                id,
                data: d.clone(),
            }),
        )
    })
    .unwrap()
    .cache_extent(0.0)
    .unwrap()
    .add_automatic_keep_alives(automatic);
    let d = data.clone();
    list = list.item_key(move |i| Key::U64(d.ids.borrow()[i] as u64));
    if map {
        let d = data.clone();
        list = list.find_child_index_callback(move |key| {
            d.ids
                .borrow()
                .iter()
                .position(|id| Key::U64(*id as u64) == *key)
        });
    }
    list.into_widget().unwrap()
}
fn setup(
    data: &Rc<Data>,
    automatic: bool,
    map: bool,
) -> (ScrollPosition, GestureBinding, ElementTree) {
    let ctx = egui::Context::default();
    let p = ScrollPosition::new(&ctx, 0.0).unwrap();
    let b = GestureBinding::new();
    let t = ElementTree::new(&ctx, config(&p, &b, data, automatic, map)).unwrap();
    (p, b, t)
}
fn settle(t: &mut ElementTree) -> LayoutBox {
    let c = BoxConstraints::tight(vec2(80.0, 40.0)).unwrap();
    for _ in 0..3 {
        t.layout(c).unwrap();
    }
    t.layout(c).unwrap()
}
fn data() -> Rc<Data> {
    Rc::new(Data {
        ids: RefCell::new((0..100).collect()),
        ..Default::default()
    })
}
#[test]
fn kept_item_preserves_actual_state_handles_and_identity_without_rebuilding_parked_description() {
    let d = data();
    d.wanted.borrow_mut().insert(0);
    let (p, _, mut t) = setup(&d, true, false);
    settle(&mut t);
    d.handles.borrow()[&0].update(|s| s.value = 7).unwrap();
    settle(&mut t);
    let before = d.values.borrow()[&0];
    d.builds.borrow_mut().clear();
    p.jump_to(200.0).unwrap();
    let geometry = settle(&mut t);
    assert_eq!(d.values.borrow()[&0], before);
    assert!(d.handles.borrow()[&0].mounted());
    assert!(
        !d.builds.borrow().contains(&0),
        "scrolling rebuilt a parked item description"
    );
    assert!(d.disposed.borrow().contains(&1));
    fn hidden(g: &LayoutBox) -> usize {
        usize::from(g.is_offstage()) + g.children.iter().map(|c| hidden(&c.layout)).sum::<usize>()
    }
    assert_eq!(hidden(&geometry), 1);
    d.handles.borrow()[&0].update(|s| s.value = 9).unwrap();
    settle(&mut t);
    p.jump_to(0.0).unwrap();
    settle(&mut t);
    assert_eq!(d.values.borrow()[&0], (9, before.1));
    t.unmount();
    assert!(!d.handles.borrow()[&0].mounted());
    assert_eq!(d.disposed.borrow().iter().filter(|id| **id == 0).count(), 1);
}
#[test]
fn unrequested_rows_dispose_and_return_as_fresh_state() {
    let d = data();
    let (p, _, mut t) = setup(&d, true, false);
    settle(&mut t);
    let old = d.values.borrow()[&0].1;
    d.handles.borrow()[&0].update(|s| s.value = 4).unwrap();
    settle(&mut t);
    p.jump_to(200.0).unwrap();
    settle(&mut t);
    assert!(!d.handles.borrow()[&0].mounted());
    p.jump_to(0.0).unwrap();
    settle(&mut t);
    assert_eq!(d.values.borrow()[&0].0, 0);
    assert_ne!(d.values.borrow()[&0].1, old);
}
#[test]
fn independent_requests_only_dispose_after_final_release_and_release_is_idempotent() {
    let d = data();
    d.request_counts.borrow_mut().insert(0, 2);
    let (p, _, mut t) = setup(&d, true, false);
    settle(&mut t);
    p.jump_to(200.0).unwrap();
    settle(&mut t);
    let mut handles = d.requests.borrow_mut().remove(&0).unwrap();
    handles[0].release();
    handles[0].release();
    settle(&mut t);
    assert!(!handles[0].is_active());
    assert!(handles[1].is_active());
    assert!(d.handles.borrow()[&0].mounted());
    drop(handles);
    settle(&mut t);
    assert!(!d.handles.borrow()[&0].mounted());
    assert_eq!(d.disposed.borrow().iter().filter(|id| **id == 0).count(), 1);
}
#[test]
fn declarative_request_can_be_withdrawn_while_parked() {
    let d = data();
    d.wanted.borrow_mut().insert(0);
    let (p, b, mut t) = setup(&d, true, false);
    settle(&mut t);
    p.jump_to(200.0).unwrap();
    settle(&mut t);
    assert!(d.handles.borrow()[&0].mounted());
    d.wanted.borrow_mut().clear();
    t.update(config(&p, &b, &d, true, false)).unwrap();
    settle(&mut t);
    assert!(!d.handles.borrow()[&0].mounted());
}
#[test]
fn reordered_cached_key_follows_mapping_and_deleted_key_releases_external_handle() {
    let d = data();
    d.request_counts.borrow_mut().insert(0, 1);
    let (p, b, mut t) = setup(&d, true, true);
    settle(&mut t);
    d.handles.borrow()[&0].update(|s| s.value = 12).unwrap();
    settle(&mut t);
    let old = d.values.borrow()[&0];
    p.jump_to(200.0).unwrap();
    settle(&mut t);
    d.ids.borrow_mut().swap(0, 20);
    t.update(config(&p, &b, &d, true, true)).unwrap();
    settle(&mut t);
    assert_eq!(d.values.borrow()[&0], old);
    p.jump_to(400.0).unwrap();
    settle(&mut t);
    assert_eq!(d.values.borrow()[&0], old);
    d.ids.borrow_mut().retain(|id| *id != 0);
    t.update(config(&p, &b, &d, true, true)).unwrap();
    settle(&mut t);
    assert!(!d.handles.borrow()[&0].mounted());
    let mut handles = d.requests.borrow_mut().remove(&0).unwrap();
    assert!(!handles[0].is_active());
    handles[0].release();
    settle(&mut t);
}
#[test]
fn changed_key_without_mapping_drops_cached_row_and_opt_out_invalidates_requests() {
    let d = data();
    d.request_counts.borrow_mut().insert(0, 1);
    let (p, b, mut t) = setup(&d, true, false);
    settle(&mut t);
    t.update(config(&p, &b, &d, false, false)).unwrap();
    settle(&mut t);
    assert!(!d.requests.borrow()[&0][0].is_active());
    p.jump_to(200.0).unwrap();
    settle(&mut t);
    assert!(!d.handles.borrow()[&0].mounted());
    let d = data();
    d.wanted.borrow_mut().insert(0);
    let (p, b, mut t) = setup(&d, true, false);
    settle(&mut t);
    p.jump_to(200.0).unwrap();
    settle(&mut t);
    d.ids.borrow_mut().swap(0, 20);
    t.update(config(&p, &b, &d, true, false)).unwrap();
    settle(&mut t);
    assert!(!d.handles.borrow()[&0].mounted());
}
#[test]
fn outside_item_scope_requests_are_absent_and_keep_alive_is_pass_through() {
    struct Outside(Rc<RefCell<Option<bool>>>);
    impl StatelessWidget for Outside {
        fn build(&self, ctx: &BuildContext) -> Widget {
            *self.0.borrow_mut() = Some(KeepAliveHandle::request(ctx).unwrap().is_none());
            Widget::keep_alive(true, Widget::leaf(vec2(30.0, 10.0), |_| {}))
        }
    }
    let result = Rc::new(RefCell::new(None));
    let mut t = ElementTree::new(
        &egui::Context::default(),
        Widget::stateless(Outside(result.clone())),
    )
    .unwrap();
    let c = BoxConstraints::loose(vec2(80.0, 40.0)).unwrap();
    assert_eq!(t.layout(c).unwrap().size, vec2(30.0, 10.0));
    assert_eq!(*result.borrow(), Some(true));
}

#[test]
fn nested_list_scopes_do_not_retain_the_outer_item_even_when_inner_requests_are_disabled() {
    for automatic in [false, true] {
        let ctx = egui::Context::default();
        let outer = ScrollPosition::new(&ctx, 0.0).unwrap();
        let inner = ScrollPosition::new(&ctx, 0.0).unwrap();
        let b = GestureBinding::new();
        let d = data();
        d.request_counts.borrow_mut().insert(0, 1);
        let inner = inner.clone();
        let binding = b.clone();
        let records = d.clone();
        let list = ListView::builder(&outer, &b, 100, 20.0, move |_, i| {
            if i == 0 {
                config(&inner, &binding, &records, automatic, false)
            } else {
                Widget::leaf(vec2(20.0, 20.0), |_| {})
            }
        })
        .unwrap()
        .cache_extent(0.0)
        .unwrap()
        .into_widget()
        .unwrap();
        let mut t = ElementTree::new(&ctx, list).unwrap();
        settle(&mut t);
        assert!(d.handles.borrow()[&0].mounted());
        assert_eq!(d.requests.borrow().contains_key(&0), automatic);
        outer.jump_to(200.0).unwrap();
        settle(&mut t);
        assert!(
            !d.handles.borrow()[&0].mounted(),
            "inner request escaped its item scope"
        );
        if automatic {
            assert!(!d.requests.borrow()[&0][0].is_active());
        }
    }
}

#[test]
fn parked_item_builder_keeps_its_own_inherited_dependencies_and_explicit_rebuild_context() {
    #[derive(PartialEq)]
    struct Theme(usize);
    let ctx = egui::Context::default();
    let p = ScrollPosition::new(&ctx, 0.0).unwrap();
    let b = GestureBinding::new();
    let item_context = Rc::new(RefCell::new(None));
    let observed = Rc::new(RefCell::new(vec![]));
    let saved = item_context.clone();
    let values = observed.clone();
    let list = ListView::builder(&p, &b, 100, 20.0, move |context, i| {
        if i == 0 {
            *saved.borrow_mut() = Some(context.clone());
            values
                .borrow_mut()
                .push(context.depend_on::<Theme>().unwrap().unwrap().0);
        }
        Widget::keep_alive(i == 0, Widget::leaf(vec2(20.0, 20.0), |_| {}))
    })
    .unwrap()
    .cache_extent(0.0)
    .unwrap()
    .into_widget()
    .unwrap();
    let mut t = ElementTree::new(&ctx, Widget::inherited(Theme(1), list)).unwrap();
    settle(&mut t);
    let provider = t.inherited_handle::<Theme>(t.root_id().unwrap()).unwrap();
    p.jump_to(200.0).unwrap();
    settle(&mut t);
    observed.borrow_mut().clear();
    provider.update(|_| Theme(2)).unwrap();
    settle(&mut t);
    assert_eq!(*observed.borrow(), vec![2]);
    observed.borrow_mut().clear();
    item_context
        .borrow()
        .as_ref()
        .unwrap()
        .request_rebuild()
        .unwrap();
    // A simultaneous parent scroll rebuild must not erase the item request.
    p.jump_to(240.0).unwrap();
    settle(&mut t);
    assert_eq!(*observed.borrow(), vec![2]);
    t.unmount();
    assert!(!item_context.borrow().as_ref().unwrap().mounted());
}

#[test]
fn focused_parked_text_editor_keeps_keyboard_activity_and_has_no_visible_paint() {
    let ctx = egui::Context::default();
    let p = ScrollPosition::new(&ctx, 0.0).unwrap();
    let b = GestureBinding::new();
    let text = Rc::new(RefCell::new(String::from("A")));
    let value = text.clone();
    let id = egui::Id::new("kept-editor");
    let list = ListView::builder(&p, &b, 100, 20.0, move |_, i| {
        let text = value.clone();
        Widget::keep_alive(
            i == 0,
            Widget::leaf(vec2(80.0, 20.0), move |ui| {
                if i == 0 {
                    ui.painter()
                        .rect_filled(ui.max_rect(), 0.0, egui::Color32::RED);
                    ui.add(egui::TextEdit::singleline(&mut *text.borrow_mut()).id(id));
                }
            }),
        )
    })
    .unwrap()
    .cache_extent(0.0)
    .unwrap()
    .into_widget()
    .unwrap();
    let mut t = ElementTree::new(&ctx, list).unwrap();
    let mut frame = |events| {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    vec2(400.0, 300.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let mut f = b.begin_egui_frame(ctx).unwrap();
                    t.show_in_gesture_frame(
                        ui,
                        BoxConstraints::tight(vec2(80.0, 40.0)).unwrap(),
                        &mut f,
                    )
                    .unwrap();
                    assert!(f.finish().unwrap().failures.is_empty());
                });
            },
        )
    };
    frame(vec![]);
    ctx.memory_mut(|m| m.request_focus(id));
    frame(vec![]);
    p.jump_to(200.0).unwrap();
    let output = frame(vec![egui::Event::Text("B".into())]);
    assert!(text.borrow().contains('B'));
    assert!(ctx.memory(|m| m.has_focus(id)));
    assert!(!output.shapes.iter().any(|s| s.clip_rect.is_positive()
        && matches!(&s.shape, egui::epaint::Shape::Rect(r) if r.fill==egui::Color32::RED)));
    p.jump_to(0.0).unwrap();
    frame(vec![]);
    assert!(text.borrow().contains('B'));
}

#[test]
fn kept_items_count_toward_realized_budget_instead_of_being_silently_evicted() {
    let ctx = egui::Context::default();
    let p = ScrollPosition::new(&ctx, 0.0).unwrap();
    let b = GestureBinding::new();
    let list = ListView::builder(&p, &b, 100, 20.0, |_, i| {
        Widget::keep_alive(i == 0, Widget::leaf(vec2(20.0, 20.0), |_| {}))
    })
    .unwrap()
    .cache_extent(0.0)
    .unwrap()
    .max_realized_items(2)
    .unwrap()
    .into_widget()
    .unwrap();
    let mut t = ElementTree::new(&ctx, list).unwrap();
    settle(&mut t);
    p.jump_to(200.0).unwrap();
    assert!(
        matches!(t.layout(BoxConstraints::tight(vec2(80.0,40.0)).unwrap()),Err(ElementError::Layout(LayoutError::InvalidConfiguration(message))) if message.contains("kept-alive"))
    );
    // Error descriptions replace the item subtree. A later valid range must
    // mount fresh descriptions rather than attempting to reuse disposed ones.
    p.jump_to(0.0).unwrap();
    settle(&mut t);
}

#[test]
fn kept_state_survives_both_axes_and_reverse_scroll_coordinates() {
    for axis in [Axis::Vertical, Axis::Horizontal] {
        for reverse in [false, true] {
            let d = data();
            let ctx = egui::Context::default();
            let p = ScrollPosition::new(&ctx, 0.0).unwrap();
            let b = GestureBinding::new();
            let first = if reverse {
                if axis == Axis::Vertical { 98 } else { 96 }
            } else {
                0
            };
            let records = d.clone();
            let list = ListView::builder(&p, &b, 100, 20.0, move |_, i| {
                Widget::keep_alive(
                    i == first,
                    Widget::stateful(Row {
                        id: i,
                        data: records.clone(),
                    }),
                )
            })
            .unwrap()
            .axis(axis)
            .reverse(reverse)
            .cache_extent(0.0)
            .unwrap()
            .into_widget()
            .unwrap();
            let mut t = ElementTree::new(&ctx, list).unwrap();
            settle(&mut t);
            let old = d.values.borrow()[&first].1;
            d.handles.borrow()[&first].update(|s| s.value = 8).unwrap();
            settle(&mut t);
            p.jump_to(200.0).unwrap();
            settle(&mut t);
            assert_eq!(d.values.borrow()[&first], (8, old));
            p.jump_to(0.0).unwrap();
            settle(&mut t);
            assert_eq!(d.values.borrow()[&first], (8, old));
        }
    }
}
#[test]
fn external_request_does_not_keep_unmounted_tree_or_item_state_alive() {
    let d = data();
    d.request_counts.borrow_mut().insert(0, 1);
    let (_, _, mut t) = setup(&d, true, false);
    settle(&mut t);
    let mut handles = d.requests.borrow_mut().remove(&0).unwrap();
    assert!(handles[0].is_active());
    t.unmount();
    assert!(!handles[0].is_active());
    assert!(!d.handles.borrow()[&0].mounted());
    handles[0].release();
    assert_eq!(d.disposed.borrow().iter().filter(|id| **id == 0).count(), 1);
}

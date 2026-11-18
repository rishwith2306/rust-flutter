use rust_flutter::{
    egui::{self, pos2, vec2},
    element::*,
    gestures::*,
    layout::*,
    scrolling::*,
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
fn size(axis: Axis) -> egui::Vec2 {
    if axis == Axis::Vertical {
        vec2(80.0, 100.0)
    } else {
        vec2(100.0, 80.0)
    }
}
fn bounds(axis: Axis) -> BoxConstraints {
    BoxConstraints::tight(size(axis)).unwrap()
}
fn widget(
    p: &ScrollPosition,
    b: &GestureBinding,
    count: usize,
    axis: Axis,
    reverse: bool,
    seen: Rc<RefCell<Vec<usize>>>,
) -> Widget {
    ListView::builder(p, b, count, 20.0, move |_, index| {
        seen.borrow_mut().push(index);
        Widget::leaf(vec2(20.0, 20.0), |_| {})
    })
    .unwrap()
    .axis(axis)
    .reverse(reverse)
    .cache_extent(0.0)
    .unwrap()
    .into_widget()
    .unwrap()
}
fn setup(
    count: usize,
    axis: Axis,
    reverse: bool,
) -> (
    egui::Context,
    ScrollPosition,
    GestureBinding,
    ElementTree,
    Rc<RefCell<Vec<usize>>>,
) {
    let ctx = egui::Context::default();
    let p = ScrollPosition::new(&ctx, 0.0).unwrap();
    let b = GestureBinding::new();
    let seen = Rc::new(RefCell::new(vec![]));
    let t = ElementTree::new(&ctx, widget(&p, &b, count, axis, reverse, seen.clone())).unwrap();
    (ctx, p, b, t, seen)
}
fn frame(
    ctx: &egui::Context,
    b: &GestureBinding,
    t: &mut ElementTree,
    axis: Axis,
    events: Vec<egui::Event>,
) {
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(600.0, 400.0),
            )),
            time: Some(ctx.input(|i| i.time) + 0.05),
            events,
            ..Default::default()
        },
        |ctx| {
            let mut f = b.begin_egui_frame(ctx).unwrap();
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut ui = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt("lazy-list-test")
                        .max_rect(egui::Rect::from_min_size(pos2(10.0, 10.0), size(axis))),
                );
                t.show_in_gesture_frame(&mut ui, bounds(axis), &mut f)
                    .unwrap();
            });
            assert!(f.finish().unwrap().failures.is_empty());
        },
    );
}
fn find(g: &LayoutBox, main: f32, axis: Axis) -> Option<&LayoutBox> {
    let length = if axis == Axis::Vertical {
        g.size.y
    } else {
        g.size.x
    };
    if length == main {
        return Some(g);
    }
    g.children.iter().find_map(|c| find(&c.layout, main, axis))
}
fn origin(g: &LayoutBox, id: egui::Id) -> Option<egui::Vec2> {
    if g.id == id {
        return Some(egui::Vec2::ZERO);
    }
    g.children
        .iter()
        .find_map(|c| origin(&c.layout, id).map(|p| p + c.offset))
}
fn button(x: f32, y: f32, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos: pos2(x, y),
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    }
}
#[test]
fn million_items_only_build_viewport_and_large_jumps_skip_offscreen_work() {
    let (ctx, p, b, mut t, seen) = setup(1_000_000, Axis::Vertical, false);
    assert!(seen.borrow().is_empty());
    frame(&ctx, &b, &mut t, Axis::Vertical, vec![]);
    assert_eq!(*seen.borrow(), vec![0, 1, 2, 3, 4]);
    assert_eq!(p.dimensions().unwrap().max_scroll_extent(), 19_999_900.0);
    seen.borrow_mut().clear();
    p.jump_to(10_000.0).unwrap();
    frame(&ctx, &b, &mut t, Axis::Vertical, vec![]);
    assert_eq!(*seen.borrow(), vec![500, 501, 502, 503, 504]);
    let g = t.layout(bounds(Axis::Vertical)).unwrap();
    let content = find(&g, 20_000_000.0, Axis::Vertical).unwrap();
    assert_eq!(content.children.len(), 5);
    seen.borrow_mut().clear();
    p.jump_to(20_000_000.0).unwrap();
    frame(&ctx, &b, &mut t, Axis::Vertical, vec![]);
    assert_eq!(*seen.borrow(), vec![999995, 999996, 999997, 999998, 999999]);
}
#[test]
fn partial_rows_rebase_to_small_local_coordinates_and_hard_clip() {
    let (ctx, p, b, mut t, seen) = setup(100, Axis::Vertical, false);
    frame(&ctx, &b, &mut t, Axis::Vertical, vec![]);
    seen.borrow_mut().clear();
    p.jump_to(25.0).unwrap();
    frame(&ctx, &b, &mut t, Axis::Vertical, vec![]);
    assert_eq!(*seen.borrow(), vec![1, 2, 3, 4, 5, 6]);
    let g = t.layout(bounds(Axis::Vertical)).unwrap();
    let content = find(&g, 2000.0, Axis::Vertical).unwrap();
    assert_eq!(content.children[0].offset.y, 0.0);
    assert_eq!(origin(&g, content.children[0].layout.id).unwrap().y, -5.0);
    assert!(g.hit_test_path(pos2(20.0, -1.0)).is_empty());
    assert!(g.hit_test_path(pos2(20.0, 101.0)).is_empty());
    assert!(!g.hit_test_path(pos2(20.0, 5.0)).is_empty());
}
#[test]
fn horizontal_reverse_and_short_reverse_content_use_correct_indices() {
    let (ctx, p, b, mut t, seen) = setup(100, Axis::Horizontal, true);
    frame(&ctx, &b, &mut t, Axis::Horizontal, vec![]);
    assert_eq!(*seen.borrow(), vec![95, 96, 97, 98, 99]);
    seen.borrow_mut().clear();
    p.jump_to(40.0).unwrap();
    frame(&ctx, &b, &mut t, Axis::Horizontal, vec![]);
    assert_eq!(*seen.borrow(), vec![93, 94, 95, 96, 97]);
    let g = t.layout(bounds(Axis::Horizontal)).unwrap();
    let content = find(&g, 2000.0, Axis::Horizontal).unwrap();
    assert_eq!(origin(&g, content.children[0].layout.id).unwrap().x, 0.0);
    let (ctx, _p, b, mut t, seen) = setup(3, Axis::Vertical, true);
    frame(&ctx, &b, &mut t, Axis::Vertical, vec![]);
    assert_eq!(*seen.borrow(), vec![0, 1, 2]);
    let g = t.layout(bounds(Axis::Vertical)).unwrap();
    assert_eq!(g.size, vec2(80.0, 100.0));
    let content = find(&g, 60.0, Axis::Vertical).unwrap();
    assert_eq!(origin(&g, content.children[0].layout.id).unwrap().y, 40.0);
}
#[test]
fn empty_and_zero_area_views_fill_constraints_without_building_rows() {
    let (ctx, p, b, mut t, seen) = setup(0, Axis::Vertical, false);
    frame(&ctx, &b, &mut t, Axis::Vertical, vec![]);
    assert!(seen.borrow().is_empty());
    assert_eq!(p.dimensions().unwrap().max_scroll_extent(), 0.0);
    assert_eq!(
        t.layout(bounds(Axis::Vertical)).unwrap().size,
        vec2(80.0, 100.0)
    );
    let (ctx, _p, _b, mut t, seen) = setup(100, Axis::Vertical, false);
    ctx.begin_pass(egui::RawInput::default());
    assert_eq!(
        t.layout(BoxConstraints::tight(vec2(80.0, 0.0)).unwrap())
            .unwrap()
            .size,
        vec2(80.0, 0.0)
    );
    assert!(seen.borrow().is_empty());
    let _ = ctx.end_pass();
}
#[test]
fn pointer_input_rebuilds_visible_range_without_losing_captured_pan() {
    let (ctx, p, b, mut t, seen) = setup(100, Axis::Vertical, false);
    frame(&ctx, &b, &mut t, Axis::Vertical, vec![]);
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Vertical,
        vec![
            egui::Event::PointerMoved(pos2(30.0, 80.0)),
            button(30.0, 80.0, true),
        ],
    );
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Vertical,
        vec![egui::Event::PointerMoved(pos2(30.0, 40.0))],
    );
    assert_eq!(p.pixels(), 40.0);
    seen.borrow_mut().clear();
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Vertical,
        vec![egui::Event::PointerMoved(pos2(30.0, 0.0))],
    );
    assert_eq!(p.pixels(), 80.0);
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Vertical,
        vec![button(30.0, 0.0, false)],
    );
    assert!(seen.borrow().contains(&4));
}
#[test]
fn wheel_and_keyboard_use_the_shared_viewport() {
    let (ctx, p, b, mut t, _seen) = setup(100, Axis::Vertical, false);
    frame(&ctx, &b, &mut t, Axis::Vertical, vec![]);
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Vertical,
        vec![
            egui::Event::PointerMoved(pos2(30.0, 30.0)),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0.0, -40.0),
                modifiers: Default::default(),
            },
        ],
    );
    assert_eq!(p.pixels(), 40.0);
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Vertical,
        vec![button(30.0, 30.0, true), button(30.0, 30.0, false)],
    );
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Vertical,
        vec![egui::Event::Key {
            key: egui::Key::End,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        }],
    );
    assert_eq!(p.pixels(), 1900.0);
}
#[derive(Default)]
struct Rows {
    ids: HashMap<usize, ElementId>,
    disposed: Vec<usize>,
}
struct Row {
    key: usize,
    rows: Rc<RefCell<Rows>>,
}
struct RowState {
    value: usize,
    key: usize,
    rows: Rc<RefCell<Rows>>,
}
impl StatefulWidget for Row {
    type State = RowState;
    fn create_state(&self) -> RowState {
        RowState {
            value: self.key,
            key: self.key,
            rows: self.rows.clone(),
        }
    }
}
impl State<Row> for RowState {
    fn init_state(&mut self, _: &Row, cx: &StateContext<Self>) {
        self.rows.borrow_mut().ids.insert(self.key, cx.element_id());
    }
    fn build(&mut self, _: &Row, _: &StateContext<Self>) -> Widget {
        Widget::leaf(vec2(20.0, 20.0), |_| {})
    }
    fn dispose(&mut self) {
        self.rows.borrow_mut().disposed.push(self.key);
    }
}
#[test]
fn cached_overlap_retains_state_and_leaving_cache_disposes_it() {
    let ctx = egui::Context::default();
    let p = ScrollPosition::new(&ctx, 0.0).unwrap();
    let b = GestureBinding::new();
    let rows = Rc::new(RefCell::new(Rows::default()));
    let log = rows.clone();
    let w = ListView::builder(&p, &b, 100, 20.0, move |_, index| {
        Widget::stateful(Row {
            key: index,
            rows: log.clone(),
        })
    })
    .unwrap()
    .cache_extent(20.0)
    .unwrap()
    .into_widget()
    .unwrap();
    let mut t = ElementTree::new(&ctx, w).unwrap();
    frame(&ctx, &b, &mut t, Axis::Vertical, vec![]);
    let id = rows.borrow().ids[&2];
    t.state_handle::<RowState>(id)
        .unwrap()
        .update(|s| s.value = 77)
        .unwrap();
    p.jump_to(20.0).unwrap();
    frame(&ctx, &b, &mut t, Axis::Vertical, vec![]);
    assert_eq!(rows.borrow().ids[&2], id);
    assert_eq!(t.state::<RowState>(id).unwrap().value, 77);
    p.jump_to(1000.0).unwrap();
    frame(&ctx, &b, &mut t, Axis::Vertical, vec![]);
    assert!(t.state::<RowState>(id).is_none());
    assert!(rows.borrow().disposed.contains(&2));
    drop(t);
    assert!(!p.is_disposed());
    assert_eq!(b.target_count(), 0);
}
#[test]
fn item_keys_preserve_data_state_when_visible_order_changes() {
    let ctx = egui::Context::default();
    let p = ScrollPosition::new(&ctx, 0.0).unwrap();
    let b = GestureBinding::new();
    let rows = Rc::new(RefCell::new(Rows::default()));
    let data = Rc::new(RefCell::new(vec![0, 1, 2, 3, 4]));
    let make = || {
        let row_data = data.clone();
        let key_data = data.clone();
        let rows = rows.clone();
        ListView::builder(&p, &b, 5, 20.0, move |_, index| {
            Widget::stateful(Row {
                key: row_data.borrow()[index],
                rows: rows.clone(),
            })
        })
        .unwrap()
        .cache_extent(0.0)
        .unwrap()
        .item_key(move |index| Key::U64(key_data.borrow()[index] as u64))
        .into_widget()
        .unwrap()
    };
    let mut t = ElementTree::new(&ctx, make()).unwrap();
    frame(&ctx, &b, &mut t, Axis::Vertical, vec![]);
    let id = rows.borrow().ids[&1];
    t.state_handle::<RowState>(id)
        .unwrap()
        .update(|s| s.value = 77)
        .unwrap();
    data.borrow_mut().swap(0, 1);
    t.update(make()).unwrap();
    frame(&ctx, &b, &mut t, Axis::Vertical, vec![]);
    assert_eq!(rows.borrow().ids[&1], id);
    assert_eq!(t.state::<RowState>(id).unwrap().value, 77);
    assert!(rows.borrow().disposed.is_empty());
}
#[test]
fn invalid_extents_bounds_and_realized_budget_return_checked_errors() {
    let ctx = egui::Context::default();
    let p = ScrollPosition::new(&ctx, 0.0).unwrap();
    let b = GestureBinding::new();
    assert!(matches!(
        ListView::builder(&p, &b, 10, 0.0, |_, _| Widget::empty()),
        Err(ScrollError::InvalidItemExtent)
    ));
    assert!(matches!(
        ListView::builder(&p, &b, usize::MAX, f32::MAX, |_, _| Widget::empty()),
        Err(ScrollError::ContentExtentOverflow)
    ));
    assert!(matches!(
        ListView::builder(&p, &b, 10, 1.0, |_, _| Widget::empty())
            .unwrap()
            .cache_extent(f32::NAN),
        Err(ScrollError::InvalidCacheExtent)
    ));
    let mut t = ElementTree::new(
        &ctx,
        ListView::builder(&p, &b, 1_000_000, 0.001, |_, _| {
            panic!("budget must reject before building rows")
        })
        .unwrap()
        .cache_extent(0.0)
        .unwrap()
        .into_widget()
        .unwrap(),
    )
    .unwrap();
    assert!(matches!(
        t.layout(bounds(Axis::Vertical)),
        Err(ElementError::Layout(LayoutError::InvalidConfiguration(
            "lazy list exceeds its configured realized item limit"
        )))
    ));
    assert!(t.root_id().is_some());
}
#[test]
fn unbounded_constraints_error_and_corrected_layout_retries_without_row_work() {
    let (ctx, _p, _b, mut t, seen) = setup(100, Axis::Vertical, false);
    ctx.begin_pass(egui::RawInput::default());
    assert!(matches!(
        t.layout(BoxConstraints::loose(egui::Vec2::INFINITY).unwrap()),
        Err(ElementError::Layout(LayoutError::UnboundedSize))
    ));
    assert!(seen.borrow().is_empty());
    assert_eq!(
        t.layout(bounds(Axis::Vertical)).unwrap().size,
        vec2(80.0, 100.0)
    );
    assert_eq!(seen.borrow().len(), 5);
    let _ = ctx.end_pass();
}
#[test]
fn dry_queries_do_not_realize_rows() {
    let (_ctx, _p, _b, mut t, seen) = setup(100, Axis::Vertical, false);
    assert!(t.dry_layout(bounds(Axis::Vertical)).is_err());
    assert!(t.intrinsic(IntrinsicDimension::MaxHeight, 80.0).is_err());
    assert!(seen.borrow().is_empty());
}

#[test]
fn resize_and_count_reduction_rebuild_range_and_clamp_offset() {
    let (ctx, p, b, mut t, seen) = setup(100, Axis::Vertical, false);
    frame(&ctx, &b, &mut t, Axis::Vertical, vec![]);
    seen.borrow_mut().clear();
    ctx.begin_pass(egui::RawInput::default());
    t.layout(BoxConstraints::tight(vec2(80.0, 160.0)).unwrap())
        .unwrap();
    let _ = ctx.end_pass();
    assert_eq!(seen.borrow().len(), 8);
    assert_eq!(p.dimensions().unwrap().viewport_dimension(), 160.0);
    p.jump_to(1800.0).unwrap();
    t.update(widget(&p, &b, 3, Axis::Vertical, false, seen.clone()))
        .unwrap();
    seen.borrow_mut().clear();
    frame(&ctx, &b, &mut t, Axis::Vertical, vec![]);
    assert_eq!(*seen.borrow(), vec![0, 1, 2]);
    assert_eq!(p.pixels(), 0.0);
    assert_eq!(p.dimensions().unwrap().max_scroll_extent(), 0.0);
}
#[test]
fn initial_out_of_range_offset_builds_the_clamped_endpoint_before_paint() {
    let ctx = egui::Context::default();
    let p = ScrollPosition::new(&ctx, 1_000_000.0).unwrap();
    let b = GestureBinding::new();
    let seen = Rc::new(RefCell::new(vec![]));
    let mut t = ElementTree::new(
        &ctx,
        widget(&p, &b, 100, Axis::Vertical, false, seen.clone()),
    )
    .unwrap();
    frame(&ctx, &b, &mut t, Axis::Vertical, vec![]);
    assert_eq!(*seen.borrow(), vec![95, 96, 97, 98, 99]);
    assert_eq!(p.pixels(), 1900.0);
}
#[test]
fn fractional_extents_realize_both_partial_boundary_rows() {
    let ctx = egui::Context::default();
    let p = ScrollPosition::new(&ctx, 0.0).unwrap();
    let b = GestureBinding::new();
    let seen = Rc::new(RefCell::new(vec![]));
    let built = seen.clone();
    let w = ListView::builder(&p, &b, 100, 1.5, move |_, index| {
        built.borrow_mut().push(index);
        Widget::empty()
    })
    .unwrap()
    .cache_extent(0.0)
    .unwrap()
    .into_widget()
    .unwrap();
    let mut t = ElementTree::new(&ctx, w).unwrap();
    let c = BoxConstraints::tight(vec2(80.0, 3.0)).unwrap();
    ctx.begin_pass(egui::RawInput::default());
    t.layout(c).unwrap();
    let _ = ctx.end_pass();
    seen.borrow_mut().clear();
    p.jump_to(0.75).unwrap();
    ctx.begin_pass(egui::RawInput::default());
    let g = t.layout(c).unwrap();
    let _ = ctx.end_pass();
    assert_eq!(*seen.borrow(), vec![0, 1, 2]);
    let content = find(&g, 150.0, Axis::Vertical).unwrap();
    assert_eq!(origin(&g, content.children[0].layout.id).unwrap().y, -0.75);
    assert_eq!(content.children[2].layout.size.y, 1.5);
}
#[derive(PartialEq)]
struct Theme(u32);
#[test]
fn item_builder_reads_inherited_values_in_the_ordinary_tree_scope() {
    let ctx = egui::Context::default();
    let p = ScrollPosition::new(&ctx, 0.0).unwrap();
    let b = GestureBinding::new();
    let values = Rc::new(RefCell::new(vec![]));
    let seen = values.clone();
    let list = ListView::builder(&p, &b, 100, 20.0, move |cx, index| {
        seen.borrow_mut()
            .push((cx.depend_on::<Theme>().unwrap().unwrap().0, index));
        Widget::empty()
    })
    .unwrap()
    .cache_extent(0.0)
    .unwrap()
    .into_widget()
    .unwrap();
    let mut t = ElementTree::new(&ctx, Widget::inherited(Theme(1), list)).unwrap();
    frame(&ctx, &b, &mut t, Axis::Vertical, vec![]);
    assert_eq!(values.borrow().len(), 5);
    let provider = t.inherited_handle::<Theme>(t.root_id().unwrap()).unwrap();
    provider.update(|_| Theme(2)).unwrap();
    values.borrow_mut().clear();
    frame(&ctx, &b, &mut t, Axis::Vertical, vec![]);
    assert_eq!(
        *values.borrow(),
        vec![(2, 0), (2, 1), (2, 2), (2, 3), (2, 4)]
    );
}
#[test]
fn duplicate_item_keys_follow_normal_validation_and_disposed_positions_error() {
    let ctx = egui::Context::default();
    let p = ScrollPosition::new(&ctx, 0.0).unwrap();
    let b = GestureBinding::new();
    let w = ListView::builder(&p, &b, 100, 20.0, |_, _| Widget::empty())
        .unwrap()
        .cache_extent(0.0)
        .unwrap()
        .item_key(|_| Key::U64(1))
        .into_widget()
        .unwrap();
    let mut t = ElementTree::new(&ctx, w).unwrap();
    assert!(matches!(
        t.layout(bounds(Axis::Vertical)),
        Err(ElementError::DuplicateKey(_))
    ));
    let (ctx, p, b, mut t, _) = setup(100, Axis::Vertical, false);
    frame(&ctx, &b, &mut t, Axis::Vertical, vec![]);
    p.dispose().unwrap();
    assert!(t.layout(bounds(Axis::Vertical)).is_err());
}
#[test]
fn scrollbar_thumb_updates_lazy_range_and_keeps_owned_drag() {
    let (ctx, p, b, _, seen) = setup(100, Axis::Vertical, false);
    let list = widget(&p, &b, 100, Axis::Vertical, false, seen.clone());
    let mut t = ElementTree::new(
        &ctx,
        Scrollbar::new(&p, &b).unwrap().into_widget(list).unwrap(),
    )
    .unwrap();
    frame(&ctx, &b, &mut t, Axis::Vertical, vec![]);
    frame(&ctx, &b, &mut t, Axis::Vertical, vec![]);
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Vertical,
        vec![
            egui::Event::PointerMoved(pos2(84.0, 20.0)),
            button(84.0, 20.0, true),
        ],
    );
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Vertical,
        vec![egui::Event::PointerMoved(pos2(84.0, 52.0))],
    );
    assert!(p.pixels() > 700.0);
    assert_eq!(p.activity(), ScrollActivity::Drag);
    seen.borrow_mut().clear();
    frame(
        &ctx,
        &b,
        &mut t,
        Axis::Vertical,
        vec![button(84.0, 52.0, false)],
    );
    assert!(seen.borrow().iter().all(|index| *index >= 35));
    assert_eq!(p.activity(), ScrollActivity::Idle);
}

#[test]
fn tiny_viewport_at_rounded_large_endpoint_still_renders_last_row() {
    let ctx = egui::Context::default();
    let p = ScrollPosition::new(&ctx, 0.0).unwrap();
    let b = GestureBinding::new();
    let seen = Rc::new(RefCell::new(vec![]));
    let calls = seen.clone();
    let list = ListView::builder(&p, &b, 1_000_000, 20.0, move |_, index| {
        calls.borrow_mut().push(index);
        Widget::empty()
    })
    .unwrap()
    .cache_extent(0.0)
    .unwrap()
    .into_widget()
    .unwrap();
    let mut t = ElementTree::new(&ctx, list).unwrap();
    let c = BoxConstraints::tight(vec2(80.0, 0.5)).unwrap();
    ctx.begin_pass(egui::RawInput::default());
    t.layout(c).unwrap();
    let _ = ctx.end_pass();
    seen.borrow_mut().clear();
    p.jump_to(20_000_000.0).unwrap();
    ctx.begin_pass(egui::RawInput::default());
    let g = t.layout(c).unwrap();
    let _ = ctx.end_pass();
    assert_eq!(*seen.borrow(), vec![999999]);
    let content = find(&g, 20_000_000.0, Axis::Vertical).unwrap();
    assert_eq!(origin(&g, content.children[0].layout.id).unwrap().y, -19.5);
}

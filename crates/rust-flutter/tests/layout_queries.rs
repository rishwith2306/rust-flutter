use rust_flutter::{
    egui::{self, vec2},
    element::*,
    layout::*,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

fn flex_tree(
    axis: Axis,
    main_size: MainAxisSize,
    cross: CrossAxisAlignment,
    fit: FlexFit,
    reverse: bool,
) -> BoxNode {
    let mut flex = Flex::new(
        axis,
        vec![
            FlexChild::fixed(BoxNode::leaf("fixed", vec2(20.0, 15.0))),
            FlexChild::flexible(BoxNode::leaf("flex1", vec2(30.0, 25.0)), 1, fit),
            FlexChild::flexible(BoxNode::leaf("flex2", vec2(17.0, 8.0)), 2, FlexFit::Loose),
        ],
    );
    flex.main_size = main_size;
    flex.cross_alignment = cross;
    flex.spacing = 3.0;
    if reverse {
        flex.text_direction = TextDirection::Rtl;
        flex.vertical_direction = VerticalDirection::Up;
    }
    BoxNode::padding("pad", Insets::all(2.0), BoxNode::flex("flex", flex))
}
#[test]
fn dry_and_wet_flex_agree_across_axes_fits_directions_and_constraints() {
    let constraints = [
        BoxConstraints::loose(vec2(300.0, 200.0)).unwrap(),
        BoxConstraints::tight(vec2(180.0, 120.0)).unwrap(),
        BoxConstraints::loose(vec2(f32::INFINITY, 200.0)).unwrap(),
        BoxConstraints::loose(vec2(300.0, f32::INFINITY)).unwrap(),
    ];
    for axis in [Axis::Horizontal, Axis::Vertical] {
        for main in [MainAxisSize::Min, MainAxisSize::Max] {
            for cross in [
                CrossAxisAlignment::Start,
                CrossAxisAlignment::End,
                CrossAxisAlignment::Center,
                CrossAxisAlignment::Stretch,
            ] {
                for fit in [FlexFit::Loose, FlexFit::Tight] {
                    for reverse in [false, true] {
                        for c in constraints {
                            let mut tree = flex_tree(axis, main, cross, fit, reverse);
                            assert_eq!(
                                tree.dry_layout(c),
                                tree.layout(c).map(|layout| layout.size),
                                "{axis:?} {main:?} {cross:?} {fit:?} {reverse} {c:?}"
                            );
                        }
                    }
                }
            }
        }
    }
}
#[test]
fn dry_wrappers_match_wet_and_stack_size_ignores_positioned_measurement_callbacks() {
    let calls = Rc::new(Cell::new(0));
    let wet = calls.clone();
    let mut stack = Stack::new(vec![
        StackChild::new(BoxNode::align(
            "align",
            Alignment::CENTER,
            BoxNode::constrained(
                "c",
                BoxConstraints::tight(vec2(40.0, 30.0)).unwrap(),
                BoxNode::leaf("leaf", vec2(10.0, 20.0)),
            ),
        )),
        StackChild::positioned(
            BoxNode::measured("positioned", move |c| {
                wet.set(wet.get() + 1);
                c.constrain(vec2(1.0, 1.0))
            }),
            Positioned::fill(),
        ),
    ]);
    stack.fit = StackFit::Loose;
    let mut tree = BoxNode::stack("stack", stack);
    let c = BoxConstraints::loose(vec2(200.0, 100.0)).unwrap();
    let dry = tree.dry_layout(c).unwrap();
    assert_eq!(calls.get(), 0);
    assert_eq!(tree.max_intrinsic_width(f32::INFINITY).unwrap(), 40.0);
    assert_eq!(tree.max_intrinsic_height(f32::INFINITY).unwrap(), 30.0);
    assert_eq!(tree.layout(c).unwrap().size, dry);
    assert_eq!(calls.get(), 1);
    let positioned = BoxNode::stack(
        "positioned-only",
        Stack::new(vec![StackChild::positioned(
            BoxNode::leaf("child", vec2(50.0, 50.0)),
            Positioned::fill(),
        )]),
    );
    assert_eq!(positioned.max_intrinsic_width(f32::INFINITY).unwrap(), 0.0);
    assert_eq!(positioned.dry_layout(c).unwrap(), vec2(200.0, 100.0));
}

type QueryLog = Rc<RefCell<Vec<(IntrinsicDimension, f32)>>>;
struct Wrapping {
    actual: Rc<Cell<usize>>,
    queries: QueryLog,
}
impl Wrapping {
    fn size(c: BoxConstraints) -> Result<egui::Vec2, LayoutError> {
        let width = 33.0_f32.clamp(c.min().x, c.max().x);
        c.constrain(vec2(width, if width <= 40.0 { 40.0 } else { 20.0 }))
    }
}
impl CustomBoxLayout for Wrapping {
    fn layout(
        &mut self,
        c: BoxConstraints,
        _: &mut BoxChildren<'_>,
    ) -> Result<BoxGeometry, LayoutError> {
        self.actual.set(self.actual.get() + 1);
        Ok(BoxGeometry::new(Self::size(c)?))
    }
    fn dry_layout(&self, c: BoxConstraints, _: &BoxQueries<'_>) -> Result<egui::Vec2, LayoutError> {
        Self::size(c)
    }
    fn intrinsic(
        &self,
        d: IntrinsicDimension,
        extent: f32,
        _: &BoxQueries<'_>,
    ) -> Result<f32, LayoutError> {
        self.queries.borrow_mut().push((d, extent));
        Ok(match d {
            IntrinsicDimension::MinWidth => 5.0,
            IntrinsicDimension::MaxWidth => 33.0,
            _ => {
                if extent <= 40.0 {
                    40.0
                } else {
                    20.0
                }
            }
        })
    }
}
fn wrapping() -> (BoxNode, Rc<Cell<usize>>, QueryLog) {
    let actual = Rc::new(Cell::new(0));
    let queries = Rc::default();
    (
        BoxNode::new(
            "wrap",
            BoxKind::Custom {
                layout: CustomBox::new(Wrapping {
                    actual: actual.clone(),
                    queries: Rc::clone(&queries),
                }),
                children: vec![],
            },
        ),
        actual,
        queries,
    )
}
#[test]
fn intrinsic_queries_distinguish_min_max_and_deflate_the_opposite_padding_axis() {
    let (child, calls, queries) = wrapping();
    let tree = BoxNode::padding(
        "padding",
        Insets {
            left: 3.0,
            right: 7.0,
            top: 4.0,
            bottom: 6.0,
        },
        child,
    );
    assert_eq!(tree.min_intrinsic_width(30.0).unwrap(), 15.0);
    assert_eq!(tree.max_intrinsic_width(30.0).unwrap(), 43.0);
    assert_eq!(tree.max_intrinsic_height(50.0).unwrap(), 50.0);
    assert_eq!(
        queries.borrow().as_slice(),
        &[
            (IntrinsicDimension::MinWidth, 20.0),
            (IntrinsicDimension::MaxWidth, 20.0),
            (IntrinsicDimension::MaxHeight, 40.0)
        ]
    );
    assert_eq!(calls.get(), 0);
    let aligned = BoxNode::new(
        "align",
        BoxKind::Align {
            alignment: Alignment::CENTER,
            width_factor: Some(2.0),
            height_factor: Some(3.0),
            child: Box::new(tree),
        },
    );
    assert_eq!(aligned.min_intrinsic_width(30.0).unwrap(), 30.0);
    assert_eq!(aligned.max_intrinsic_height(50.0).unwrap(), 150.0);
}
#[test]
fn flex_intrinsics_use_flex_fractions_and_cross_axis_max_content_allocations() {
    let mut flex = Flex::new(
        Axis::Horizontal,
        vec![
            FlexChild::fixed(BoxNode::leaf("fixed", vec2(10.0, 7.0))),
            FlexChild::expanded(BoxNode::leaf("one", vec2(20.0, 8.0)), 1),
            FlexChild::expanded(BoxNode::leaf("two", vec2(60.0, 9.0)), 2),
        ],
    );
    flex.spacing = 5.0;
    let tree = BoxNode::flex("row", flex);
    assert_eq!(tree.min_intrinsic_width(100.0).unwrap(), 110.0); // 10 + gaps 10 + max(20/1,60/2)*3
    assert_eq!(tree.max_intrinsic_height(70.0).unwrap(), 9.0);
    let (wrap, _, queries) = wrapping();
    let tree = BoxNode::flex(
        "cross",
        Flex::new(
            Axis::Horizontal,
            vec![
                FlexChild::fixed(BoxNode::leaf("fixed", vec2(10.0, 5.0))),
                FlexChild::expanded(wrap, 1),
            ],
        ),
    );
    assert_eq!(tree.max_intrinsic_height(50.0).unwrap(), 40.0);
    assert!(
        queries
            .borrow()
            .contains(&(IntrinsicDimension::MaxHeight, 40.0))
    );
}
#[test]
fn intrinsic_wrappers_snap_and_clamp_without_running_wet_layout_during_queries() {
    let (child, calls, queries) = wrapping();
    let mut tree = BoxNode::intrinsic_width("width", Some(10.0), Some(15.0), child);
    let c = BoxConstraints::loose(vec2(100.0, 100.0)).unwrap();
    assert_eq!(tree.dry_layout(c).unwrap(), vec2(40.0, 30.0));
    assert_eq!(calls.get(), 0);
    assert!(
        queries
            .borrow()
            .contains(&(IntrinsicDimension::MaxHeight, 100.0))
    ); // original parent width
    assert_eq!(tree.layout(c).unwrap().size, vec2(40.0, 30.0));
    assert_eq!(calls.get(), 1);
    let tight = BoxConstraints::tight(vec2(20.0, 10.0)).unwrap();
    assert_eq!(tree.dry_layout(tight).unwrap(), vec2(20.0, 10.0));
    assert_eq!(tree.min_intrinsic_width(100.0).unwrap(), 40.0);
    assert_eq!(tree.max_intrinsic_height(f32::INFINITY).unwrap(), 45.0);
    for step in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        let (child, _, _) = wrapping();
        let mut bad = BoxNode::intrinsic_width("bad", Some(step), None, child);
        assert!(matches!(
            bad.dry_layout(c),
            Err(LayoutError::InvalidConfiguration(_))
        ));
        assert!(matches!(
            bad.layout(c),
            Err(LayoutError::InvalidConfiguration(_))
        ));
    }
}
#[test]
fn intrinsic_height_and_width_bound_stretched_flex_on_the_unbounded_axis() {
    let mut row = Flex::new(
        Axis::Horizontal,
        vec![
            FlexChild::fixed(BoxNode::leaf("A", vec2(20.0, 20.0))),
            FlexChild::fixed(BoxNode::leaf("B", vec2(30.0, 40.0))),
        ],
    );
    row.cross_alignment = CrossAxisAlignment::Stretch;
    let mut tree = BoxNode::intrinsic_height("height", BoxNode::flex("row", row));
    let c = BoxConstraints::loose(vec2(100.0, f32::INFINITY)).unwrap();
    assert_eq!(tree.dry_layout(c).unwrap(), vec2(100.0, 40.0));
    let layout = tree.layout(c).unwrap();
    assert_eq!(layout.size, vec2(100.0, 40.0));
    assert!(
        layout.children[0]
            .layout
            .children
            .iter()
            .all(|child| child.layout.size.y == 40.0)
    );
    let mut column = Flex::new(
        Axis::Vertical,
        vec![
            FlexChild::fixed(BoxNode::leaf("A", vec2(20.0, 20.0))),
            FlexChild::fixed(BoxNode::leaf("B", vec2(30.0, 40.0))),
        ],
    );
    column.cross_alignment = CrossAxisAlignment::Stretch;
    let mut tree = BoxNode::intrinsic_width("width", None, None, BoxNode::flex("column", column));
    let c = BoxConstraints::loose(vec2(f32::INFINITY, 100.0)).unwrap();
    assert_eq!(tree.dry_layout(c).unwrap(), vec2(30.0, 100.0));
    assert_eq!(tree.layout(c).unwrap().size, vec2(30.0, 100.0));
}
#[test]
fn unsupported_measurements_and_invalid_intrinsic_arguments_do_not_execute_layout() {
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let tree = BoxNode::measured("opaque", move |c| {
        count.set(count.get() + 1);
        c.constrain(vec2(20.0, 20.0))
    });
    assert!(matches!(
        tree.dry_layout(BoxConstraints::default()),
        Err(LayoutError::UnsupportedQuery(_))
    ));
    assert!(matches!(
        tree.min_intrinsic_width(30.0),
        Err(LayoutError::UnsupportedQuery(_))
    ));
    for extent in [f32::NAN, -1.0, f32::NEG_INFINITY] {
        assert!(matches!(
            tree.max_intrinsic_height(extent),
            Err(LayoutError::InvalidConfiguration(_))
        ));
    }
    assert_eq!(calls.get(), 0);
    let fixed = BoxNode::constrained(
        "fixed",
        BoxConstraints::tight(vec2(50.0, 60.0)).unwrap(),
        tree,
    );
    assert_eq!(fixed.min_intrinsic_width(100.0).unwrap(), 50.0); // no child query required
    assert_eq!(fixed.max_intrinsic_height(100.0).unwrap(), 60.0);
}

#[derive(Clone, PartialEq)]
struct Pad(f32);
struct Proxy {
    actual: Rc<Cell<usize>>,
}
struct ProxyBox {
    actual: Rc<Cell<usize>>,
    pad: f32,
}
impl RenderObjectWidget for Proxy {
    type Object = ProxyBox;
    fn create_render_object(&self, _: &RenderContext) -> ProxyBox {
        ProxyBox {
            actual: self.actual.clone(),
            pad: 0.0,
        }
    }
    fn update_render_object(&self, object: &mut ProxyBox, ctx: &RenderContext) {
        object.pad = ctx.depend_on::<Pad>().unwrap().map_or(0.0, |pad| pad.0);
    }
}
impl RenderBox for ProxyBox {
    fn dry_layout(
        &self,
        c: BoxConstraints,
        children: &BoxQueries<'_>,
    ) -> Result<egui::Vec2, LayoutError> {
        c.constrain(
            children.dry_layout(0, c.deflate(Insets::all(self.pad))?)?
                + vec2(self.pad * 2.0, self.pad * 2.0),
        )
    }
    fn intrinsic(
        &self,
        d: IntrinsicDimension,
        extent: f32,
        children: &BoxQueries<'_>,
    ) -> Result<f32, LayoutError> {
        Ok(children.intrinsic(0, d, (extent - self.pad * 2.0).max(0.0))? + self.pad * 2.0)
    }
    fn layout(
        &mut self,
        c: BoxConstraints,
        children: &mut BoxChildren<'_>,
        _: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        self.actual.set(self.actual.get() + 1);
        // Speculation through a wet layout's immutable child view does not place it.
        let dry = children
            .queries()
            .dry_layout(0, c.deflate(Insets::all(self.pad))?)?;
        let size = children.layout(0, c.deflate(Insets::all(self.pad))?)?;
        assert_eq!(size, dry);
        children.place(0, vec2(self.pad, self.pad))?;
        Ok(BoxGeometry::new(
            c.constrain(size + vec2(self.pad * 2.0, self.pad * 2.0))?,
        ))
    }
}
#[test]
fn retained_queries_flush_inherited_updates_without_running_layout_or_invalidating_its_cache() {
    let ctx = egui::Context::default();
    let calls = Rc::new(Cell::new(0));
    let mut tree = ElementTree::new(
        &ctx,
        Widget::inherited(
            Pad(5.0),
            Widget::render_object(
                Proxy {
                    actual: calls.clone(),
                },
                vec![Widget::leaf(vec2(40.0, 20.0), |_| {})],
            ),
        ),
    )
    .unwrap();
    let handle = tree
        .inherited_handle::<Pad>(tree.root_id().unwrap())
        .unwrap();
    let c = BoxConstraints::loose(vec2(200.0, 200.0)).unwrap();
    assert_eq!(tree.dry_layout(c).unwrap(), vec2(50.0, 30.0));
    assert_eq!(
        tree.intrinsic(IntrinsicDimension::MaxWidth, 100.0).unwrap(),
        50.0
    );
    assert_eq!(calls.get(), 0);
    tree.layout(c).unwrap();
    assert_eq!(calls.get(), 1);
    tree.dry_layout(c).unwrap();
    tree.intrinsic(IntrinsicDimension::MinHeight, 100.0)
        .unwrap();
    tree.layout(c).unwrap();
    assert_eq!(calls.get(), 1);
    handle.set(Pad(10.0)).unwrap();
    assert_eq!(tree.dry_layout(c).unwrap(), vec2(60.0, 40.0));
    assert_eq!(calls.get(), 1);
    assert_eq!(tree.layout(c).unwrap().size, vec2(60.0, 40.0));
    assert_eq!(calls.get(), 2);
}
struct Opaque(Rc<Cell<usize>>);
impl CustomBoxLayout for Opaque {
    fn layout(
        &mut self,
        c: BoxConstraints,
        _: &mut BoxChildren<'_>,
    ) -> Result<BoxGeometry, LayoutError> {
        self.0.set(self.0.get() + 1);
        Ok(BoxGeometry::new(c.constrain(vec2(10.0, 10.0))?))
    }
}
struct InvalidQuery {
    size: egui::Vec2,
    value: f32,
}
impl CustomBoxLayout for InvalidQuery {
    fn layout(
        &mut self,
        c: BoxConstraints,
        _: &mut BoxChildren<'_>,
    ) -> Result<BoxGeometry, LayoutError> {
        Ok(BoxGeometry::new(c.constrain(vec2(10.0, 10.0))?))
    }
    fn dry_layout(&self, _: BoxConstraints, _: &BoxQueries<'_>) -> Result<egui::Vec2, LayoutError> {
        Ok(self.size)
    }
    fn intrinsic(
        &self,
        _: IntrinsicDimension,
        _: f32,
        _: &BoxQueries<'_>,
    ) -> Result<f32, LayoutError> {
        Ok(self.value)
    }
}
#[test]
fn custom_queries_validate_results_and_do_not_fall_back_to_wet_layout() {
    let calls = Rc::new(Cell::new(0));
    let handle = CustomBox::new(Opaque(calls.clone()));
    let mut opaque = BoxNode::new(
        "opaque",
        BoxKind::Custom {
            layout: handle,
            children: vec![],
        },
    );
    let c = BoxConstraints::loose(vec2(100.0, 100.0)).unwrap();
    assert!(matches!(
        opaque.dry_layout(c),
        Err(LayoutError::UnsupportedQuery(_))
    ));
    assert!(matches!(
        opaque.max_intrinsic_width(100.0),
        Err(LayoutError::UnsupportedQuery(_))
    ));
    assert_eq!(calls.get(), 0);
    opaque.layout(c).unwrap();
    assert_eq!(calls.get(), 1);
    for value in [f32::NAN, f32::INFINITY, -1.0] {
        let bad = BoxNode::new(
            "bad",
            BoxKind::Custom {
                layout: CustomBox::new(InvalidQuery {
                    size: vec2(value, 20.0),
                    value,
                }),
                children: vec![],
            },
        );
        assert!(matches!(
            bad.dry_layout(c),
            Err(LayoutError::ChildViolatedConstraints(_))
        ));
        assert!(matches!(
            bad.max_intrinsic_width(20.0),
            Err(LayoutError::InvalidConfiguration(_))
        ));
    }
}
#[test]
fn tight_intrinsic_axes_skip_unneeded_child_queries_and_extreme_finite_steps_remain_finite() {
    let calls = Rc::new(Cell::new(0));
    let mut node = BoxNode::intrinsic_width(
        "width",
        None,
        None,
        BoxNode::new(
            "opaque",
            BoxKind::Custom {
                layout: CustomBox::new(Opaque(calls.clone())),
                children: vec![],
            },
        ),
    );
    assert_eq!(
        node.layout(BoxConstraints::tight(vec2(15.0, 25.0)).unwrap())
            .unwrap()
            .size,
        vec2(15.0, 25.0)
    );
    assert_eq!(calls.get(), 1);
    assert!(matches!(
        node.layout(BoxConstraints::loose(vec2(100.0, 100.0)).unwrap()),
        Err(LayoutError::UnsupportedQuery(_))
    ));
    let tiny = BoxNode::intrinsic_width(
        "tiny-step",
        Some(f32::MIN_POSITIVE),
        None,
        BoxNode::leaf("leaf", vec2(33.0, 20.0)),
    );
    assert_eq!(
        tiny.dry_layout(BoxConstraints::default()).unwrap(),
        vec2(33.0, 20.0)
    );
    let huge = BoxNode::intrinsic_width(
        "huge-step",
        Some(f32::MAX),
        None,
        BoxNode::leaf("leaf", vec2(f32::MIN_POSITIVE, 20.0)),
    );
    assert_eq!(huge.max_intrinsic_width(100.0).unwrap(), f32::MAX);
}
#[test]
fn intrinsic_widgets_retain_child_state_after_step_changes_and_query_errors() {
    #[derive(Default)]
    struct Data {
        marker: usize,
    }
    struct Child;
    impl StatefulWidget for Child {
        type State = Data;
        fn create_state(&self) -> Data {
            Data::default()
        }
    }
    impl State<Child> for Data {
        fn build(&mut self, _: &Child, _: &StateContext<Self>) -> Widget {
            Widget::leaf(vec2(33.0, 17.0), |_| {})
        }
    }
    let ctx = egui::Context::default();
    let root =
        |step| Widget::intrinsic_width(Some(step), None, Widget::stateful(Child).keyed("child"));
    let mut tree = ElementTree::new(&ctx, root(10.0)).unwrap();
    let child = tree
        .elements()
        .iter()
        .find(|e| e.key == Some(Key::from("child")))
        .unwrap()
        .id;
    tree.state_handle::<Data>(child)
        .unwrap()
        .update(|state| state.marker = 42)
        .unwrap();
    let c = BoxConstraints::loose(vec2(200.0, 200.0)).unwrap();
    assert_eq!(tree.dry_layout(c).unwrap().x, 40.0);
    assert_eq!(tree.state::<Data>(child).unwrap().marker, 42);
    tree.update(root(0.0)).unwrap();
    assert!(tree.dry_layout(c).is_err());
    assert!(tree.state::<Data>(child).is_some());
    tree.update(root(25.0)).unwrap();
    assert_eq!(tree.layout(c).unwrap().size.x, 50.0);
    assert_eq!(tree.state::<Data>(child).unwrap().marker, 42);
}

#[test]
fn query_paths_validate_tree_identity_depth_and_child_indices() {
    let duplicate = BoxNode::flex(
        "row",
        Flex::new(
            Axis::Horizontal,
            vec![
                FlexChild::fixed(BoxNode::leaf("same", vec2(10.0, 10.0))),
                FlexChild::fixed(BoxNode::leaf("same", vec2(10.0, 10.0))),
            ],
        ),
    );
    assert!(matches!(
        duplicate.dry_layout(BoxConstraints::default()),
        Err(LayoutError::DuplicateKey(_))
    ));
    assert!(matches!(
        duplicate.max_intrinsic_width(10.0),
        Err(LayoutError::DuplicateKey(_))
    ));
    let mut deep = BoxNode::leaf("leaf", vec2(1.0, 1.0));
    for level in 0..260 {
        deep = BoxNode::padding(level, Insets::default(), deep);
    }
    assert_eq!(
        deep.dry_layout(BoxConstraints::default()),
        Err(LayoutError::TreeTooDeep)
    );
    struct BadIndex;
    impl CustomBoxLayout for BadIndex {
        fn layout(
            &mut self,
            _: BoxConstraints,
            _: &mut BoxChildren<'_>,
        ) -> Result<BoxGeometry, LayoutError> {
            unreachable!("queries must not run wet layout")
        }
        fn dry_layout(
            &self,
            c: BoxConstraints,
            children: &BoxQueries<'_>,
        ) -> Result<egui::Vec2, LayoutError> {
            children.dry_layout(99, c)
        }
        fn intrinsic(
            &self,
            d: IntrinsicDimension,
            extent: f32,
            children: &BoxQueries<'_>,
        ) -> Result<f32, LayoutError> {
            children.intrinsic(99, d, extent)
        }
    }
    let tree = BoxNode::new(
        "bad",
        BoxKind::Custom {
            layout: CustomBox::new(BadIndex),
            children: vec![],
        },
    );
    assert!(matches!(
        tree.dry_layout(BoxConstraints::default()),
        Err(LayoutError::InvalidConfiguration(_))
    ));
    assert!(matches!(
        tree.max_intrinsic_width(10.0),
        Err(LayoutError::InvalidConfiguration(_))
    ));
}

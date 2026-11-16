use rust_flutter::{
    egui::{self, vec2},
    element::*,
    layout::*,
};
use std::{cell::Cell, rc::Rc};
const A: TextBaseline = TextBaseline::Alphabetic;
const I: TextBaseline = TextBaseline::Ideographic;
fn metrics(a: f32, i: f32) -> Baselines {
    Baselines {
        alphabetic: Some(a),
        ideographic: Some(i),
    }
}
fn leaf(key: &str, size: egui::Vec2, a: f32, i: f32) -> BoxNode {
    BoxNode::baseline_leaf(key, size, metrics(a, i))
}
fn loose() -> BoxConstraints {
    BoxConstraints::loose(vec2(200.0, 200.0)).unwrap()
}
fn row(kind: TextBaseline, up: bool, rtl: bool, missing: bool) -> BoxNode {
    let mut children = vec![
        FlexChild::fixed(leaf("small", vec2(10.0, 20.0), 8.0, 11.0)),
        FlexChild::fixed(leaf("large", vec2(15.0, 30.0), 20.0, 24.0)),
    ];
    if missing {
        children.push(FlexChild::fixed(BoxNode::leaf("missing", vec2(12.0, 40.0))));
    }
    let mut flex = Flex::new(Axis::Horizontal, children);
    flex.main_size = MainAxisSize::Min;
    flex.spacing = 2.0;
    flex.cross_alignment = CrossAxisAlignment::Baseline(kind);
    if up {
        flex.vertical_direction = VerticalDirection::Up;
    }
    if rtl {
        flex.text_direction = TextDirection::Rtl;
    }
    BoxNode::flex("row", flex)
}
#[test]
fn rows_align_real_baselines_and_include_ascent_plus_descent() {
    for up in [false, true] {
        for rtl in [false, true] {
            let mut tree = row(A, up, rtl, false);
            assert_eq!(tree.dry_layout(loose()).unwrap(), vec2(27.0, 32.0));
            assert_eq!(tree.dry_baseline(loose(), A).unwrap(), Some(20.0));
            assert_eq!(tree.dry_baseline(loose(), I).unwrap(), Some(23.0));
            let layout = tree.layout(loose()).unwrap();
            assert_eq!(layout.size, vec2(27.0, 32.0));
            assert_eq!(layout.children[0].offset.y, 12.0);
            assert_eq!(layout.children[1].offset.y, 0.0);
            assert_eq!(layout.baseline(A), Some(20.0));
            assert_eq!(layout.baseline(I), Some(23.0));
            for child in &layout.children {
                assert_eq!(child.offset.y + child.layout.baseline(A).unwrap(), 20.0);
            }
            assert_eq!(layout.children[0].offset.x, if rtl { 17.0 } else { 0.0 });
        }
    }
}
#[test]
fn ideographic_alignment_and_missing_baselines_do_not_use_bottom_fallback_for_rows() {
    let mut tree = row(I, true, false, true);
    let dry = tree.dry_layout(loose()).unwrap();
    let layout = tree.layout(loose()).unwrap();
    assert_eq!(dry, vec2(41.0, 40.0));
    assert_eq!(layout.size, dry);
    assert_eq!(layout.children[0].offset.y, 13.0);
    assert_eq!(layout.children[1].offset.y, 0.0);
    assert_eq!(layout.children[2].offset.y, 0.0);
    assert_eq!(layout.children[2].layout.baseline(I), None);
    assert_eq!(layout.baseline(I), Some(24.0));
    assert_eq!(layout.baseline(A), Some(20.0));
    assert_eq!(tree.dry_baseline(loose(), A).unwrap(), Some(20.0));
    let fallback = &layout.children[2].layout;
    assert_eq!(fallback.distance_to_baseline(A, true), None);
    assert_eq!(fallback.distance_to_baseline(A, false), Some(40.0));
}
#[test]
fn column_baselines_follow_child_list_order_with_reversed_visual_order() {
    let mut flex = Flex::new(
        Axis::Vertical,
        vec![
            FlexChild::fixed(leaf("first", vec2(10.0, 20.0), 8.0, 11.0)),
            FlexChild::fixed(leaf("second", vec2(15.0, 30.0), 20.0, 24.0)),
        ],
    );
    flex.main_size = MainAxisSize::Min;
    flex.spacing = 2.0;
    flex.vertical_direction = VerticalDirection::Up;
    flex.text_direction = TextDirection::Rtl;
    flex.cross_alignment = CrossAxisAlignment::Baseline(A);
    let mut tree = BoxNode::flex("column", flex);
    let layout = tree.layout(loose()).unwrap();
    assert_eq!(layout.children[0].offset, vec2(0.0, 32.0));
    assert_eq!(layout.baseline(A), Some(40.0));
    assert_eq!(tree.dry_baseline(loose(), A).unwrap(), Some(40.0));
}
#[test]
fn shifted_and_intrinsic_wrappers_translate_baselines_without_inventing_real_ones() {
    let mut tree = BoxNode::baseline(
        "baseline",
        30.0,
        A,
        BoxNode::padding(
            "pad",
            Insets::all(5.0),
            leaf("leaf", vec2(20.0, 20.0), 8.0, 11.0),
        ),
    );
    let layout = tree.layout(loose()).unwrap();
    assert_eq!(layout.size, vec2(30.0, 47.0));
    assert_eq!(layout.children[0].offset.y, 17.0);
    assert_eq!(layout.baseline(A), Some(30.0));
    assert_eq!(layout.baseline(I), Some(33.0));
    assert_eq!(tree.dry_layout(loose()).unwrap(), layout.size);
    assert_eq!(tree.dry_baseline(loose(), I).unwrap(), Some(33.0));
    let mut ignored = BoxNode::ignore_baseline("ignore", tree);
    let geometry = ignored.layout(loose()).unwrap();
    assert_eq!(geometry.baseline(A), None);
    assert_eq!(geometry.children[0].layout.baseline(A), Some(30.0));
    assert_eq!(ignored.dry_baseline(loose(), A).unwrap(), None);
    let mut no_real = BoxNode::baseline("bottom", 30.0, A, BoxNode::leaf("leaf", vec2(20.0, 20.0)));
    let geometry = no_real.layout(loose()).unwrap();
    assert_eq!(geometry.size.y, 30.0);
    assert_eq!(geometry.children[0].offset.y, 10.0);
    assert_eq!(geometry.baseline(A), None);
    assert_eq!(no_real.dry_baseline(loose(), A).unwrap(), None);
    let mut other_kind = BoxNode::baseline(
        "other-kind",
        30.0,
        I,
        BoxNode::baseline_leaf(
            "alphabetic-only",
            vec2(20.0, 20.0),
            Baselines {
                alphabetic: Some(8.0),
                ideographic: None,
            },
        ),
    );
    let geometry = other_kind.layout(loose()).unwrap();
    assert_eq!(geometry.baseline(A), Some(18.0));
    assert_eq!(geometry.baseline(I), None);
    assert_eq!(other_kind.dry_baseline(loose(), A).unwrap(), Some(18.0));
    assert_eq!(other_kind.dry_baseline(loose(), I).unwrap(), None);
    let mut intrinsic = BoxNode::intrinsic_width(
        "width",
        Some(10.0),
        None,
        leaf("metric", vec2(33.0, 20.0), 8.0, 11.0),
    );
    assert_eq!(intrinsic.layout(loose()).unwrap().baseline(A), Some(8.0));
    assert_eq!(intrinsic.dry_baseline(loose(), A).unwrap(), Some(8.0));
}
#[test]
fn negative_offsets_preserve_overflow_but_do_not_expand_parent_hit_bounds() {
    let mut tree = BoxNode::baseline(
        "baseline",
        2.0,
        A,
        leaf("child", vec2(20.0, 20.0), 8.0, 11.0),
    );
    let layout = tree.layout(loose()).unwrap();
    assert_eq!(layout.size.y, 14.0);
    assert_eq!(layout.children[0].offset.y, -6.0);
    assert_eq!(layout.overflow.top, 6.0);
    assert_eq!(layout.hit_test(egui::pos2(10.0, -1.0)), None);
    assert_eq!(
        layout.hit_test(egui::pos2(10.0, 1.0)),
        Some(egui::Id::new("child"))
    );
    assert_eq!(layout.baseline(A), Some(2.0));
    assert_eq!(tree.dry_baseline(loose(), A).unwrap(), Some(2.0));
    let mut negative = BoxNode::baseline(
        "negative",
        -30.0,
        A,
        leaf("child", vec2(20.0, 20.0), 8.0, 11.0),
    );
    assert_eq!(negative.layout(loose()).unwrap().size.y, 0.0);
    assert_eq!(negative.dry_layout(loose()).unwrap().y, 0.0);
}
#[test]
fn positioned_stacks_report_the_highest_translated_real_baseline() {
    for fit in [StackFit::Loose, StackFit::Passthrough, StackFit::Expand] {
        let mut stack = Stack::new(vec![
            StackChild::new(leaf("natural", vec2(80.0, 70.0), 40.0, 45.0)),
            StackChild::positioned(
                leaf("top", vec2(20.0, 20.0), 8.0, 11.0),
                Positioned {
                    top: Some(10.0),
                    left: Some(0.0),
                    ..Default::default()
                },
            ),
            StackChild::positioned(
                leaf("bottom", vec2(20.0, 20.0), 8.0, 11.0),
                Positioned {
                    bottom: Some(5.0),
                    right: Some(0.0),
                    ..Default::default()
                },
            ),
        ]);
        stack.fit = fit;
        stack.alignment = Alignment::CENTER;
        let mut tree = BoxNode::stack("stack", stack);
        let layout = tree.layout(loose()).unwrap();
        assert_eq!(layout.baseline(A), Some(18.0));
        assert_eq!(tree.dry_baseline(loose(), A).unwrap(), Some(18.0));
        assert_eq!(layout.baseline(I), Some(21.0));
        assert_eq!(tree.dry_baseline(loose(), I).unwrap(), Some(21.0));
    }
}
struct MetricPolicy {
    calls: Rc<Cell<usize>>,
}
impl CustomBoxLayout for MetricPolicy {
    fn dry_layout(&self, c: BoxConstraints, _: &BoxQueries<'_>) -> Result<egui::Vec2, LayoutError> {
        c.constrain(vec2(20.0, 20.0))
    }
    fn dry_baseline(
        &self,
        _: BoxConstraints,
        kind: TextBaseline,
        _: &BoxQueries<'_>,
    ) -> Result<Option<f32>, LayoutError> {
        Ok(metrics(8.0, 11.0).get(kind))
    }
    fn layout(
        &mut self,
        c: BoxConstraints,
        _: &mut BoxChildren<'_>,
    ) -> Result<BoxGeometry, LayoutError> {
        self.calls.set(self.calls.get() + 1);
        let mut geometry = BoxGeometry::new(c.constrain(vec2(20.0, 20.0))?);
        geometry.baselines = metrics(8.0, 11.0);
        Ok(geometry)
    }
}
#[test]
fn dry_baseline_queries_do_not_execute_layout_or_discard_a_custom_cache() {
    let calls = Rc::new(Cell::new(0));
    let mut tree = BoxNode::new(
        "custom",
        BoxKind::Custom {
            layout: CustomBox::new(MetricPolicy {
                calls: calls.clone(),
            }),
            children: vec![],
        },
    );
    assert_eq!(tree.dry_baseline(loose(), A).unwrap(), Some(8.0));
    assert_eq!(calls.get(), 0);
    tree.layout(loose()).unwrap();
    tree.dry_baseline(loose(), I).unwrap();
    tree.layout(loose()).unwrap();
    assert_eq!(calls.get(), 1);
}
#[test]
fn baseline_flex_intrinsics_include_baseline_group_height() {
    let tree = row(A, false, false, false);
    assert_eq!(tree.max_intrinsic_height(100.0).unwrap(), 32.0);
    assert_eq!(tree.min_intrinsic_height(100.0).unwrap(), 32.0);
    assert_eq!(tree.max_intrinsic_width(100.0).unwrap(), 27.0);
}
#[test]
fn baseline_widget_updates_preserve_child_state_and_recover_from_invalid_targets() {
    struct Child;
    #[derive(Default)]
    struct Data {
        marker: usize,
    }
    impl StatefulWidget for Child {
        type State = Data;
        fn create_state(&self) -> Data {
            Data::default()
        }
    }
    impl State<Child> for Data {
        fn build(&mut self, _: &Child, _: &StateContext<Self>) -> Widget {
            Widget::baseline_leaf(vec2(20.0, 20.0), metrics(8.0, 11.0), |_| {})
        }
    }
    let ctx = egui::Context::default();
    let root = |distance| Widget::baseline(distance, A, Widget::stateful(Child).keyed("child"));
    let mut tree = ElementTree::new(&ctx, root(30.0)).unwrap();
    let child = tree
        .elements()
        .iter()
        .find(|e| e.key == Some(Key::from("child")))
        .unwrap()
        .id;
    tree.state_handle::<Data>(child)
        .unwrap()
        .update(|s| s.marker = 42)
        .unwrap();
    assert_eq!(tree.dry_baseline(loose(), A).unwrap(), Some(30.0));
    assert_eq!(tree.layout(loose()).unwrap().baseline(A), Some(30.0));
    tree.update(root(f32::NAN)).unwrap();
    assert!(tree.layout(loose()).is_err());
    assert!(tree.state::<Data>(child).is_some());
    tree.update(root(10.0)).unwrap();
    assert_eq!(tree.layout(loose()).unwrap().baseline(A), Some(10.0));
    assert_eq!(tree.state::<Data>(child).unwrap().marker, 42);
}

#[test]
fn baseline_rows_keep_wet_and_dry_results_equal_under_flex_and_tight_constraints() {
    for kind in [A, I] {
        for fit in [FlexFit::Loose, FlexFit::Tight] {
            for reverse in [false, true] {
                for c in [
                    loose(),
                    BoxConstraints::tight(vec2(80.0, 10.0)).unwrap(),
                    BoxConstraints::loose(vec2(80.0, 100.0)).unwrap(),
                ] {
                    let mut flex = Flex::new(
                        Axis::Horizontal,
                        vec![
                            FlexChild::flexible(leaf("small", vec2(10.0, 20.0), 8.0, 11.0), 1, fit),
                            FlexChild::flexible(
                                leaf("large", vec2(15.0, 30.0), 20.0, 24.0),
                                2,
                                FlexFit::Loose,
                            ),
                        ],
                    );
                    flex.cross_alignment = CrossAxisAlignment::Baseline(kind);
                    if reverse {
                        flex.text_direction = TextDirection::Rtl;
                        flex.vertical_direction = VerticalDirection::Up;
                    }
                    let mut tree = BoxNode::flex("row", flex);
                    let dry = tree.dry_layout(c).unwrap();
                    let a = tree.dry_baseline(c, A).unwrap();
                    let i = tree.dry_baseline(c, I).unwrap();
                    let wet = tree.layout(c).unwrap();
                    assert_eq!(dry, wet.size);
                    assert_eq!(a, wet.baseline(A));
                    assert_eq!(i, wet.baseline(I));
                }
            }
        }
    }
}
struct PartialPolicy;
impl CustomBoxLayout for PartialPolicy {
    fn layout(
        &mut self,
        c: BoxConstraints,
        _: &mut BoxChildren<'_>,
    ) -> Result<BoxGeometry, LayoutError> {
        let mut g = BoxGeometry::new(c.constrain(vec2(20.0, 20.0))?);
        g.baselines = metrics(8.0, 11.0);
        Ok(g)
    }
    fn dry_baseline(
        &self,
        _: BoxConstraints,
        kind: TextBaseline,
        _: &BoxQueries<'_>,
    ) -> Result<Option<f32>, LayoutError> {
        Ok(metrics(8.0, 11.0).get(kind))
    }
}
#[test]
fn baseline_queries_can_avoid_unknown_positioned_child_sizes_but_do_not_fake_support() {
    let positioned = |position| {
        BoxNode::stack(
            "stack",
            Stack::new(vec![
                StackChild::new(BoxNode::leaf("base", vec2(80.0, 70.0))),
                StackChild::positioned(
                    BoxNode::new(
                        "custom",
                        BoxKind::Custom {
                            layout: CustomBox::new(PartialPolicy),
                            children: vec![],
                        },
                    ),
                    position,
                ),
            ]),
        )
    };
    let mut top = positioned(Positioned {
        top: Some(10.0),
        ..Default::default()
    });
    assert_eq!(top.dry_baseline(loose(), A).unwrap(), Some(18.0));
    assert_eq!(top.layout(loose()).unwrap().baseline(A), Some(18.0));
    let bottom = positioned(Positioned {
        bottom: Some(10.0),
        ..Default::default()
    });
    assert!(matches!(
        bottom.dry_baseline(loose(), A),
        Err(LayoutError::UnsupportedQuery(_))
    ));
    let measured = BoxNode::measured("opaque", |_| {
        panic!("must not run wet layout for baseline queries")
    });
    assert_eq!(measured.dry_baseline(loose(), A).unwrap(), None);
    let ignored = BoxNode::ignore_baseline("ignore", bottom);
    assert_eq!(ignored.dry_baseline(loose(), A).unwrap(), None);
}
#[test]
fn nonfinite_baselines_are_errors_while_finite_outside_box_distances_remain_valid() {
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut tree = leaf("bad", vec2(20.0, 20.0), value, 10.0);
        assert!(tree.layout(loose()).is_err());
        assert!(tree.dry_baseline(loose(), A).is_err());
        let mut target =
            BoxNode::baseline("target", value, A, BoxNode::leaf("leaf", vec2(20.0, 20.0)));
        assert!(target.layout(loose()).is_err());
        assert!(target.dry_layout(loose()).is_err());
    }
    let mut outside = leaf("outside", vec2(20.0, 20.0), -5.0, 30.0);
    assert_eq!(outside.layout(loose()).unwrap().baseline(A), Some(-5.0));
    assert_eq!(outside.dry_baseline(loose(), I).unwrap(), Some(30.0));
}
struct Proxy(Rc<Cell<usize>>);
impl RenderObjectWidget for Proxy {
    type Object = ProxyBox;
    fn create_render_object(&self, _: &RenderContext) -> ProxyBox {
        ProxyBox(self.0.clone())
    }
    fn update_render_object(&self, _: &mut ProxyBox, _: &RenderContext) {}
}
struct ProxyBox(Rc<Cell<usize>>);
impl RenderBox for ProxyBox {
    fn dry_baseline(
        &self,
        c: BoxConstraints,
        kind: TextBaseline,
        children: &BoxQueries<'_>,
    ) -> Result<Option<f32>, LayoutError> {
        Ok(children
            .dry_baseline(0, c.deflate(Insets::all(5.0))?, kind)?
            .map(|b| b + 5.0))
    }
    fn layout(
        &mut self,
        c: BoxConstraints,
        children: &mut BoxChildren<'_>,
        _: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        self.0.set(self.0.get() + 1);
        assert!(children.baseline(0, A, true).is_err());
        let size = children.layout(0, c.deflate(Insets::all(5.0))?)?;
        children.place(0, vec2(5.0, 5.0))?;
        let mut geometry = BoxGeometry::new(c.constrain(size + vec2(10.0, 10.0))?);
        geometry.baselines = Baselines {
            alphabetic: children.baseline(0, A, true)?.map(|b| b + 5.0),
            ideographic: children.baseline(0, I, true)?.map(|b| b + 5.0),
        };
        Ok(geometry)
    }
}
#[test]
fn retained_custom_boxes_report_laid_out_child_baselines_and_query_without_invalidating_cache() {
    let ctx = egui::Context::default();
    let calls = Rc::new(Cell::new(0));
    let mut tree = ElementTree::new(
        &ctx,
        Widget::render_object(
            Proxy(calls.clone()),
            vec![Widget::baseline_leaf(
                vec2(20.0, 20.0),
                metrics(8.0, 11.0),
                |_| {},
            )],
        ),
    )
    .unwrap();
    assert_eq!(tree.dry_baseline(loose(), A).unwrap(), Some(13.0));
    assert_eq!(calls.get(), 0);
    assert_eq!(tree.layout(loose()).unwrap().baseline(I), Some(16.0));
    assert_eq!(calls.get(), 1);
    tree.dry_baseline(loose(), I).unwrap();
    tree.layout(loose()).unwrap();
    assert_eq!(calls.get(), 1);
}

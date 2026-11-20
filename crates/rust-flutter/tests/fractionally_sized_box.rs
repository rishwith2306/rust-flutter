use rust_flutter::{
    egui::{self, vec2},
    element::*,
    layout::*,
};
fn widget(w: Option<f32>, h: Option<f32>, alignment: Alignment) -> Widget {
    FractionallySizedBox {
        width_factor: w,
        height_factor: h,
        alignment,
    }
    .into_widget(
        Widget::baseline_leaf(
            vec2(20.0, 10.0),
            Baselines {
                alphabetic: Some(8.0),
                ideographic: Some(9.0),
            },
            |_| {},
        )
        .keyed("content"),
    )
}
fn tree(w: Option<f32>, h: Option<f32>) -> ElementTree {
    ElementTree::new(&egui::Context::default(), widget(w, h, Alignment::CENTER)).unwrap()
}
#[test]
fn fractional_axes_are_tight_and_box_shrink_wraps_under_loose_constraints() {
    let mut t = tree(Some(0.5), Some(0.25));
    let c = BoxConstraints::loose(vec2(200.0, 80.0)).unwrap();
    let g = t.layout(c).unwrap();
    assert_eq!(g.size, vec2(100.0, 20.0));
    assert_eq!(g.children[0].layout.size, g.size);
    assert_eq!(g.children[0].offset, vec2(0.0, 0.0));
    assert_eq!(t.dry_layout(c).unwrap(), g.size);
}
#[test]
fn tight_parent_keeps_own_size_and_alignment_places_smaller_child() {
    let mut t = tree(Some(0.5), Some(0.25));
    let c = BoxConstraints::tight(vec2(200.0, 80.0)).unwrap();
    for (a, offset) in [
        (Alignment::TOP_LEFT, vec2(0.0, 0.0)),
        (Alignment::CENTER, vec2(50.0, 30.0)),
        (Alignment::BOTTOM_RIGHT, vec2(100.0, 60.0)),
        (Alignment { x: 2.0, y: 2.0 }, vec2(150.0, 90.0)),
    ] {
        t.update(widget(Some(0.5), Some(0.25), a)).unwrap();
        let g = t.layout(c).unwrap();
        assert_eq!(g.size, c.min());
        assert_eq!(g.children[0].layout.size, vec2(100.0, 20.0));
        assert_eq!(g.children[0].offset, offset);
        assert_eq!(g.baseline(TextBaseline::Alphabetic), Some(8.0 + offset.y));
        assert_eq!(
            t.dry_baseline(c, TextBaseline::Alphabetic).unwrap(),
            g.baseline(TextBaseline::Alphabetic)
        );
        assert_eq!(
            t.dry_baseline(c, TextBaseline::Ideographic).unwrap(),
            Some(9.0 + offset.y)
        );
    }
}
#[test]
fn larger_factors_overflow_without_clipping_and_translate_negative_baselines() {
    let mut t = tree(Some(2.0), Some(3.0));
    let c = BoxConstraints::loose(vec2(40.0, 30.0)).unwrap();
    let g = t.layout(c).unwrap();
    assert_eq!(g.size, vec2(40.0, 30.0));
    assert_eq!(g.children[0].layout.size, vec2(80.0, 90.0));
    assert_eq!(g.children[0].offset, vec2(-20.0, -30.0));
    assert_eq!(g.clip, Clip::None);
    assert_eq!(g.baseline(TextBaseline::Alphabetic), Some(-22.0));
    assert_eq!(
        t.dry_baseline(c, TextBaseline::Alphabetic).unwrap(),
        Some(-22.0)
    );
}
#[test]
fn absent_factors_pass_parent_minimums_and_unbounded_axes_to_child() {
    let mut t = tree(None, None);
    let c = BoxConstraints::new(vec2(30.0, 25.0), vec2(f32::INFINITY, f32::INFINITY)).unwrap();
    assert_eq!(t.layout(c).unwrap().size, vec2(30.0, 25.0));
    assert_eq!(t.dry_layout(c).unwrap(), vec2(30.0, 25.0));
    t.update(widget(Some(0.5), None, Alignment::CENTER))
        .unwrap();
    let c = BoxConstraints::new(vec2(90.0, 25.0), vec2(100.0, f32::INFINITY)).unwrap();
    let g = t.layout(c).unwrap();
    assert_eq!(g.size, vec2(90.0, 25.0));
    assert_eq!(g.children[0].layout.size, vec2(50.0, 25.0));
    assert_eq!(g.children[0].offset, vec2(20.0, 0.0));
}
#[test]
fn zero_factors_work_with_bounded_parents_and_invalid_configuration_is_rejected() {
    let mut t = tree(Some(0.0), Some(0.0));
    let c = BoxConstraints::tight(vec2(50.0, 40.0)).unwrap();
    let g = t.layout(c).unwrap();
    assert_eq!(g.children[0].layout.size, vec2(0.0, 0.0));
    assert_eq!(g.children[0].offset, vec2(25.0, 20.0));
    for f in [f32::NAN, -0.1, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(FractionallySizedBox::new(Some(f), None).is_err());
        assert!(FractionallySizedBox::new(None, Some(f)).is_err());
    }
    t.update(widget(Some(-1.0), None, Alignment::CENTER))
        .unwrap();
    assert!(t.dry_layout(c).is_err());
    t.update(widget(
        None,
        None,
        Alignment {
            x: f32::NAN,
            y: 0.0,
        },
    ))
    .unwrap();
    assert!(t.layout(c).is_err());
}
#[test]
fn scaled_unbounded_and_overflowed_dimensions_return_errors_and_tree_recovers() {
    let mut t = tree(Some(0.5), None);
    assert!(matches!(
        t.layout(BoxConstraints::default()),
        Err(ElementError::Layout(LayoutError::UnboundedSize))
    ));
    assert!(t.dry_layout(BoxConstraints::default()).is_err());
    t.update(widget(Some(0.0), None, Alignment::CENTER))
        .unwrap();
    assert!(t.layout(BoxConstraints::default()).is_err());
    t.update(widget(Some(f32::MAX), None, Alignment::CENTER))
        .unwrap();
    assert!(
        t.layout(BoxConstraints::loose(vec2(2.0, 20.0)).unwrap())
            .is_err()
    );
    t.update(widget(Some(0.5), None, Alignment::CENTER))
        .unwrap();
    assert_eq!(
        t.layout(BoxConstraints::loose(vec2(80.0, 20.0)).unwrap())
            .unwrap()
            .size,
        vec2(40.0, 10.0)
    );
}
#[test]
fn intrinsics_scale_opposite_extent_and_divide_the_requested_axis() {
    #[derive(Clone)]
    struct Probe;
    impl RenderObjectWidget for Probe {
        type Object = Self;
        fn create_render_object(&self, _: &RenderContext) -> Self {
            Self
        }
        fn update_render_object(&self, _: &mut Self, _: &RenderContext) {}
    }
    impl RenderBox for Probe {
        fn layout(
            &mut self,
            _: BoxConstraints,
            _: &mut BoxChildren<'_>,
            _: &RenderContext,
        ) -> Result<BoxGeometry, LayoutError> {
            panic!("pure query must not layout")
        }
        fn intrinsic(
            &self,
            d: IntrinsicDimension,
            extent: f32,
            _: &BoxQueries<'_>,
        ) -> Result<f32, LayoutError> {
            Ok(extent
                + match d {
                    IntrinsicDimension::MinWidth | IntrinsicDimension::MinHeight => 2.0,
                    _ => 4.0,
                })
        }
    }
    let mut t = ElementTree::new(
        &egui::Context::default(),
        FractionallySizedBox::new(Some(0.5), Some(0.25))
            .unwrap()
            .into_widget(Widget::render_object(Probe, vec![])),
    )
    .unwrap();
    for (d, expected) in [
        (IntrinsicDimension::MinWidth, 14.0),
        (IntrinsicDimension::MaxWidth, 18.0),
        (IntrinsicDimension::MinHeight, 48.0),
        (IntrinsicDimension::MaxHeight, 56.0),
    ] {
        assert_eq!(t.intrinsic(d, 20.0).unwrap(), expected);
    }
    t.update(
        FractionallySizedBox::new(Some(1.0), Some(f32::MAX))
            .unwrap()
            .into_widget(Widget::render_object(Probe, vec![])),
    )
    .unwrap();
    assert!(t.intrinsic(IntrinsicDimension::MaxWidth, 2.0).is_err());
    t.update(widget(Some(0.0), Some(0.0), Alignment::CENTER))
        .unwrap();
    assert!(t.intrinsic(IntrinsicDimension::MaxWidth, 10.0).is_err());
    assert!(
        t.intrinsic(IntrinsicDimension::MaxHeight, f32::INFINITY)
            .is_err()
    );
}
#[test]
fn no_child_sizes_from_fractional_constraints_and_multiple_children_are_rejected() {
    let config = FractionallySizedBox::new(Some(0.5), Some(0.25)).unwrap();
    let mut t = ElementTree::new(
        &egui::Context::default(),
        Widget::render_object(config, vec![]),
    )
    .unwrap();
    let c = BoxConstraints::loose(vec2(200.0, 80.0)).unwrap();
    assert_eq!(t.layout(c).unwrap().size, vec2(100.0, 20.0));
    assert_eq!(t.dry_layout(c).unwrap(), vec2(100.0, 20.0));
    assert_eq!(t.dry_baseline(c, TextBaseline::Alphabetic).unwrap(), None);
    assert_eq!(
        t.intrinsic(IntrinsicDimension::MaxWidth, 10.0).unwrap(),
        0.0
    );
    t.update(Widget::render_object(
        config,
        vec![Widget::empty(), Widget::empty()],
    ))
    .unwrap();
    assert!(t.layout(c).is_err());
}
#[test]
fn configuration_changes_keep_child_identity_and_clear_previous_fractional_constraints() {
    let mut t = tree(Some(0.5), Some(0.25));
    let c = BoxConstraints::tight(vec2(200.0, 80.0)).unwrap();
    let first = t.layout(c).unwrap();
    t.update(widget(None, None, Alignment::TOP_LEFT)).unwrap();
    let second = t.layout(c).unwrap();
    assert_eq!(first.children[0].layout.id, second.children[0].layout.id);
    assert_eq!(second.children[0].layout.size, c.min());
    assert_eq!(second.children[0].offset, vec2(0.0, 0.0));
}

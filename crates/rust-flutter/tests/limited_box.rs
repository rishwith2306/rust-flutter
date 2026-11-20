use rust_flutter::{
    egui::{self, vec2},
    element::*,
    layout::*,
};
fn tree(w: f32, h: f32) -> ElementTree {
    ElementTree::new(
        &egui::Context::default(),
        LimitedBox::new(w, h)
            .unwrap()
            .into_widget(Widget::measured(|c| Ok(c.max()), |_| {})),
    )
    .unwrap()
}
#[test]
fn bounded_axes_ignore_limits_and_unbounded_axes_receive_them() {
    let mut t = tree(30.0, 40.0);
    for (max, expected) in [
        (vec2(100.0, 80.0), vec2(100.0, 80.0)),
        (vec2(f32::INFINITY, 80.0), vec2(30.0, 80.0)),
        (vec2(100.0, f32::INFINITY), vec2(100.0, 40.0)),
        (vec2(f32::INFINITY, f32::INFINITY), vec2(30.0, 40.0)),
    ] {
        let g = t.layout(BoxConstraints::loose(max).unwrap()).unwrap();
        assert_eq!(g.size, expected);
        assert_eq!(g.children[0].layout.size, expected);
    }
}
#[test]
fn parent_minimum_overrides_smaller_limits() {
    let mut t = tree(30.0, 40.0);
    let c = BoxConstraints::new(vec2(70.0, 90.0), vec2(f32::INFINITY, f32::INFINITY)).unwrap();
    assert_eq!(t.layout(c).unwrap().size, vec2(70.0, 90.0));
    assert_eq!(
        t.layout(BoxConstraints::tight(vec2(80.0, 100.0)).unwrap())
            .unwrap()
            .size,
        vec2(80.0, 100.0)
    );
}
#[test]
fn limits_are_maxima_and_do_not_force_expansion() {
    let mut t = ElementTree::new(
        &egui::Context::default(),
        LimitedBox::new(30.0, 40.0)
            .unwrap()
            .into_widget(Widget::leaf(vec2(10.0, 12.0), |_| {})),
    )
    .unwrap();
    assert_eq!(
        t.layout(BoxConstraints::default()).unwrap().size,
        vec2(10.0, 12.0)
    );
    assert_eq!(
        t.dry_layout(BoxConstraints::default()).unwrap(),
        vec2(10.0, 12.0)
    );
    assert_eq!(
        t.intrinsic(IntrinsicDimension::MaxWidth, 0.0).unwrap(),
        10.0
    );
}
#[test]
fn dry_layout_and_baseline_use_the_same_limited_constraints_as_layout() {
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
        fn dry_layout(
            &self,
            c: BoxConstraints,
            _: &BoxQueries<'_>,
        ) -> Result<egui::Vec2, LayoutError> {
            Ok(c.max())
        }
        fn dry_baseline(
            &self,
            c: BoxConstraints,
            _: TextBaseline,
            _: &BoxQueries<'_>,
        ) -> Result<Option<f32>, LayoutError> {
            Ok(Some(c.max().y / 2.0))
        }
        fn layout(
            &mut self,
            c: BoxConstraints,
            _: &mut BoxChildren<'_>,
            _: &RenderContext,
        ) -> Result<BoxGeometry, LayoutError> {
            let mut g = BoxGeometry::new(c.max());
            g.baselines.alphabetic = Some(c.max().y / 2.0);
            Ok(g)
        }
    }
    let mut t = ElementTree::new(
        &egui::Context::default(),
        LimitedBox::new(30.0, 40.0)
            .unwrap()
            .into_widget(Widget::render_object(Probe, vec![])),
    )
    .unwrap();
    let c = BoxConstraints::default();
    assert_eq!(t.dry_layout(c).unwrap(), vec2(30.0, 40.0));
    assert_eq!(
        t.dry_baseline(c, TextBaseline::Alphabetic).unwrap(),
        Some(20.0)
    );
    assert_eq!(
        t.layout(c).unwrap().baseline(TextBaseline::Alphabetic),
        Some(20.0)
    );
}
#[test]
fn changing_limits_preserves_child_identity_and_refreshes_geometry() {
    let mut intrinsic_tree = ElementTree::new(
        &egui::Context::default(),
        LimitedBox::new(30.0, 40.0)
            .unwrap()
            .into_widget(Widget::leaf(vec2(200.0, 200.0), |_| {})),
    )
    .unwrap();
    assert_eq!(
        intrinsic_tree
            .intrinsic(IntrinsicDimension::MaxWidth, 0.0)
            .unwrap(),
        200.0
    );
    let widget = |w| {
        Widget::limited_box(
            w,
            40.0,
            Widget::leaf(vec2(100.0, 100.0), |_| {}).keyed("child"),
        )
        .unwrap()
    };
    let mut t = ElementTree::new(&egui::Context::default(), widget(30.0)).unwrap();
    let first = t.layout(BoxConstraints::default()).unwrap();
    t.update(widget(60.0)).unwrap();
    let second = t.layout(BoxConstraints::default()).unwrap();
    assert_eq!(first.children[0].layout.id, second.children[0].layout.id);
    assert_eq!(second.size, vec2(60.0, 40.0));
}
#[test]
fn zero_infinite_and_invalid_limits_are_checked_including_direct_configuration() {
    for value in [-1.0, f32::NEG_INFINITY, f32::NAN] {
        assert!(LimitedBox::new(value, 1.0).is_err());
        assert!(LimitedBox::new(1.0, value).is_err());
    }
    assert_eq!(
        tree(0.0, 0.0)
            .layout(BoxConstraints::default())
            .unwrap()
            .size,
        vec2(0.0, 0.0)
    );
    let mut t = tree(f32::INFINITY, f32::INFINITY);
    assert!(t.layout(BoxConstraints::default()).is_err());
    t.update(Widget::render_object(
        LimitedBox {
            max_width: f32::NAN,
            ..Default::default()
        },
        vec![],
    ))
    .unwrap();
    assert!(t.dry_layout(BoxConstraints::default()).is_err());
}
#[test]
fn absent_child_uses_parent_minimum_and_multiple_children_are_rejected() {
    let mut t = ElementTree::new(
        &egui::Context::default(),
        Widget::render_object(LimitedBox::default(), vec![]),
    )
    .unwrap();
    let c = BoxConstraints::new(vec2(12.0, 14.0), vec2(100.0, 100.0)).unwrap();
    assert_eq!(t.layout(c).unwrap().size, c.min());
    assert_eq!(t.dry_layout(c).unwrap(), c.min());
    assert_eq!(t.dry_baseline(c, TextBaseline::Alphabetic).unwrap(), None);
    assert_eq!(t.intrinsic(IntrinsicDimension::MaxWidth, 0.0).unwrap(), 0.0);
    t.update(Widget::render_object(
        LimitedBox::default(),
        vec![Widget::empty(), Widget::empty()],
    ))
    .unwrap();
    assert!(t.layout(c).is_err());
}

use rust_flutter::{
    egui::{self, vec2},
    element::*,
    layout::*,
};
use std::{cell::Cell, rc::Rc};
fn tree(ratio: f32) -> ElementTree {
    ElementTree::new(
        &egui::Context::default(),
        AspectRatio::new(ratio)
            .unwrap()
            .into_widget(Widget::leaf(vec2(1.0, 1.0), |_| {})),
    )
    .unwrap()
}
#[test]
fn bounded_maxima_choose_the_largest_fitting_ratio_and_tight_child() {
    let mut t = tree(2.0);
    let g = t
        .layout(BoxConstraints::loose(vec2(100.0, 100.0)).unwrap())
        .unwrap();
    assert_eq!(g.size, vec2(100.0, 50.0));
    assert_eq!(g.children[0].layout.size, g.size);
    let g = t
        .layout(BoxConstraints::loose(vec2(100.0, 30.0)).unwrap())
        .unwrap();
    assert_eq!(g.size, vec2(60.0, 30.0));
}
#[test]
fn unbounded_axis_uses_the_other_axis_and_both_unbounded_is_checked() {
    let mut t = tree(2.0);
    assert_eq!(
        t.layout(BoxConstraints::loose(vec2(f32::INFINITY, 30.0)).unwrap())
            .unwrap()
            .size,
        vec2(60.0, 30.0)
    );
    assert_eq!(
        t.layout(BoxConstraints::loose(vec2(100.0, f32::INFINITY)).unwrap())
            .unwrap()
            .size,
        vec2(100.0, 50.0)
    );
    assert!(matches!(
        t.layout(BoxConstraints::default()),
        Err(ElementError::Layout(LayoutError::UnboundedSize))
    ));
}
#[test]
fn tight_and_conflicting_constraints_override_ratio_without_invalid_child_sizes() {
    let mut t = tree(2.0);
    let c = BoxConstraints::tight(vec2(40.0, 40.0)).unwrap();
    assert_eq!(t.layout(c).unwrap().size, vec2(40.0, 40.0));
    let c = BoxConstraints::new(vec2(80.0, 60.0), vec2(100.0, 80.0)).unwrap();
    let g = t.layout(c).unwrap();
    assert_eq!(g.size, vec2(100.0, 60.0));
    assert_eq!(g.children[0].layout.size, g.size);
}
#[test]
fn pure_size_queries_do_not_measure_child_and_finite_intrinsics_use_ratio() {
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let mut t = ElementTree::new(
        &egui::Context::default(),
        AspectRatio::new(2.0).unwrap().into_widget(Widget::measured(
            move |c| {
                count.set(count.get() + 1);
                Ok(c.min())
            },
            |_| {},
        )),
    )
    .unwrap();
    assert_eq!(
        t.dry_layout(BoxConstraints::loose(vec2(100.0, 100.0)).unwrap())
            .unwrap(),
        vec2(100.0, 50.0)
    );
    assert_eq!(calls.get(), 0);
    assert_eq!(
        t.intrinsic(IntrinsicDimension::MinWidth, 10.0).unwrap(),
        20.0
    );
    assert_eq!(
        t.intrinsic(IntrinsicDimension::MaxHeight, 20.0).unwrap(),
        10.0
    );
    assert!(matches!(
        t.intrinsic(IntrinsicDimension::MinWidth, f32::INFINITY),
        Err(ElementError::Layout(LayoutError::UnsupportedQuery(_)))
    ));
    assert_eq!(calls.get(), 0);
}
#[test]
fn baseline_and_child_identity_survive_ratio_updates() {
    let child = || {
        Widget::baseline_leaf(
            vec2(10.0, 10.0),
            Baselines {
                alphabetic: Some(5.0),
                ideographic: None,
            },
            |_| {},
        )
        .keyed("content")
    };
    let mut t = ElementTree::new(
        &egui::Context::default(),
        AspectRatio::new(2.0).unwrap().into_widget(child()),
    )
    .unwrap();
    let c = BoxConstraints::loose(vec2(100.0, 100.0)).unwrap();
    let g = t.layout(c).unwrap();
    let id = g.children[0].layout.id;
    assert_eq!(g.baseline(TextBaseline::Alphabetic), Some(5.0));
    assert_eq!(
        t.dry_baseline(c, TextBaseline::Alphabetic).unwrap(),
        Some(5.0)
    );
    t.update(AspectRatio::new(1.0).unwrap().into_widget(child()))
        .unwrap();
    let g = t.layout(c).unwrap();
    assert_eq!(g.children[0].layout.id, id);
    assert_eq!(g.size, vec2(100.0, 100.0));
}
#[test]
fn invalid_ratios_zero_constraints_and_extreme_finite_values_are_checked() {
    for r in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert!(AspectRatio::new(r).is_err());
    }
    let mut t = tree(f32::MAX);
    assert_eq!(
        t.layout(BoxConstraints::tight(vec2(0.0, 0.0)).unwrap())
            .unwrap()
            .size,
        vec2(0.0, 0.0)
    );
    assert!(
        t.layout(BoxConstraints::loose(vec2(f32::INFINITY, f32::MAX)).unwrap())
            .is_err()
    );
    t.update(Widget::render_object(
        AspectRatio { aspect_ratio: -1.0 },
        vec![],
    ))
    .unwrap();
    assert!(
        t.layout(BoxConstraints::loose(vec2(100.0, 100.0)).unwrap())
            .is_err()
    );
}

use rust_flutter::{
    egui::{self, vec2},
    element::*,
    layout::*,
};
fn leaves() -> Vec<Widget> {
    [(20.0, 10.0), (20.0, 20.0), (20.0, 30.0)]
        .into_iter()
        .enumerate()
        .map(|(i, (w, h))| {
            Widget::baseline_leaf(
                vec2(w, h),
                Baselines {
                    alphabetic: Some(5.0),
                    ideographic: Some(6.0),
                },
                |_| {},
            )
            .keyed(i as u64)
        })
        .collect()
}
fn tree(style: Wrap) -> ElementTree {
    ElementTree::new(&egui::Context::default(), style.into_widget(leaves())).unwrap()
}
fn style() -> Wrap {
    Wrap {
        spacing: 5.0,
        run_spacing: 3.0,
        ..Wrap::default()
    }
}
#[test]
fn children_flow_into_runs_and_reflow_on_resize_with_stable_identity() {
    let mut t = tree(style());
    let c = BoxConstraints::loose(vec2(50.0, 100.0)).unwrap();
    let first = t.layout(c).unwrap();
    assert_eq!(first.size, vec2(45.0, 53.0));
    assert_eq!(
        first.children.iter().map(|c| c.offset).collect::<Vec<_>>(),
        vec![vec2(0.0, 0.0), vec2(25.0, 0.0), vec2(0.0, 23.0)]
    );
    assert_eq!(t.dry_layout(c).unwrap(), first.size);
    let c = BoxConstraints::loose(vec2(100.0, 100.0)).unwrap();
    let second = t.layout(c).unwrap();
    assert_eq!(second.size, vec2(70.0, 30.0));
    assert_eq!(second.children[2].offset, vec2(50.0, 0.0));
    assert_eq!(first.children[2].layout.id, second.children[2].layout.id);
}
#[test]
fn exact_fit_zero_width_and_oversized_children_do_not_create_empty_runs() {
    let mut t = tree(style());
    assert_eq!(
        t.layout(BoxConstraints::loose(vec2(45.0, 100.0)).unwrap())
            .unwrap()
            .size,
        vec2(45.0, 53.0)
    );
    let c = BoxConstraints::loose(vec2(0.0, 100.0)).unwrap();
    let g = t.layout(c).unwrap();
    assert_eq!(g.size, vec2(0.0, 66.0));
    assert_eq!(g.children[0].offset.y, 0.0);
    assert_eq!(g.children[1].offset.y, 13.0);
    assert_eq!(g.children[2].offset.y, 36.0);
    let c = BoxConstraints::loose(vec2(12.0, 100.0)).unwrap();
    let g = t.layout(c).unwrap();
    assert_eq!(g.children[0].layout.size, vec2(12.0, 10.0));
    assert_eq!(g.size, vec2(12.0, 66.0));
}
#[test]
fn rtl_and_up_directions_mirror_runs_children_and_cross_alignment() {
    let c = BoxConstraints::tight(vec2(50.0, 60.0)).unwrap();
    let original = tree(style()).layout(c).unwrap();
    for rtl in [false, true] {
        for up in [false, true] {
            let mut s = style();
            if rtl {
                s.text_direction = TextDirection::Rtl;
            }
            if up {
                s.vertical_direction = VerticalDirection::Up;
            }
            let mut t = tree(s);
            let g = t.layout(c).unwrap();
            for (base, child) in original.children.iter().zip(&g.children) {
                let expected = vec2(
                    if rtl {
                        50.0 - base.offset.x - base.layout.size.x
                    } else {
                        base.offset.x
                    },
                    if up {
                        60.0 - base.offset.y - base.layout.size.y
                    } else {
                        base.offset.y
                    },
                );
                assert_eq!(child.offset, expected);
            }
            assert_eq!(
                t.dry_baseline(c, TextBaseline::Alphabetic).unwrap(),
                g.baseline(TextBaseline::Alphabetic)
            );
        }
    }
}
#[test]
fn vertical_runs_use_height_limit_and_transpose_geometry() {
    let mut t = ElementTree::new(
        &egui::Context::default(),
        Wrap {
            direction: Axis::Vertical,
            ..style()
        }
        .into_widget(
            [(10.0, 20.0), (20.0, 20.0), (30.0, 20.0)]
                .into_iter()
                .map(|(w, h)| Widget::leaf(vec2(w, h), |_| {}))
                .collect(),
        ),
    )
    .unwrap();
    let c = BoxConstraints::loose(vec2(100.0, 50.0)).unwrap();
    let g = t.layout(c).unwrap();
    assert_eq!(g.size, vec2(53.0, 45.0));
    assert_eq!(g.children[1].offset, vec2(0.0, 25.0));
    assert_eq!(g.children[2].offset, vec2(23.0, 0.0));
    assert_eq!(t.dry_layout(c).unwrap(), g.size);
}
#[test]
fn main_alignment_distributes_free_space_without_losing_fixed_spacing() {
    for (alignment, expected) in [
        (WrapAlignment::Start, [0.0, 25.0, 50.0]),
        (WrapAlignment::End, [30.0, 55.0, 80.0]),
        (WrapAlignment::Center, [15.0, 40.0, 65.0]),
        (WrapAlignment::SpaceBetween, [0.0, 40.0, 80.0]),
        (WrapAlignment::SpaceAround, [5.0, 40.0, 75.0]),
        (WrapAlignment::SpaceEvenly, [7.5, 40.0, 72.5]),
    ] {
        let mut t = tree(Wrap {
            alignment,
            ..style()
        });
        let g = t
            .layout(BoxConstraints::tight(vec2(100.0, 50.0)).unwrap())
            .unwrap();
        assert_eq!(
            g.children.iter().map(|c| c.offset.x).collect::<Vec<_>>(),
            expected
        );
    }
}
#[test]
fn run_alignment_and_cross_alignment_distribute_extra_height() {
    for (run_alignment, ys) in [
        (WrapAlignment::Start, [0.0, 23.0]),
        (WrapAlignment::End, [47.0, 70.0]),
        (WrapAlignment::Center, [23.5, 46.5]),
        (WrapAlignment::SpaceBetween, [0.0, 70.0]),
        (WrapAlignment::SpaceAround, [11.75, 58.25]),
        (WrapAlignment::SpaceEvenly, [47.0 / 3.0, 23.0 + 94.0 / 3.0]),
    ] {
        let g = tree(Wrap {
            run_alignment,
            cross_axis_alignment: WrapCrossAlignment::End,
            ..style()
        })
        .layout(BoxConstraints::tight(vec2(50.0, 100.0)).unwrap())
        .unwrap();
        assert!((g.children[0].offset.y - (ys[0] + 10.0)).abs() < 0.00001);
        assert!((g.children[1].offset.y - ys[0]).abs() < 0.00001);
        assert!((g.children[2].offset.y - ys[1]).abs() < 0.00001);
    }
    let g = tree(Wrap {
        cross_axis_alignment: WrapCrossAlignment::Center,
        ..style()
    })
    .layout(BoxConstraints::tight(vec2(50.0, 100.0)).unwrap())
    .unwrap();
    assert_eq!(g.children[0].offset.y, 5.0);
}
#[test]
fn intrinsic_main_extents_use_child_intrinsics_and_cross_extent_uses_dry_runs() {
    let mut t = tree(style());
    assert_eq!(
        t.intrinsic(IntrinsicDimension::MinWidth, 0.0).unwrap(),
        20.0
    );
    // Flutter's main maximum intrinsic sums children without adding spacing.
    assert_eq!(
        t.intrinsic(IntrinsicDimension::MaxWidth, 0.0).unwrap(),
        60.0
    );
    assert_eq!(
        t.intrinsic(IntrinsicDimension::MinHeight, 50.0).unwrap(),
        53.0
    );
    assert_eq!(
        t.intrinsic(IntrinsicDimension::MaxHeight, 100.0).unwrap(),
        30.0
    );
    let mut s = style();
    s.direction = Axis::Vertical;
    t.update(s.into_widget(leaves())).unwrap();
    assert_eq!(
        t.intrinsic(IntrinsicDimension::MinHeight, 0.0).unwrap(),
        30.0
    );
    assert_eq!(
        t.intrinsic(IntrinsicDimension::MaxHeight, 0.0).unwrap(),
        60.0
    );
    assert_eq!(
        t.intrinsic(IntrinsicDimension::MaxWidth, 40.0).unwrap(),
        43.0
    );
}
#[test]
fn highest_translated_real_baseline_is_forwarded_and_no_baseline_is_invented() {
    let mut t = tree(Wrap {
        cross_axis_alignment: WrapCrossAlignment::End,
        ..style()
    });
    let c = BoxConstraints::loose(vec2(50.0, 100.0)).unwrap();
    let g = t.layout(c).unwrap();
    assert_eq!(g.baseline(TextBaseline::Alphabetic), Some(5.0));
    assert_eq!(
        t.dry_baseline(c, TextBaseline::Alphabetic).unwrap(),
        Some(5.0)
    );
    assert_eq!(
        t.dry_baseline(c, TextBaseline::Ideographic).unwrap(),
        Some(6.0)
    );
    t.update(style().into_widget(vec![Widget::leaf(vec2(20.0, 10.0), |_| {})]))
        .unwrap();
    assert_eq!(
        t.layout(c).unwrap().baseline(TextBaseline::Alphabetic),
        None
    );
    assert_eq!(t.dry_baseline(c, TextBaseline::Alphabetic).unwrap(), None);
}
#[test]
fn clipping_applies_to_overflow_and_unbounded_main_axis_keeps_one_run() {
    let mut t = tree(Wrap {
        clip: Clip::HardEdge,
        ..style()
    });
    let c = BoxConstraints::tight(vec2(50.0, 10.0)).unwrap();
    let g = t.layout(c).unwrap();
    assert_eq!(g.clip, Clip::HardEdge);
    assert_eq!(g.children[2].offset.y, 23.0);
    let g = t.layout(BoxConstraints::default()).unwrap();
    assert_eq!(g.size, vec2(70.0, 30.0));
    assert_eq!(g.clip, Clip::None);
}
#[test]
fn empty_children_invalid_spacing_overflow_and_negative_overlap_are_checked() {
    let ctx = egui::Context::default();
    let mut t = ElementTree::new(&ctx, Wrap::default().into_widget(vec![])).unwrap();
    let c = BoxConstraints::tight(vec2(30.0, 20.0)).unwrap();
    assert_eq!(t.layout(c).unwrap().size, c.min());
    assert_eq!(t.dry_layout(c).unwrap(), c.min());
    assert_eq!(t.dry_baseline(c, TextBaseline::Alphabetic).unwrap(), None);
    for spacing in [f32::NAN, f32::INFINITY] {
        t.update(Wrap { spacing, ..style() }.into_widget(leaves()))
            .unwrap();
        assert!(t.layout(c).is_err());
        assert!(t.dry_layout(c).is_err());
    }
    t.update(
        Wrap {
            spacing: -5.0,
            ..style()
        }
        .into_widget(leaves()),
    )
    .unwrap();
    assert_eq!(
        t.layout(BoxConstraints::default()).unwrap().size,
        vec2(50.0, 30.0)
    );
    t.update(
        Wrap {
            spacing: -100.0,
            ..style()
        }
        .into_widget(leaves()),
    )
    .unwrap();
    assert!(t.layout(BoxConstraints::default()).is_err());
    t.update(
        Wrap {
            spacing: f32::MAX,
            ..style()
        }
        .into_widget(leaves()),
    )
    .unwrap();
    assert!(t.layout(BoxConstraints::default()).is_err());
}
#[test]
fn pure_queries_never_execute_mutable_measurement_callbacks() {
    let mut t = ElementTree::new(
        &egui::Context::default(),
        Wrap::default().into_widget(vec![Widget::measured(
            |_| panic!("must not measure"),
            |_| {},
        )]),
    )
    .unwrap();
    assert!(matches!(
        t.dry_layout(BoxConstraints::default()),
        Err(ElementError::Layout(LayoutError::UnsupportedQuery(_)))
    ));
    assert!(t.intrinsic(IntrinsicDimension::MinWidth, 10.0).is_err());
}

#[test]
fn overflowing_translated_baselines_return_checked_errors() {
    let mut t = ElementTree::new(
        &egui::Context::default(),
        Wrap::new(Axis::Vertical).into_widget(vec![
            Widget::leaf(vec2(1.0, f32::MAX / 2.0), |_| {}),
            Widget::baseline_leaf(
                vec2(1.0, 1.0),
                Baselines {
                    alphabetic: Some(f32::MAX),
                    ideographic: None,
                },
                |_| {},
            ),
        ]),
    )
    .unwrap();
    assert!(t.layout(BoxConstraints::default()).is_err());
    assert!(
        t.dry_baseline(BoxConstraints::default(), TextBaseline::Alphabetic)
            .is_err()
    );
}

#[test]
fn pointer_activation_follows_child_bounds_after_reflow() {
    use std::{cell::Cell, rc::Rc};
    let ctx = egui::Context::default();
    let clicks = Rc::new(Cell::new(0));
    let rects: Vec<_> = (0..3)
        .map(|_| Rc::new(Cell::new(egui::Rect::NOTHING)))
        .collect();
    let children = rects
        .iter()
        .enumerate()
        .map(|(i, rect)| {
            let rect = rect.clone();
            let clicks = clicks.clone();
            Widget::leaf(vec2(60.0, 24.0), move |ui| {
                rect.set(ui.max_rect());
                if ui
                    .put(ui.max_rect(), egui::Button::new(format!("{i}")))
                    .clicked()
                {
                    clicks.set(clicks.get() + 1);
                }
            })
            .keyed(i as u64)
        })
        .collect();
    let mut tree = ElementTree::new(
        &ctx,
        Wrap {
            spacing: 8.0,
            run_spacing: 4.0,
            ..Wrap::default()
        }
        .into_widget(children),
    )
    .unwrap();
    let frame = |tree: &mut ElementTree, width, events| {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    vec2(400.0, 200.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    tree.show(ui, BoxConstraints::loose(vec2(width, 100.0)).unwrap())
                        .unwrap();
                });
            },
        );
    };
    for width in [220.0, 140.0] {
        frame(&mut tree, width, vec![]);
        frame(&mut tree, width, vec![]);
        if width == 220.0 {
            assert_eq!(rects[2].get().top(), rects[0].get().top());
        } else {
            assert!(rects[2].get().top() > rects[0].get().bottom());
        }
        let pos = rects[2].get().center();
        for pressed in [true, false] {
            frame(
                &mut tree,
                width,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
    }
    assert_eq!(clicks.get(), 2);
}

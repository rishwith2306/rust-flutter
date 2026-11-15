use rust_flutter::{
    egui::{self, Id, Vec2, pos2, vec2},
    layout::*,
};
fn tight(x: f32, y: f32) -> BoxConstraints {
    BoxConstraints::tight(vec2(x, y)).unwrap()
}
fn leaf(id: &str, x: f32, y: f32) -> BoxNode {
    BoxNode::leaf(id, vec2(x, y))
}
fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.001, "{actual} != {expected}");
}

#[test]
fn constraints_validate_deflate_intersect_and_reject_infinite_results() {
    for (min, max) in [
        (vec2(-1.0, 0.0), vec2(5.0, 5.0)),
        (vec2(f32::NAN, 0.0), Vec2::INFINITY),
        (vec2(6.0, 0.0), vec2(5.0, 5.0)),
    ] {
        assert_eq!(
            BoxConstraints::new(min, max),
            Err(LayoutError::InvalidConstraints)
        );
    }
    let c = BoxConstraints::new(vec2(10.0, 20.0), vec2(100.0, 80.0)).unwrap();
    assert_eq!(c.constrain(vec2(1.0, 500.0)).unwrap(), vec2(10.0, 80.0));
    assert_eq!(
        c.deflate(Insets::all(12.0)).unwrap().max(),
        vec2(76.0, 56.0)
    );
    assert_eq!(c.deflate(Insets::all(100.0)).unwrap(), tight(0.0, 0.0));
    assert_eq!(tight(200.0, 2.0).enforce(c), tight(100.0, 20.0));
    assert_eq!(
        BoxConstraints::default().constrain(Vec2::INFINITY),
        Err(LayoutError::UnboundedSize)
    );
}
#[test]
fn padding_alignment_and_sized_constraints_accumulate_offsets() {
    let child = BoxNode::constrained("sized", tight(20.0, 10.0), leaf("content", 1.0, 1.0));
    let mut tree = BoxNode::padding(
        "padding",
        Insets::all(5.0),
        BoxNode::align("align", Alignment::CENTER, child),
    );
    let layout = tree.layout(tight(100.0, 60.0)).unwrap();
    assert_eq!(layout.size, vec2(100.0, 60.0));
    assert_eq!(
        layout.rect_for(Id::new("content")).unwrap().min,
        pos2(40.0, 25.0)
    );
    assert_eq!(
        layout.find(Id::new("content")).unwrap().size,
        vec2(20.0, 10.0)
    );
    assert_eq!(layout.hit_test(pos2(41.0, 26.0)), Some(Id::new("content")));
    assert_eq!(layout.hit_test(pos2(10.0, 10.0)), None);
}
#[test]
fn expanded_weights_and_stretch_use_remaining_space_after_fixed_children_and_gaps() {
    let mut flex = Flex::new(
        Axis::Horizontal,
        vec![
            FlexChild::fixed(leaf("fixed", 20.0, 10.0)),
            FlexChild::expanded(leaf("one", 1.0, 2.0), 1),
            FlexChild::expanded(leaf("two", 1.0, 2.0), 2),
        ],
    );
    flex.spacing = 10.0;
    flex.cross_alignment = CrossAxisAlignment::Stretch;
    let layout = BoxNode::flex("row", flex)
        .layout(tight(300.0, 60.0))
        .unwrap();
    close(layout.children[1].layout.size.x, 260.0 / 3.0);
    close(layout.children[2].layout.size.x, 520.0 / 3.0);
    close(layout.children[1].offset.x, 30.0);
    assert!(layout.children.iter().all(|c| c.layout.size.y == 60.0));
}
#[test]
fn loose_flex_shrink_wraps_and_spacing_alignment_uses_actual_child_sizes() {
    let mut flex = Flex::new(
        Axis::Horizontal,
        vec![
            FlexChild::fixed(leaf("a", 20.0, 10.0)),
            FlexChild::flexible(leaf("b", 10.0, 10.0), 1, FlexFit::Loose),
        ],
    );
    flex.main_size = MainAxisSize::Min;
    flex.spacing = 5.0;
    let layout = BoxNode::flex("row", flex)
        .layout(BoxConstraints::loose(vec2(200.0, 50.0)).unwrap())
        .unwrap();
    assert_eq!(layout.size, vec2(35.0, 10.0));
    let mut flex = Flex::new(
        Axis::Horizontal,
        vec![
            FlexChild::fixed(leaf("a", 20.0, 10.0)),
            FlexChild::fixed(leaf("b", 20.0, 10.0)),
        ],
    );
    flex.main_alignment = MainAxisAlignment::SpaceEvenly;
    let layout = BoxNode::flex("row", flex)
        .layout(tight(100.0, 20.0))
        .unwrap();
    assert_eq!(layout.children[0].offset, vec2(20.0, 5.0));
    assert_eq!(layout.children[1].offset, vec2(60.0, 5.0));
}
#[test]
fn unbounded_flex_stretch_and_invalid_measurements_are_diagnosed() {
    let mut tight_flex = BoxNode::flex(
        "row",
        Flex::new(
            Axis::Horizontal,
            vec![FlexChild::expanded(leaf("a", 10.0, 10.0), 1)],
        ),
    );
    assert_eq!(
        tight_flex.layout(BoxConstraints::default()).unwrap_err(),
        LayoutError::UnboundedFlex
    );
    let mut flex = Flex::new(
        Axis::Horizontal,
        vec![FlexChild::flexible(
            leaf("a", 10.0, 10.0),
            1,
            FlexFit::Loose,
        )],
    );
    flex.main_size = MainAxisSize::Min;
    assert_eq!(
        BoxNode::flex("row", flex)
            .layout(BoxConstraints::default())
            .unwrap()
            .size,
        vec2(10.0, 10.0)
    );
    let mut flex = Flex::new(
        Axis::Horizontal,
        vec![FlexChild::fixed(leaf("a", 10.0, 10.0))],
    );
    flex.cross_alignment = CrossAxisAlignment::Stretch;
    assert_eq!(
        BoxNode::flex("row", flex)
            .layout(BoxConstraints::default())
            .unwrap_err(),
        LayoutError::UnboundedStretch
    );
    let mut bad = BoxNode::measured("bad", |_| Ok(vec2(2.0, 1.0)));
    assert_eq!(
        bad.layout(tight(1.0, 1.0)).unwrap_err(),
        LayoutError::ChildViolatedConstraints(Id::new("bad"))
    );
}
#[test]
fn horizontal_rtl_and_vertical_up_resolve_start_edges_on_both_axes() {
    let mut row = Flex::new(
        Axis::Horizontal,
        vec![
            FlexChild::fixed(leaf("a", 10.0, 20.0)),
            FlexChild::fixed(leaf("b", 20.0, 30.0)),
        ],
    );
    row.text_direction = TextDirection::Rtl;
    row.vertical_direction = VerticalDirection::Up;
    row.cross_alignment = CrossAxisAlignment::Start;
    let layout = BoxNode::flex("row", row)
        .layout(tight(100.0, 80.0))
        .unwrap();
    assert_eq!(layout.children[0].offset, vec2(90.0, 60.0));
    assert_eq!(layout.children[1].offset, vec2(70.0, 50.0));
    let mut column = Flex::new(
        Axis::Vertical,
        vec![FlexChild::fixed(leaf("a", 10.0, 20.0))],
    );
    column.text_direction = TextDirection::Rtl;
    column.vertical_direction = VerticalDirection::Up;
    column.cross_alignment = CrossAxisAlignment::Start;
    assert_eq!(
        BoxNode::flex("column", column)
            .layout(tight(100.0, 80.0))
            .unwrap()
            .children[0]
            .offset,
        vec2(90.0, 60.0)
    );
}
#[test]
fn stack_sizes_from_nonpositioned_children_and_hits_last_painted_leaf() {
    let mut tree = BoxNode::stack(
        "stack",
        Stack::new(vec![
            StackChild::new(leaf("background", 120.0, 80.0)),
            StackChild::positioned(
                leaf("overlay", 0.0, 0.0),
                Positioned {
                    right: Some(10.0),
                    bottom: Some(5.0),
                    width: Some(30.0),
                    height: Some(20.0),
                    ..Default::default()
                },
            ),
        ]),
    );
    let layout = tree
        .layout(BoxConstraints::loose(vec2(200.0, 200.0)).unwrap())
        .unwrap();
    assert_eq!(layout.size, vec2(120.0, 80.0));
    assert_eq!(layout.children[1].offset, vec2(80.0, 55.0));
    assert_eq!(layout.hit_test(pos2(85.0, 60.0)), Some(Id::new("overlay")));
    assert_eq!(layout.hit_test(pos2(5.0, 5.0)), Some(Id::new("background")));
    assert_eq!(layout.hit_test(pos2(120.0, 60.0)), None);
}
#[test]
fn positioned_fill_conflicts_unbounded_stack_and_empty_containers() {
    let mut tree = BoxNode::stack(
        "stack",
        Stack::new(vec![StackChild::positioned(
            leaf("fill", 1.0, 1.0),
            Positioned::fill(),
        )]),
    );
    assert_eq!(
        tree.layout(tight(100.0, 80.0)).unwrap().children[0]
            .layout
            .size,
        vec2(100.0, 80.0)
    );
    assert_eq!(
        tree.layout(BoxConstraints::default()).unwrap_err(),
        LayoutError::UnboundedSize
    );
    if let BoxKind::Stack(stack) = &mut tree.kind {
        stack.children[0].position.width = Some(4.0);
    }
    assert!(matches!(
        tree.layout(tight(100.0, 80.0)),
        Err(LayoutError::InvalidConfiguration(_))
    ));
    let layout = BoxNode::flex("empty", Flex::new(Axis::Horizontal, vec![]))
        .layout(tight(50.0, 20.0))
        .unwrap();
    assert!(!layout.is_leaf());
    assert_eq!(layout.hit_test(pos2(5.0, 5.0)), None);
}
#[test]
fn clipping_bounds_overflow_and_duplicate_keys_are_explicit() {
    let make = |clip| {
        let mut stack = Stack::new(vec![
            StackChild::new(leaf("base", 50.0, 50.0)),
            StackChild::positioned(
                leaf("outside", 30.0, 20.0),
                Positioned {
                    left: Some(-10.0),
                    top: Some(45.0),
                    ..Default::default()
                },
            ),
        ]);
        stack.clip = clip;
        BoxNode::stack("stack", stack)
    };
    let clipped = make(Clip::HardEdge).layout(tight(50.0, 50.0)).unwrap();
    assert_eq!(clipped.overflow.left, 10.0);
    assert_eq!(clipped.overflow.bottom, 15.0);
    assert_eq!(clipped.paint_bounds.min, pos2(0.0, 0.0));
    let open = make(Clip::None).layout(tight(50.0, 50.0)).unwrap();
    assert_eq!(open.paint_bounds.min, pos2(-10.0, 0.0));
    assert_eq!(open.hit_test(pos2(-5.0, 48.0)), None);
    let mut duplicate = BoxNode::flex(
        "row",
        Flex::new(
            Axis::Horizontal,
            vec![
                FlexChild::fixed(leaf("same", 1.0, 1.0)),
                FlexChild::fixed(leaf("same", 1.0, 1.0)),
            ],
        ),
    );
    assert_eq!(
        duplicate.layout(tight(20.0, 20.0)).unwrap_err(),
        LayoutError::DuplicateKey(Id::new("same"))
    );
}
#[test]
fn egui_bridge_keeps_leaf_widget_ids_when_siblings_reorder_and_respects_clips() {
    let ctx = egui::Context::default();
    let mut tree = BoxNode::flex(
        "row",
        Flex::new(
            Axis::Horizontal,
            vec![
                FlexChild::fixed(leaf("a", 20.0, 20.0)),
                FlexChild::fixed(leaf("b", 20.0, 20.0)),
            ],
        ),
    );
    let mut frames = Vec::new();
    for _ in 0..2 {
        let layout = tree.layout(tight(80.0, 20.0)).unwrap();
        let mut ids = std::collections::HashMap::new();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    pos2(0.0, 0.0),
                    vec2(200.0, 200.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    show_layout(ui, &layout, |key, ui| {
                        let response = ui.put(ui.max_rect(), egui::Button::new("x"));
                        ids.insert(key, response.id);
                        assert!(ui.clip_rect().width() <= 80.0);
                    });
                });
            },
        );
        assert!(!output.shapes.is_empty());
        frames.push(ids);
        if let BoxKind::Flex(flex) = &mut tree.kind {
            flex.children.reverse();
        }
    }
    assert_eq!(frames[0], frames[1]);
}

#[test]
fn egui_stack_pointer_targets_the_top_leaf_and_geometry_updates_without_new_keys() {
    let ctx = egui::Context::default();
    let mut tree = BoxNode::stack(
        "stack",
        Stack::new(vec![
            StackChild::new(leaf("back", 100.0, 100.0)),
            StackChild::positioned(
                leaf("front", 40.0, 40.0),
                Positioned {
                    left: Some(20.0),
                    top: Some(20.0),
                    ..Default::default()
                },
            ),
        ]),
    );
    let mut front_rect = egui::Rect::NOTHING;
    let mut back_hits = 0;
    let mut front_hits = 0;
    let mut frame = |events| {
        let layout = tree.layout(tight(100.0, 100.0)).unwrap();
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    pos2(0.0, 0.0),
                    vec2(200.0, 200.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    show_layout(ui, &layout, |key, ui| {
                        let response = ui.put(ui.max_rect(), egui::Button::new("hit"));
                        if key == Id::new("front") {
                            front_rect = response.rect;
                            if response.clicked() {
                                front_hits += 1;
                            }
                        } else if response.clicked() {
                            back_hits += 1;
                        }
                    });
                });
            },
        )
    };
    let _ = frame(vec![]);
    let _ = frame(vec![]);
    // Recover geometry independently: central panel margin is 8 logical points.
    let point = pos2(48.0, 48.0);
    for pressed in [true, false] {
        let _ = frame(vec![
            egui::Event::PointerMoved(point),
            egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            },
        ]);
    }
    assert!(front_rect.contains(point));
    assert_eq!((front_hits, back_hits), (1, 0));
    if let BoxKind::Stack(stack) = &mut tree.kind {
        stack.children[1].position.left = Some(30.0);
    }
    let moved = tree.layout(tight(100.0, 100.0)).unwrap();
    assert_eq!(
        moved.rect_for(Id::new("front")).unwrap().min,
        pos2(30.0, 20.0)
    );
}

#[test]
fn every_main_axis_spacing_mode_places_children_on_both_axes() {
    for axis in [Axis::Horizontal, Axis::Vertical] {
        for (alignment, leading, gap) in [
            (MainAxisAlignment::Start, 0.0, 4.0),
            (MainAxisAlignment::End, 76.0, 4.0),
            (MainAxisAlignment::Center, 38.0, 4.0),
            (MainAxisAlignment::SpaceBetween, 0.0, 80.0),
            (MainAxisAlignment::SpaceAround, 19.0, 42.0),
            (MainAxisAlignment::SpaceEvenly, 76.0 / 3.0, 4.0 + 76.0 / 3.0),
        ] {
            let mut flex = Flex::new(
                axis,
                vec![
                    FlexChild::fixed(leaf("a", 10.0, 10.0)),
                    FlexChild::fixed(leaf("b", 10.0, 10.0)),
                ],
            );
            flex.spacing = 4.0;
            flex.main_alignment = alignment;
            let layout = BoxNode::flex("root", flex)
                .layout(tight(100.0, 100.0))
                .unwrap();
            let main = |v: Vec2| if axis == Axis::Horizontal { v.x } else { v.y };
            close(main(layout.children[0].offset), leading);
            close(main(layout.children[1].offset), leading + 10.0 + gap);
        }
    }
}
#[test]
fn flex_reports_overflow_on_the_correct_side_when_direction_reverses() {
    for (direction, left, right) in [
        (TextDirection::Ltr, 0.0, 100.0),
        (TextDirection::Rtl, 100.0, 0.0),
    ] {
        let mut flex = Flex::new(
            Axis::Horizontal,
            vec![FlexChild::fixed(leaf("wide", 200.0, 20.0))],
        );
        flex.text_direction = direction;
        flex.clip = Clip::None;
        let layout = BoxNode::flex("root", flex)
            .layout(tight(100.0, 20.0))
            .unwrap();
        assert_eq!(layout.overflow.left, left);
        assert_eq!(layout.overflow.right, right);
        assert_eq!(
            layout.find(Id::new("wide")).unwrap().size,
            vec2(200.0, 20.0)
        );
    }
}

#[test]
fn finite_inputs_that_overflow_paint_coordinate_arithmetic_return_an_error() {
    let mut tree = BoxNode::stack(
        "root",
        Stack::new(vec![StackChild::positioned(
            leaf("large", 3.0e38, 1.0),
            Positioned {
                left: Some(3.0e38),
                top: Some(0.0),
                ..Default::default()
            },
        )]),
    );
    assert_eq!(
        tree.layout(tight(100.0, 100.0)).unwrap_err(),
        LayoutError::UnboundedSize
    );
}

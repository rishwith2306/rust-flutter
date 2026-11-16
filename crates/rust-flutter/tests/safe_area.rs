use rust_flutter::{
    display::{KeyboardVisibility, ViewEdges, ViewMetrics, ViewOcclusion},
    egui::{self, epaint::MarginF32},
    element::*,
    layout::{BoxConstraints, Insets, LayoutError},
};
struct Child;
#[derive(Default)]
struct ChildState {
    builds: usize,
    view: Option<ViewMetrics>,
    marker: usize,
}
impl StatefulWidget for Child {
    type State = ChildState;
    fn create_state(&self) -> Self::State {
        ChildState::default()
    }
}
impl State<Child> for ChildState {
    fn build(&mut self, _: &Child, ctx: &StateContext<Self>) -> Widget {
        self.builds += 1;
        self.view = ctx.depend_on::<ViewMetrics>().unwrap().map(|v| *v);
        Widget::leaf(egui::vec2(50.0, 60.0), |_| {})
    }
}
fn child() -> Widget {
    Widget::stateful(Child).keyed("child")
}
fn child_id(tree: &ElementTree) -> ElementId {
    tree.elements()
        .iter()
        .find(|e| e.key == Some(Key::from("child")))
        .unwrap()
        .id
}
fn raw(top: f32) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(400.0, 600.0),
        )),
        safe_area_insets: Some(egui::SafeAreaInsets(MarginF32 {
            left: 10.0,
            top,
            right: 30.0,
            bottom: 40.0,
        })),
        ..Default::default()
    }
}
fn loose() -> BoxConstraints {
    BoxConstraints::loose(egui::vec2(400.0, 600.0)).unwrap()
}

#[test]
fn nested_safe_areas_consume_padding_once_and_relayout_without_rebuilding_the_child() {
    let ctx = egui::Context::default();
    let mut tree = None;
    let _ = ctx.run(raw(20.0), |ctx| {
        tree = Some(ElementTree::new(ctx, Widget::safe_area(Widget::safe_area(child()))).unwrap());
    });
    let mut tree = tree.unwrap();
    let id = child_id(&tree);
    let first = tree.layout(loose()).unwrap();
    assert_eq!(first.size, egui::vec2(90.0, 120.0));
    assert_eq!(first.children[0].offset, egui::vec2(10.0, 20.0));
    assert_eq!(
        first.children[0].layout.children[0].offset,
        egui::Vec2::ZERO
    );
    assert_eq!(
        tree.state::<ChildState>(id)
            .unwrap()
            .view
            .unwrap()
            .padding(),
        MarginF32::ZERO
    );
    tree.state_handle::<ChildState>(id)
        .unwrap()
        .update(|s| s.marker = 42)
        .unwrap();
    tree.flush().unwrap();
    let _ = ctx.run(raw(50.0), |_| {
        let changed = tree.flush().unwrap();
        assert!(changed);
        let second = tree.layout(loose()).unwrap();
        assert_eq!(second.size, egui::vec2(90.0, 150.0));
        assert_eq!(second.children[0].offset, egui::vec2(10.0, 50.0));
    });
    let state = tree.state::<ChildState>(id).unwrap();
    assert_eq!(state.builds, 2); // only the explicitly queued state update built it again
    assert_eq!(state.marker, 42);
    assert_eq!(child_id(&tree), id);
}

#[test]
fn minimum_padding_applies_on_disabled_edges_without_consuming_their_inherited_insets() {
    let ctx = egui::Context::default();
    let mut tree = None;
    let _ = ctx.run(raw(20.0), |ctx| {
        tree = Some(
            ElementTree::new(
                ctx,
                Widget::safe_area_with(
                    SafeAreaOptions {
                        edges: ViewEdges {
                            top: true,
                            ..ViewEdges::NONE
                        },
                        minimum: Insets {
                            left: 5.0,
                            top: 30.0,
                            right: 5.0,
                            bottom: 7.0,
                        },
                        ..Default::default()
                    },
                    Widget::safe_area(child()),
                ),
            )
            .unwrap(),
        );
    });
    let mut tree = tree.unwrap();
    let layout = tree.layout(loose()).unwrap();
    assert_eq!(layout.children[0].offset, egui::vec2(5.0, 30.0));
    assert_eq!(
        layout.children[0].layout.children[0].offset,
        egui::vec2(10.0, 0.0)
    );
    assert_eq!(layout.size, egui::vec2(100.0, 137.0));
}

#[test]
fn bottom_view_padding_can_be_maintained_under_the_keyboard_and_restores_after_dismissal() {
    let ctx = egui::Context::default();
    for maintain in [false, true] {
        let mut tree = None;
        let _ = ctx.run(raw(20.0), |ctx| {
            let mut metrics = rust_flutter::display::view_metrics(ctx);
            metrics.safe_area = egui::SafeAreaInsets(MarginF32 {
                top: 20.0,
                bottom: 24.0,
                ..MarginF32::ZERO
            });
            metrics.keyboard = KeyboardVisibility::Visible;
            metrics.occlusion = ViewOcclusion::from_physical(
                metrics.safe_area.0,
                MarginF32 {
                    bottom: 300.0,
                    ..MarginF32::ZERO
                },
                1.0,
            );
            tree = Some(
                ElementTree::new(
                    ctx,
                    Widget::inherited_model(
                        metrics,
                        Widget::safe_area_with(
                            SafeAreaOptions {
                                maintain_bottom_view_padding: maintain,
                                ..Default::default()
                            },
                            child(),
                        ),
                    ),
                )
                .unwrap(),
            );
        });
        let mut tree = tree.unwrap();
        let handle = tree
            .inherited_handle::<ViewMetrics>(tree.root_id().unwrap())
            .unwrap();
        let layout = tree.layout(loose()).unwrap();
        assert_eq!(
            layout.size,
            egui::vec2(50.0, if maintain { 104.0 } else { 80.0 })
        );
        let seen = tree
            .state::<ChildState>(child_id(&tree))
            .unwrap()
            .view
            .unwrap();
        assert_eq!(seen.occlusion.unwrap().padding, MarginF32::ZERO);
        assert_eq!(seen.occlusion.unwrap().view_insets.bottom, 300.0);
        assert_eq!(seen.occlusion.unwrap().view_padding.bottom, 24.0);
        handle
            .update(|old| ViewMetrics {
                keyboard: KeyboardVisibility::Hidden,
                occlusion: ViewOcclusion::from_physical(old.safe_area.0, MarginF32::ZERO, 1.0),
                ..*old
            })
            .unwrap();
        assert_eq!(tree.layout(loose()).unwrap().size, egui::vec2(50.0, 104.0));
        assert_eq!(
            tree.state::<ChildState>(child_id(&tree))
                .unwrap()
                .view
                .unwrap()
                .occlusion
                .unwrap()
                .view_padding,
            MarginF32::ZERO
        );
    }
}

#[test]
fn invalid_minimum_padding_uses_layout_errors_and_keeps_mounted_state_for_recovery() {
    let ctx = egui::Context::default();
    for minimum in [-1.0, f32::NAN, f32::INFINITY, f32::MAX] {
        let mut tree = ElementTree::new(
            &ctx,
            Widget::safe_area_with(
                SafeAreaOptions {
                    minimum: Insets::all(minimum),
                    ..Default::default()
                },
                child(),
            ),
        )
        .unwrap();
        let id = child_id(&tree);
        assert!(matches!(
            tree.layout(loose()),
            Err(ElementError::Layout(LayoutError::InvalidConfiguration(_)))
        ));
        assert!(tree.state::<ChildState>(id).is_some());
        tree.update(Widget::safe_area(child())).unwrap();
        assert!(tree.layout(loose()).is_ok());
        assert_eq!(child_id(&tree), id);
    }
}

#[test]
fn padded_leaf_hit_regions_follow_layout_in_a_raw_viewport_ui() {
    use std::{cell::Cell, rc::Rc};
    let ctx = egui::Context::default();
    let clicks = Rc::new(Cell::new(0));
    let captured = Rc::new(Cell::new(egui::Rect::NOTHING));
    let count = clicks.clone();
    let rect = captured.clone();
    let mut tree = ElementTree::new(
        &ctx,
        Widget::safe_area(Widget::leaf(egui::vec2(50.0, 60.0), move |ui| {
            rect.set(ui.max_rect());
            if ui.put(ui.max_rect(), egui::Button::new("Tap")).clicked() {
                count.set(count.get() + 1);
            }
        })),
    )
    .unwrap();
    let mut frame = |events| {
        let mut input = raw(20.0);
        input.events = events;
        let _ = ctx.run(input, |ctx| {
            egui::Area::new(egui::Id::new("raw viewport"))
                .fixed_pos(egui::Pos2::ZERO)
                .constrain(false)
                .movable(false)
                .show(ctx, |ui| {
                    tree.show(ui, loose()).unwrap();
                });
        });
    };
    frame(vec![]);
    assert_eq!(captured.get().min, egui::pos2(10.0, 20.0));
    for (pos, expected) in [(egui::pos2(5.0, 5.0), 0), (egui::pos2(15.0, 25.0), 1)] {
        frame(vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        frame(vec![egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }]);
        assert_eq!(clicks.get(), expected);
    }
}

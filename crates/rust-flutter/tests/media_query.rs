use rust_flutter::{
    display::{Orientation, ViewAspect, ViewMetrics},
    egui::{self, epaint::MarginF32},
    element::*,
    layout::Axis,
};

struct Observe {
    aspects: Vec<ViewAspect>,
    all: bool,
    listen: bool,
}
#[derive(Default)]
struct Observation {
    builds: usize,
    hooks: usize,
    latest: Option<ViewMetrics>,
    initial: Option<ViewMetrics>,
    editable: bool,
    context: Option<BuildContext>,
}
impl StatefulWidget for Observe {
    type State = Observation;
    fn create_state(&self) -> Observation {
        Observation::default()
    }
}
impl State<Observe> for Observation {
    fn init_state(&mut self, _: &Observe, ctx: &StateContext<Self>) {
        self.initial = ctx.read::<ViewMetrics>().unwrap().map(|v| *v);
        self.editable = ctx.inherited_handle::<ViewMetrics>().unwrap().is_some();
        self.context = Some((**ctx).clone());
    }
    fn did_change_dependencies(&mut self, _: &Observe, _: &StateContext<Self>) {
        self.hooks += 1;
    }
    fn build(&mut self, widget: &Observe, ctx: &StateContext<Self>) -> Widget {
        self.builds += 1;
        let mut value = None;
        for aspect in &widget.aspects {
            value = ctx.depend_on_aspect::<ViewMetrics>(*aspect).unwrap();
        }
        if widget.all {
            value = ctx.depend_on::<ViewMetrics>().unwrap();
        }
        if !widget.listen {
            value = ctx.read::<ViewMetrics>().unwrap();
        }
        self.latest = value.map(|v| *v);
        Widget::empty()
    }
}
fn watch(name: &str, aspects: &[ViewAspect]) -> Widget {
    Widget::stateful(Observe {
        aspects: aspects.to_vec(),
        all: name == "all",
        listen: name != "read",
    })
    .keyed(name)
}
fn row(children: Vec<Widget>) -> Widget {
    Widget::flex(
        FlexStyle::new(Axis::Horizontal),
        children.into_iter().map(FlexItem::fixed).collect(),
    )
}
fn id(tree: &ElementTree, name: &str) -> ElementId {
    tree.elements()
        .iter()
        .find(|e| e.key == Some(Key::from(name)))
        .unwrap()
        .id
}
fn state<'a>(tree: &'a ElementTree, name: &str) -> &'a Observation {
    tree.state(id(tree, name)).unwrap()
}
fn input(size: egui::Vec2, density: f32, top: f32) -> egui::RawInput {
    let mut raw = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
        safe_area_insets: Some(egui::SafeAreaInsets(MarginF32 {
            top,
            ..MarginF32::ZERO
        })),
        ..Default::default()
    };
    raw.viewports
        .get_mut(&egui::ViewportId::ROOT)
        .unwrap()
        .native_pixels_per_point = Some(density);
    raw
}
fn flush(ctx: &egui::Context, raw: egui::RawInput, tree: &mut ElementTree) -> bool {
    let mut changed = false;
    let _ = ctx.run(raw, |_| changed |= tree.flush().unwrap());
    changed
}

#[test]
fn ambient_metrics_rebuild_only_observed_properties_and_keep_element_state() {
    let ctx = egui::Context::default();
    let mut tree = None;
    let first = input(egui::vec2(400.0, 600.0), 2.0, 20.0);
    let _ = ctx.run(first.clone(), |ctx| {
        tree = Some(
            ElementTree::new(
                ctx,
                row(vec![
                    watch("size", &[ViewAspect::Size]),
                    watch("orientation", &[ViewAspect::Orientation]),
                    watch("density", &[ViewAspect::NativePixelsPerPoint]),
                    watch("safe", &[ViewAspect::SafeArea]),
                    watch("padding", &[ViewAspect::Padding]),
                    watch("all", &[]),
                    watch("read", &[]),
                    watch("union", &[ViewAspect::Size, ViewAspect::SafeArea]),
                ]),
            )
            .unwrap(),
        );
    });
    let mut tree = tree.unwrap();
    let ids: Vec<_> = tree.elements().iter().map(|e| e.id).collect();
    let original = state(&tree, "size").latest.unwrap();
    assert_eq!(original.viewport.size(), egui::vec2(400.0, 600.0));
    assert_eq!(original.pixels_per_point, 2.0);
    assert_eq!(state(&tree, "size").initial, Some(original));
    assert!(!state(&tree, "size").editable);
    assert!(!flush(&ctx, first, &mut tree));
    assert!(flush(
        &ctx,
        input(egui::vec2(450.0, 650.0), 2.0, 20.0),
        &mut tree
    ));
    assert_eq!(state(&tree, "size").builds, 2);
    assert_eq!(state(&tree, "orientation").builds, 1);
    assert_eq!(state(&tree, "density").builds, 1);
    assert_eq!(state(&tree, "safe").builds, 1);
    assert_eq!(state(&tree, "padding").builds, 1);
    assert_eq!(state(&tree, "all").builds, 2);
    assert_eq!(state(&tree, "read").builds, 1);
    assert!(flush(
        &ctx,
        input(egui::vec2(650.0, 450.0), 3.0, 30.0),
        &mut tree
    ));
    for name in ["orientation", "density", "safe", "padding"] {
        assert_eq!(state(&tree, name).builds, 2);
        assert_eq!(state(&tree, name).hooks, 2);
    }
    assert_eq!(state(&tree, "union").builds, 3);
    assert_eq!(state(&tree, "union").hooks, 3); // two changed aspects coalesce
    assert_eq!(
        state(&tree, "orientation").latest.unwrap().orientation(),
        Orientation::Landscape
    );
    assert_eq!(state(&tree, "read").latest, Some(original));
    assert_eq!(
        ids,
        tree.elements().iter().map(|e| e.id).collect::<Vec<_>>()
    );
    let retained_context = state(&tree, "size").context.clone().unwrap();
    tree.unmount();
    assert_eq!(
        retained_context.read::<ViewMetrics>().unwrap_err(),
        DependencyError::Unmounted
    );
    let _ = ctx.run(input(egui::vec2(300.0, 500.0), 1.0, 0.0), |_| {
        tree.update(watch("size", &[ViewAspect::Size])).unwrap();
    });
    assert_eq!(state(&tree, "size").builds, 1);
    assert_eq!(
        state(&tree, "size").latest.unwrap().viewport.size(),
        egui::vec2(300.0, 500.0)
    );
}

#[test]
fn explicit_view_models_shadow_the_host_and_can_be_updated_independently() {
    let ctx = egui::Context::default();
    let mut tree = None;
    let _ = ctx.run(input(egui::vec2(400.0, 600.0), 2.0, 20.0), |ctx| {
        let mut override_metrics = rust_flutter::display::view_metrics(ctx);
        override_metrics.viewport =
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(100.0, 200.0));
        tree = Some(
            ElementTree::new(
                ctx,
                row(vec![
                    watch("outer", &[ViewAspect::Size]),
                    Widget::inherited_model(override_metrics, watch("inner", &[ViewAspect::Size]))
                        .keyed("override"),
                ]),
            )
            .unwrap(),
        );
    });
    let mut tree = tree.unwrap();
    let handle = tree
        .inherited_handle::<ViewMetrics>(id(&tree, "override"))
        .unwrap();
    assert!(state(&tree, "inner").editable);
    assert!(flush(
        &ctx,
        input(egui::vec2(800.0, 900.0), 2.0, 20.0),
        &mut tree
    ));
    assert_eq!(state(&tree, "outer").builds, 2);
    assert_eq!(state(&tree, "inner").builds, 1);
    assert_eq!(
        state(&tree, "inner").latest.unwrap().viewport.size(),
        egui::vec2(100.0, 200.0)
    );
    handle
        .update(|old| ViewMetrics {
            viewport: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(120.0, 220.0)),
            ..*old
        })
        .unwrap();
    assert!(tree.flush().unwrap());
    assert_eq!(state(&tree, "inner").builds, 2);
    assert_eq!(state(&tree, "outer").builds, 2);
    tree.unmount();
    assert!(!handle.mounted());
}

#[test]
fn native_density_and_effective_zoom_have_independent_subscriptions() {
    let ctx = egui::Context::default();
    let mut tree = None;
    let _ = ctx.run(input(egui::vec2(400.0, 600.0), 2.0, 0.0), |ctx| {
        tree = Some(
            ElementTree::new(
                ctx,
                row(vec![
                    watch("zoom", &[ViewAspect::PixelsPerPoint]),
                    watch("native", &[ViewAspect::NativePixelsPerPoint]),
                    watch("orientation", &[ViewAspect::Orientation]),
                    watch("size", &[ViewAspect::Size]),
                ]),
            )
            .unwrap(),
        );
    });
    let mut tree = tree.unwrap();
    ctx.set_zoom_factor(2.0);
    assert!(flush(
        &ctx,
        input(egui::vec2(400.0, 600.0), 2.0, 0.0),
        &mut tree
    ));
    assert_eq!(state(&tree, "zoom").builds, 2);
    assert_eq!(state(&tree, "zoom").latest.unwrap().pixels_per_point, 4.0);
    assert_eq!(state(&tree, "native").builds, 1);
    assert_eq!(state(&tree, "orientation").builds, 1);
    assert_eq!(state(&tree, "size").builds, 2);
    assert_eq!(
        state(&tree, "size").latest.unwrap().viewport.size(),
        egui::vec2(200.0, 300.0)
    );
    assert!(!flush(
        &ctx,
        input(egui::vec2(200.0, 300.0), 2.0, 0.0),
        &mut tree
    ));
}

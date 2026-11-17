use rust_flutter::{
    decoration::*,
    egui::{self, Color32, vec2},
    element::*,
    layout::*,
};
use std::{cell::RefCell, rc::Rc};
fn c() -> BoxConstraints {
    BoxConstraints::loose(vec2(100.0, 100.0)).unwrap()
}
fn fills(output: &egui::FullOutput) -> Vec<Color32> {
    fn visit(shape: &egui::Shape, out: &mut Vec<Color32>) {
        match shape {
            egui::Shape::Rect(r) if r.fill != Color32::TRANSPARENT => out.push(r.fill),
            egui::Shape::Vec(v) => {
                for s in v {
                    visit(s, out);
                }
            }
            _ => {}
        }
    }
    let mut result = vec![];
    for s in &output.shapes {
        visit(&s.shape, &mut result);
    }
    result
}
#[test]
fn decoration_position_orders_fill_before_or_after_opaque_child() {
    for position in [
        DecorationPosition::Background,
        DecorationPosition::Foreground,
    ] {
        let ctx = egui::Context::default();
        ctx.begin_pass(egui::RawInput::default());
        let child = Widget::leaf(vec2(30.0, 20.0), |ui| {
            ui.painter().rect_filled(ui.max_rect(), 0.0, Color32::BLUE);
        });
        let widget = DecoratedBox {
            decoration: BoxDecoration::solid(Color32::RED),
            position,
        }
        .into_widget(child);
        let mut tree = ElementTree::new(&ctx, widget).unwrap();
        egui::CentralPanel::default().show(&ctx, |ui| {
            tree.show(ui, c()).unwrap();
        });
        let output = ctx.end_pass();
        let colors = fills(&output)
            .into_iter()
            .filter(|c| *c == Color32::RED || *c == Color32::BLUE)
            .collect::<Vec<_>>();
        assert_eq!(
            colors,
            if position == DecorationPosition::Background {
                vec![Color32::RED, Color32::BLUE]
            } else {
                vec![Color32::BLUE, Color32::RED]
            }
        );
    }
}
#[test]
fn decoration_passes_constraints_intrinsics_and_real_baselines_without_padding() {
    let ctx = egui::Context::default();
    let widget = DecoratedBox::new(BoxDecoration {
        border: egui::Stroke::new(8.0_f32, Color32::RED),
        corner_radius: egui::CornerRadius::same(12),
        ..Default::default()
    })
    .into_widget(Widget::baseline_leaf(
        vec2(30.0, 20.0),
        Baselines {
            alphabetic: Some(12.0),
            ideographic: Some(15.0),
        },
        |_| {},
    ));
    let mut tree = ElementTree::new(&ctx, widget).unwrap();
    for c in [c(), BoxConstraints::tight(vec2(10.0, 4.0)).unwrap()] {
        let dry = tree.dry_layout(c).unwrap();
        let wet = tree.layout(c).unwrap();
        assert_eq!(wet.size, dry);
        assert_eq!(wet.clip, Clip::None);
        assert_eq!(wet.children[0].offset, egui::Vec2::ZERO);
        for kind in [TextBaseline::Alphabetic, TextBaseline::Ideographic] {
            assert_eq!(tree.dry_baseline(c, kind).unwrap(), wet.baseline(kind));
        }
    }
    assert_eq!(
        tree.intrinsic(IntrinsicDimension::MinWidth, 0.0).unwrap(),
        30.0
    );
    assert_eq!(
        tree.intrinsic(IntrinsicDimension::MaxHeight, 0.0).unwrap(),
        20.0
    );
}
#[test]
fn invalid_decoration_and_child_arity_fail_queries_and_layout_without_unmounting() {
    let ctx = egui::Context::default();
    let mut tree =
        ElementTree::new(&ctx, DecoratedBox::default().into_widget(Widget::empty())).unwrap();
    let id = tree.root_id();
    for width in [-1.0, f32::NAN, f32::INFINITY] {
        let widget = DecoratedBox::new(BoxDecoration {
            border: egui::Stroke::new(width, Color32::RED),
            ..Default::default()
        })
        .into_widget(Widget::empty());
        tree.update(widget).unwrap();
        assert!(tree.layout(c()).is_err());
        assert!(tree.dry_layout(c()).is_err());
        assert_eq!(tree.root_id(), id);
    }
    for children in [vec![], vec![Widget::empty(), Widget::empty()]] {
        tree.update(Widget::render_object(DecoratedBox::default(), children))
            .unwrap();
        assert!(tree.layout(c()).is_err());
        assert!(tree.dry_layout(c()).is_err());
        assert!(tree.intrinsic(IntrinsicDimension::MaxWidth, 0.0).is_err());
    }
    tree.update(DecoratedBox::default().into_widget(Widget::empty()))
        .unwrap();
    assert!(tree.layout(c()).is_ok());
    assert_eq!(tree.root_id(), id);
}
type Record = (&'static str, PaintPhase, egui::Id, egui::Rect, egui::Rect);
#[derive(Clone)]
struct Layer {
    name: &'static str,
    log: Rc<RefCell<Vec<Record>>>,
}

impl RenderObjectWidget for Layer {
    type Object = Self;
    fn create_render_object(&self, _: &RenderContext) -> Self {
        self.clone()
    }
    fn update_render_object(&self, object: &mut Self, _: &RenderContext) {
        *object = self.clone();
    }
}
impl Layer {
    fn record(&self, phase: PaintPhase, ui: &egui::Ui) {
        self.log
            .borrow_mut()
            .push((self.name, phase, ui.id(), ui.max_rect(), ui.clip_rect()));
    }
}
impl RenderBox for Layer {
    fn layout(
        &mut self,
        c: BoxConstraints,
        children: &mut BoxChildren<'_>,
        _: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        let size = if children.is_empty() {
            c.constrain(vec2(20.0, 20.0))?
        } else {
            let s = children.layout(0, c)?;
            children.place(0, egui::Vec2::ZERO)?;
            s
        };
        let mut g = BoxGeometry::new(size);
        g.clip = Clip::HardEdge;
        Ok(g)
    }
    fn paint(&mut self, ui: &mut egui::Ui, _: &RenderContext) {
        self.record(PaintPhase::Background, ui);
    }
    fn paint_foreground(&mut self, ui: &mut egui::Ui, _: &RenderContext) {
        self.record(PaintPhase::Foreground, ui);
    }
}
#[test]
fn nested_render_objects_receive_balanced_phases_with_same_ui_rect_and_clip() {
    let ctx = egui::Context::default();
    ctx.begin_pass(egui::RawInput::default());
    let log = Rc::new(RefCell::new(vec![]));
    let layer = |name| Layer {
        name,
        log: log.clone(),
    };
    let mut tree = ElementTree::new(
        &ctx,
        Widget::render_object(
            layer("outer"),
            vec![Widget::render_object(layer("inner"), vec![])],
        ),
    )
    .unwrap();
    egui::CentralPanel::default().show(&ctx, |ui| {
        tree.show(ui, c()).unwrap();
    });
    let records = log.borrow();
    assert_eq!(
        records.iter().map(|r| (r.0, r.1)).collect::<Vec<_>>(),
        vec![
            ("outer", PaintPhase::Background),
            ("inner", PaintPhase::Background),
            ("inner", PaintPhase::Foreground),
            ("outer", PaintPhase::Foreground)
        ]
    );
    assert_eq!(records[0].2, records[3].2);
    assert_eq!(records[0].3, records[3].3);
    assert_eq!(records[0].4, records[3].4);
    assert_eq!(records[1].2, records[2].2);
    assert_ne!(records[0].2, records[1].2);
    let _ = ctx.end_pass();
}
#[test]
fn low_level_legacy_paint_stays_single_phase_and_phased_api_visits_leaves_twice() {
    let ctx = egui::Context::default();
    ctx.begin_pass(egui::RawInput::default());
    let mut node = BoxNode::padding(
        "padding",
        Insets::all(4.0),
        BoxNode::leaf("leaf", vec2(20.0, 20.0)),
    );
    let layout = node.layout(c()).unwrap();
    let mut legacy = 0;
    let mut phases = vec![];
    egui::CentralPanel::default().show(&ctx, |ui| {
        show_layout(ui, &layout, |_, _| legacy += 1);
        show_layout_with_phases(ui, &layout, |_, phase, _| phases.push(phase));
    });
    assert_eq!(legacy, 1);
    assert_eq!(phases, vec![PaintPhase::Background, PaintPhase::Foreground]);
    let _ = ctx.end_pass();
}

#[test]
fn retained_decoration_updates_border_rounding_and_paint_position_without_replacing_children() {
    let ctx = egui::Context::default();
    ctx.begin_pass(egui::RawInput::default());
    let child = || {
        Widget::leaf(vec2(30.0, 20.0), |ui| {
            ui.painter().rect_filled(ui.max_rect(), 0.0, Color32::BLUE);
        })
    };
    let mut tree = ElementTree::new(
        &ctx,
        DecoratedBox::new(BoxDecoration::solid(Color32::RED)).into_widget(child()),
    )
    .unwrap();
    tree.layout(c()).unwrap();
    let ids = tree.elements().iter().map(|e| e.id).collect::<Vec<_>>();
    let decoration = BoxDecoration {
        color: Color32::GREEN,
        border: egui::Stroke::new(3.0_f32, Color32::YELLOW),
        corner_radius: egui::CornerRadius::same(7),
    };
    tree.update(
        DecoratedBox {
            decoration,
            position: DecorationPosition::Foreground,
        }
        .into_widget(child()),
    )
    .unwrap();
    assert_eq!(
        tree.elements().iter().map(|e| e.id).collect::<Vec<_>>(),
        ids
    );
    egui::CentralPanel::default().show(&ctx, |ui| {
        tree.show(ui, c()).unwrap();
    });
    let output = ctx.end_pass();
    let colors = fills(&output)
        .into_iter()
        .filter(|color| [Color32::RED, Color32::BLUE, Color32::GREEN].contains(color))
        .collect::<Vec<_>>();
    assert_eq!(colors, vec![Color32::BLUE, Color32::GREEN]);
    let shape = output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Rect(r) if r.fill == Color32::GREEN => Some(r),
            _ => None,
        })
        .unwrap();
    assert_eq!(shape.stroke, decoration.border);
    assert_eq!(shape.corner_radius, decoration.corner_radius);
    assert_eq!(shape.stroke_kind, egui::StrokeKind::Inside);
}

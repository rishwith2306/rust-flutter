use rust_flutter::{
    egui::{self, Color32, FontId, Vec2, text::TextFormat, vec2},
    element::*,
    layout::*,
    text::{Paragraph, TextOverflow, TextSpan},
};
use std::{cell::Cell, rc::Rc};
fn format(size: f32) -> TextFormat {
    TextFormat {
        font_id: FontId::proportional(size),
        color: Color32::WHITE,
        ..Default::default()
    }
}
fn paragraph(text: &str) -> Paragraph {
    Paragraph::new(text, format(20.0))
}
fn loose(width: f32) -> BoxConstraints {
    BoxConstraints::loose(vec2(width, 1000.0)).unwrap()
}
fn frame(ctx: &egui::Context) {
    ctx.begin_pass(egui::RawInput::default());
}
fn text_shapes(shapes: &[egui::epaint::ClippedShape]) -> Vec<&egui::epaint::TextShape> {
    fn collect<'a>(s: &'a egui::Shape, out: &mut Vec<&'a egui::epaint::TextShape>) {
        match s {
            egui::Shape::Text(s) => out.push(s),
            egui::Shape::Vec(v) => {
                for s in v {
                    collect(s, out);
                }
            }
            _ => {}
        }
    }
    let mut out = vec![];
    for s in shapes {
        collect(&s.shape, &mut out);
    }
    out
}
#[test]
fn wrapped_paragraph_queries_match_actual_and_publish_real_baseline() {
    let ctx = egui::Context::default();
    frame(&ctx);
    let mut tree = ElementTree::new(
        &ctx,
        paragraph("Alpha beta gamma delta epsilon").into_widget(),
    )
    .unwrap();
    for width in [0.0, 1.0, 80.0, 300.0, f32::INFINITY] {
        let c = loose(width);
        let dry = tree.dry_layout(c).unwrap();
        let baseline = tree.dry_baseline(c, TextBaseline::Alphabetic).unwrap();
        let wet = tree.layout(c).unwrap();
        assert_eq!(wet.size, dry);
        assert_eq!(wet.baseline(TextBaseline::Alphabetic), baseline);
        assert_eq!(wet.baseline(TextBaseline::Ideographic), None);
        assert!(dry.is_finite());
    }
    assert!(tree.dry_layout(loose(80.0)).unwrap().y > tree.dry_layout(loose(300.0)).unwrap().y);
    let _ = ctx.end_pass();
}
#[test]
fn intrinsic_width_uses_words_and_preserves_nonbreaking_segments() {
    let ctx = egui::Context::default();
    frame(&ctx);
    let mut tree = ElementTree::new(&ctx, paragraph("Alpha beta").into_widget()).unwrap();
    let width = |s: &str| {
        ctx.fonts_mut(|f| f.layout_no_wrap(s.into(), FontId::proportional(20.0), Color32::WHITE))
            .size()
            .x
    };
    assert_eq!(
        tree.intrinsic(IntrinsicDimension::MinWidth, f32::INFINITY)
            .unwrap(),
        width("Alpha").max(width("beta"))
    );
    assert_eq!(
        tree.intrinsic(IntrinsicDimension::MaxWidth, 0.0).unwrap(),
        width("Alpha beta")
    );
    tree.update(paragraph("Alpha\u{a0}beta").into_widget())
        .unwrap();
    assert_eq!(
        tree.intrinsic(IntrinsicDimension::MinWidth, 0.0).unwrap(),
        width("Alpha\u{a0}beta")
    );
    for d in [IntrinsicDimension::MinHeight, IntrinsicDimension::MaxHeight] {
        assert_eq!(
            tree.intrinsic(d, 60.0).unwrap(),
            tree.dry_layout(loose(60.0)).unwrap().y
        );
    }
    let _ = ctx.end_pass();
}
#[test]
fn rich_utf8_sections_and_readonly_queries_preserve_painted_galley() {
    let ctx = egui::Context::default();
    frame(&ctx);
    let mut red = format(14.0);
    red.color = Color32::RED;
    let mut blue = format(28.0);
    blue.color = Color32::BLUE;
    blue.underline = egui::Stroke::new(1.0_f32, Color32::BLUE);
    let p = Paragraph::rich(vec![
        TextSpan::new("hé ", red.clone()),
        TextSpan::new("large", blue.clone()),
    ]);
    let mut tree = ElementTree::new(&ctx, p.into_widget()).unwrap();
    let wide = tree.layout(loose(400.0)).unwrap();
    assert!(tree.dry_layout(loose(12.0)).unwrap().y > wide.size.y);
    let _ = tree.intrinsic(IntrinsicDimension::MinWidth, 0.0).unwrap();
    egui::CentralPanel::default().show(&ctx, |ui| {
        tree.show(ui, loose(400.0)).unwrap();
    });
    let output = ctx.end_pass();
    let texts = text_shapes(&output.shapes);
    assert_eq!(texts.len(), 1);
    let galley = &texts[0].galley;
    assert_eq!(galley.size(), wide.size);
    assert_eq!(galley.job.text, "hé large");
    assert_eq!(galley.job.sections[0].byte_range, 0..4);
    assert_eq!(galley.job.sections[1].byte_range, 4..9);
    assert_eq!(galley.job.sections[0].format, red);
    assert_eq!(galley.job.sections[1].format, blue);
}
#[test]
fn max_lines_ellipsis_and_clip_have_explicit_geometry_and_paint() {
    for overflow in [
        TextOverflow::Clip,
        TextOverflow::Ellipsis,
        TextOverflow::Visible,
    ] {
        let ctx = egui::Context::default();
        frame(&ctx);
        let mut p = paragraph("This is much too long to fit into this tiny line");
        p.max_lines = Some(1);
        p.overflow = overflow;
        let mut tree = ElementTree::new(&ctx, p.into_widget()).unwrap();
        let g = tree.layout(loose(70.0)).unwrap();
        assert_eq!(
            g.clip,
            if overflow == TextOverflow::Visible {
                Clip::None
            } else {
                Clip::HardEdge
            }
        );
        egui::CentralPanel::default().show(&ctx, |ui| {
            tree.show(ui, loose(70.0)).unwrap();
        });
        let output = ctx.end_pass();
        let shapes = text_shapes(&output.shapes);
        let galley = &shapes[0].galley;
        assert!(galley.elided);
        assert_eq!(galley.rows.len(), 1);
        let last = galley.rows[0].glyphs.last().unwrap().chr;
        assert_eq!(last == '…', overflow == TextOverflow::Ellipsis);
    }
}
#[test]
fn disabling_soft_wrap_preserves_explicit_newlines_and_allows_overflow() {
    let ctx = egui::Context::default();
    frame(&ctx);
    let mut p = paragraph("A long explicit line\nsecond line");
    p.soft_wrap = false;
    p.overflow = TextOverflow::Visible;
    let mut tree = ElementTree::new(&ctx, p.into_widget()).unwrap();
    let small = tree.layout(loose(10.0)).unwrap();
    let wide = tree.layout(loose(500.0)).unwrap();
    assert_eq!(small.size.x, 10.0);
    assert_eq!(small.size.y, wide.size.y);
    assert!(wide.size.x > small.size.x);
    let tight = BoxConstraints::tight(vec2(5.0, 2.0)).unwrap();
    assert_eq!(tree.dry_layout(tight).unwrap(), vec2(5.0, 2.0));
    assert_eq!(tree.layout(tight).unwrap().size, vec2(5.0, 2.0));
    let _ = ctx.end_pass();
}
#[test]
fn invalid_text_configuration_fails_without_unmounting_and_can_recover() {
    let ctx = egui::Context::default();
    frame(&ctx);
    let mut tree = ElementTree::new(&ctx, paragraph("valid").into_widget()).unwrap();
    let id = tree.root_id();
    for bad_format in [
        TextFormat {
            font_id: FontId::proportional(f32::NAN),
            ..format(20.0)
        },
        TextFormat {
            extra_letter_spacing: f32::INFINITY,
            ..format(20.0)
        },
        TextFormat {
            line_height: Some(-1.0),
            ..format(20.0)
        },
        TextFormat {
            expand_bg: f32::NAN,
            ..format(20.0)
        },
    ] {
        tree.update(Paragraph::new("bad", bad_format).into_widget())
            .unwrap();
        assert!(tree.layout(loose(100.0)).is_err());
        assert_eq!(tree.root_id(), id);
    }
    let mut p = paragraph("bad");
    p.max_lines = Some(0);
    tree.update(p.into_widget()).unwrap();
    assert!(tree.dry_layout(loose(100.0)).is_err());
    let mut p = paragraph("bad");
    p.overflow = TextOverflow::Ellipsis;
    tree.update(p.into_widget()).unwrap();
    assert!(tree.layout(loose(100.0)).is_err());
    tree.update(paragraph("recovered").into_widget()).unwrap();
    assert!(tree.layout(loose(100.0)).is_ok());
    assert_eq!(tree.root_id(), id);
    let _ = ctx.end_pass();
}
struct Parent(Rc<Cell<usize>>);
impl RenderObjectWidget for Parent {
    type Object = Self;
    fn create_render_object(&self, _: &RenderContext) -> Self {
        Self(self.0.clone())
    }
    fn update_render_object(&self, _: &mut Self, _: &RenderContext) {}
}
impl RenderBox for Parent {
    fn layout(
        &mut self,
        c: BoxConstraints,
        children: &mut BoxChildren<'_>,
        _: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        self.0.set(self.0.get() + 1);
        let size = children.layout(0, c)?;
        children.place(0, Vec2::ZERO)?;
        Ok(BoxGeometry::new(size))
    }
}
#[test]
fn font_collection_changes_refresh_retained_ancestor_caches_without_rebuilding_widgets() {
    let ctx = egui::Context::default();
    frame(&ctx);
    let layouts = Rc::new(Cell::new(0));
    let mut tree = ElementTree::new(
        &ctx,
        Widget::render_object(
            Parent(layouts.clone()),
            vec![paragraph("iiii WWWW").into_widget()],
        ),
    )
    .unwrap();
    let before = tree.layout(loose(500.0)).unwrap().size;
    tree.layout(loose(500.0)).unwrap();
    assert_eq!(layouts.get(), 1);
    let ids = tree.elements().iter().map(|e| e.id).collect::<Vec<_>>();
    let _ = ctx.end_pass();
    let mut fonts = egui::FontDefinitions::default();
    let mono = fonts.families[&egui::FontFamily::Monospace].clone();
    fonts.families.insert(egui::FontFamily::Proportional, mono);
    ctx.set_fonts(fonts);
    frame(&ctx);
    let after = tree.layout(loose(500.0)).unwrap().size;
    assert_ne!(before.x, after.x);
    assert_eq!(layouts.get(), 2);
    assert_eq!(
        tree.elements().iter().map(|e| e.id).collect::<Vec<_>>(),
        ids
    );
    let _ = ctx.end_pass();
}
#[test]
fn alignment_anchors_text_inside_the_constrained_box() {
    for align in [egui::Align::Min, egui::Align::Center, egui::Align::Max] {
        let ctx = egui::Context::default();
        frame(&ctx);
        let mut p = paragraph("alignment");
        p.align = align;
        let mut tree = ElementTree::new(&ctx, p.into_widget()).unwrap();
        let mut rect = egui::Rect::NOTHING;
        egui::CentralPanel::default().show(&ctx, |ui| {
            rect = tree
                .show(ui, BoxConstraints::tight(vec2(300.0, 40.0)).unwrap())
                .unwrap()
                .rect;
        });
        let out = ctx.end_pass();
        let texts = text_shapes(&out.shapes);
        let text = texts[0];
        let drawn = text.galley.rect.translate(text.pos.to_vec2());
        let expected = rect.left() + align.to_factor() * (rect.width() - drawn.width());
        // Backend row offsets round to physical pixels.
        assert!(
            (drawn.left() - expected).abs() <= 0.5 / ctx.pixels_per_point(),
            "{align:?} drawn={drawn:?} expected={expected} rect={rect:?} pos={:?} galley={:?}",
            text.pos,
            text.galley.rect
        );
    }
}

#[test]
fn native_density_changes_refresh_galleys_without_remounting() {
    let ctx = egui::Context::default();
    frame(&ctx);
    let mut tree = ElementTree::new(&ctx, paragraph("Ag scale").into_widget()).unwrap();
    tree.layout(loose(300.0)).unwrap();
    let id = tree.root_id();
    let _ = ctx.end_pass();
    let mut input = egui::RawInput::default();
    input
        .viewports
        .get_mut(&egui::ViewportId::ROOT)
        .unwrap()
        .native_pixels_per_point = Some(2.0);
    ctx.begin_pass(input);
    assert_eq!(ctx.pixels_per_point(), 2.0);
    egui::CentralPanel::default().show(&ctx, |ui| {
        tree.show(ui, loose(300.0)).unwrap();
    });
    assert_eq!(tree.root_id(), id);
    let out = ctx.end_pass();
    let texts = text_shapes(&out.shapes);
    assert_eq!(texts[0].galley.pixels_per_point, 2.0);
}
#[test]
fn explicit_empty_first_line_has_no_fabricated_baseline() {
    let ctx = egui::Context::default();
    frame(&ctx);
    let mut tree = ElementTree::new(&ctx, paragraph("\nsecond").into_widget()).unwrap();
    assert_eq!(
        tree.dry_baseline(loose(300.0), TextBaseline::Alphabetic)
            .unwrap(),
        None
    );
    let wet = tree.layout(loose(300.0)).unwrap();
    assert_eq!(wet.baseline(TextBaseline::Alphabetic), None);
    assert!(wet.size.y > 20.0);
    tree.update(Paragraph::rich(vec![]).into_widget()).unwrap();
    assert_eq!(tree.dry_layout(loose(300.0)).unwrap(), Vec2::ZERO);
    assert_eq!(
        tree.intrinsic(IntrinsicDimension::MinWidth, 0.0).unwrap(),
        0.0
    );
    let _ = ctx.end_pass();
}

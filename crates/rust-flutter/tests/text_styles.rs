use rust_flutter::{
    egui::{self, Color32, FontFamily, Stroke, vec2},
    element::*,
    layout::*,
    text::*,
};
use std::{cell::Cell, rc::Rc};
fn style(size: f32, color: Color32) -> TextStyle {
    TextStyle {
        font_size: Some(size),
        color: Some(color),
        ..Default::default()
    }
}
fn defaults(size: f32, color: Color32) -> DefaultTextStyle {
    DefaultTextStyle {
        style: style(size, color),
        ..Default::default()
    }
}
fn constraints() -> BoxConstraints {
    BoxConstraints::loose(vec2(400.0, 1000.0)).unwrap()
}
fn ids(tree: &ElementTree) -> Vec<ElementId> {
    tree.elements().into_iter().map(|e| e.id).collect()
}
fn galleys(output: &egui::FullOutput) -> Vec<std::sync::Arc<egui::Galley>> {
    fn add(shape: &egui::Shape, out: &mut Vec<std::sync::Arc<egui::Galley>>) {
        match shape {
            egui::Shape::Text(text) => out.push(text.galley.clone()),
            egui::Shape::Vec(shapes) => {
                for s in shapes {
                    add(s, out);
                }
            }
            _ => {}
        }
    }
    let mut out = vec![];
    for s in &output.shapes {
        add(&s.shape, &mut out);
    }
    out
}
#[test]
fn style_merge_overlays_every_property_and_honors_replacement() {
    let base = TextStyle {
        font_size: Some(32.0),
        font_family: Some(FontFamily::Monospace),
        color: Some(Color32::RED),
        background: Some(Color32::BLUE),
        letter_spacing: Some(2.0),
        line_height: Some(LineHeight::Points(40.0)),
        italics: Some(true),
        underline: Some(Stroke::new(2.0_f32, Color32::RED)),
        strikethrough: Some(Stroke::new(1.0_f32, Color32::GREEN)),
        vertical_align: Some(egui::Align::Min),
        background_expansion: Some(3.0),
        ..Default::default()
    };
    let overlay = TextStyle {
        font_size: Some(20.0),
        font_family: Some(FontFamily::Proportional),
        color: Some(Color32::GREEN),
        background: Some(Color32::TRANSPARENT),
        letter_spacing: Some(0.0),
        line_height: Some(LineHeight::Normal),
        italics: Some(false),
        underline: Some(Stroke::NONE),
        strikethrough: Some(Stroke::NONE),
        vertical_align: Some(egui::Align::Max),
        background_expansion: Some(0.0),
        ..Default::default()
    };
    assert_eq!(base.merge(&TextStyle::default()), base);
    assert_eq!(base.merge(&overlay), overlay);
    let replacement = TextStyle {
        inherit: false,
        color: Some(Color32::YELLOW),
        ..Default::default()
    };
    assert_eq!(base.merge(&replacement), replacement);
    let resolved = base.merge(&replacement).to_format();
    assert_eq!(resolved.font_id.size, 14.0);
    assert_eq!(resolved.font_id.family, FontFamily::Proportional);
    assert_eq!(resolved.color, Color32::YELLOW);
    assert_eq!(resolved.line_height, None);
    assert!(!resolved.italics);
}
#[test]
fn nested_spans_resolve_depth_first_without_leaking_sibling_styles() {
    let mut child = InlineSpan::new("hé").styled(TextStyle {
        inherit: false,
        color: Some(Color32::GREEN),
        ..Default::default()
    });
    child
        .children
        .push(InlineSpan::new(" big").styled(TextStyle {
            font_size: Some(30.0),
            ..Default::default()
        }));
    let content = InlineSpan::group(vec![child, InlineSpan::new(" sibling")]);
    let mut text = Text::rich(content);
    text.style.italics = Some(true);
    let p = text.resolve(&defaults(18.0, Color32::RED));
    assert_eq!(
        p.spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(),
        vec!["hé", " big", " sibling"]
    );
    assert_eq!(
        p.spans
            .iter()
            .map(|s| s.format.font_id.size)
            .collect::<Vec<_>>(),
        vec![18.0, 30.0, 18.0]
    );
    assert_eq!(
        p.spans.iter().map(|s| s.format.color).collect::<Vec<_>>(),
        vec![Color32::GREEN, Color32::GREEN, Color32::RED]
    );
    assert!(p.spans.iter().all(|s| s.format.italics));
    let mut text = Text::new("replaced");
    text.style.inherit = false;
    let p = text.resolve(&defaults(40.0, Color32::RED));
    assert_eq!(p.spans[0].format.font_id.size, 14.0);
    assert_eq!(p.spans[0].format.color, Color32::WHITE);
}
#[test]
fn explicit_line_policy_can_override_or_remove_inherited_limits() {
    let mut d = defaults(16.0, Color32::WHITE);
    d.max_lines = Some(2);
    d.soft_wrap = false;
    d.align = egui::Align::Max;
    d.overflow = TextOverflow::Clip;
    let mut text = Text::new("a\nb\nc");
    let p = text.resolve(&d);
    assert_eq!(p.max_lines, Some(2));
    assert!(!p.soft_wrap);
    assert_eq!(p.align, egui::Align::Max);
    text.line_limit = LineLimit::Unlimited;
    text.soft_wrap = Some(true);
    text.align = Some(egui::Align::Center);
    text.overflow = Some(TextOverflow::Visible);
    let p = text.resolve(&d);
    assert_eq!(p.max_lines, None);
    assert!(p.soft_wrap);
    assert_eq!(p.align, egui::Align::Center);
    assert_eq!(p.overflow, TextOverflow::Visible);
    text.line_limit = LineLimit::Lines(1);
    assert_eq!(text.resolve(&d).max_lines, Some(1));
}
#[test]
fn text_scale_is_applied_once_after_nested_style_resolution() {
    for factor in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert!(TextScale::linear(factor).is_err());
    }
    let mut d = defaults(16.0, Color32::WHITE);
    d.scale = TextScale::linear(1.5).unwrap();
    d.style.line_height = Some(LineHeight::Points(24.0));
    d.style.letter_spacing = Some(2.0);
    let mut text = Text::rich(InlineSpan::new("nested").styled(TextStyle {
        font_size: Some(20.0),
        ..Default::default()
    }));
    let format = &text.resolve(&d).spans[0].format;
    assert_eq!(format.font_id.size, 30.0);
    assert_eq!(format.line_height, Some(36.0));
    assert_eq!(format.extra_letter_spacing, 2.0);
    text.scale = Some(TextScale::default());
    assert_eq!(text.resolve(&d).spans[0].format.font_id.size, 20.0);
    assert_eq!(d.scale.factor(), 1.5);
}
#[test]
fn inherited_updates_reconfigure_existing_text_and_paragraph_elements() {
    let ctx = egui::Context::default();
    ctx.begin_pass(egui::RawInput::default());
    let mut tree = ElementTree::new(
        &ctx,
        defaults(16.0, Color32::RED).provide(Text::new("Ag inherited").into_widget()),
    )
    .unwrap();
    let original = ids(&tree);
    let before = tree.layout(constraints()).unwrap();
    let handle = tree
        .inherited_handle::<DefaultTextStyle>(tree.root_id().unwrap())
        .unwrap();
    handle
        .update(|old| {
            let mut d = old.clone();
            d.style.font_size = Some(30.0);
            d.style.color = Some(Color32::GREEN);
            d
        })
        .unwrap();
    let dry = tree.dry_layout(constraints()).unwrap();
    let after = tree.layout(constraints()).unwrap();
    assert_eq!(dry, after.size);
    assert!(after.size.x > before.size.x);
    assert!(after.size.y > before.size.y);
    assert_eq!(ids(&tree), original);
    assert!(after.baseline(TextBaseline::Alphabetic) > before.baseline(TextBaseline::Alphabetic));
    egui::CentralPanel::default().show(&ctx, |ui| {
        tree.show(ui, constraints()).unwrap();
    });
    let out = ctx.end_pass();
    let rendered = galleys(&out);
    assert_eq!(rendered.len(), 1);
    assert_eq!(rendered[0].job.sections[0].format.color, Color32::GREEN);
    assert_eq!(rendered[0].job.sections[0].format.font_id.size, 30.0);
}
struct Watch {
    builds: Rc<Cell<usize>>,
}
impl StatelessWidget for Watch {
    fn build(&self, ctx: &BuildContext) -> Widget {
        self.builds.set(self.builds.get() + 1);
        Text::new(format!(
            "{}",
            DefaultTextStyle::of(ctx).unwrap().style.font_size.unwrap()
        ))
        .into_widget()
    }
}
#[test]
fn nested_default_styles_shadow_outer_updates_and_equal_values_do_not_notify() {
    let ctx = egui::Context::default();
    ctx.begin_pass(egui::RawInput::default());
    let builds = Rc::new(Cell::new(0));
    let nested = defaults(20.0, Color32::BLUE).provide(Widget::stateless(Watch {
        builds: builds.clone(),
    }));
    let mut tree = ElementTree::new(&ctx, defaults(16.0, Color32::RED).provide(nested)).unwrap();
    let initial = ids(&tree);
    let handle = tree
        .inherited_handle::<DefaultTextStyle>(tree.root_id().unwrap())
        .unwrap();
    assert_eq!(builds.get(), 1);
    handle
        .update(|old| {
            let mut d = old.clone();
            d.style.font_size = Some(40.0);
            d
        })
        .unwrap();
    tree.flush().unwrap();
    assert_eq!(builds.get(), 1);
    let inner = tree.elements()[1].id;
    let inner_handle = tree.inherited_handle::<DefaultTextStyle>(inner).unwrap();
    inner_handle
        .update(|old| {
            let mut d = old.clone();
            d.style.font_size = Some(20.0);
            d
        })
        .unwrap();
    tree.flush().unwrap();
    assert_eq!(builds.get(), 1);
    inner_handle
        .update(|old| {
            let mut d = old.clone();
            d.style.font_size = Some(22.0);
            d
        })
        .unwrap();
    tree.flush().unwrap();
    assert_eq!(builds.get(), 2);
    assert_eq!(ids(&tree), initial);
    let _ = ctx.end_pass();
}
#[test]
fn high_level_text_uses_fallback_defaults_and_empty_groups_do_not_emit_rows() {
    let ctx = egui::Context::default();
    ctx.begin_pass(egui::RawInput::default());
    let mut tree = ElementTree::new(&ctx, Text::new("fallback").into_widget()).unwrap();
    egui::CentralPanel::default().show(&ctx, |ui| {
        tree.show(ui, constraints()).unwrap();
    });
    let out = ctx.end_pass();
    let rendered = galleys(&out);
    let format = &rendered[0].job.sections[0].format;
    assert_eq!(format.font_id.size, 14.0);
    assert_eq!(format.color, Color32::WHITE);
    ctx.begin_pass(egui::RawInput::default());
    tree.update(Text::rich(InlineSpan::group(vec![])).into_widget())
        .unwrap();
    assert_eq!(tree.dry_layout(constraints()).unwrap(), egui::Vec2::ZERO);
    let empty = Text::new("").resolve(&DefaultTextStyle::default());
    assert_eq!(empty.spans.len(), 1);
    let _ = ctx.end_pass();
}
#[test]
fn inherited_invalid_geometry_is_recoverable_and_scaling_overflow_is_rejected() {
    let ctx = egui::Context::default();
    ctx.begin_pass(egui::RawInput::default());
    let mut tree = ElementTree::new(
        &ctx,
        defaults(16.0, Color32::WHITE).provide(Text::new("valid").into_widget()),
    )
    .unwrap();
    let original = ids(&tree);
    let handle = tree
        .inherited_handle::<DefaultTextStyle>(tree.root_id().unwrap())
        .unwrap();
    handle
        .update(|old| {
            let mut d = old.clone();
            d.max_lines = Some(0);
            d
        })
        .unwrap();
    assert!(tree.layout(constraints()).is_err());
    assert_eq!(ids(&tree), original);
    handle
        .update(|old| {
            let mut d = old.clone();
            d.max_lines = None;
            d.style.font_size = Some(f32::MAX);
            d.scale = TextScale::linear(2.0).unwrap();
            d
        })
        .unwrap();
    assert!(tree.dry_layout(constraints()).is_err());
    handle
        .update(|old| {
            let mut d = old.clone();
            d.style.font_size = Some(18.0);
            d.scale = TextScale::default();
            d
        })
        .unwrap();
    assert!(tree.layout(constraints()).is_ok());
    assert_eq!(ids(&tree), original);
    let _ = ctx.end_pass();
}
#[test]
fn scaled_nested_spans_reach_actual_galley_with_utf8_ranges_and_baselines() {
    let ctx = egui::Context::default();
    ctx.begin_pass(egui::RawInput::default());
    let mut d = defaults(16.0, Color32::WHITE);
    d.scale = TextScale::linear(1.25).unwrap();
    let text = Text::rich(InlineSpan::group(vec![
        InlineSpan::new("hé "),
        InlineSpan::new("large").styled(style(24.0, Color32::LIGHT_BLUE)),
    ]));
    let mut tree = ElementTree::new(&ctx, d.provide(text.into_widget())).unwrap();
    let dry = tree
        .dry_baseline(constraints(), TextBaseline::Alphabetic)
        .unwrap();
    let actual = tree.layout(constraints()).unwrap();
    assert_eq!(actual.baseline(TextBaseline::Alphabetic), dry);
    egui::CentralPanel::default().show(&ctx, |ui| {
        tree.show(ui, constraints()).unwrap();
    });
    let out = ctx.end_pass();
    let rendered = galleys(&out);
    let sections = &rendered[0].job.sections;
    assert_eq!(sections[0].byte_range, 0..4);
    assert_eq!(sections[1].byte_range, 4..9);
    assert_eq!(sections[0].format.font_id.size, 20.0);
    assert_eq!(sections[1].format.font_id.size, 30.0);
    assert_eq!(sections[1].format.color, Color32::LIGHT_BLUE);
}

# Retained paragraphs

`text::Paragraph` is a persistent render-object widget for plain text and flat,
fully formatted `TextSpan` runs. It uses the active egui font collection for
measurement and painting on desktop and Android. It participates in flex,
constraints, padding, stacks, intrinsic wrappers and baseline alignment without
application measurement or painting callbacks.

```rust
use rust_flutter::{egui, element::ElementTree, layout::BoxConstraints,
    text::{Paragraph, TextOverflow, TextSpan}};

let ctx = egui::Context::default();
// Fonts are initialized by the first pass; normal applications already have one.
ctx.begin_pass(egui::RawInput::default());
let format = egui::text::TextFormat {
    font_id: egui::FontId::proportional(18.0),
    color: egui::Color32::WHITE,
    ..Default::default()
};
let mut emphasis = format.clone();
emphasis.color = egui::Color32::LIGHT_BLUE;
let mut paragraph = Paragraph::rich(vec![
    TextSpan::new("Build in ", format),
    TextSpan::new("Rust", emphasis),
]);
paragraph.max_lines = Some(2);
paragraph.overflow = TextOverflow::Ellipsis;
let mut tree = ElementTree::new(&ctx, paragraph.into_widget())?;
let constraints = BoxConstraints::loose(egui::vec2(240.0, 100.0))?;
let expected = tree.dry_layout(constraints)?;
egui::CentralPanel::default().show(&ctx, |ui| {
    let response = tree.show(ui, constraints).unwrap();
    assert_eq!(response.rect.size(), expected);
});
let _ = ctx.end_pass();
# Ok::<(), Box<dyn std::error::Error>>(())
```

Formatting includes font family/size, color, background, letter spacing, explicit
line height, italics, underline and strikethrough through `egui::text::TextFormat`.
Install custom font families with `Context::set_fonts`. Each span is fully
resolved; nested style inheritance, ambient DefaultTextStyle and a font asset
loader are not yet supplied. `TextFormat`'s default color is gray, so set a color
explicitly for a themed application.

## Sizing and overflow

Soft wrapping defaults to enabled. Explicit newlines remain breaks when it is
disabled. The box fits its longest laid-out row, then obeys parent constraints;
it does not implement Flutter's configurable TextWidthBasis. `align` positions
rows left, center or right inside the allocated width. Backend row positions are
rounded to physical pixels. This is alignment, not bidirectional reordering.

`max_lines` must be positive when supplied. `Clip` uses the box's hard clip;
`Visible` lets ink extend through it, subject to ancestor clipping. Both can
truncate at a line limit without adding a marker. `Ellipsis` requires a line
limit and appends the backend ellipsis when text is omitted. Without soft wrap,
ellipsis requires exactly one line. Height constraints clip ink rather than
selecting a new line limit. Zero-width boxes remain valid.

Dry layout and baseline queries use the same font/layout policy as normal
layout. They may populate the backend font cache but do not replace the retained
galley selected for painting, lay out children, or mutate application state.
All font measurements require fonts initialized by a first egui pass. Querying
before that is a host API precondition violation, as with `Context::fonts_mut`.

Maximum intrinsic width measures the complete untruncated text without soft
wrap. Minimum intrinsic width measures the longest Unicode line-break segment,
trimming breakable trailing whitespace while retaining nonbreaking spaces and
span formats. The pinned `unicode-linebreak` 0.1.5 implementation supplies UAX #14
opportunities. This is a content contribution; egui's emergency character wrapping
can still fit smaller widths. Min/max intrinsic heights use the configured line
limit at the supplied width. The measurement and backend wrapping policies are
not an audited match for Flutter's engine, especially for complex scripts.

The first glyph of the first row supplies an alphabetic baseline, matching the
galley that is painted. An empty row without glyphs has no real baseline.
Ideographic metrics are unavailable and return `None`. Mixed font runs retain
the backend's vertical-alignment policy; they do not implement a complete
TextPainter/strut/text-height model.

## Retention and invalidation

Updating a Paragraph preserves its render-object identity. Pure queries do not
replace painted geometry. The new `RenderBox::layout_revision` hook samples
external layout inputs during tree synchronization and invalidates the tree's
coarse layout cache when a revision changes, including ancestor custom boxes.
Other render boxes default to revision zero and retain their existing caching.

The current font backend exposes no font-collection generation. Paragraphs use
the current egui pass number as their revision, so text and cached ancestors
measure again each pass; repeated layouts in the same pass reuse geometry.
The backend still caches layout jobs. This conservative policy covers font
replacement, scale/zoom and density changes without requiring widget rebuilds.
A font-generation signal and precise per-child dependencies remain future work.

## Verification and remaining scope

`tests/paragraphs.rs` covers constraints, wrapping, explicit newlines, UTF-8 styled
runs, line limits/ellipsis, overflow clips, dry/actual agreement, intrinsic widths
and heights, text alignment, recoverable invalid configurations, query/paint
separation, and a font-family change beneath a cached custom ancestor.
The native gallery's mixed-size baseline fixture now uses Paragraph widgets and
positioned background boxes. The Linux and Android smoke probes inspect the
resulting font boxes and reference baseline in actual screenshots.

This is an experimental paragraph primitive. Full shaping, bidi, RTL traversal,
fallback/language coverage, selection/caret geometry, editable text, semantics,
inline widgets, nested TextSpan inheritance, struts, locale-aware line breaking,
nonlinear accessibility text scaling, and complete Flutter Text/RichText member
compatibility remain unfinished.

References: [RenderParagraph](https://api.flutter.dev/flutter/rendering/RenderParagraph-class.html),
[dry layout](https://api.flutter.dev/flutter/rendering/RenderParagraph/computeDryLayout.html),
[minimum intrinsic width](https://api.flutter.dev/flutter/rendering/RenderParagraph/computeMinIntrinsicWidth.html),
[unicode-linebreak](https://docs.rs/unicode-linebreak/0.1.5/unicode_linebreak/).

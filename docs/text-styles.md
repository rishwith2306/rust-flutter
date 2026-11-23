# Inherited text and nested spans

Use `text::Text` for text that follows subtree typography. It is a stateless
component which resolves the closest `DefaultTextStyle`, flattens nested
`InlineSpan` descriptions, and returns the retained [Paragraph](paragraphs.md)
render widget. Direct Paragraph/TextSpan users continue to supply fully resolved
formats; their behavior is unchanged.

```rust
use rust_flutter::{egui, element::ElementTree, layout::BoxConstraints,
    text::{DefaultTextStyle, InlineSpan, Text, TextScale, TextStyle}};

let ctx = egui::Context::default();
ctx.begin_pass(egui::RawInput::default());
let defaults = DefaultTextStyle {
    style: TextStyle {
        font_size: Some(18.0),
        color: Some(egui::Color32::WHITE),
        ..Default::default()
    },
    scale: TextScale::linear(1.25)?,
    ..Default::default()
};
let content = InlineSpan::group(vec![
    InlineSpan::new("Build with "),
    InlineSpan::new("Rust").styled(TextStyle {
        color: Some(egui::Color32::LIGHT_BLUE),
        ..Default::default()
    }),
]);
let mut tree = ElementTree::new(&ctx, defaults.provide(Text::rich(content).into_widget()))?;
egui::CentralPanel::default().show(&ctx, |ui| {
    tree.show(ui, BoxConstraints::loose(egui::vec2(280.0, 100.0)).unwrap()).unwrap();
});
let _ = ctx.end_pass();
# Ok::<(), Box<dyn std::error::Error>>(())
```

`Text::new("Hello")` is the plain-text equivalent. Public Text fields override
inherited defaults. `style` overrides individual format properties; `align`,
`soft_wrap`, `overflow` and `scale` use optional overrides. `line_limit` is
`LineLimit::Inherit` by default. `Unlimited` explicitly removes an inherited
limit, while `Lines(n)` sets one. Removing a limit also requires overriding an
inherited Ellipsis overflow with Clip or Visible. Zero lines and invalid final formats fail during
paragraph measurement and preserve mounted elements for recovery.

## Inheritance rules

`TextStyle` contains optional font size/family/weight, foreground/background
color, letter spacing, line height, italics, underline, strikethrough, vertical
alignment and background expansion. `None` leaves a property unspecified.
`Some(false)`, transparent colors and `Stroke::NONE` can explicitly clear
inherited effects. `LineHeight::Normal` restores font-derived line height;
`LineHeight::Points(n)` sets a logical-point line height.

`FontWeight` represents weights 100 through 900 in steps of 100. To render an
actual weight face, register its font data in egui under the family returned by
`FontWeight::family_variant(&base_family)`. Paragraph layout selects that family
when present and falls back to `base_family` otherwise. For example, after
adding a medium font file as `inter-medium` in `definitions.font_data`:

```rust
let inter = egui::FontFamily::Name("Inter".into());
let medium = FontWeight::W500.family_variant(&inter);
definitions.families.insert(medium, vec!["inter-medium".into()]);
```

`base.merge(&overlay)` overlays specified properties. If `overlay.inherit` is
false, it replaces the base entirely. The remaining unspecified fields resolve
to 14-point proportional white text, normal line height, zero letter spacing,
transparent background and no effects. Configure a scoped color appropriate to
your application theme; these fallback defaults do not follow host visuals.

An InlineSpan emits its own optional text before its children. Each child fills
unspecified properties from its parent span, irrespective of its style's
`inherit` flag. This follows the distinction in Flutter's documented style and
inline-span inheritance. A Text's outer `style.inherit = false` still replaces
the ambient text style. Siblings resolve independently; formatting from one
sibling does not leak into the next. Flattening uses an explicit traversal stack
and constructs UTF-8-safe paragraph sections. `InlineSpan::group` emits no text
itself; `InlineSpan::new("")` preserves an explicit empty text run.

`DefaultTextStyle::provide` installs a complete defaults value for a subtree,
shadowing any outer defaults of the same type. It does not implicitly merge the
two scopes. During a component build, `DefaultTextStyle::of(ctx)?` listens to the
closest value (or supplies framework fallbacks if absent). To merge only style,
use `DefaultTextStyle::of(ctx)?.merged_style(&overlay).provide(child)`; layout
policy and scale remain those of the receiver. A non-listening initialization
lookup can use `BuildContext::read::<DefaultTextStyle>()`.

Providers use the ordinary inherited-value queue, so callers can retrieve an
`InheritedHandle<DefaultTextStyle>` for the provider element and use `set` or
`update`. The update callback receives `&DefaultTextStyle` and returns a new
value. Unequal defaults notify dependents; equal values do not. Text rebuilds
resolve a new Paragraph configuration while preserving both the Text element
and its Paragraph render-object identity. Nested providers shield their text
from outer changes. Subscription granularity currently covers the whole
DefaultTextStyle value, including properties a particular Text overrides.

## Scaling and scope

`TextScale::linear(factor)` accepts finite, positive factors and defaults to one.
`TextScale::nonlinear(samples)` accepts finite `(authored_size, rendered_size)`
pairs with positive sizes, strictly increasing authored sizes, and
nondecreasing rendered sizes. It interpolates between samples and extends the
first or last segment outside the sample range. The selected scale applies once,
after all span inheritance, to font sizes and explicit point line heights.
Letter spacing, decoration widths and background expansion remain in logical
points. A Text can override the inherited scale. Overflow after scaling is
rejected by the paragraph's geometry validation.

For a custom curve, pass authored/rendered sizes in logical points:

```rust
let scale = TextScale::nonlinear([
    (12.0, 12.0),
    (20.0, 20.0),
    (32.0, 30.0),
    (48.0, 40.0),
])?;
let defaults = DefaultTextStyle {
    scale,
    ..DefaultTextStyle::default()
};
```

The Android host obtains the user's actual nonlinear SP conversion from
`TypedValue.applyDimension` and supplies a sampled curve to the retained text
tree. The macOS host does not yet read system accessibility settings. System
bold/spacing settings are also not applied.

The implementation supplies the subset represented by egui TextFormat and the
current Paragraph policy. Font variable axes, shaping, bidi, locale-aware
fallback, inline widget placeholders, span recognizers, semantics, selection,
TextWidthBasis, struts, ambient Directionality and animated default styles remain
unfinished. System bold-text accessibility settings are not applied. The types
are original Rust APIs; no complete Flutter member-level compatibility is
claimed.

## Verification

`tests/text_styles.rs` covers style overlay/replacement, nested inheritance,
sibling isolation, explicit clearing, line-policy overrides, scaling, live
inherited updates with stable element IDs, nested shadowing/equality gates,
fallback/empty text behavior, recoverable invalid inherited values, scaling
overflow and actual galley formatting/UTF-8 ranges/baselines. The native gallery
uses scoped defaults and nested Text spans in its mixed-size baseline fixture;
Linux and Android smoke tests check the resulting rendered boxes and baseline.
Live style changes are covered by headless render tests, not a device theme or
accessibility test.

References: [DefaultTextStyle](https://api.flutter.dev/flutter/widgets/DefaultTextStyle-class.html),
[Text.style](https://api.flutter.dev/flutter/widgets/Text/style.html),
[TextStyle.merge](https://api.flutter.dev/flutter/painting/TextStyle/merge.html),
[inline inheritance distinction](https://api.flutter.dev/flutter/painting/TextStyle/inherit.html).

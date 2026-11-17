# Box decorations and foreground painting

`decoration::DecoratedBox` paints a solid rounded fill and uniform inside border
before or after its single child. It preserves the child's constraints, size,
position, intrinsic contributions and real baselines. It neither adds padding
nor clips the child to the rounded shape.

```rust
use rust_flutter::{egui, decoration::{BoxDecoration, DecoratedBox, DecorationPosition},
    element::ElementTree, layout::BoxConstraints, text::Text};
let ctx = egui::Context::default();
ctx.begin_pass(egui::RawInput::default());
let border = DecoratedBox {
    decoration: BoxDecoration {
        border: egui::Stroke::new(2.0_f32, egui::Color32::LIGHT_BLUE),
        corner_radius: egui::CornerRadius::same(6),
        ..Default::default()
    },
    position: DecorationPosition::Foreground,
};
let mut tree = ElementTree::new(&ctx, border.into_widget(Text::new("Rust UI").into_widget()))?;
egui::CentralPanel::default().show(&ctx, |ui| {
    tree.show(ui, BoxConstraints::tight(egui::vec2(180.0, 40.0)).unwrap()).unwrap();
});
let _ = ctx.end_pass();
# Ok::<(), Box<dyn std::error::Error>>(())
```

Use `DecoratedBox::new(BoxDecoration::solid(color))` for a background fill.
Foreground fills can intentionally obscure child content. A foreground border
stays above opaque child painting. Add `Widget::padding` explicitly when content
should avoid border space. `Widget::empty()` can be supplied as a child for an
otherwise empty decorated region; tight constraints determine its size.

Border widths must be finite and nonnegative. Invalid configuration and wrong
child counts report LayoutError during normal and pure measurement and leave
mounted elements available for recovery. Corner radii use egui's per-corner u8
logical-point values. Borders are drawn inside the allocated rectangle. Rounded
corners affect paint only: geometry hit testing remains rectangular and the
decorator does not add a self hit target. Child controls keep their ordinary
input handling. Child paint outside rounded corners remains visible unless an
ancestor supplies clipping. Decoration updates retain matching child identity.

## Custom foreground painting

RenderBox now has two optional callbacks:

- `paint(ui, ctx)` runs before placed descendants.
- `paint_foreground(ui, ctx)` runs after all placed descendants.

The traversal is depth-first: parent background, each child's complete subtree
in placement order, then parent foreground. Both parent phases receive the same
Ui with its stable ID and box geometry. Painters should fit the allocated box
and preserve UI clip/layout settings they change. Inherited and hard-edge box
clips apply to both phases. Paint ordering does not change layout placement or
geometry hit-test order. `RenderHandle::mark_needs_paint` requests a frame without
invalidating custom layout caches, as before.

Standalone geometry consumers can use `layout::show_layout_with_phases`, whose
callback receives `PaintPhase::Background` or `PaintPhase::Foreground`. The legacy
`show_layout` calls its callback once per participating box, before descendants,
so existing callers retain their callback count. Boxes with `paint_self = false`
receive neither phase. Leaves participate in both phases in the new API.

The implementation still paints through egui each frame. It does not add retained
display lists, arbitrary child/painter interleaving, transforms, saveLayer effects,
rounded clipping, repaint boundaries or a complete Flutter PaintingContext.
BoxDecoration currently lacks gradients, images, shadows, circle shapes, separate
edge borders and Flutter's decoration painter/configuration lifecycle.

## Verification

`tests/decoration.rs` covers opaque-child ordering for both positions, nested
RenderBox phase ordering and shared Ui identity/geometry/clip, legacy callback
counts, dry/actual geometry and baseline forwarding, intrinsic sizes, invalid
configuration recovery, child arity, retained property updates, border/rounding
shape data and inside stroke policy.

The native gallery uses decorated backgrounds beneath its two font samples and
a separate 24-point foreground fill over an opaque child of a different color.
The Linux and Android screenshot probe requires the foreground's exact RGB
interior, correct physical size, absence of the hidden child color, and the
existing mixed-font baseline alignment. This verifies solid foreground ordering;
it does not establish cross-platform rounded-border antialiasing or all alpha
compositing behavior.

References: [DecoratedBox](https://api.flutter.dev/flutter/widgets/DecoratedBox-class.html),
[DecorationPosition](https://api.flutter.dev/flutter/rendering/DecorationPosition.html),
[RenderDecoratedBox.paint](https://api.flutter.dev/flutter/rendering/RenderDecoratedBox/paint.html).

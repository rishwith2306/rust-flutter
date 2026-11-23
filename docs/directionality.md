# Text direction

`Directionality` provides an inherited `layout::TextDirection`. `MaterialApp`
provides left-to-right by default and accepts an explicit override with
`.text_direction(...)`:

```rust
use rust_flutter::{
    element::{BuildContext, Directionality, FlexStyle},
    layout::TextDirection,
};

fn row_style(context: &BuildContext) -> FlexStyle {
    let mut style = FlexStyle::new(rust_flutter::layout::Axis::Horizontal);
    style.text_direction = Directionality::of(context).unwrap();
    style
}

let _rtl = TextDirection::Rtl;
```

`Directionality::of` subscribes the current component build, so changing the
scope rebuilds consumers. Widget builders must apply the value to direction
sensitive layout (for example, `FlexStyle::text_direction`); the scope does not
rewrite arbitrary widget descriptions. Text bidi shaping, automatic direction
selection from locale, and RTL adaptation across all controls and scroll views
remain unfinished. See the Flutter
[Directionality API](https://api.flutter.dev/flutter/widgets/Directionality-class.html).

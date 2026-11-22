# Text input

`TextFormField` connects a retained text editor to `FormController`. It can own
its initial text or share a `TextEditingController` with application state.
The shared controller is useful for programmatic edits and reading the final
value after a reset or save.

```rust
use rust_flutter::forms::{
    TextEditingController, TextFormField, TextFieldDecoration,
};

let controller = TextEditingController::new("");
let widget = TextFormField::with_controller(controller.clone())
    .decoration(TextFieldDecoration {
        label: Some("Name".into()),
        hint: "Enter your name".into(),
        ..Default::default()
    })
    .validator(|value| value.trim().is_empty().then(|| "Required".into()))
    .on_changed(|value| println!("changed: {value}"))
    .into_widget()?;

controller.set_text("Ada")?;
assert_eq!(controller.text(), "Ada");
# let _ = widget;
# Ok::<(), rust_flutter::forms::TextEditingError>(())
```

`TextEditingValue` stores text, an optional selection and an optional composing
range. Offsets use UTF-16 code units to match mobile text APIs; setters reject
out-of-range offsets and positions inside a surrogate pair without partially
applying the value. `set_text` clears selection and composing state. `set_value`
validates and applies all three together. Text edits mark the form field as
interacted; selection-only changes do not. Form reset restores the initial
text and clears selection/composing state.

Use `TextFormField::multiline(min_lines, max_lines)` for a vertically scrollable
editor. `read_only(true)` retains focus and selection while using an immutable
text buffer. `enabled(false)` disables interaction. `obscure_text(true)` masks
single-line input, and `max_chars(Some(n))` limits the editor's character count.

This is the framework's egui-backed input adapter. It does not implement
Flutter's Material decoration, platform IME composition bridge, autofill,
restoration, accessibility announcements, full bidi shaping or complete
`EditableText` keyboard and selection behavior. See [the form limitations](forms.md#typed-forms)
and the [parity audit](flutter-parity.md) for the remaining work.

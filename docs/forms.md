# Typed forms

`Form` provides a retained scope; `FormField<T>` retains a typed value and adapts
it to a control through a builder. Its fields register with the nearest form,
unregister on removal, and move to a replacement form controller without losing
values. Fields also work without a form. Nullable values use `Option<T>`.

```rust
use rust_flutter::{egui, element::Widget, forms::{Form, FormController, FormField}};
let form = FormController::new();
let name = FormField::new(String::new(), |_, field| {
    Widget::leaf(egui::vec2(240.0, 40.0), move |ui| {
        let mut text = field.current_value();
        if ui.text_edit_singleline(&mut text).changed() {
            let _ = field.did_change(text);
        }
    })
}).validator(|name| name.trim().is_empty().then(|| "Enter a name".into()));
let widget = Form::new(form.clone()).into_widget(name.into_widget());
// After mounting and user input, validate before saving:
if form.validate()? { form.save()?; }
# Ok::<(), rust_flutter::forms::FormError>(())
```

The builder is observed through the existing Listenable widget bridge and
rebuilds for value/error changes. Use `error_text()` and
`snapshot().validation_failure` to render validation feedback. `current_value()`
returns a clone; call `did_change` to report user edits. Mutating interior
state inside a cloned value without reporting the change cannot trigger
notifications or supersession detection. It marks interaction even
for equal values. `snapshot().enabled` lets an adapter disable its actual control;
the controller does not implicitly intercept that control's input.

`FormController::validate` validates all registered fields, including disabled
fields, and returns a checked bool. `validate_granularly` returns invalid field
IDs; `field_id(&field)` maps a typed controller to its form-local ID. `save`
invokes each `on_saved` callback and does not validate implicitly. `reset`
restores each field's latest initial value, clears errors and interaction, and
invokes `on_reset`. Changing widget properties preserves the current value;
changing `initial_value` affects the next reset. `clear_errors` clears published
errors and interaction without replacing values. These operations are synchronous.

Field-level automatic validation supports Disabled, Always, OnUserInteraction,
OnUnfocus and OnUserInteractionIfError. Configure the public `autovalidate_mode`
on FormField. Always runs during field builds, edits and reset. Interaction modes
use that field's interaction state; the error-only mode also requires a current
published error. OnUnfocus requires an adapter to call `did_unfocus`, for example
when an egui response reports `lost_focus()`. Disabled fields skip automatic
validation. There is no form-wide automatic validation policy or framework Focus
node integration yet. Clearing errors in Always mode can make them reappear on
the next builder pass; unchanged automatic results do not cause a rebuild loop.
Validators should be side-effect-free because builds may validate repeatedly.

`force_error_text` publishes an externally supplied error and skips the validator
when checking validity. `is_valid` checks without publishing errors or changing
interaction. Callback panics become `FormError::CallbackPanicked`; Rust's normal
panic hook can still log them. Automatic validator failures are exposed separately
as `validation_failure`, so absence of an error string does not imply success.
Recursive validation, save and reset operations return Reentrant. Validators that edit values through the controller or replace
configuration return Superseded, and stale error results are not published.
Whole-form validation also detects a later validator changing an earlier field,
or registration membership changes during the operation.

Commands snapshot registration order, skip removed members, and report
Superseded when membership changes. Newly registered fields participate in the
next command. Validation/saving/resetting are not atomic transactions: completed
callbacks and previously published errors are not rolled back after a later
failure. Keep side effects in save callbacks and handle Result values.

For a model outside a retained widget, construct a FormFieldController, set its
validator, and retain the guard returned by `form.register(&field)`. Registrations
hold weak fields; they cannot retain a field or form by themselves. Multiple
registrations of the same field share an ID and require all guards to be dropped
before removal. Controllers retained by applications remain usable after their
widget unmounts, while its automatic membership is removed. `downgrade()` supplies
weak controller handles for callbacks that refer back to their own field.

FormController implements Listenable and field controllers additionally implement
ValueListenable. Form notifications include field state, validation and membership
changes; this is broader than Flutter Form.onChanged. Nested forms isolate their
own fields. The gallery's Forms page demonstrates a name field and required terms
checkbox, validation, save and reset. The shared native probe uses injected input
and rendered feedback without a form-state mutation hook.

The implementation was reviewed against official [FormState](https://api.flutter.dev/flutter/widgets/FormState-class.html),
[FormFieldState](https://api.flutter.dev/flutter/widgets/FormFieldState-class.html),
[AutovalidateMode](https://api.flutter.dev/flutter/widgets/AutovalidateMode.html) and
[form source](https://github.com/flutter/flutter/blob/master/packages/flutter/lib/src/widgets/form.dart).
It remains partial: form-wide policies, TextFormField/Material controls, async
validation, autofill, restoration, navigation vetoes, semantics announcements,
focus management and full member compatibility are unfinished. See
[verification](verification.md) for actual headless and native evidence.

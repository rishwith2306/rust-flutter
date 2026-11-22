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
validation. Form-wide policies are described below; a framework Focus node is not yet
integrated. Clearing errors in Always mode can make them reappear on
the next builder pass; unchanged automatic results do not cause a rebuild loop.
Validators should be side-effect-free because builds may validate repeatedly.

`TextFormField` supplies a retained egui text editor attached to the same
typed form model. Choose either `TextFormField::new(initial)` or
`TextFormField::with_controller(controller)`, then configure its decoration,
validator and save/change/submit callbacks. A controller exposes an atomic
`TextEditingValue` containing UTF-8 text, optional selection and composing
ranges expressed as UTF-16 offsets. Invalid offsets, including offsets inside
a surrogate pair, are rejected without changing state. Replacing text clears
selection and composition; use `set_value` for an atomic edit. Reset restores
the form's initial text and clears editor selection/composition. Selection-only
updates do not mark the form as user-interacted. The controller is externally
owned when supplied by the application and remains alive after the field is
removed; internally created controllers are disposed with the field.

This field currently supports single-line and multiline input, labels, hints,
read-only/disabled states, password display, character limits, selection,
reset, focus retention in lazy lists and UTF-16 controller offsets. It is an
egui backend adapter, not Flutter's Material `TextFormField` or `EditableText`:
platform IME composition, autofill, restoration, accessibility semantics,
advanced bidi/shaping and Flutter's complete selection/keyboard behavior still
need implementation and macOS/Android device verification.

Form-wide policies use the same enum:

```rust
use rust_flutter::forms::{AutovalidateMode, FormController};
let form = FormController::new();
form.set_autovalidate_mode(AutovalidateMode::OnUserInteraction);
```

Always validates on membership, configuration and value changes, including
initial registration. OnUserInteraction validates related fields after any user
edit; explicit form validation also marks the form as interacted. New fields
join that active policy. OnUserInteractionIfError additionally requires an
already published field error, so it stops checking after all errors clear.
OnUnfocus validates only the departing field when its adapter calls
`did_unfocus`. Disabled performs no group auto-validation. Field-level policies
remain independent and may cause additional validator calls.

Set `Form.autovalidate_mode` to `Some(mode)` to override the controller policy
when constructing that scope; None preserves the controller's existing setting.
Group automatic passes skip disabled fields. Explicit form validation still
includes them. This enabled-field policy follows the public FormField contract;
exact lifecycle behavior of Flutter's form-wide implementation still requires a
pinned SDK comparison. Field errors becoming visible are notifications, not new
user edits, and cannot recursively trigger another group pass.

Form reset batches all initial-value changes before field-level Always checks
and a pending group pass. In
Always mode it then revalidates; interaction modes become pristine until another
edit. Clear resets interaction too, with the same Always policy. Save does not
request validation itself. Completing a command can flush an automatic pass
already deferred by an earlier event or requested by one of its callbacks. Commands take effect in registration order, as described below.

Automatic callbacks that mutate values/membership or change the form policy are
detected as Superseded.
At most one automatic pass runs per event; further requests during that pass
remain pending. A later event or `flush_validation()` can process them, and
callbacks cannot cause a synchronous retry loop. `last_auto_failure()` exposes
the most recent automatic pass's infrastructure failure; a successful subsequent
automatic pass clears it. It is distinct from ordinary invalid-field errors and
is not a bool validity result. Inspect it when responding to automatically
validated input. Recursive explicit commands and flushes return Reentrant while
a pass is active. Changes to unrelated registered values also supersede a pass,
including an unfocus-only pass, so cross-field reads cannot silently use stale
controller revisions. Interior mutations still need `did_change` reporting.

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
It remains partial: full form-wide lifecycle/member compatibility,
TextFormField/Material controls, async
validation, autofill, restoration, navigation vetoes, semantics announcements,
focus management and full member compatibility are unfinished. See
[verification](verification.md) for actual headless and native evidence.

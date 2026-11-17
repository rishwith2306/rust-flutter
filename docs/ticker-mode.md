# Scoped tickers and managed animation lifetime

`Widget::ticker_mode(enabled, child)` controls automatic ticks for controllers
created through the child's BuildContext. `ctx.animation_controller(config)`
creates a controller owned by that element. It follows inherited ticker mode
and is automatically retired when its owner unmounts, including when external
controller or BuildContext clones remain alive.

```rust
use rust_flutter::{egui, animation::{AnimationConfig, AnimationController, AnimationOutcome},
    element::*, layout::BoxConstraints};
use std::time::Duration;
struct Pulse;
struct PulseState { controller: Option<AnimationController> }
impl StatefulWidget for Pulse {
    type State = PulseState;
    fn create_state(&self) -> PulseState { PulseState { controller: None } }
}
impl State<Pulse> for PulseState {
    fn init_state(&mut self, _: &Pulse, ctx: &StateContext<Self>) {
        self.controller = Some(ctx.animation_controller(AnimationConfig {
            duration: Duration::from_secs(1), ..Default::default()
        }).unwrap());
    }
    fn build(&mut self, _: &Pulse, _: &StateContext<Self>) -> Widget {
        Widget::value_listenable_builder(self.controller.as_ref().unwrap().clone(), |_, value| {
            Widget::leaf(egui::vec2(20.0 + 20.0 * value, 20.0), |_| {})
        })
    }
}
let ctx = egui::Context::default();
ctx.begin_pass(egui::RawInput { time: Some(0.0), ..Default::default() });
let mut tree = ElementTree::new(&ctx, Widget::ticker_mode(false, Widget::stateful(Pulse)))?;
let owner = tree.elements()[1].id;
let controller = tree.state::<PulseState>(owner).unwrap().controller.as_ref().unwrap().clone();
let run = controller.forward()?;
let _ = ctx.end_pass();
ctx.begin_pass(egui::RawInput { time: Some(0.5), ..Default::default() });
assert_eq!(controller.value(), 0.0); // muted, while time still elapses
let _ = ctx.end_pass();
tree.update(Widget::ticker_mode(true, Widget::stateful(Pulse)))?;
ctx.begin_pass(egui::RawInput { time: Some(0.75), ..Default::default() });
assert_eq!(controller.value(), 0.75); // same controller catches up
assert_eq!(tree.elements()[1].id, owner);
egui::CentralPanel::default().show(&ctx, |ui| {
    tree.show(ui, BoxConstraints::loose(egui::vec2(100.0, 80.0)).unwrap()).unwrap();
});
let _ = ctx.end_pass();
tree.unmount();
assert!(controller.is_disposed());
assert_eq!(run.outcome(), Some(AnimationOutcome::Canceled));
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Mode semantics

No scope means enabled tickers. Built-in nested ticker modes combine with
logical AND; a nested true mode cannot enable tickers beneath a false ancestor.
`TickerMode { enabled }` is the effective inherited snapshot. During build,
`ctx.depend_on::<TickerMode>()?` can observe it, with a missing value meaning true.
The built-in model is read-only and has no InheritedHandle. The wrapper adds no
render box and preserves layout, intrinsic contributions and reported baselines.
It does not hide, remove, disable input to, or suppress painting of its child.

Managed controllers combine scoped muting with explicit `set_muted`: either
reason is sufficient to mute. Clearing explicit muting cannot override a disabled
scope; enabling a scope does not clear explicit muting. Commands and direct
set_value still work while muted. Active animation time continues to elapse,
and the next owning frame after re-enabling catches up, possibly to completion.

Scopes refresh during widget reconciliation and ElementTree flush. Controller
owners refresh their permission directly without needing to rebuild, subscribe
to a mode value, or implement did_change_dependencies. Pending mode changes are
applied when the tree processes them; a controller's beginning-of-pass tick can
precede a mode change built later in that same pass. This is a ticker policy,
not a guarantee that no previous-mode sample can appear in the transition frame.

`AnimationController::new(ctx, config)` remains an unmanaged constructor. It does
not infer a widget scope and is unaffected by Widget::ticker_mode. Use the
BuildContext helper when element lifetime and scoped permission should apply.
Create owned controllers once in initialization or render-object creation,
rather than creating a new controller on every build. Multiple controllers per
owner are supported. The helper is also available through StateContext and
RenderContext dereferencing.

## Teardown

An element retires its managed controllers after closing its queue and before
its State/render-object disposal hooks run. Pending runs resolve Canceled,
commands/subscriptions fail, and snapshots remain readable through clones.
Managed controllers therefore need no extra manual dispose call in State::dispose.
An already manually disposed controller is skipped safely. A retained context
cannot create a new controller after unmount: the helper returns
AnimationError::Unmounted. Failed mounts also retire controllers initialized
before the failure.

If unmount happens inside a value/status listener, managed retirement marks the
controller disposed and cancels playback immediately. It defers clearing the
notifier lists until the outermost dispatch releases its borrows. New commands
and subscriptions fail during that interval, and no later status dispatch is
started for the retired source. Existing in-progress notifier dispatch keeps
its ordinary reentrant listener semantics. Manual dispose still rejects calls
during notification; deferred retirement is an internal ownership operation.

This is an original Rust ownership API, not a TickerProviderStateMixin/State API
clone. Force-frame policy, public Ticker scheduling objects, single-ticker
provider diagnostics, resync/adoption across replacement contexts, global-key
reparenting and complete Flutter ticker/member compatibility remain unfinished.

## Verification

`tests/ticker_mode.rs` covers nested AND, elapsed-time catch-up, stable identities,
mode refresh without owner rebuild, explicit/scoped mute composition, unmanaged
controller independence, multiple owned controllers, retained context/handle
teardown, failed initial mounts, value/status-listener unmount, read-only mode
and geometry/intrinsic/baseline forwarding. Existing manual controller tests
continue to verify ordinary disposal rejection and playback behavior.

The native gallery uses a managed marker controller below TickerMode. Its Ticking
button changes to Muted. The Linux/Android probe starts animation, disables ticks,
requires an intermediate rendered width to stay fixed longer than the duration,
re-enables ticks and checks elapsed-time catch-up, then reverses to its initial
width. Endpoint screenshots must settle at the expected physical size. Existing
foreground color, covered-child and baseline checks remain enabled. These tests
cover native visible mode behavior; they do not validate every host lifecycle or
force-frame/accessibility policy.

References: [TickerMode](https://api.flutter.dev/flutter/widgets/TickerMode-class.html),
[effective ticker values](https://api.flutter.dev/flutter/widgets/TickerModeData-class.html),
[TickerProviderStateMixin](https://api.flutter.dev/flutter/widgets/TickerProviderStateMixin-mixin.html).

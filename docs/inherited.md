# Inherited values and dependency notifications

`Widget::inherited` introduces a typed value for its descendants. A component's
`BuildContext::depend_on::<T>()` finds the nearest provider of exactly T and
registers a rebuild dependency. `read::<T>()` returns the same value without
subscribing. Nested providers of T shadow outer ones; sibling/descendant providers
are not visible. Different types coexist, and separate ElementTrees are isolated.

```rust
use rust_flutter::{egui, element::*};

#[derive(PartialEq)]
struct CounterStep(i32);
struct Counter;
struct CounterState { value: i32 }
impl StatefulWidget for Counter {
    type State = CounterState;
    fn create_state(&self) -> CounterState { CounterState { value: 0 } }
}
impl State<Counter> for CounterState {
    fn build(&mut self, _: &Counter, ctx: &StateContext<Self>) -> Widget {
        let step = ctx.depend_on::<CounterStep>()
            .expect("lookup while building").map_or(1, |step| step.0);
        let value = self.value;
        let handle = ctx.handle();
        Widget::leaf(egui::vec2(120.0, 44.0), move |ui| {
            if ui.put(ui.max_rect(), egui::Button::new(value.to_string())).clicked() {
                let _ = handle.update(move |state| state.value += step);
            }
        })
    }
}
// Mount this once through ElementTree::new:
let root = Widget::inherited(CounterStep(1), Widget::stateful(Counter));
```

Values are addressed by their Rust TypeId. Use distinct newtypes for different
roles that happen to contain the same underlying data. Lookup returns an owned
`Rc<T>` snapshot. Replacing the provider value does not alter an older snapshot.
Keep T immutable, or explicitly call notify if application-owned interior-mutable
data changes. Value and context access stays on the UI thread; owning a snapshot
can deliberately extend its resource lifetime beyond provider unmount.

The default constructor requires PartialEq and notifies when old != new.
`Widget::inherited_with(value, should_notify, child)` accepts a custom predicate:

```rust
let root = Widget::inherited_with(
    CounterStep(1),
    |old, new| old.0 != new.0,
    Widget::stateful(Counter),
);
```

The predicate receives old/new data of the same type and should compare those
arguments without side effects. A declarative provider update uses the new
configuration's predicate. The new value is stored even when notification is
suppressed: a later non-listening read or independent rebuild sees that value.
Updating the provider's child description through `tree.update` still performs
normal property reconciliation; that can rebuild components independently of
inherited notifications.

## Update handles and selective rebuilding

Use an InheritedHandle to change a value without supplying a new child description:

```rust
let provider_id = tree.root_id().expect("mounted inherited root");
let step = tree.inherited_handle::<CounterStep>(provider_id).expect("step provider");
step.set(CounterStep(2)).expect("mounted provider");
// The next show/layout/flush processes this command and rebuilds subscribers.
```

A mounted BuildContext can obtain its nearest provider's handle through
`inherited_handle::<T>()` without subscribing. Handles are typed, cloneable and
sendable. `update` queues a `Send + 'static` function from &T to a replacement T;
the function executes on the UI thread. T itself can own non-Send resources.
`set` requires Send because it moves an already constructed T through the queue.
`notify()` requests notification without replacing the value. Commands are
processed once per build pass, wake egui, and return Unmounted after the provider
closes. Pending commands can be discarded by unmount, as with StateHandle.

Only contexts registered with the notifying provider are marked dirty. A nested
ordinary provider shadows outer providers for nearest-value subscriptions.
[Inherited models](inherited-models.md) additionally support aspect filtering and
subscriptions through unsupported nested models.
Several notifications before a consumer builds coalesce into one dependency hook
and build, using the latest snapshots. Updating an inherited value preserves the
consumer's ElementId, local state and render identity. The adapter still performs
its ordinary box layout; this is selective component building, not a completed
dirty RenderObject pipeline.

## Dependency lifecycle

State now has `did_change_dependencies(widget, context)`. It runs after init_state
and before the initial build, then before builds triggered by inherited
notifications. Ordinary local state builds do not invoke it. Registering a
provider only in this hook remains sufficient for future notifications.

Dependencies persist for the mounted lifetime, even if a later branch stops
calling depend_on. They are released at unmount, or reset if an ancestor binding
changes. This follows Flutter's Element dependency lifetime rather than a
per-build reactive read-set model. Missing lookups are also recorded, so changed
ancestor bindings can invalidate them. GlobalKey activation/reparenting is still
unfinished; this binding refresh does not establish that feature. [Flutter
Element activation/dependency handling](https://github.com/flutter/flutter/blob/master/packages/flutter/lib/src/widgets/framework.dart).

Listening lookups are available while the context is mounted, including build,
dependency hooks, measurement, paint and idle UI-event callbacks. Calling depend_on
from init_state returns DependencyError::InvalidPhase; use read for a non-listening
lookup there. Both methods fail after unmount. Measurements/painting register the
component whose BuildContext is used, rather than introducing a new render-node
subscription identity. A returned Rc is a snapshot; a later read obtains the latest
value. Contexts are UI-local; typed update handles can be sent to workers.

Scopes retain only live provider links. Provider links use weak subscriber
references; unmount unregisters dependencies, clears retained scopes and closes
queues. A retained context cannot read its old provider after unmount. Tests
verify provider values are released even if such a context is kept externally.
Explicitly owned Rc snapshots remain valid until their owners drop them.

## Current coverage

The inherited tests cover nearest exact-type lookup, shadowing, isolated trees,
listening/non-listening behavior, predicate/equality suppression, coalesced changes,
initial hook ordering, persistent subscriptions, immutable snapshots, workers with
non-Send UI data, cleanup, stale contexts/handles, and preserved render identity.
The gallery provides a Step control above its retained counters. Its rendered
probe changes Step from 1 to 2 without changing counts, then verifies one click
matches the previous two-click counter glyphs. Platform results are recorded in
[verification](verification.md).

InheritedNotifier/ChangeNotifier adapters, the complete InheritedModel member surface,
full ancestor traversal, GlobalKey reparenting
and the rest of Flutter's lifecycle/member surface remain incomplete. This is an
original Rust API; it does not parse or emulate Dart Widget objects.

Primary references: [inherited lookup](https://api.flutter.dev/flutter/widgets/BuildContext/dependOnInheritedWidgetOfExactType.html),
[notification predicate](https://api.flutter.dev/flutter/widgets/InheritedWidget/updateShouldNotify.html),
[dependency hook](https://api.flutter.dev/flutter/widgets/State/didChangeDependencies.html).

For aspect-specific subscriptions and nested supported-aspect traversal, see
[inherited models](inherited-models.md).

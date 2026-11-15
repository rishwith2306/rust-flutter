# Observable state and notifier-driven widgets

`rust_flutter::notifier` supplies UI-local `Listenable`, `ValueListenable`,
`ChangeNotifier`, `ValueNotifier<T>` and `MergedListenable` APIs. They complement
thread-safe Signal and queued state/inherited handles; they do not replace them.
Notifiers and their synchronous callbacks use Rc and must stay on the UI thread.

```rust
use rust_flutter::{egui, element::Widget, notifier::ValueNotifier};

let count = ValueNotifier::new(0_i32);
let actions = count.clone();
let root = Widget::value_listenable_builder(count.clone(), move |_, value| {
    let actions = actions.clone();
    Widget::leaf(egui::vec2(200.0, 44.0), move |ui| {
        if ui.button(format!("Count: {value}")).clicked() {
            actions.update(|value| value + 1).expect("live source");
        }
    })
});
// Mount root once in an ElementTree; changes wake its egui context automatically.
```

`ValueNotifier::set` compares old and new values using PartialEq. An equal value
is ignored; an unequal value is stored before callbacks run. `update` computes
a replacement from a borrowed value. Do not recursively mutate the value inside
that read closure. Callbacks run after the value borrow is released and may read
or recursively replace it. `get`/`ValueListenable::value` clone a snapshot; use Rc
for expensive or shared values. `read` also supports values that are not Clone.
Interior mutations do not automatically emit events; use `notify_listeners` when
appropriate. Successful set/update returns whether equality changed.

## Listeners and ownership

A Listener is `Rc<dyn Fn()>`. `source.subscribe(listener)` returns a Subscription;
keep it alive for as long as the listener should remain registered. Dropping or
cancelling it unregisters once. The source does not stay alive merely because a
ChangeNotifier subscription guard exists. For explicit registration,
`ChangeNotifier::add_listener` returns a ListenerId removed by `remove_listener`.
Duplicate callback registrations have distinct IDs. Removal after disposal or a
second removal is harmless. IDs cannot accidentally remove another source's
registration.

Dispatch preserves registration order. A listener removed before its turn is
skipped; one added during a dispatch is deferred until a subsequent dispatch.
Recursive dispatch is permitted and has its own iteration boundary. Captured
values are released outside the listener-list borrow, so destructors can inspect
the source. A panicking callback is caught and later callbacks still run;
NotificationReport records invocation and panic counts for that dispatch. The
normal Rust panic hook still runs. Recursive reports are separate.

Use `downgrade`/weak handles when a callback needs its own notifier: capturing a
strong source handle in that source's listener can form an Rc cycle. Explicit
cancellation/disposal also breaks registrations. A source's owner controls its
lifetime; widgets detach subscriptions but do not dispose externally owned data.

`dispose` removes callbacks without notifying and rejects calls during active
dispatch. Later registration, notification and value mutation return errors.
Reading a retained ValueNotifier value is still possible after disposal. Sources
should be replaced or unmounted before their owner disposes them.

`MergedListenable` owns a fixed source list and forwards registrations to each
source, rolling them back if one fails. Repeating the same source registers
repeated callbacks deliberately. Cancelling the merged subscription removes all
of its registrations. Custom sources implement subscribe and same_source; custom
Subscription cleanup must unregister its callback. Shared Rc sources, including
`Rc<dyn ValueListenable<Value = T>>`, can be used with the widget adapters.

## Inherited notifier and builders

`Widget::inherited_notifier(Some(source), child)` exposes
`InheritedNotifier<SourceType>` to descendants. A component can subscribe with:

```rust
use rust_flutter::{element::{BuildContext, InheritedNotifier}, notifier::ValueNotifier};
fn counter(ctx: &BuildContext) -> Option<i32> {
    ctx.depend_on::<InheritedNotifier<ValueNotifier<i32>>>()
        .expect("mounted context outside init_state")
        .and_then(|model| model.notifier.as_ref().map(ValueNotifier::get))
}
```

`read` performs a non-listening lookup. None is a valid optional source. Replacing
a source notifies by default if its identity changes, including to/from None.
Clones of ChangeNotifier and ValueNotifier share source identity; Rc wrappers
compare allocation identity. `inherited_notifier_with` customizes the identity
notification predicate. Listener rebinding still follows source identity even
when that predicate suppresses a notification. Actual source events always
notify dependent components.

Events request a repaint immediately but dependency hooks/builds coalesce at the
next tree pass and see the latest source/value. Non-listening components are not
marked dirty. Ordinary property reconciliation can still rebuild descendants.
Nested same-type providers shadow outer events, as with ordinary inherited values.
Old source subscriptions are detached on replacement and all source subscriptions
are detached on unmount. Source updates can also be queued through an
InheritedHandle of the wrapper type; non-Send replacement sources must be
constructed on the UI thread by its update closure, not captured by a Send command.

Subscribing to a disposed source returns `ElementError::DisposedNotifier` during
tree creation/update/flush. A failed dynamic pass unmounts the tree and closes its
handles, matching existing component failure cleanup. The error is never silently
converted into an inert working source.

`Widget::listenable_builder(source, |ctx, source| ...)` subscribes one subtree.
`Widget::value_listenable_builder(source, |ctx, latest_value| ...)` additionally
supplies its current cloned value. Their mounted provider and stateless component
use the existing reconciliation and inherited dependency machinery. If their
optional provider is explicitly cleared through its inherited handle, they render
an empty subtree until a source is supplied again. Child
prebuilding/const widget-instance reuse, AnimatedWidget's complete API and the
persistent RenderObject pipeline remain work items.

## Reference and verification scope

This original Rust implementation reviewed the pinned Flutter 3.38.0
[ChangeNotifier/ValueNotifier source](https://github.com/flutter/flutter/blob/3.38.0/packages/flutter/lib/src/foundation/change_notifier.dart)
and the [InheritedNotifier lifecycle](https://api.flutter.dev/flutter/widgets/InheritedNotifier-class.html).
It implements synchronous listeners and coalesced inherited rebuilds, but does
not claim Dart API equivalence. Explicit registration IDs remove exact
registrations rather than emulating Flutter's callback-identity ambiguity for
duplicates. Checked release-mode disposal errors, panic reports and RAII guards
are Rust contracts; FlutterError diagnostics and allocation instrumentation are
not implemented. Worker-thread notifications require a UI dispatch bridge.

Tests in `tests/notifier.rs` and `tests/inherited_notifier.rs` cover dispatch
mutation/reentrancy, duplicates, panic isolation, destructor reentrancy, disposal,
weak/scoped ownership, equality, interior mutation, merged sources and rollback,
source swaps/removal, pending events, nested scopes, selective/coalesced hooks,
state preservation, mount failures, unmount and both builder adapters. The gallery
has an observable counter/reset/burst demo. Its shared desktop/Android image-mask
probe compares a burst of three changes with three individually clicked updates.
See [verification](verification.md) for actual platform results and limitations.

## Focused API audit

| Flutter foundation/widget API | Rust evidence | Remaining differences |
| --- | --- | --- |
| Listenable / ValueListenable | Object-safe subscription traits, shared/type-erased handles and cloned value snapshots | Rust scoped registration and value ownership differ; complete Flutter diagnostics remain |
| ChangeNotifier | Ordered mutation-safe recursive dispatch, checked disposal, panic isolation, weak handles and removal tokens | Exact registration IDs replace callback-identity ambiguity; FlutterError and allocation instrumentation remain |
| ValueNotifier | PartialEq suppression, synchronous readable/reentrant notifications and explicit interior-mutation invalidation | UI-local Rust value ownership and checked errors; no cross-thread dispatch bridge |
| Listenable.merge | Fixed source list, duplicates, fan-out cleanup and failed-registration rollback | Optional sources can be filtered before construction; the Rust collection is fixed |
| InheritedNotifier | Optional source, identity gate, rebinding, latest-source/coalesced notification and unmount cleanup | Full member/diagnostic surface and global-key activation remain |
| ListenableBuilder / ValueListenableBuilder | Retained subtree subscriptions and latest values; source replacement and cleanup tests | Prebuilt-child/const widget reuse and AnimatedWidget's complete API remain |

Every row is partial relative to full Flutter API parity. These are scoped
behavior claims verified by the referenced tests, not completed SDK-member audits.

# Inherited models and selective dependencies

`Widget::inherited_model(value, child)` provides a typed `InheritedModel`.
A consumer calls `ctx.depend_on_aspect::<Model>(aspect)` to observe part of that
model. State, identity and subscription lifetime follow the ordinary inherited
value API described in [inherited values](inherited.md).

```rust
use rust_flutter::element::{BuildContext, InheritedModel, Widget};
use std::collections::HashSet;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Aspect { Foreground, Background }
struct Colors { foreground: u32, background: u32 }
impl InheritedModel for Colors {
    type Aspect = Aspect;
    fn update_should_notify(&self, old: &Self) -> bool {
        self.foreground != old.foreground || self.background != old.background
    }
    fn update_should_notify_dependent(&self, old: &Self, aspects: &HashSet<Aspect>) -> bool {
        (self.foreground != old.foreground && aspects.contains(&Aspect::Foreground))
            || (self.background != old.background && aspects.contains(&Aspect::Background))
    }
}
fn foreground(ctx: &BuildContext) -> Option<u32> {
    ctx.depend_on_aspect::<Colors>(Aspect::Foreground)
        .expect("mounted context outside init_state").map(|colors| colors.foreground)
}
let root = Widget::inherited_model(
    Colors { foreground: 0xffffff, background: 0x000000 }, Widget::empty(),
);
```

## Notification rules

The new model's `update_should_notify(old)` gates notification. If false, no
consumer's aspect predicate is called, although the new snapshot is stored and
subsequent reads see it. If true, `update_should_notify_dependent(old, aspects)`
receives each dependent's accumulated set of aspects. A true result schedules
its dependency hook and build. Repeated subscriptions to the same aspect do not
create duplicate set entries. Predicates should compare arguments without side
effects; interior mutation requires an explicit notification or a new snapshot.

`ctx.depend_on::<Model>()` creates an unconditional dependency on the nearest
provider. Once registered, that dependency supersedes aspect filtering for the
same provider, including later aspect lookups. `ctx.read::<Model>()` remains a
non-listening nearest-provider lookup. Subscriptions from dependency hooks,
builds, measurement, painting and idle events share these rules. Listening in
init_state is rejected; calls through retained unmounted contexts fail.

As with ordinary inherited values, dependencies persist until unmount or an
ancestor binding change. Stopping an aspect lookup in a later build does not
unsubscribe it. The renderer still uses the existing box-layout adapter; selective
component rebuilding does not establish a persistent RenderObject pipeline.

## Nested models

`is_supported_aspect` defaults to true. A model may override it to support only
some aspects. An aspect lookup visits same-type ancestors from nearest outward,
registering against **every** visited provider, and stops after the first provider
supporting that aspect. It returns that provider's immutable `Rc<Model>` snapshot.
If none support the aspect, it returns the outermost visited model; if none exist,
it returns None. All visited models can subsequently cause a dependency rebuild.
Models of other Rust types, siblings and descendants do not participate.

This follows Flutter's [inheritFrom traversal](https://api.flutter.dev/flutter/widgets/InheritedModel/inheritFrom.html)
and [per-dependent notification rules](https://api.flutter.dev/flutter/widgets/InheritedModelElement/notifyDependent.html).
The Rust trait's associated Aspect type is statically checked, rather than
Flutter's runtime Object aspect argument. The complete Flutter member/diagnostic
surface and GlobalKey deactivation/reactivation are still outstanding.

## Updating a model

Use the existing `InheritedHandle<Model>` from the provider's ElementId. The
context's `inherited_handle::<Model>()` selects the nearest provider, independently
of which ancestor supports an aspect; use the desired provider ID when updating
an outer model. Handles can queue functions from a worker thread even if Model
owns UI-local resources. `set` requires Send; `update` constructs the replacement
on the UI thread and can return non-Send values.

Each queued eligible transition is checked. For example, changes to A and then B
before the next pass notify both sets of subscribers. Changes that later revert
can still notify a dependent. The consumer's hook/build coalesces and sees the
latest model. This is a conservative queue-based Rust contract, distinct from
Flutter's synchronous setState closures and widget-frame update timing.
`notify()` explicitly invalidates every registered dependent, bypassing both
model predicates, for application-owned interior changes.

## Executable evidence

`crates/rust-flutter/tests/inherited_model.rs` covers selective builds and hooks,
local-state preservation, aspect unions, unconditional precedence, notification
gates, nested unsupported models, missing/unsupported ancestors, worker-queued
transitions, hook-only subscriptions, unmount and ordinary-provider compatibility.
The existing inherited-value tests continue to cover rendering identity and
measurement/paint/event subscription phases.

The gallery's retained counter page uses A/B model aspects. The general Step
control changes both; the purple A step control changes only A's step. Build
counts make selective rebuilding visible. The shared rendered smoke probe checks
count preservation and continued input after the aspect update; headless tests
establish that B does not rebuild. These checks do not prove all Flutter features.

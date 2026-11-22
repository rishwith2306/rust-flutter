# Stateful widgets and retained elements

`rust_flutter::element` separates a widget description from its mounted identity
and state. `ElementTree` reconciles descriptions, builds dirty components and
adapts their render widgets to the constraint-based box layout. Host a tree
inside an `Application`, or pass it to `run_widget` to keep it mounted across
native frames without writing an application wrapper.

Typed inherited values and dependency hooks are documented in [inherited values](inherited.md); selective aspect subscriptions are in [inherited models](inherited-models.md).

A stateful component declares its configuration and state types:

```rust
use rust_flutter::{egui, element::*};

struct Counter { label: &'static str }
struct CounterState { value: u64 }
impl StatefulWidget for Counter {
    type State = CounterState;
    fn create_state(&self) -> CounterState { CounterState { value: 0 } }
}
impl State<Counter> for CounterState {
    fn build(&mut self, widget: &Counter, ctx: &StateContext<Self>) -> Widget {
        let label = widget.label;
        let value = self.value;
        let handle = ctx.handle();
        Widget::leaf(egui::vec2(120.0, 44.0), move |ui| {
            if ui.put(ui.max_rect(), egui::Button::new(format!("{label}: {value}"))).clicked() {
                let _ = handle.update(|state| state.value += 1);
            }
        })
    }
}
```

`State<W>` also offers `init_state`, `did_update_widget(old, new, context)` and
`dispose` hooks. `did_change_dependencies` runs after init and on inherited notifications. The framework creates state and calls init once for each mount.
A matching new widget updates properties and calls `did_update_widget`, then
builds. Removing or replacing that element disposes state once. Descendants are
disposed before their parent. Disposal closes update handles before invoking the
hook, so cleanup cannot schedule more work on the removed component. This is a
basic lifecycle; deactivate/reactivate and hot-reload
reassembly hooks are not implemented yet.

Mount the tree once, then show it from `Application::build`:

```rust
use rust_flutter::{Application, AppContext, egui, element::*, layout::*};

#[derive(Default)]
struct App { tree: Option<ElementTree> }
impl Application for App {
    fn initialize(&mut self, ctx: &AppContext) {
        let mut style = FlexStyle::new(Axis::Horizontal);
        style.spacing = 8.0;
        self.tree = Some(ElementTree::new(&ctx.egui, Widget::flex(style, vec![
            FlexItem::expanded(Widget::stateful(Counter { label: "A" }).keyed("A"), 1),
            FlexItem::expanded(Widget::stateful(Counter { label: "B" }).keyed("B"), 1),
        ])).expect("valid initial tree"));
    }
    fn build(&mut self, ui: &mut egui::Ui) {
        let constraints = BoxConstraints::tight(egui::vec2(ui.available_width(), 80.0))
            .expect("finite viewport");
        if let Some(tree) = &mut self.tree {
            if let Err(error) = tree.show(ui, constraints) {
                ui.colored_label(egui::Color32::LIGHT_RED, error.to_string());
            }
        }
    }
    fn shutdown(&mut self) { self.tree.take(); }
}
```

`StatelessWidget::build(&self, &BuildContext)` constructs a description without
framework-owned component state. `Widget::stateless` and `Widget::stateful` wrap
concrete Rust types; type identity is checked using `TypeId`, not a display name.
A widget value is owned by its element configuration. To mount equivalent
configurations in multiple locations, construct separate values (or clone your
configuration type); each mount receives independent state.

## Keys and updates

`tree.update(new_root)` reconciles a new root description. Matching requires the
same widget kind/component Rust type and the same optional key. Changing either
replaces that element and its subtree. Properties can change without resetting
state. A keyed child is found among the old siblings by its key, so it can move
or survive an insertion/removal elsewhere in the list. Unkeyed children match
only at the same sibling index; use domain keys for reorderable lists.

Keys support owned strings and `u64` values. They must be unique among siblings;
the same local key can appear under different parents. Local keys do not move
state between parents. `ElementId` is a process-local mounted identity, distinct
from the declarative key. Removing an element and later reinserting its key
creates a new ID and fresh state. GlobalKey-style reparenting is future work.

Do not call `tree.update` merely to paint an unchanged description each frame.
`show` flushes queued state updates, rebuilding only dirty components and any new
configurations they produce. Inherited notifications also mark only registered consumers dirty. An unchanged parent or sibling does not run its
component builder when only a child is dirty. Calling `update` explicitly runs
property-update/build hooks on matching components even when their property
values happen to compare equal; property equality is not inferred automatically.

## State updates and scheduling

`ctx.handle()` returns a typed, cloneable `StateHandle<S>`. `update` enqueues a
`Send + 'static` closure, marks the component dirty and requests an egui repaint.
The closure executes on the UI thread during the next build pass. State itself
does not need to implement Send: tests include state that owns `Rc<RefCell<_>>`
while a worker sends updates through its handle. You can also request a rebuild
without a state command. `mounted` reports whether the handle still targets a
mounted component; `update` returns `Unmounted` if it has closed.

Commands accepted before unmount can be discarded when their target is removed.
Dispose/unmount clears pending commands and releases the queue's repaint context.
Commands and their captured resources are dropped outside the queue lock, allowing
capture destructors to inspect handles without deadlocking. A producer arriving
while another command executes schedules a later pass; updates are not cleared
after a build in a way that loses that wakeup. Requests made during a build are
deferred to a subsequent flush pass, with at most one build per element in each
pass. `ElementTree::flush` is available for renderer-independent tests and explicit
build scheduling; `show`/`layout` perform a flush themselves.

State commands must be short. UI widget callbacks should enqueue updates rather
than trying to borrow/reenter the tree being painted. The queue is not a task
executor or a bounded stream. Cancellation and notifier/stream adapters remain separate features. Mutating shared
objects outside a handle does not automatically mark a component dirty.

## Render widgets and inspection

`Widget` constructors cover preferred-size leaves, measured leaves, padding,
extra constraints, alignment/size factors, flex and positioned stacks. FlexItem
and StackItem carry child sizing/position information. A measured leaf receives
actual parent constraints during layout and returns a size inside them. Use this
for content-dependent measurements; build/init can run before an egui frame, so
font measurement that needs an active frame belongs in the measurement callback.
Leaf painting receives its own stable egui child UI. Fit controls to `ui.max_rect()`
or paint directly. Layout/input limitations are documented in [layout](layout.md).

The adapter reuses element/leaf IDs while regenerating the box description after
a component rebuild. It caches that description between builds and computes
geometry for the current constraints. It does not yet retain dirty RenderObject
instances or skip unchanged layout subtrees. The underlying painter, fonts,
keyboard focus, input dispatch and GPU renderer still use egui; this does not
establish a complete Flutter rendering/gesture/semantics pipeline.

`elements()` returns snapshots of mounted IDs, sibling keys, concrete widget type
names and child IDs. `state::<S>(id)` reads a component's typed state without
mutating it, and `state_handle::<S>(id)` returns a matching typed update handle.
These support inspection/testing; they are not a persistent state format. Component
state lasts for the mounted element. Application/process/activity restoration
requires explicit serialization and recreation of state, as with other app data.

## Failure behavior and verification

Duplicate sibling keys or excessive depth in a supplied static description are
rejected before changing the existing tree. A dynamic component build can fail
validation after update hooks have run; that failure unmounts the entire ElementTree
rather than leaving partial mounted state. A valid `update` can mount a fresh root
afterward. Layout errors preserve mounted component state so the application can
retry with valid constraints. A tree belongs to one egui context and rejects
painting through another. User hook panics are not caught or rolled back.
Trees are limited to 256 element levels.

Tests cover keyed reorders, insertions/removals, type/key replacement, local key
scoping, slot-based unkeyed identity, state/init/update/disposal counts, worker
wakes, updates racing command execution, dirty-child isolation, deferred builds,
cleanup after validation/depth failures, stale handles, measured layout, context checks
and real pointer input into a component's egui leaf. The gallery adds two local
counters, swap/remove/reinsert controls and lifecycle totals. Its visual smoke
compares exact trimmed foreground glyph masks across those actions without reading component state
through an application backdoor. See [verification](verification.md) for executed
platform gates; full Flutter parity remains unfinished.

Primary references: [Flutter widget update identity](https://api.flutter.dev/flutter/widgets/Widget/canUpdate.html),
[State lifecycle](https://api.flutter.dev/flutter/widgets/State-class.html),
[child reconciliation](https://api.flutter.dev/flutter/widgets/Element/updateChildren.html).

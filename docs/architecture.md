# Architecture

Applications implement `Application` and own their domain state. `Runtime` calls
`initialize` once, dispatches events, calls `configure`, and builds the central
panel. Desktop uses eframe/winit; Android owns an `android-activity` event loop
and renders egui through wgpu directly. Android activity and surface lifetimes
are separate: surface loss releases the native window while retaining GPU/UI
state, whereas activity destruction releases the GPU, shuts down the application,
and returns from `android_main`. A recreated activity gets a new host and context.

```
Application model / routes / platform services
                    |
Rust Flutter: widgets · Navigator<R> · Signal<T> · Tween · Runtime
                    |
egui: layout · interaction · text · paint
                    |
       +------------+----------------+
       |                             |
Desktop: eframe/winit         Android: android-activity
       |                     owned event loop + JNI services
       +------------+----------------+
                    |
             egui-wgpu / wgpu
                    |
        Metal / Vulkan / GLES
```

`run_with_options` accepts desktop window options and Android GPU/activity options.
Android rejects winit builder hooks and non-default depth/stencil/MSAA surfaces
explicitly. Those options need a matching host implementation before support can
be claimed. The Android host does not create or cache any winit event loop.

Most widgets here are functions that run during UI construction. `BoxNode` adds
an owned constraint-based layout tree with explicit geometry, clipping and hit
testing, rendered through stable leaf UIs. `ElementTree` reconciles declarative
stateful/stateless widgets by type and sibling key, owns component state and
queues state updates for selective component builds. Typed inherited scopes
register consumers and notify their dependency hooks without resetting state. See `layout.md` and
`components.md` / `inherited.md`. Typed model aspects and supported-aspect
ancestor traversal are described in `inherited-models.md`. Observable sources and notifier-driven inherited/build adapters are described in
`notifiers.md`. Every retained tree also supplies a host-owned view-metrics model,
refreshed from the active viewport before each build/flush. Selective geometry
subscriptions and explicit subtree overrides are described in `display.md`.
Custom persistent box objects, ordered render-child slots and coarse layout/paint
invalidation are described in `render-objects.md`. Built-in box adapters still
regenerate descriptions. [Layout-time builders](layout-builders.md) batch pending
constraint-dependent builds between layout attempts, outside render borrows,
then return final geometry before painting. Their children use ordinary element
reconciliation, keys and inherited scopes. The full RenderObject pipeline and
GlobalKey reparenting remain unfinished. Rust owns data and lifetimes; closure composition avoids a custom
language and a separate runtime. Long-lived state belongs in the application or mounted component state,
not temporary widget variables. Stable IDs preserve egui's internal input state.

`Navigator<R>` keeps one or more typed routes and prevents popping the root.
Applications choose how to render a route. It currently does not manage browser
history, deep links, transition overlays or restoration.

`Signal<T>` shares data through `Arc<Mutex<T>>` and calls the egui repaint API on
mutation. It is coarse-grained: there is no dependency graph or automatic subtree
rebuild optimization. `spawn_task` publishes results and panics through a signal.
An application can integrate Tokio or another executor independently.

`rust-flutter-assets` compiles a manifest into embedded bytes without a UI build
dependency. `AssetBundle` supports substitution, while `AssetImages` selects
density variants and retains a bounded LRU of textures scoped to one egui context.
See `assets.md` for limits and packaging differences from Flutter.

`JsonStore` provides atomic versioned snapshots in a platform-private data directory.
`Localizations` wraps Fluent resources, CLDR locale negotiation and per-message
fallback. These are concrete application services; see `application-services.md`.

`AnimationController` drives bounded scalar values from its owning viewport's
frame passes, with cancellation futures, repeat/reverse, scoped ticker muting
and element-owned teardown. Checked friction and spring simulations can drive
the same controller. Typed `TweenAnimationBuilder` supports implicit value
changes while preserving its child. The full derived-animation, transition,
sequence and platform motion catalogs remain unfinished; see
[animation controllers](animation-controllers.md), [simulation playback](simulation-animations.md)
and [implicit animations](implicit-animation.md). [Derived animations](curved-animations.md)
share an Animation value/status contract with controllers. CurvedAnimation keeps
its selected direction through mid-flight reversals, owns parent subscriptions
and evaluates custom curves outside configuration borrows with supersession checks.
[Animatable mappings](value-tweens.md) compose typed tweens and weighted sections
without clocks; DrivenAnimation supplies the parent clock/status and owns its
forwarding links. Value/curve tween edits share configuration across handles
without manufacturing parent notifications.

Retained scrolling separates externally owned `ScrollPosition` state from
viewport layout and input. A viewport lays out an eager child, publishes measured
dimensions outside render-object borrows and subscribes to offset changes.
One shared `GestureBinding` routes hit paths, pointer arenas and wheel signals
across painted roots. Keyboard shortcuts use viewport focus; scrollbar thumb
drags carry command identities so synchronous application callbacks can supersede
them safely. See [viewports](scroll-viewports.md) and [scrollbars](scrollbars.md).
Fixed-extent [lazy lists](lazy-lists.md) build only a viewport/cache range,
retaining overlapping keyed items in the same element tree. [Keep-alive](keep-alive.md)
requests retain selected off-cache items under Offstage; per-item build contexts
keep their inherited and explicit rebuilds active. A list-owned registry counts
requests and follows optional key-index mappings without scanning the dataset. A content render
box reports the full extent and rebases realized row coordinates near the
viewport. Variable-extent delegates, slivers, coordinated nested scrolling and
full platform scroll physics remain unfinished.

[Offstage](offstage.md) keeps a child mounted and measured while reporting the
parent constraints' minimum size. Hidden layout flags suppress hit paths and
cancel pointer capture; inherited UI tags and empty clips suppress raster output
while allowing focused keyboard input. Backend callbacks and tickers continue.
Desktop accessibility containers carry hidden metadata. This does not provide
lazy-list keep-alive storage or the complete platform semantics/focus pipeline.

`Services` stores implementations by Rust type. Store `Arc<dyn Trait>` to abstract
an OS API. Missing services return `None`. This is dependency injection within the
Rust process, not a JNI/Swift message bridge. Native implementations must be added
and exercised on each OS before being advertised as supported plugins.

The initial backend is deliberately a separate layer: Flutter compatibility needs
additional retained layout, semantics, design systems and developer tools. Reusing
egui does not supply Flutter APIs or Material/Cupertino fidelity automatically.

The pinned wgpu-hal Android EGL patch requests RGBA8 native presentation even
without sRGB window-surface support. Generated app manifests select the reviewed
source explicitly; see `renderer-patches.md` for origin, verification and manual
consumer setup. This does not replace wgpu or establish full color management.

[Retained transitions](transitions.md) bind animation sources directly to render
objects. Size/alignment changes invalidate layout; valid decoration changes
request paint only. Cached checked samples keep user source callbacks outside
paint, preserve children and expose failures through typed layout errors.

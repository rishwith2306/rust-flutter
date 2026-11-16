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
The full RenderObject pipeline and
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

`Tween` samples a scalar with an explicit monotonic time and a duration. The UI
adapter uses egui's frame time and requests frames until completion. Springs,
sequences and implicit property animations remain future work.

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

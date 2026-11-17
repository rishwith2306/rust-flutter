# Changelog

## Unreleased — 0.1.0 development

- Add pure checked friction, bounded contact and spring simulations with explicit tolerances and all damping regimes; controller/scroll integration remains pending.

- Shared Rust desktop and Android application model with an egui/wgpu renderer.
- Navigation, signals, background tasks, themes, localization and JSON snapshots.
- Embedded assets, density variants and a bounded decoded-image cache.
- Constraint layout, retained keyed components and typed inherited values.
- Foreground render-object paint phases and retained solid/border DecoratedBox.
- Frame-driven animation controllers with forward/reverse/repeat, observable builders, cancellation futures and manual muting.
- Nested TickerMode and element-managed animation lifetime, including safe cancellation during listener-triggered unmount.
- Typed implicit TweenAnimationBuilder with continuous retargeting and post-pass completion callbacks.
- Reentrant gesture arena with eager/deferred resolution, hold/release/sweep, cancellation and pointer-generation safety.
- Explicit pointer router with per-pointer/global subscriptions, owned cleanup tokens, weak handles and reentrant dispatch.
- Primary tap/pan recognizers with arena-gated completion, configurable slop/start position, normalized samples, cancellation and reentrant tracking.
- Ordered layout hit paths, opaque/deferred/translucent behavior, shared root input binding and retained GestureDetector with safe callback updates/unmount cleanup.
- Shared-frame collection across roots, layer/clip/modal ordering, hidden-root cancellation and opt-in GestureButton with arena pointer actions, keyboard focus and desktop Click-action bridging.
- Bounded quadratic velocity estimation, configurable release/fling thresholds and rich DragEndDetails; PanCallbacks.on_end now takes details instead of PointerEvent.
- Retained paragraphs with styled spans, wrapping, line limits/ellipsis, intrinsic sizing and real alphabetic baselines.
- Inherited Text/DefaultTextStyle, nested span styles, explicit linear text scaling and line-limit overrides.
- Inherited models with aspect subscriptions, selective notifications and nested fallback.
- ChangeNotifier/ValueNotifier, merged sources, scoped listeners, inherited notifier and builder adapters.
- Native Android lifecycle handling and smoke-test tooling.
- Android launch/resume probes require a fresh native resume and focused GPU presentation; stale geometry logs cannot satisfy readiness.
- Android EGL requests RGBA8 native surfaces; generated apps pin the reviewed renderer patch.
- CLI starter apps with pinned GitHub dependencies and local contributor mode.

Experimental APIs; full Flutter parity and macOS runtime verification remain open.

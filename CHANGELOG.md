# Changelog

## Unreleased — 0.1.0 development

- Add an initial Material theme foundation with core semantic `ColorScheme`
  roles, inherited `ThemeData`, an egui global-style adapter, and retained
  filled/outlined/text buttons. Dynamic palette generation and most of the
  Material component catalog remain open.

- Add `FocusScopeNode` / `FocusScope` to group retained text/button targets,
  queue initial requests, explicitly traverse mounted targets, and cycle with
  Tab / Shift+Tab. Nested Flutter focus policies remain unfinished.

- Connect inherited `FocusNode` handles to retained `GestureButton` controls as
  well as text fields, preserving one-control focus ownership.

- Add an application-owned `FocusNode` that queues pre-mount focus requests,
  exposes focus changes as a Listenable, and attaches through `Focus` to a
  descendant `TextFormField`; full Flutter focus scopes and traversal remain open.

- Add `TextEditingController` with validated UTF-16 selection/composition offsets
  and a retained `TextFormField` adapter for forms, including reset, callbacks,
  validation feedback, read-only/password modes and lazy-list focus retention.
  The adapter uses egui and does not claim Material `EditableText` or platform
  IME/accessibility parity.

- Add form-wide automatic validation policies, related-field revalidation, reset batching, unfocus targets and bounded deferred passes with observable failures; require native error clearing before resubmission.

- Add retained typed forms with nearest-scope owned registration, validation/save/reset, field auto-validation, error/interaction state and callback supersession guards; add a gallery form and shared native input probe.
- Record the gallery aspect-ratio child’s allocated constraints before screen translation, avoiding f32 endpoint-cancellation noise in geometry traces while retaining strict pixel checks.

- Add eager retained Wrap rows/columns with run and child alignment, direction-aware placement, clipping, pure sizing/intrinsic/baseline queries and native responsive swatch checks; start compact native pan probes deeper inside their shared detector to avoid system back gestures.

- Add retained FractionallySizedBox with checked tight fractional child constraints, aligned overflow, intrinsic/baseline queries and child retention; compose it with LimitedBox in the native gallery probe.

- Add retained LimitedBox constraints for unbounded axes, preserving parent minimums, keyed children, pure queries and reported baselines.

- Keep gallery animation controls fully visible on compact screens by stacking the baseline demonstration; require full ticker-button bounds and exercise compact/wide native resizing.

- Add retained AspectRatio layout with checked constraint fitting, pure queries, baseline forwarding and native ratio-child geometry checks.

- Add generic AlwaysStoppedAnimation values and checked ReverseAnimation views that reverse value/status while retaining owned listener cleanup and typed tween composition.

- Retry failed transition subscriptions when the same source is rebound, restoring future animation notifications while retaining child state.

- Add retained SizeTransition, AlignTransition and DecoratedBoxTransition with owned source links, typed sample errors, pure size queries and child-preserving layout/paint updates; add Alignment interpolation and native clipped-marker checks.

- Add typed driven animations, shared mutable value/curve tweens, callback/constant mappings, function composition and weighted sequences; preserve custom normalized interpolation contracts and validate explicit extrapolation; drive the gallery marker with a Vec2 tween.

- Add a generic Animation value/status contract, checked cubic/flipped curves and derived CurvedAnimation with latched forward/reverse easing, owned subscription cleanup and reentrant sample guards; exercise derived marker geometry in native probes.

- Retain requested lazy item subtrees with counted owned/declarative keep-alive requests, key-index remapping, per-item build contexts and offstage keyboard activity; add a native row-owned counter retention probe.

- Add retained Offstage subtrees with measured hidden children, preserved state/focused keyboard input, canceled pointer capture, clipped backend output and desktop accessibility hiding; keep animations active unless explicitly muted.

- Add finite fixed-extent lazy ListView builders with bounded cached ranges, stable item keys, rebased row placement and existing viewport input; add a 10,002-row native gallery probe.
- Make the gallery renderer pixel profile explicit with dithering disabled on both hosts; retain strict color/glyph gates and discard stale smoke result JSON before a run.
- Add layout-time widget builders with keyed/inherited child retention, batched layout transactions, pure-query errors and bounded convergence; exercise scroll content construction through native probes.
- Add retained proportional scrollbars with axis/reverse geometry, captured thumb drags, track paging and wheel fallthrough; protect drag ownership from callback-created commands.
- Add focused retained viewport keyboard scrolling, Tab/blank-tap focus, child focus preservation and an opt-out; native probes now require key-driven movement and reset pixels.
- Add separate-process Android OS mouse-wheel injection and strict result validation to the ordinary APK runtime probe; rendered wheel/reset evidence is required before reporting success.

- Add axis drag recognition and named horizontal/vertical recognizers with primary update/release metadata, axis-projected fling classification and orthogonal viewport competition; keep unrestricted pan as the default.

- Route immediate wheel/point-scroll signals through shared hit paths and the reentrant binding queue, with first-willing resolution, viewport unit conversion and basic edge handoff; add real Linux wheel/reset probes.

- Add retained eager SingleChildScrollView layout/clipping, horizontal/reverse offsets, captured pan input and deferred metrics; preserve children while sequence keys cancel changed gesture owners; add native scroll/reset probes.

- Add observable scroll positions, validated dimensions, drag/programmatic/fluid-release motion and owned multi-client controller attachments; full trackpad and platform scroll integration remain pending.

- Drive checked physics simulations through bounded animation controllers with forward/backward status, muting/cancellation, reentrant sampling and explicit Failed outcomes; add a native spring marker and probes.

- Add pure checked friction, bounded contact and spring simulations with explicit tolerances and all damping regimes; scroll integration remains pending.

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

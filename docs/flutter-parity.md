# Flutter feature audit and implementation backlog

This is an original engineering inventory based on the official documentation,
not a copy of the Flutter documentation or a claim to exhaustively enumerate its
APIs. The [machine-generated API inventory](flutter-api-inventory.md) records
3,649 top-level declarations across 20 SDK/companion libraries. No declaration
has a completed member-by-member semantic audit yet; an [initial component review](flutter-components-audit.md)
records partial implementations and explicit differences. The objective remains a Rust framework with Flutter's capabilities, with
macOS and Android as the first deployment targets. Nothing in the current tests
proves full parity. Individual APIs within each category require further inventory.

Status terms: **implemented** means there is local framework code; **backend**
means directly available through the re-exported egui API but not necessarily
validated on both platforms; **partial** means a subset; **missing** means work
remains. Verification is recorded separately in `verification.md`.

| Capability | Current implementation | Status / required work |
| --- | --- | --- |
| Rust app entrypoint and shared source | `Application`, desktop/Android hosts, retained-tree `run_widget` entry points, and gallery | Implemented as two supported app models; platform runtime gates remain |
| Declarative composition and persistent state | Closure composition, `ElementTree` components, typed state handles, application struct and `Signal<T>` | Partial; typed inherited dependencies, model aspects and observable notifier adapters work; full Flutter lifecycle remains |
| Widget/Element/RenderObject lifecycle | Type/key-based element reconciliation, stateful/stateless components, init/property-update/dispose hooks, queued updates, selective dirty component builds and stable render IDs | Partial; inherited lookup/predicates and dependency hooks now work; custom persistent render boxes and coarse layout invalidation now work; GlobalKey reparenting, deactivate/reactivate, reassembly and the complete dirty RenderObject pipeline remain |
| Layout basics | Owned BoxNode geometry: validated constraints, fixed/loose/tight flex, alignment, padding, sized constraints, stack/positioned, direction, overflow, clip bounds and leaf UI adapter | Partial; dry/intrinsic box protocols and sizing wrappers now work; reported-baseline layout, layout-time child construction and retained Offstage hiding and aspect-ratio fitting and conditional unbounded-axis limits and fractional child sizing and wrapping rows/columns now work; full render-object pipeline and further widget inventory remain |
| Responsive/adaptive UI | Logical-point breakpoints, safe-area conversion, sampled view/keyboard metrics, selective ambient view dependencies and retained SafeArea | Partial; notch/system-bar insets verified on one emulator; wider device validation and adaptive controls remain |
| Text and typography | Retained Paragraph, inherited Text/DefaultTextStyle, nested spans, explicit linear text scale, wrapping, line limits/ellipsis, dry/intrinsic sizing and alphabetic baseline, backed by egui fonts | Partial; complex shaping, bidi, RTL, font fallback, selection, semantics and complete Text/RichText compatibility remain |
| Forms and input | Retained typed form scopes with validation/save/reset and field/form-wide auto-validation; egui `TextFormField` adapter and a controller with checked UTF-16 selection/composing ranges | Partial; Material/EditableText parity, async validation, autofill, restoration, semantics, advanced text shaping and native IME/device audit remain |
| Focus, shortcuts and keyboard | egui focus/input, retained viewport shortcuts, `FocusNode` adapters for `TextFormField`/`GestureButton`, and `FocusScopeNode` with explicit and Tab / Shift+Tab cyclic traversal | Partial; nested focus ancestry, remembered children, traversal policies/groups, arrow traversal, general control attachment, Shortcuts/Actions, key bubbling and platform keyboard audit remain; see [focus](focus.md) and [keyboard scope](keyboard-scrolling.md) |
| Pointer and gestures | Backend click/drag/touch plus public reentrant gesture arena, generation-safe entries, explicit per-pointer/global routing with owned subscriptions, primary tap/pan and horizontal/vertical drag recognizers, ordered hit paths and retained GestureDetector, multi-root viewport frames, opt-in arena-backed GestureButton and timestamped quadratic velocity/fling release details | Partial; global multi-viewport binding and broader backend-control arbitration, full recognizer catalog/teams, multitouch routing, precise raw timing/platform velocity strategies, scroll simulation integration and platform physics remain |
| Physics | Pure checked friction, bounded contact and spring simulation queries with all damping regimes and explicit tolerances | Partial; scroll driving, constant deceleration, other simulation families and full member compatibility remain |
| Scrolling | egui vertical helper plus observable scroll positions, owned controller attachments, bounded motion with configurable exponential fling drag, eager retained SingleChildScrollView clipping with horizontal/reverse geometry and primary pan input, focused keyboard shortcuts, retained interactive scrollbars, fixed-extent lazy ListView builders with requested item retention, wheel/point scroll routing and basic signal edge handoff (Linux native and Android OS-injected wheel probes pass) | Partial; full trackpad/device input, variable-extent lists, slivers, nested scrolling, platform-specific ballistic/overscroll physics |
| Navigation | Generic route stack, listenable `NavigatorController` with retained route view, checked Material/Cupertino named routes, optional Serde JSON route-stack snapshots, and priority-based `BackDispatcher` shared by navigation and retained modal hosts | Partial; OS-managed restoration, deep links, transitions and declarative Router integration remain |
| Theme | Light/dark and accent | Partial; explicit scoped linear text scale works; system/nonlinear scale, semantic theme tokens and adaptive theme remain |
| Material controls | Full current `ColorScheme` role names including fixed-tone and surface-container roles plus deprecated aliases, inherited `ThemeData`, theme-providing `MaterialApp` root with `NavigatorController` integration and checked named route table, egui global-style adapter, retained filled/outlined/text buttons, four-style controlled `IconButton` with selected icon support, elevated/filled/outlined `Card` surfaces with tap callbacks, themed retained `TextField`, controlled binary/tristate `Checkbox`, controlled `Switch`, generic controlled `Radio<T>`, continuous/discrete controlled `Slider`, themed determinate/indeterminate linear and circular progress indicators, horizontal/vertical `Divider`, fixed-height Material `ListTile`, themed regular/mini/large/extended `FloatingActionButton` positioned by `Scaffold`, persistent `Scaffold.bottom_sheet`, `AlertDialog` surface and retained `MaterialDialogHost` with scrim dismissal, typed dialog results and configurable barrier dismissal, retained `ModalBottomSheetHost` with scrim dismissal, typed results, configurable barrier, bounded height and handle-based drag dismissal, `AppBar`, expanding-body `Scaffold` with retained controlled `Drawer`, scrim and configured automatic menu action, controlled text-glyph `NavigationBar`, and Material 3 `NavigationDrawer` destinations with optional header/footer and selected state | Partial foundation; ColorScheme light/dark palettes are static and do not implement dynamic seed generation, high-contrast variants or system theme mode; `MaterialApp` still lacks localization; typography/component themes and most of the widget catalog remain; route transitions, deep links, route restoration and declarative Router integration remain; modal sheet drag-to-dismiss is limited to its handle and does not animate; entrance/exit animation and full size/shape configuration remain; persistent sheet dismissal and animation behavior remain; Card elevation/shadow, clipping, ink feedback, semantics, and full theme support remain; Divider radius, theme-based color resolution and complete layout behavior remain; IconButton tooltip, ink feedback, focus/hover behavior and accessibility semantics remain; safe areas, keyboard insets and icon assets require explicit implementation; dialog transitions, alert icon and scrollable content remain; drawer animation, edge-swipe, end drawer, native-back wiring and scrollable/lazy destination lists remain; ListTile pressed/focus/hover/accessibility semantics remain; FAB elevation, hero transitions, tooltip/accessibility semantics and interaction states remain; full control semantics, radio toggleable groups, slider indicators/keyboard input, and switch color/shape state animation remain; see [Material theme](material.md) |
| Cupertino controls | `CupertinoThemeData`, `CupertinoApp` with generic retained navigation and checked named route tables, retained `CupertinoPageScaffold` with safe-area insets and centered-title `CupertinoNavigationBar`, controlled `CupertinoTabBar`/retained `CupertinoTabScaffold` with independent optional route stacks, edge-to-edge/notched `CupertinoListTile` with press feedback, edge-to-edge/inset-grouped `CupertinoListSection`, regular/filled `CupertinoButton`, controlled `CupertinoSwitch`, range `CupertinoSlider`, `CupertinoActivityIndicator`, tap and sliding controlled segmented controls, a basic `CupertinoTextField` adapter, rounded `CupertinoAlertDialog`/`CupertinoActionSheet` panels, styled dialog/sheet actions, and retained modal popup host with tap-to-dismiss scrim | Partial foundation; optional BackDispatcher integration is available; transition animation, route results and per-tab state restoration, persistent tile activation while async tap work runs, pickers, adaptive behavior, accessibility, and the rest of the Cupertino catalog remain; Cupertino switch animation, slider indicators/semantics, text-field decoration parity, navigation-bar transitions/blur, and system appearance following are explicit app work; see [Cupertino](cupertino.md) |
| Animation | Scalar tweens and frame-driven bounded controllers, repeat/reverse, futures/cancellation, manual/scoped muting, element-owned controller teardown, observable builders, typed implicit TweenAnimationBuilder, checked custom simulation playback and generic value/status animations with derived cubic/reverse easing, typed driven tweens, weighted sequences, constant/reverse views and retained size/alignment/decoration/slide transitions | Partial; full ticker provider/force-frame/resync semantics, unbounded/velocity controllers, further derived animation families and full implicit/transition widget families, Hero and full animation catalog remain |
| Painting and effects | egui painter API, render-box background/foreground phases and retained DecoratedBox | Partial; canvas/path compatibility, clipping and compositing audit; Android GLES now requests RGBA8 through a pinned renderer patch; full color-space/HDR/gradient/alpha validation remains |
| Images, icons and assets | Build-time embedded bundles, platform filtering, density variants; PNG/JPEG/static WebP decoding and bounded texture cache | Partial; SVG, animated images, streams, automatic package assets, native asset lookup and font tooling remain |
| Accessibility | Desktop AccessKit feature with GestureButton Click actions plus role/state for Material Switch, Checkbox, and Radio; Android NativeActivity lacks semantics integration | Partial; semantics facade and TalkBack/VoiceOver device validation |
| Localization | Fluent FTL, CLDR locale negotiation, plurals/selects, message/attribute fallback; English/Spanish gallery | Partial; device locale discovery, framework translations, RTL layout/shaping, date/currency tooling |
| Asynchronous work | Background task signals with result/panic states; UI-local ChangeNotifier/ValueNotifier and merged Listenable sources | Partial; streams, cancellation, executor integration |
| Platform plugins | Typed service registry | Partial; no working native plugin catalog or JNI/Swift bridge |
| OS services | Startup/shutdown, Android lifecycle and surface events, best-effort NativeActivity saved-instance byte hooks, Android text-copy clipboard output and OpenUrl intents, focus/back/close hooks, private app data path | Partial; permissions, clipboard image output and typed clipboard service remain; durable state requires application-managed storage |
| Camera, location, sensors, notifications | No implementation | Missing platform plugins and permission flows |
| Storage, networking, serialization | Versioned atomic JSON snapshots, schema/size validation, gallery restoration | Partial; network client, database and secure-storage plugins |
| Webviews, native views, add-to-app | No implementation | Missing platform embedding/compositing |
| macOS deployment | eframe window host | Partial; interactive verification, bundles, signing, notarization |
| Android deployment | NativeActivity cdylib and package metadata | Partial; emulator touch/text/back/resume/restart and three same-process activity recreations pass; OS/device matrix and release packaging remain |
| iOS, Windows, Linux, web | Linux host works toward testing; backend has other targets | Outside first targets; Flutter-wide deployment parity still pending |
| Hot reload | Ordinary Cargo rebuild | Missing state-preserving Rust code reload; rebuild is not hot reload |
| CLI/project generation | `create`, `doctor`, `run`, `build`; shared-source retained-widget template; unsigned macOS `.app` bundle generation | Partial; device selection, richer diagnostics, hot reload, bundle icons/signing/notarization |
| Inspector and profiling | Standard Rust tooling and backend inspection APIs | Missing widget inspector, frame timeline, memory tooling integration |
| Testing | Core integration and headless pointer tests | Partial; golden tests, device automation, full widget harness |
| Documentation and examples | Getting started, architecture, gallery, platform guide | Partial; complete API cookbook and migration guide |
| Package ecosystem | Cargo package | Partial; versioning, publication, plugin discovery, compatibility policy |
| Performance | Backend GPU rendering | Missing benchmark budgets, release size/startup/frame measurements |

## Implementation sequence

1. Establish real desktop and Android builds and device smoke tests; preserve
   verifiable evidence for every advertised target.
2. Add a CLI and generated projects that share application source and make native
   build failures actionable.
3. Extend application lifecycle, OS back handling, asset loading, localization,
   restoration and state persistence with tests.
4. Complete layout/gesture/text/semantics foundations with platform validation.
5. Implement audited Material and Cupertino component inventories and visual tests.
6. Add native plugins, platform views, packaging/release automation and examples.
7. Build hot reload, inspector/profiler and performance regression infrastructure.
8. Review every Flutter SDK library and widget against the implementation and keep
   remaining APIs explicitly tracked before announcing parity.

## Official sources reviewed

- [Architecture](https://docs.flutter.dev/resources/architectural-overview): layered
  engine/framework/embedding structure, reactive composition, rendering and interop.
- [Widget catalog](https://docs.flutter.dev/ui/widgets): layout, controls, animation,
  painting, accessibility, assets, asynchronous UI and design-system categories.
- [Deployment platforms](https://docs.flutter.dev/reference/supported-platforms):
  OS and CPU targets; this framework does not inherit Flutter's support guarantees.
- [Flutter documentation](https://docs.flutter.dev/): further API-by-API audit
  remains required; no claim is made that all linked pages were read.

The dependencies' official references are linked from the platform guide. Version
0.33.3 is pinned so the implementation is reproducible while API changes in newer
egui/eframe versions are reviewed separately.

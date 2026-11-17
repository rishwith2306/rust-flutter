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
| Rust app entrypoint and shared source | `Application`, desktop and Android hosts, gallery | Implemented; platform runtime gates remain |
| Declarative composition and persistent state | Closure composition, `ElementTree` components, typed state handles, application struct and `Signal<T>` | Partial; typed inherited dependencies, model aspects and observable notifier adapters work; full Flutter lifecycle remains |
| Widget/Element/RenderObject lifecycle | Type/key-based element reconciliation, stateful/stateless components, init/property-update/dispose hooks, queued updates, selective dirty component builds and stable render IDs | Partial; inherited lookup/predicates and dependency hooks now work; custom persistent render boxes and coarse layout invalidation now work; GlobalKey reparenting, deactivate/reactivate, reassembly and the complete dirty RenderObject pipeline remain |
| Layout basics | Owned BoxNode geometry: validated constraints, fixed/loose/tight flex, alignment, padding, sized constraints, stack/positioned, direction, overflow, clip bounds and leaf UI adapter | Partial; dry/intrinsic box protocols and sizing wrappers now work; reported-baseline layout and layout-time child construction now work; full render-object pipeline and further widget inventory remain |
| Responsive/adaptive UI | Logical-point breakpoints, safe-area conversion, sampled view/keyboard metrics, selective ambient view dependencies and retained SafeArea | Partial; notch/system-bar insets verified on one emulator; wider device validation and adaptive controls remain |
| Text and typography | Retained Paragraph, inherited Text/DefaultTextStyle, nested spans, explicit linear text scale, wrapping, line limits/ellipsis, dry/intrinsic sizing and alphabetic baseline, backed by egui fonts | Partial; complex shaping, bidi, RTL, font fallback, selection, semantics and complete Text/RichText compatibility remain |
| Forms and input | Text fields, checkbox/slider via egui | Partial; form validation model, autofill, mobile IME audit |
| Focus, shortcuts and keyboard | egui focus/input plus retained viewport Tab/blank-tap focus and ordered line/page/boundary shortcuts, preserving focused child controls | Partial; public Focus/Shortcuts/Actions APIs, nested keyboard bubbling and cross-platform traversal audit remain; see [keyboard scope](keyboard-scrolling.md) |
| Pointer and gestures | Backend click/drag/touch plus public reentrant gesture arena, generation-safe entries, explicit per-pointer/global routing with owned subscriptions, primary tap/pan and horizontal/vertical drag recognizers, ordered hit paths and retained GestureDetector, multi-root viewport frames, opt-in arena-backed GestureButton and timestamped quadratic velocity/fling release details | Partial; global multi-viewport binding and broader backend-control arbitration, full recognizer catalog/teams, multitouch routing, precise raw timing/platform velocity strategies, scroll simulation integration and platform physics remain |
| Physics | Pure checked friction, bounded contact and spring simulation queries with all damping regimes and explicit tolerances | Partial; scroll driving, constant deceleration, other simulation families and full member compatibility remain |
| Scrolling | egui vertical helper plus observable scroll positions, owned controller attachments, bounded motion and eager retained SingleChildScrollView clipping with horizontal/reverse geometry and primary pan input, focused keyboard shortcuts, retained interactive scrollbars, wheel/point scroll routing and basic signal edge handoff (Linux native and Android OS-injected wheel probes pass) | Partial; full trackpad/device input, virtual lists, slivers, nested scrolling, overscroll physics |
| Navigation | Generic route stack and back-request hook | Partial; native back validation, deep links, route restoration, transitions, declarative routing |
| Theme | Light/dark and accent | Partial; explicit scoped linear text scale works; system/nonlinear scale, semantic theme tokens and adaptive theme remain |
| Material controls | No local Material implementation | Missing Material design system and component catalog |
| Cupertino controls | No local Cupertino implementation | Missing Cupertino design system and component catalog |
| Animation | Scalar tweens and frame-driven bounded controllers, repeat/reverse, futures/cancellation, manual/scoped muting, element-owned controller teardown, observable builders, typed implicit TweenAnimationBuilder and checked custom simulation playback | Partial; full ticker provider/force-frame/resync semantics, unbounded/velocity controllers, derived animation objects and full implicit/transition widget families, Hero and full animation catalog remain |
| Painting and effects | egui painter API, render-box background/foreground phases and retained DecoratedBox | Partial; canvas/path compatibility, clipping and compositing audit; Android GLES now requests RGBA8 through a pinned renderer patch; full color-space/HDR/gradient/alpha validation remains |
| Images, icons and assets | Build-time embedded bundles, platform filtering, density variants; PNG/JPEG/static WebP decoding and bounded texture cache | Partial; SVG, animated images, streams, automatic package assets, native asset lookup and font tooling remain |
| Accessibility | Desktop AccessKit feature plus a headless-tested GestureButton Click bridge; Android NativeActivity lacks integration | Partial; semantics facade and TalkBack/VoiceOver device validation |
| Localization | Fluent FTL, CLDR locale negotiation, plurals/selects, message/attribute fallback; English/Spanish gallery | Partial; device locale discovery, framework translations, RTL layout/shaping, date/currency tooling |
| Asynchronous work | Background task signals with result/panic states; UI-local ChangeNotifier/ValueNotifier and merged Listenable sources | Partial; streams, cancellation, executor integration |
| Platform plugins | Typed service registry | Partial; no working native plugin catalog or JNI/Swift bridge |
| OS services | Startup/shutdown, Android lifecycle and surface events, focus/back/close hooks, private app data path | Partial; native saved-instance bytes, permissions and clipboard implementation |
| Camera, location, sensors, notifications | No implementation | Missing platform plugins and permission flows |
| Storage, networking, serialization | Versioned atomic JSON snapshots, schema/size validation, gallery restoration | Partial; network client, database and secure-storage plugins |
| Webviews, native views, add-to-app | No implementation | Missing platform embedding/compositing |
| macOS deployment | eframe window host | Partial; interactive verification, bundles, signing, notarization |
| Android deployment | NativeActivity cdylib and package metadata | Partial; emulator touch/text/back/resume/restart and three same-process activity recreations pass; OS/device matrix and release packaging remain |
| iOS, Windows, Linux, web | Linux host works toward testing; backend has other targets | Outside first targets; Flutter-wide deployment parity still pending |
| Hot reload | Ordinary Cargo rebuild | Missing state-preserving Rust code reload; rebuild is not hot reload |
| CLI/project generation | `create`, `doctor`, `run`, `build`; shared-source template | Partial; device selection, richer diagnostics, hot reload |
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

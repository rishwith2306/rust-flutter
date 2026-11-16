# Initial Flutter component API review

This is a scoped behavior review of component foundations, based on the official
API references linked below and the current Rust implementation. It does not
complete the semantic audit of all 3,649 inventoried declarations or their members.
The upstream documentation/index is unversioned; release compatibility still
requires a pinned Flutter SDK and a member-by-member review. No row claims full
API parity, even where the principal lifecycle behavior now works.

| Flutter API / behavior | Rust evidence | Finding and remaining work |
| --- | --- | --- |
| [Widget.canUpdate](https://api.flutter.dev/flutter/widgets/Widget/canUpdate.html) | `Element::matches` checks concrete component TypeId / render kind and optional key; key/type replacement tests | Partial. Basic update identity is implemented. Owned Rust descriptions do not implement Flutter const/widget-instance reuse or its complete diagnostic/API surface. |
| [Element.updateChildren](https://api.flutter.dev/flutter/widgets/Element/updateChildren.html) | Keyed old-child lookup, ordered child list reconciliation; tests cover reorder, insert, remove, local scopes and unkeyed slots | Partial. Keyed state follows siblings; unkeyed state follows exact indices. Flutter's prefix/suffix matching optimizations, inactive lists and GlobalKey adoption are not implemented. |
| [StatefulWidget / State](https://api.flutter.dev/flutter/widgets/State-class.html) | Typed create_state, init_state, did_change_dependencies, did_update_widget, build, disposal; queue and lifecycle tests | Partial. init/update/dependency/disposal hooks and dirty builds work. Deactivate/reactivate, reassemble, automatic restoration and other State APIs remain. Queue-based Rust updates execute at a later pass instead of synchronously executing Flutter setState's closure. |
| [StatelessWidget](https://api.flutter.dev/flutter/widgets/StatelessWidget-class.html) | Typed StatelessWidget::build and BuildContext; test verifies a dirty child does not rebuild its stateless parent | Partial. Property updates, explicit invalidation and inherited notifications build correctly; complete Element APIs remain. |
| [BuildContext](https://api.flutter.dev/flutter/widgets/BuildContext-class.html) | Mounted ElementId, egui context, scoped read/depend_on, inherited handles and explicit rebuild; typed StateContext adds its state handle | Partial. Exact-type provider lookup and subscriptions work. General ancestor/descendant widget traversal and owner/render-object access and broader Flutter context APIs remain missing. |
| [GlobalKey](https://api.flutter.dev/flutter/widgets/GlobalKey-class.html) | Element IDs and sibling-local string/u64 keys; no global adoption registry or inactive tree pool | Missing. Same-frame cross-parent reparenting, state/context access through a global key, deactivation/reactivation and dependent rebuilds are not provided. An ElementId is not a GlobalKey equivalent. |
| [InheritedWidget](https://api.flutter.dev/flutter/widgets/InheritedWidget-class.html) | Scoped typed providers, exact-type lookup, predicate notifications, typed updates and dependency hooks; inherited tests and the Step gallery probe | Partial. Core lookup/subscription/notification behavior works, including mounted-lifetime registrations and listening lookups from build, measurement, paint and idle events. Activation/reparenting and the full member surface remain. This is distinct from the Services registry. |
| [InheritedModel](https://api.flutter.dev/flutter/widgets/InheritedModel-class.html) | Typed aspect sets, global and per-dependent gates, unconditional precedence, unsupported-aspect ancestor traversal; model tests and gallery | Partial. Principal selective-notification and traversal behavior works. Queue timing is explicitly documented; full member diagnostics and GlobalKey activation remain. |
| [InheritedNotifier](https://api.flutter.dev/flutter/widgets/InheritedNotifier-class.html) | UI-local Listenable adapters, source identity and optional sources, rebinding, event coalescing, scoped unmount cleanup, observable builders; notifier tests | Partial. Core notification/lifetime behavior works. Rust registration IDs and disposal errors are explicit differences; complete Flutter diagnostics, animation adapters and worker/UI dispatch remain. |
| [MediaQuery](https://api.flutter.dev/flutter/widgets/MediaQuery-class.html) | Automatic host-owned `ViewMetrics` inherited model, selective `ViewAspect` dependencies, explicit subtree overrides, keyboard samples and native resize probe | Partial. Geometry subscriptions and aspect unions work; this is not the complete MediaQueryData/API surface. Text scaling, accessibility preferences, display features, additional inset-removal widgets, animation listeners and wider native validation remain. |
| [SafeArea](https://api.flutter.dev/flutter/widgets/SafeArea/build.html) | Retained padding wrapper, selected edges, per-edge minimum, stable-bottom option, consumed descendant metrics and state-preserving relayout; geometry/pointer tests | Partial. Reviewed box/consumption behavior works. SliverSafeArea, automatic consumption across egui panels and full device/configuration validation remain. |
| [RenderObjectWidget](https://api.flutter.dev/flutter/widgets/RenderObjectWidget-class.html) | Render widget descriptions adapt to BoxNode, with stable render IDs and measured leaves | Partial foundation. The adapter regenerates box descriptions; persistent RenderObject instances, create/update/unmount hooks, slotted child management, pipeline ownership and dirty render phases remain. |

Current executable evidence is in `crates/rust-flutter/tests/elements.rs` and the
shared rendered-state probe `scripts/component_probe.py`. The latter compares
counter text after actual clicks, sibling swapping, removal and reinsertion;
it does not inspect or mutate component state through a production test hook.
Platform results and exact test scope are recorded in [verification](verification.md).
Implementation usage and failure behavior are in [components](components.md) and
[inherited values](inherited.md). The inherited source review also checked
[Element dependency lifetime](https://github.com/flutter/flutter/blob/master/packages/flutter/lib/src/widgets/framework.dart):
subscriptions persist through ordinary rebuilds and are reset on activation.
GlobalKey activation remains pending in this framework.

These findings advance the original Flutter-capability objective, while retaining
all outstanding lifecycle, rendering, dependency, widget-catalog and tooling work.
Future reviews should expand this ledger by declaration/member and pair every
claimed behavior with source and platform evidence.

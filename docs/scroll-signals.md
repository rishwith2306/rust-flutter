# Wheel and trackpad scroll signals

`PointerScrollEvent` is an immediate scroll signal, separate from down/move/up
recognition. `GestureTargetConfig.on_scroll` receives the event and returns true
to claim it or false to decline. The first willing live target in hit-path order
handles the whole event; no arena entry or pointer capture is created.

```rust
use rust_flutter::{egui::{pos2,vec2,Id},gestures::*};
use std::{cell::Cell,rc::Rc,time::Duration};
let binding=GestureBinding::new();
let calls=Rc::new(Cell::new(0));
let log=calls.clone();
let id=Id::new("wheel-target");
let target=binding.register_target(id,GestureTargetConfig {
    on_scroll:Some(Rc::new(move |event| {
        assert_eq!(event.scroll_delta.y,20.0);
        log.set(log.get()+1);
        true
    })),
    ..Default::default()
})?;
let report=binding.dispatch_scroll(PointerScrollEvent {
    position:pos2(10.0,10.0),scroll_delta:vec2(0.0,20.0),
    unit:ScrollSignalUnit::Points,time:Duration::ZERO,modifiers:Default::default(),
},[id]);
assert_eq!(report.scroll_claims.len(),1);
assert!(report.scroll_failures.is_empty());
assert_eq!(calls.get(),1);
assert_eq!(binding.active_pointers(),0);
drop(target);
# Ok::<(),Box<dyn std::error::Error>>(())
```

## Dispatch and failures

Positions and point deltas use global logical coordinates. Positive delta means
forward content offset; the egui adapter negates egui's content-movement delta.
Units are Points, Lines and Pages. Timestamps are explicit Duration values; signals
do not maintain a per-pointer movement history. Finite position/delta validation
precedes callbacks, and zero vectors are quiet.

Targets are snapshotted by identity when queued. Removed/replaced targets are
skipped; newly registered targets do not join an earlier packet. Callback-only
updates use current callbacks without replacing tap/pan recognition. Duplicated
hit IDs are visited once. A false return should mean the handler did not consume
scroll movement. The framework cannot roll back arbitrary callback side effects.
Panicking handlers abort that event with ScrollCallbackPanicked instead of
falling through after possible side effects. Other queued packets still run.
Panic isolation requires unwinding.

Signals and pointer packets share a FIFO queue. Reentrant calls return queued;
the outer report contains their resulting claims/failures. No state/configuration
borrow is held while callbacks execute. Disposing the binding stops subsequent
delivery; disposing its arena/router also makes later signal dispatch terminal.

DispatchReport now includes `scroll_claims` and `scroll_failures`, alongside
ordinary pointer failures. `processed` counts attempted packets of both kinds.
Inspect both failure collections. Adding on_scroll extends the experimental
GestureTargetConfig API: explicit exhaustive struct literals must add the field;
`..Default::default()` keeps it absent without changing existing recognizers.

## Host adapters and target selection

GestureFrame uses the same fresh root geometry, reverse paint order, clips,
layer transforms, enabled state and modal/backend-area occlusion for signals
as it does for pointer hits. Geometry is snapshotted for the pass; state changes
request a later layout rather than recompute geometry between same-pass packets.
Each wheel packet is hit-tested at its event-time
cursor position; it does not follow drag capture. Pointer movement/button/gone
events update that position in packet order. Stationary cursor position persists
between frames. An unfinished collector invalidates its cursor snapshot so the
next frame can use current backend hover state. Repeated egui passes do not
replay signals. Unfocused frames do not dispatch input.

The older single-root adapter also supports wheel packets using its root/clip
coordinate contract; transformed/multiple roots should use GestureFrame. Hosts
can bypass egui normalization with `dispatch_scroll` / `dispatch_scroll_layout`.

## Retained viewport policy

SingleChildScrollView now registers a signal callback automatically. It uses the
axis component, flips for reverse scrolling, and clamps to measured content
extents. Point deltas divide by positive layer scale. Lines multiply by a fixed
20 local logical points, while Pages multiply by viewport length; these two units
are dimensionless counts and do not divide by layer scale. The line constant is
an explicit current policy, not an audited platform setting.

Shift maps a purely vertical wheel delta to a horizontal viewport; a vertical
viewport declines that shifted delta. Ctrl/Command signals are declined to leave
zoom shortcuts available. A viewport with no possible offset change declines,
allowing an ancestor to handle it. Partial movement to an edge still claims the
whole event: unused distance/components are not redistributed. This is basic
edge handoff, not coordinated nested scrolling.

A handled signal performs an immediate position jump, canceling existing
programmatic/ballistic/drag activity. Later captured drag moves are ignored until
a fresh drag starts. Raw point bursts, including backend-provided momentum, move
by their supplied deltas; no extra wheel inertia or trackpad phase model is
invented. The latest measured dimensions are published outside render borrows
before applying movement.

## Evidence and remaining work

Tests cover first-willing resolution, validation, panic isolation, callback updates,
removed/replaced targets, reentrant FIFO ordering, service disposal, viewport
unit/scale/modifier conversion, motion interruption, raw frame deduplication,
stationary/event-time cursor handling, single-root input, overlapping disabled
roots and nested at-edge handoff. The Linux native probe sends real X11 wheel
input to the gallery and requires changed content, then a controller reset back
to the exact original pixels. Android still runs its touch/clip/reset path;
the Android host now emits the scroll packet's current pointer position before
its wheel event. OS-injected Android mouse scroll now passes on the API-35 emulator; see
[Android input evidence](android-wheel-input.md). Physical Android mouse and
macOS trackpad hardware remain unverified.

Retained targets share this resolver. Ordinary egui ScrollArea controls remain
independent: clearing available raw/smoothed delta after a claim does not undo an
earlier backend handler or clear egui's private smoothing backlog. Mixing nested
backend and retained scroll areas is not a verified arbitration path. Distinct
bindings in one viewport also do not share a global signal resolver. Use one
shared binding for the retained scene. Full backend arbitration, trackpad
pan/zoom phases, device-specific settings, residual redistribution, nested scroll
activities, scrollbars, accessibility scrolling and platform feel remain pending.

Reviewed references: [Flutter signal resolver](https://api.flutter.dev/flutter/gestures/PointerSignalResolver-class.html),
[PointerScrollEvent](https://api.flutter.dev/flutter/gestures/PointerScrollEvent-class.html),
and [egui 0.33.3 input event definitions](https://github.com/emilk/egui/blob/0.33.3/crates/egui/src/data/input.rs).
This is an original partial Rust protocol, not complete Flutter member parity.

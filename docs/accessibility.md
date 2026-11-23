# Accessibility semantics

The framework uses AccessKit for desktop and Android accessibility. egui builds
the semantics tree from framework controls and app content. On Android,
NativeActivity forwards that tree to the decor view and returns accessibility
actions to the UI event loop. The bridge supports TalkBack integration, but
screen-reader behavior still needs testing on physical devices.

## Label custom content

Wrap custom visuals with `element::Semantics` to provide a role and accessible
name. Keep pointer or keyboard interaction on the child widget; `on_click` adds
the corresponding screen-reader action.

```rust,ignore
use rust_flutter::{egui, element::{Semantics, Widget}};

let accessible_play = Semantics::new(egui::accesskit::Role::Button)
    .label("Play")
    .on_click(|| start_playback())
    .into_widget(play_icon);
```

The wrapper can also set a value, enabled state, selected state, and toggle state.
Retained `Text` and `Paragraph` widgets expose their combined span text as a
screen-reader label, including rich paragraphs with multiple styled spans.
`Semantics::hidden(true)` removes a node and its descendants from accessibility
and hit testing. `live_region(Live::Polite)` and
`live_region(Live::Assertive)` mark announcement regions, and
`heading_level(level)` attaches a level to a `Role::Heading` node.
Material and Cupertino buttons expose their button role and names; navigation
tabs expose selected state. Tappable Material cards, list tiles, and floating
action buttons expose action roles and states. Material `IconButton` and
`FloatingActionButton` accept optional
`semantics_label` values for icon-only actions.
Cupertino sliding segmented controls expose each option as a labeled radio
with selected and disabled state. `CupertinoListTile` exposes list-item/button
roles and an optional name. Material and Cupertino sliders expose numeric range,
current value, increment/decrement, and set-value actions. `CupertinoSwitch` exposes its switch role,
on/off state, disabled state, and optional accessible name.
Material linear/circular and Cupertino activity progress indicators expose the
progress role; determinate values are numeric percentages, and the Cupertino
indicator accepts an optional accessible name. Material `SnackBar` exposes its
message as a labeled alert.

```rust,ignore
let save = IconButton::new(&binding, save_icon, save_document)
    .semantics_label("Save document")
    .enabled(can_save)
    .into_widget()?;
```

`Semantics::on_click` does not add pointer input by itself. Use it with a child
that already handles pointer input, or with a child whose only action is
screen-reader activation. A disabled wrapper omits the click action and reports
the disabled state.

## Current scope

The retained wrapper supports one child, role, label, value, enabled, selected,
toggled, hidden state, heading level, live region, and click action. Retained
text currently exposes a label and bounds; per-run text ranges, selection, and
text-navigation actions remain unfinished.
Full Flutter semantics support remains incomplete:
merging and excluding descendants, traversal ordering, focus actions, custom
action catalogs, and some control-specific metadata are not implemented.

Desktop builds use AccessKit through eframe. Android builds use the direct
NativeActivity bridge. Compile checks do not verify VoiceOver or TalkBack; follow
the interactive gates in [platform development](platforms.md) and record the OS,
device, and screen reader versions used.

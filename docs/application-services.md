# Lifecycle, restoration and localization

The application struct owns persistent state. The runner calls `initialize` once
before the first frame. Its `AppContext` supplies an egui context and optional
private data directory. Desktop resolves a directory under the OS application data
root, using `NativeOptions.viewport.app_id` (or the title when omitted). Set a
stable reverse-domain app ID in production. On Android the runner uses the
activity's internal app directory; no storage permission is needed for that path.

`on_event` receives focus changes, back requests and window-close requests.
Returning `EventResponse::Consumed` from a close request cancels closing. A
consumed back request can pop an app-owned `Navigator`. On Android, an unconsumed
back request asks the viewport to close. Desktop escape is not treated as back,
so it remains available for editing, dialogs and keyboard shortcuts.

```rust
use rust_flutter::prelude::*;

struct App { routes: Navigator<&'static str> }
impl Application for App {
    fn on_event(&mut self, _: &egui::Context, event: AppEvent) -> EventResponse {
        if event == AppEvent::BackRequested && self.routes.pop().is_some() {
            EventResponse::Consumed
        } else {
            EventResponse::Continue
        }
    }
    fn build(&mut self, ui: &mut Ui) { text(ui, *self.routes.current()); }
}
```

`shutdown` is called once on a graceful exit. Mobile OS process termination can
skip it. Save important state when it changes. Android also dispatches `AppEvent::Lifecycle`: start, resume, pause, stop,
destroy, surface creation/loss, low-memory, configuration change and save-state
requests. These arrive without requiring a renderable surface. Destroy is terminal
and invokes shutdown once. Native saved-instance-state byte serialization remains
a separate integration; the gallery persists its JSON model on pause/save-state. `Runtime` drives
these same hooks inside `egui::Context::run` for headless tests.

## JSON snapshots

`storage::JsonStore` accepts a path and schema number. `save` serializes to a
separate file in the same directory, syncs it, and replaces the old snapshot. On
Unix it also syncs the parent directory. Concurrent writers use distinct temporary
files; the last successful replacement wins. This does not provide multi-process
transactions or encrypted storage.

```rust
use rust_flutter::storage::JsonStore;
let dir = tempfile::tempdir()?;
let store = JsonStore::new(dir.path().join("count.json"), 1);
store.save(&7_i32)?;
assert_eq!(store.load::<i32>()?, Some(7));
# Ok::<(), Box<dyn std::error::Error>>(())
```

App data types implement `serde::Serialize` and `serde::Deserialize`. Missing files
return `Ok(None)`; malformed data, incompatible schemas and oversized files return
errors. Schema errors are detected before deserializing an application's changed
data type. The default size limit is 1 MiB and can be changed with `with_max_bytes`.
Do not silently overwrite unreadable state with defaults. The gallery preserves
such a file and displays the failure. It stores preferences immediately after
changes and verifies restoration through a fresh process in the desktop smoke test.

## Platform services

`services::Services` is an immutable, typed registry available from
`AppContext.services` during initialization, restoration and saved-state hooks.
Register a target-specific implementation before launching, then clone an
`Arc` handle during `initialize` if the app needs it in later callbacks:

```rust,ignore
let mut services = rust_flutter::services::Services::default();
services.insert::<rust_flutter::services::ClipboardService>(
    std::sync::Arc::new(MyClipboard::new_for_this_platform()),
);
rust_flutter::run_with_services("Notes", app, services)?;

impl Application for NotesApp {
    fn initialize(&mut self, context: &AppContext) {
        self.clipboard = context
            .services
            .get::<rust_flutter::services::ClipboardService>()
            .cloned();
    }
}
```

`run_with_options_and_services` and the retained-tree launchers accept the same
registry. Implementations are app-owned and can call JNI, Objective-C or another
platform library. The framework does not yet bundle a native plugin catalog or
provide a cross-platform method-channel transport.

`AppContext::clipboard()` is a built-in host-backed service. It queues text on
egui's platform output; Android sends it through the system `ClipboardManager`,
and desktop eframe delivers it to the native clipboard. The returned `Result`
reports whether the request was queued, not a later OS permission or delivery
failure. Register `ClipboardService` in `Services` to replace the default.

## Fluent translations

`localization::Localizations` loads Fluent FTL resources. The first locale is the
fallback and cannot be removed. Add complete resources for additional locales,
then select preferred language identifiers in priority order. Negotiation uses
Fluent's CLDR-aware language matching. Missing messages or attributes fall through
the negotiated locale chain to the fallback resource.

```rust
use rust_flutter::localization::{FluentArgs, Localizations};
let mut strings = Localizations::new("en", "greeting = Hello, { $name }!\n")?;
strings.add_locale("es", "greeting = ¡Hola, { $name }!\n")?;
strings.select(&["es-MX"])?;
let mut args = FluentArgs::new();
args.set("name", "Ada");
let greeting = strings.format("greeting", Some(&args))?;
assert!(greeting.contains("¡Hola,"));
# Ok::<(), Box<dyn std::error::Error>>(())
```

Fluent handles plural/select expressions and isolates interpolated values using
Unicode FSI/PDI marks. Invalid resources, duplicate locales, missing messages and
formatting failures are explicit errors. Adding invalid resources or selecting
invalid identifiers leaves the previous catalog/selection intact. Tests exercise
Spanish regional fallback, Chinese script selection, Polish plural categories,
missing arguments and per-message fallback.

This module does not discover the device locale, supply translated framework
controls, or implement RTL layout and complex text shaping. The gallery lets the
user choose English or Spanish explicitly. Date, currency and time formatting,
ARB compatibility and localization code generation remain future work.

References: [Flutter internationalization](https://docs.flutter.dev/ui/internationalization),
[Fluent bundle](https://docs.rs/fluent-bundle/0.16.0/fluent_bundle/),
[Fluent negotiation](https://docs.rs/fluent-langneg/0.13.1/fluent_langneg/).

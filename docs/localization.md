# App localization

The framework's `Localizations` catalog parses Fluent Translation List (FTL)
resources, negotiates requested language identifiers against supported locales,
and falls back per message to the configured fallback catalog. Add the catalog
to `MaterialApp` to expose it to descendant components:

```rust,ignore
use rust_flutter::{
    element::{BuildContext, StatelessWidget, Widget},
    localization::Localizations,
    material::MaterialApp,
    text::Text,
};

struct Greeting;
impl StatelessWidget for Greeting {
    fn build(&self, context: &BuildContext) -> Widget {
        let catalog = Localizations::of(context).unwrap().unwrap();
        Text::new(catalog.text("greeting").unwrap()).into_widget()
    }
}

let mut catalog = Localizations::new("en-US", "greeting = Hello")?;
catalog.add_locale("es", "greeting = Hola")?;
catalog.select(&["es-MX"])?;
let app = MaterialApp::new(Widget::stateless(Greeting)).localizations(catalog);
let root = app.into_widget();
# let _ = root;
```

`Localizations::of(context)` subscribes the component to catalog replacement.
Use `text`, `format`, or `attribute` to resolve Fluent messages; Fluent's plural
rules and Unicode isolation marks are retained. `AppContext::preferred_locales`
contains a startup snapshot of host language preferences in BCP 47 form. macOS
uses the preferred language list; Android uses the activity configuration's
language and country. Call `Localizations::select` with those values after
loading the catalog:

```rust,ignore
let preferred: Vec<&str> = app_context
    .preferred_locales
    .iter()
    .map(String::as_str)
    .collect();
catalog.select(&preferred)?;
```

Locale changes after startup are not observed automatically, and directionality
is still explicit: configure `MaterialApp::text_direction` when needed. RTL text
shaping, font fallback, and translated component catalogs are also incomplete.
See the official
[Fluent syntax guide](https://projectfluent.org/fluent/guide/) and the
[Flutter localization overview](https://docs.flutter.dev/ui/accessibility-and-internationalization/internationalization).

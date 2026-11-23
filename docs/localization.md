# App localization

The framework's `Localizations` catalog parses Fluent Translation List (FTL)
resources, negotiates requested language identifiers against supported locales,
and falls back per message to the configured fallback catalog. Add the catalog
to `MaterialApp` to expose it to descendant components:

```rust
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
rules and Unicode isolation marks are retained. Catalog locale selection is
explicit: hosts do not yet discover the operating system's preferred locales or
change directionality automatically; configure `MaterialApp::text_direction`
explicitly when needed. RTL text shaping, font fallback, and
translated component catalogs are also incomplete. See the official
[Fluent syntax guide](https://projectfluent.org/fluent/guide/) and the
[Flutter localization overview](https://docs.flutter.dev/ui/accessibility-and-internationalization/internationalization).

use rust_flutter::{
    localization::{FluentArgs, LocalizationError, Localizations},
    storage::{JsonStore, StoreError},
};
use serde::{Deserialize, Serialize};

const EN: &str = "hello = Hello, { $name }!\nfallback = Default text\ncount = { $n ->\n    [one] One item\n   *[other] { $n } items\n}\ninput =\n    .placeholder = Your name\n";

#[test]
fn locale_negotiation_and_per_message_fallback_preserve_preferences() {
    let mut l10n = Localizations::new("en-US", EN).unwrap();
    l10n.add_locale("es", "hello = ¡Hola, { $name }!\n")
        .unwrap();
    l10n.add_locale("zh-Hans", "script = Simplified\n").unwrap();
    l10n.add_locale("zh-Hant", "script = Traditional\n")
        .unwrap();
    assert_eq!(l10n.select(&["es-MX"]).unwrap().to_string(), "es");
    assert_eq!(l10n.text("fallback").unwrap(), "Default text");
    assert_eq!(
        l10n.attribute("input", "placeholder", None).unwrap(),
        "Your name"
    );
    assert_eq!(l10n.select(&["zh-TW"]).unwrap().to_string(), "zh-Hant");
    assert_eq!(l10n.text("script").unwrap(), "Traditional");
    assert_eq!(l10n.select(&["fr-CA"]).unwrap().to_string(), "en-US");
    assert_eq!(l10n.select(&[]).unwrap().to_string(), "en-US");
}

#[test]
fn fluent_uses_locale_specific_plural_rules_and_isolates_arguments() {
    let mut l10n = Localizations::new("en-US", EN).unwrap();
    l10n.add_locale(
        "pl",
        "count = { $n ->\n    [one] jeden\n    [few] kilka\n   *[many] wiele\n}\n",
    )
    .unwrap();
    l10n.select(&["pl"]).unwrap();
    for (n, expected) in [
        (1, "jeden"),
        (2, "kilka"),
        (5, "wiele"),
        (22, "kilka"),
        (12, "wiele"),
    ] {
        let mut args = FluentArgs::new();
        args.set("n", n);
        assert_eq!(l10n.format("count", Some(&args)).unwrap(), expected);
    }
    let mut args = FluentArgs::new();
    args.set("name", "Ada");
    assert_eq!(
        l10n.format("hello", Some(&args)).unwrap(),
        "Hello, \u{2068}Ada\u{2069}!"
    );
}

#[test]
fn invalid_resources_and_selections_are_explicit_and_leave_catalog_intact() {
    let mut l10n = Localizations::new("en", EN).unwrap();
    assert!(l10n.add_locale("de", "broken = { invalid").is_err());
    assert_eq!(l10n.supported().len(), 1);
    assert!(matches!(
        l10n.add_locale("en", EN),
        Err(LocalizationError::DuplicateLocale(_))
    ));
    assert!(l10n.select(&["not_a_locale!"]).is_err());
    assert_eq!(l10n.locale().to_string(), "en");
    assert!(matches!(
        l10n.text("hello"),
        Err(LocalizationError::Format { .. })
    ));
    assert!(matches!(
        l10n.text("absent"),
        Err(LocalizationError::MissingMessage(_))
    ));
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Preferences {
    name: String,
    count: i32,
}
#[test]
fn snapshots_roundtrip_replace_and_reject_incompatible_schema_without_data_loss() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested/preferences.json");
    let store = JsonStore::new(&path, 2);
    assert_eq!(store.load::<Preferences>().unwrap(), None);
    let before = Preferences {
        name: "नाम".into(),
        count: 7,
    };
    store.save(&before).unwrap();
    assert_eq!(store.load::<Preferences>().unwrap(), Some(before));
    let after = Preferences {
        name: "Ada".into(),
        count: 9,
    };
    store.save(&after).unwrap();
    assert_eq!(store.load::<Preferences>().unwrap(), Some(after));
    assert!(matches!(
        JsonStore::new(&path, 3).load::<Vec<u8>>(),
        Err(StoreError::Schema {
            expected: 3,
            found: 2
        })
    ));
    assert_eq!(
        std::fs::read_dir(path.parent().unwrap()).unwrap().count(),
        1,
        "temporary writes must not leak files"
    );
}

#[test]
fn corrupt_and_oversized_snapshots_are_not_silently_reset_or_overwritten_on_failed_save() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.json");
    let store = JsonStore::new(&path, 1);
    store.save(&42).unwrap();
    let original = std::fs::read(&path).unwrap();
    let small = store.clone().with_max_bytes(4);
    assert!(matches!(
        small.load::<i32>(),
        Err(StoreError::TooLarge { .. })
    ));
    assert!(matches!(small.save(&99), Err(StoreError::TooLarge { .. })));
    assert_eq!(std::fs::read(&path).unwrap(), original);
    std::fs::write(&path, "{broken}").unwrap();
    assert!(matches!(store.load::<i32>(), Err(StoreError::Json(_))));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "{broken}");
}

#[test]
fn concurrent_snapshots_are_complete_and_do_not_share_temporary_files() {
    let dir = tempfile::tempdir().unwrap();
    let store = JsonStore::new(dir.path().join("state.json"), 1);
    let workers: Vec<_> = (0..8)
        .map(|n| {
            let store = store.clone();
            std::thread::spawn(move || store.save(&vec![n; 1000]).unwrap())
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    let value = store.load::<Vec<usize>>().unwrap().unwrap();
    assert_eq!(value.len(), 1000);
    assert!(value.iter().all(|n| *n == value[0]));
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

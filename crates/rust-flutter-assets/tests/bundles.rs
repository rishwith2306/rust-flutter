use rust_flutter_assets::{AssetBundle, AssetError, EmbeddedBundle};

static BUNDLE: EmbeddedBundle = EmbeddedBundle::new(&[
    (
        "icon.png",
        &[
            ("4x/icon.png", 4.0, b"four"),
            ("icon.png", 1.0, b"one"),
            ("2x/icon.png", 2.0, b"two"),
        ],
    ),
    ("no-base.png", &[("3x/no-base.png", 3.0, b"three")]),
    ("hello.txt", &[("hello.txt", 1.0, "Hello, नमस्ते".as_bytes())]),
    ("binary", &[("binary", 1.0, &[255])]),
]);
#[test]
fn density_selection_matches_low_dpr_bias_nearest_high_dpr_and_exact_ties() {
    for (density, expected) in [
        (0.5, 1.0),
        (1.0, 1.0),
        (1.25, 2.0),
        (2.0, 2.0),
        (2.25, 2.0),
        (3.0, 2.0),
        (3.25, 4.0),
        (8.0, 4.0),
    ] {
        assert_eq!(BUNDLE.resolve("icon.png", density).unwrap().scale, expected);
    }
    assert_eq!(BUNDLE.resolve("no-base.png", 0.5).unwrap().bytes, b"three");
    assert_eq!(BUNDLE.resolve("icon.png", 4.0).unwrap().path, "4x/icon.png");
}
#[test]
fn missing_invalid_density_invalid_text_and_empty_bundle_fail_explicitly() {
    assert_eq!(BUNDLE.load_string("hello.txt").unwrap(), "Hello, नमस्ते");
    assert!(matches!(
        BUNDLE.load_string("binary"),
        Err(AssetError::Utf8 { .. })
    ));
    assert!(matches!(
        BUNDLE.load("absent"),
        Err(AssetError::NotFound(_))
    ));
    for ratio in [0.0, -1.0, f32::INFINITY, f32::NAN] {
        assert!(matches!(
            BUNDLE.resolve("icon.png", ratio),
            Err(AssetError::InvalidDensity(_))
        ));
    }
    let empty = EmbeddedBundle::new(&[("bad", &[])]);
    assert!(matches!(
        empty.load("bad"),
        Err(AssetError::InvalidBundle(_))
    ));
}

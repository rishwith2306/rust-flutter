use rust_flutter::{assets::*, egui};
static BUNDLE: EmbeddedBundle = EmbeddedBundle::new(&[
    (
        "badge",
        &[
            (
                "badge.png",
                1.0,
                include_bytes!("../../../examples/gallery/assets/badge.png"),
            ),
            (
                "2x/badge.png",
                2.0,
                include_bytes!("../../../examples/gallery/assets/2.0x/badge.png"),
            ),
            (
                "3x/badge.png",
                3.0,
                include_bytes!("../../../examples/gallery/assets/3.0x/badge.png"),
            ),
        ],
    ),
    (
        "other",
        &[(
            "other.png",
            1.0,
            include_bytes!("../../../examples/gallery/assets/badge.png"),
        )],
    ),
    ("broken", &[("broken.png", 1.0, b"not an image")]),
]);
#[test]
fn density_variants_preserve_logical_size_and_repeated_frames_reuse_textures() {
    let ctx = egui::Context::default();
    let mut images = AssetImages::new(&BUNDLE);
    let low = images.load_at_density(&ctx, "badge", 1.0).unwrap();
    assert_eq!(low.texture.size(), [48, 48]);
    let same = images.load_at_density(&ctx, "badge", 1.0).unwrap();
    assert_eq!(low.texture.id(), same.texture.id());
    let high = images.load_at_density(&ctx, "badge", 2.625).unwrap();
    assert_eq!(high.texture.size(), [144, 144]);
    assert_eq!(high.scale, 3.0);
    assert_eq!(high.logical_size(), low.logical_size());
    assert_eq!(images.cached_entries(), 2);
    assert_eq!(images.cached_bytes(), (48 * 48 + 144 * 144) * 4);
}
#[test]
fn lru_eviction_and_context_replacement_release_cache_owned_textures() {
    let ctx = egui::Context::default();
    let mut images = AssetImages::with_limits(
        &BUNDLE,
        ImageCacheLimits {
            max_bytes: 64 * 1024,
            max_entries: 1,
        },
    );
    let first = images.load(&ctx, "badge").unwrap();
    let id = first.texture.id();
    drop(first);
    let second = images.load(&ctx, "other").unwrap();
    assert!(ctx.tex_manager().read().meta(id).is_none());
    assert_eq!(images.cached_entries(), 1);
    let id = second.texture.id();
    drop(second);
    let fresh = egui::Context::default();
    let next = images.load(&fresh, "badge").unwrap();
    assert!(ctx.tex_manager().read().meta(id).is_none());
    assert!(fresh.tex_manager().read().meta(next.texture.id()).is_some());
    images.clear();
    assert_eq!(images.cached_bytes(), 0);
    assert!(fresh.tex_manager().read().meta(next.texture.id()).is_some()); // Caller holds a clone.
    let id = next.texture.id();
    drop(next);
    assert!(fresh.tex_manager().read().meta(id).is_none());
}
#[test]
fn invalid_images_missing_assets_and_budget_limits_fail_without_filling_the_cache() {
    let ctx = egui::Context::default();
    let mut images = AssetImages::with_limits(
        &BUNDLE,
        ImageCacheLimits {
            max_bytes: 100,
            max_entries: 1,
        },
    );
    assert!(matches!(
        images.load(&ctx, "badge"),
        Err(ImageAssetError::TooLarge { .. })
    ));
    assert!(images.load(&ctx, "broken").is_err());
    assert!(matches!(
        images.load(&ctx, "missing"),
        Err(ImageAssetError::Asset(AssetError::NotFound(_)))
    ));
    assert_eq!(images.cached_entries(), 0);
}
#[test]
fn png_jpeg_and_webp_decode_to_correct_rgba_dimensions() {
    struct Memory {
        bytes: Vec<u8>,
    }
    impl AssetBundle for Memory {
        fn resolve(&self, _: &str, _: f32) -> Result<ResolvedAsset<'_>, AssetError> {
            Ok(ResolvedAsset {
                path: "memory",
                scale: 1.0,
                bytes: &self.bytes,
            })
        }
    }
    for format in [
        image::ImageFormat::Png,
        image::ImageFormat::Jpeg,
        image::ImageFormat::WebP,
    ] {
        let mut encoded = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(5, 7)
            .write_to(&mut encoded, format)
            .unwrap();
        let mut images = AssetImages::new(Memory {
            bytes: encoded.into_inner(),
        });
        assert_eq!(
            images
                .load(&egui::Context::default(), "image")
                .unwrap()
                .texture
                .size(),
            [5, 7]
        );
    }
}

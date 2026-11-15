#![cfg(feature = "build")]
use rust_flutter_assets::build::compile_for_target;
use std::{fs, path::Path};
fn manifest(root: &Path, assets: &str) {
    fs::write(
        root.join("assets.json"),
        format!(r#"{{"version":1,"assets":{assets}}}"#),
    )
    .unwrap();
}
#[test]
fn packs_files_directories_variants_platforms_and_tracks_changes_deterministically() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    fs::create_dir_all(p.join("assets/2.0x")).unwrap();
    fs::create_dir_all(p.join("assets/nested")).unwrap();
    fs::write(p.join("assets/note.txt"), "hello").unwrap();
    fs::write(p.join("assets/icon.png"), b"base").unwrap();
    fs::write(p.join("assets/2.0x/icon.png"), b"hi").unwrap();
    fs::write(p.join("assets/nested/ignored.txt"), "ignored").unwrap();
    manifest(
        p,
        r#"["assets/",{"path":"android-only.txt","platforms":["android"]}]"#,
    );
    let out = p.join("bundle.rs");
    compile_for_target(p.join("assets.json"), &out, "macos").unwrap();
    let first = fs::read_to_string(&out).unwrap();
    assert!(first.contains("assets/note.txt") && first.contains("assets/2.0x/icon.png"));
    assert!(!first.contains("ignored") && !first.contains("android-only"));
    compile_for_target(p.join("assets.json"), &out, "macos").unwrap();
    assert_eq!(first, fs::read_to_string(&out).unwrap());
    fs::create_dir(p.join("assets/3x")).unwrap();
    fs::write(p.join("assets/3x/icon.png"), b"more").unwrap();
    compile_for_target(p.join("assets.json"), &out, "macos").unwrap();
    assert!(
        fs::read_to_string(&out)
            .unwrap()
            .contains("assets/3x/icon.png")
    );
    assert!(compile_for_target(p.join("assets.json"), &out, "android").is_err());
}
#[test]
fn missing_base_is_allowed_but_invalid_inputs_preserve_previous_output() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    fs::create_dir_all(p.join("2x")).unwrap();
    fs::write(p.join("2x/icon.png"), b"image").unwrap();
    let out = p.join("bundle.rs");
    manifest(p, r#"["icon.png"]"#);
    compile_for_target(p.join("assets.json"), &out, "linux").unwrap();
    let good = fs::read(&out).unwrap();
    for input in [
        r#"["../escape"]"#,
        r#"["/absolute"]"#,
        r#"["missing"]"#,
        r#"[{"path":"icon.png","platforms":["typo"]}]"#,
    ] {
        manifest(p, input);
        assert!(compile_for_target(p.join("assets.json"), &out, "linux").is_err());
        assert_eq!(fs::read(&out).unwrap(), good);
    }
    fs::create_dir(p.join("2.0x")).unwrap();
    fs::write(p.join("2.0x/icon.png"), b"duplicate").unwrap();
    manifest(p, r#"["icon.png"]"#);
    assert!(compile_for_target(p.join("assets.json"), &out, "linux").is_err());
}
#[cfg(unix)]
#[test]
fn symlinks_cannot_embed_files_outside_the_manifest_root() {
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::NamedTempFile::new().unwrap();
    std::os::unix::fs::symlink(outside.path(), dir.path().join("secret")).unwrap();
    manifest(dir.path(), r#"["secret"]"#);
    assert!(
        compile_for_target(
            dir.path().join("assets.json"),
            dir.path().join("bundle.rs"),
            "linux"
        )
        .is_err()
    );
}

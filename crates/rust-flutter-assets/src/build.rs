//! Build-script compiler for a versioned JSON manifest. See `docs/assets.md`.
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs, io,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    assets: Vec<Entry>,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum Entry {
    Path(String),
    Options(Options),
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Options {
    path: String,
    platforms: Vec<String>,
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}
fn valid_path(path: &str) -> io::Result<()> {
    let trimmed = path.strip_suffix('/').unwrap_or(path);
    if trimmed.is_empty()
        || path.contains(['\\', '\n', '\r', '\0'])
        || trimmed
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == ".." || p.contains(':'))
    {
        return Err(invalid(format!(
            "asset path must be relative with normalized / separators: {path:?}"
        )));
    }
    Ok(())
}
fn checked(root: &Path, path: &Path) -> io::Result<PathBuf> {
    let canonical = path.canonicalize()?;
    if !canonical.starts_with(root)
        || canonical.to_str().is_none()
        || canonical.to_string_lossy().contains(['\n', '\r'])
    {
        return Err(invalid(format!(
            "asset path escapes manifest directory or cannot be tracked: {}",
            path.display()
        )));
    }
    Ok(canonical)
}
fn children(root: &Path, directory: &Path) -> io::Result<Vec<PathBuf>> {
    checked(root, directory)?;
    let mut paths = fs::read_dir(directory)?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<io::Result<Vec<_>>>()?;
    paths.sort();
    Ok(paths)
}
fn density(path: &Path) -> Option<f32> {
    let name = path.file_name()?.to_str()?.strip_suffix('x')?;
    if !name.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
        return None;
    }
    let value: f32 = name.parse().ok()?;
    (value.is_finite() && value > 0.0).then_some(value)
}

/// Read `CARGO_CFG_TARGET_OS` and compile a manifest relative to its own directory.
/// The output is a Rust slice expression accepted by `EmbeddedBundle::new`.
/// Cargo rerun directives track files and discovery directories, including newly
/// added density variants. Build dependencies need the `build` feature.
pub fn compile(manifest: impl AsRef<Path>, output: impl AsRef<Path>) -> io::Result<()> {
    let target = env::var("CARGO_CFG_TARGET_OS")
        .map_err(|_| invalid("CARGO_CFG_TARGET_OS missing; call compile from build.rs"))?;
    compile_for_target(manifest, output, &target)
}

/// Explicit target form for tooling/tests; platform names use Rust target OS names.
pub fn compile_for_target(
    manifest: impl AsRef<Path>,
    output: impl AsRef<Path>,
    target: &str,
) -> io::Result<()> {
    let manifest = manifest.as_ref().canonicalize()?;
    let root = manifest
        .parent()
        .ok_or_else(|| invalid("manifest has no parent"))?;
    let parsed: Manifest = serde_json::from_slice(&fs::read(&manifest)?)
        .map_err(|e| invalid(format!("{}: {e}", manifest.display())))?;
    if parsed.version != 1 {
        return Err(invalid("unsupported asset manifest version; expected 1"));
    }
    let mut watched = BTreeSet::from([manifest.clone()]);
    let mut keys = BTreeSet::new();
    for entry in parsed.assets {
        let (path, enabled) = match entry {
            Entry::Path(path) => (path, true),
            Entry::Options(options) => {
                if options.platforms.is_empty()
                    || options.platforms.iter().any(|p| {
                        ![
                            "android",
                            "macos",
                            "linux",
                            "windows",
                            "ios",
                            "freebsd",
                            "openbsd",
                            "netbsd",
                            "emscripten",
                            "unknown",
                        ]
                        .contains(&p.as_str())
                    })
                {
                    return Err(invalid("platforms must be nonempty Rust target OS names"));
                }
                let enabled = options.platforms.iter().any(|p| p == target);
                (options.path, enabled)
            }
        };
        valid_path(&path)?;
        if !enabled {
            continue;
        }
        if path.ends_with('/') {
            let directory = root.join(&path);
            watched.insert(checked(root, &directory)?);
            for file in children(root, &directory)? {
                if file.is_file() {
                    let name = file
                        .file_name()
                        .and_then(|n| n.to_str())
                        .ok_or_else(|| invalid("asset filename must be UTF-8"))?;
                    keys.insert(format!("{path}{name}"));
                }
            }
        } else {
            keys.insert(path);
        }
    }
    let mut bundle = BTreeMap::new();
    for key in keys {
        valid_path(&key)?;
        let file = root.join(&key);
        let parent = file
            .parent()
            .ok_or_else(|| invalid("asset has no parent"))?;
        watched.insert(checked(root, parent)?);
        let mut variants = Vec::new();
        if file.is_file() {
            variants.push((1.0_f32, key.clone(), checked(root, &file)?));
        }
        for directory in children(root, parent)? {
            if directory.is_dir()
                && let Some(scale) = density(&directory)
            {
                watched.insert(checked(root, &directory)?);
                let variant = directory.join(file.file_name().unwrap());
                if variant.is_file() {
                    let relative = variant
                        .strip_prefix(root)
                        .unwrap()
                        .to_str()
                        .ok_or_else(|| invalid("asset path must be UTF-8"))?
                        .to_owned();
                    valid_path(&relative)?;
                    variants.push((scale, relative, checked(root, &variant)?));
                }
            }
        }
        if variants.is_empty() {
            return Err(invalid(format!(
                "asset has no file or density variants: {key}"
            )));
        }
        variants.sort_by(|a, b| a.0.total_cmp(&b.0));
        if variants.windows(2).any(|v| v[0].0 == v[1].0) {
            return Err(invalid(format!("duplicate density for asset: {key}")));
        }
        for (_, _, path) in &variants {
            watched.insert(path.clone());
        }
        bundle.insert(key, variants);
    }
    let mut source = String::from("// Generated asset bundle. Do not edit.\n&[\n");
    for (key, variants) in bundle {
        source.push_str(&format!("    ({key:?}, &[\n"));
        for (scale, relative, path) in variants {
            source.push_str(&format!(
                "        ({relative:?}, {scale:?}_f32, include_bytes!({:?})),\n",
                path.to_str().unwrap()
            ));
        }
        source.push_str("    ]),\n");
    }
    source.push_str("]\n");
    // Validate completely before replacing a previous valid generated bundle.
    if fs::read_to_string(output.as_ref()).ok().as_deref() != Some(&source) {
        fs::write(output, source)?;
    }
    for path in watched {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    Ok(())
}

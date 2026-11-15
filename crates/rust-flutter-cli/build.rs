use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-env-changed=RUST_FLUTTER_FRAMEWORK_REV");
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let git_revision = || {
        if !root.join(".git").exists() {
            return None;
        }
        for path in [".git/HEAD", ".git/refs/heads", ".git/packed-refs"] {
            println!("cargo:rerun-if-changed={}", root.join(path).display());
        }
        let result = Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(["rev-parse", "HEAD"])
            .output()
            .ok()?;
        result
            .status
            .success()
            .then(|| String::from_utf8_lossy(&result.stdout).trim().to_owned())
    };
    let revision = env::var("RUST_FLUTTER_FRAMEWORK_REV")
        .ok()
        .or_else(git_revision)
        .unwrap_or_else(|| format!("v{}", env!("CARGO_PKG_VERSION")));
    assert!(!revision.contains(['\n', '\r']), "invalid source revision");
    println!("cargo:rustc-env=RUST_FLUTTER_SOURCE_REV={revision}");
}

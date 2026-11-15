#[cfg(not(target_os = "android"))]
fn main() -> Result<(), rust_flutter::Error> {
    rust_flutter::run(
        "Rust Flutter Gallery",
        rust_flutter_gallery::Gallery::default(),
    )
}
#[cfg(target_os = "android")]
fn main() {}

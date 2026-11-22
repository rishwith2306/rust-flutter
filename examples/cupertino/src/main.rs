#[cfg(not(target_os = "android"))]
fn main() -> Result<(), rust_flutter::Error> {
    rust_flutter::run_widget_with_options(
        "Rust Flutter Cupertino Showcase",
        rust_flutter::native_options(rust_flutter::GraphicsBackend::Auto),
        rust_flutter_cupertino_example::showcase(),
    )
}

#[cfg(target_os = "android")]
fn main() {}

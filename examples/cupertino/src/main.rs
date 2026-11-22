#[cfg(not(target_os = "android"))]
fn main() -> Result<(), rust_flutter::Error> {
    let (root, on_back) = rust_flutter_cupertino_example::showcase_with_back_handler();
    rust_flutter::run_widget_with_options_and_back(
        "Rust Flutter Cupertino Showcase",
        rust_flutter::native_options(rust_flutter::GraphicsBackend::Auto),
        root,
        on_back,
    )
}

#[cfg(target_os = "android")]
fn main() {}

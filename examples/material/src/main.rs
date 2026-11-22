#[cfg(not(target_os = "android"))]
fn main() -> Result<(), rust_flutter::Error> {
    rust_flutter::run_widget_with_options(
        "Rust Flutter Material Showcase",
        rust_flutter::native_options(rust_flutter::GraphicsBackend::Auto),
        rust_flutter_material_example::showcase(),
    )
}

#[cfg(target_os = "android")]
fn main() {}

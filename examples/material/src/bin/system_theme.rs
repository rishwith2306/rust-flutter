fn main() -> Result<(), rust_flutter::Error> {
    rust_flutter::run_widget(
        "Rust Flutter System Theme",
        rust_flutter_material_example::platform_theme_showcase(),
    )
}

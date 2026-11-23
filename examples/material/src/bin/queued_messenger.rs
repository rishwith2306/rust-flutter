fn main() -> Result<(), rust_flutter::Error> {
    rust_flutter::run_widget(
        "Rust Flutter Snackbar Queue",
        rust_flutter_material_example::queued_messenger_showcase(),
    )
}

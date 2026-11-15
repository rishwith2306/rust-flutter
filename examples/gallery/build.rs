fn main() {
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("assets.rs");
    rust_flutter_assets::build::compile("assets.json", output)
        .expect("compile gallery asset bundle");
}

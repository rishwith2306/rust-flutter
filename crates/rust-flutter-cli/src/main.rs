use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

const HELP: &str = "rust-flutter — experimental Rust app tools

Usage:
  rust-flutter create <directory> [--name <package-name>] [--framework-path <checkout>]
  rust-flutter doctor [--android]
  rust-flutter devices
  rust-flutter run [--android] [--release] [--target <triple>] [--device <serial>]
  rust-flutter build [--android] [--release] [--target <triple>]

Run build/run inside the app directory. Android requires cargo-apk and SDK/NDK.
create pins GitHub dependencies to the CLI source revision. No hot reload is provided.";

fn main() -> ExitCode {
    match execute(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
fn execute(args: Vec<String>) -> Result<(), String> {
    let Some(command) = args.first().map(String::as_str) else {
        println!("{HELP}");
        return Ok(());
    };
    match command {
        "--help" | "-h" | "help" if args.len() == 1 => {
            println!("{HELP}");
            Ok(())
        }
        "--version" if args.len() == 1 => {
            println!("rust-flutter {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        "create" => {
            let destination = args
                .get(1)
                .filter(|s| !s.starts_with('-'))
                .ok_or("create requires a directory")?;
            let destination = Path::new(destination);
            let mut name = destination.file_name().and_then(|s| s.to_str());
            let mut framework = None;
            let (options, remainder) = args[2..].as_chunks::<2>();
            for pair in options {
                match pair[0].as_str() {
                    "--name" => name = Some(&pair[1]),
                    "--framework-path" if framework.is_none() => {
                        let root = PathBuf::from(&pair[1])
                            .canonicalize()
                            .map_err(|e| format!("framework checkout unavailable: {e}"))?;
                        let library = if root.join("crates/rust-flutter/Cargo.toml").is_file() {
                            root.join("crates/rust-flutter")
                        } else {
                            root
                        };
                        if !library.join("Cargo.toml").is_file()
                            || !library
                                .with_file_name("rust-flutter-assets")
                                .join("Cargo.toml")
                                .is_file()
                        {
                            return Err("framework path must contain the framework and sibling assets crate".into());
                        }
                        framework = Some(library);
                    }
                    _ => return Err(format!("unknown or repeated create option: {}", pair[0])),
                }
            }
            if !remainder.is_empty() {
                return Err("create option requires a value".into());
            }
            let name = name.ok_or("use --name for this directory")?;
            create(destination, name, framework.as_deref())?;
            println!(
                "Created {} at {}\nEnter the directory and run rust-flutter run.\nAndroid: rust-flutter doctor, then rust-flutter run --android",
                name,
                destination.display()
            );
            Ok(())
        }
        "doctor" if args.len() == 1 => doctor(false),
        "doctor" if args.len() == 2 && args[1] == "--android" => doctor(true),
        "devices" if args.len() == 1 => {
            let adb = env::var_os("ANDROID_HOME")
                .map(|p| PathBuf::from(p).join("platform-tools/adb"))
                .filter(|p| p.is_file())
                .unwrap_or_else(|| PathBuf::from("adb"));
            let status = Command::new(adb)
                .args(["devices", "-l"])
                .status()
                .map_err(|e| format!("could not start adb: {e}"))?;
            if status.success() {
                Ok(())
            } else {
                Err(format!("adb exited with {status}"))
            }
        }
        "run" | "build" => {
            let cargo_args = build_args(command, &args[1..])?;
            let mut cargo = Command::new("cargo");
            cargo.args(cargo_args);
            let status = cargo
                .status()
                .map_err(|e| format!("could not start cargo: {e}"))?;
            if status.success() {
                Ok(())
            } else {
                Err(format!("cargo exited with {status}"))
            }
        }
        _ => Err(format!("unknown command or options\n{HELP}")),
    }
}

fn valid_name(name: &str) -> bool {
    let mut chars = name.bytes();
    matches!(chars.next(), Some(b'a'..=b'z'))
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_' || c == b'-')
        && ![
            "self",
            "super",
            "crate",
            "mod",
            "fn",
            "type",
            "move",
            "ref",
            "use",
            "pub",
            "async",
            "await",
            "loop",
            "match",
            "impl",
            "trait",
            "struct",
            "enum",
            "const",
            "static",
            "let",
            "mut",
            "return",
            "true",
            "false",
            "in",
            "as",
            "break",
            "continue",
            "else",
            "extern",
            "for",
            "if",
            "unsafe",
            "where",
            "while",
            "dyn",
            "abstract",
            "become",
            "box",
            "do",
            "final",
            "macro",
            "override",
            "priv",
            "typeof",
            "unsized",
            "virtual",
            "yield",
            "try",
            "gen",
            "test",
            "build",
            "deps",
            "examples",
            "incremental",
            "rust-flutter",
        ]
        .contains(&name)
}
fn toml_string(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    )
}
fn create(destination: &Path, name: &str, framework: Option<&Path>) -> Result<(), String> {
    if !valid_name(name) {
        return Err("package name must start with a lowercase letter, contain only a-z, 0-9, _ or -, and not be reserved".into());
    }
    let (framework, asset_framework) = match framework {
        Some(path) => (
            format!(
                "path = {}",
                toml_string(path.to_str().ok_or("framework path is not UTF-8")?)
            ),
            format!(
                "path = {}",
                toml_string(
                    path.with_file_name("rust-flutter-assets")
                        .to_str()
                        .ok_or("asset path is not UTF-8")?
                )
            ),
        ),
        None => {
            let source = format!(
                "git = {}, rev = {}",
                toml_string(env!("CARGO_PKG_REPOSITORY")),
                toml_string(env!("RUST_FLUTTER_SOURCE_REV"))
            );
            (source.clone(), source)
        }
    };
    // Creating the directory exclusively prevents overwriting an existing project.
    fs::create_dir(destination)
        .map_err(|e| format!("cannot create {}: {e}", destination.display()))?;
    let result = (|| -> std::io::Result<()> {
        fs::create_dir(destination.join("src"))?;
        fs::create_dir(destination.join("assets"))?;
        let render = |source: &str| {
            source
                .replace("{{name}}", name)
                .replace("{{crate_name}}", &name.replace('-', "_"))
                .replace("{{android_name}}", &name.replace(['-', '_'], ""))
                .replace("{{framework}}", &framework)
                .replace("{{asset_framework}}", &asset_framework)
        };
        for (path, source) in [
            (
                "Cargo.toml",
                include_str!("../templates/Cargo.toml.template"),
            ),
            ("src/lib.rs", include_str!("../templates/lib.rs.template")),
            ("src/main.rs", include_str!("../templates/main.rs.template")),
            ("build.rs", include_str!("../templates/build.rs.template")),
            (
                "assets.json",
                include_str!("../templates/assets.json.template"),
            ),
        ] {
            fs::write(destination.join(path), render(source))?;
        }
        fs::write(
            destination.join("assets/welcome.txt"),
            "Welcome to your Rust application.\n",
        )?;
        fs::write(
            destination.join(".gitignore"),
            "/target/\n*.apk\n*.keystore\n",
        )?;
        fs::write(
            destination.join("README.md"),
            format!(
                "# {name}\n\nRun `rust-flutter run` for desktop or `rust-flutter run --android` for Android.\nRun `rust-flutter doctor` to inspect build prerequisites.\nThe framework source is specified in Cargo.toml; commit Cargo.lock for reproducible builds.\n\nFiles in `assets/` are embedded by `build.rs`; edit `assets.json` to declare additional directories. Numeric density folders such as `2x/` are discovered automatically.\n"
            ),
        )?;
        Ok(())
    })();
    if let Err(error) = result {
        return Err(format!(
            "project creation failed: {error}; partial files remain at {}",
            destination.display()
        ));
    }
    Ok(())
}
fn build_args(command: &str, flags: &[String]) -> Result<Vec<String>, String> {
    let mut android = false;
    let mut release = false;
    let mut target = None;
    let mut device = None;
    let mut flags = flags.iter();
    while let Some(flag) = flags.next() {
        match flag.as_str() {
            "--android" => android = true,
            "--release" => release = true,
            "--target" | "--device" => {
                let value = flags
                    .next()
                    .filter(|v| !v.is_empty() && !v.starts_with('-'))
                    .ok_or_else(|| format!("{flag} requires a value"))?
                    .clone();
                if flag == "--target" {
                    target = Some(value);
                } else {
                    device = Some(value);
                }
            }
            _ => return Err(format!("unknown option: {flag}")),
        }
    }
    if device.is_some() && (!android || command != "run") {
        return Err("--device requires run --android".into());
    }
    let mut args = Vec::new();
    if android {
        args.push("apk".into());
    }
    args.push(command.into());
    if android {
        args.push("--lib".into());
    }
    if release {
        args.push("--release".into());
    }
    if let Some(target) = target {
        args.extend(["--target".into(), target]);
    }
    if let Some(device) = device {
        args.extend(["--device".into(), device]);
    }
    Ok(args)
}

fn doctor(android: bool) -> Result<(), String> {
    let mut missing = Vec::new();
    for (program, args, required) in [
        ("rustc", vec!["--version"], true),
        ("cargo", vec!["--version"], true),
        ("cargo", vec!["apk", "--help"], android),
        ("java", vec!["-version"], android),
    ] {
        match Command::new(program).args(&args).output() {
            Ok(output) if output.status.success() => {
                let bytes = if output.stdout.is_empty() {
                    &output.stderr
                } else {
                    &output.stdout
                };
                println!(
                    "OK: {} {}: {}",
                    program,
                    args.join(" "),
                    String::from_utf8_lossy(bytes).lines().next().unwrap_or("")
                );
            }
            _ => {
                println!(
                    "MISSING: {program} {}{}",
                    args.join(" "),
                    if required {
                        ""
                    } else {
                        " (needed for Android)"
                    }
                );
                if required {
                    missing.push(if program == "cargo" && args.first() == Some(&"apk") {
                        "cargo-apk"
                    } else {
                        program
                    });
                }
            }
        }
    }
    for key in ["ANDROID_HOME", "ANDROID_NDK_HOME"] {
        match env::var_os(key).filter(|p| Path::new(p).is_dir()) {
            Some(path) => println!("OK: {key}={}", PathBuf::from(path).display()),
            None => {
                println!("MISSING: {key} must point to an installed directory for Android builds");
                if android {
                    missing.push(key);
                }
            }
        }
    }
    if android {
        for (key, required) in [
            ("ANDROID_HOME", "platform-tools"),
            ("ANDROID_NDK_HOME", "toolchains/llvm/prebuilt"),
        ] {
            if !env::var_os(key).is_some_and(|path| PathBuf::from(path).join(required).is_dir()) {
                println!("MISSING: {key}/{required}");
                missing.push(required);
            }
        }
        let targets = Command::new("rustup")
            .args(["target", "list", "--installed"])
            .output();
        if !targets.is_ok_and(|o| {
            o.status.success()
                && String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .any(|t| t.ends_with("-linux-android") || t == "armv7-linux-androideabi")
        }) {
            println!("MISSING: Android Rust target (rustup target add aarch64-linux-android)");
            missing.push("Android Rust target");
        }
    }
    if cfg!(target_os = "macos") {
        let status = Command::new("xcode-select").arg("-p").output();
        if !status.is_ok_and(|v| v.status.success()) {
            println!("MISSING: Xcode Command Line Tools");
            missing.push("Xcode tools");
        }
    }
    println!("Doctor checks tools only; device, driver and signing validation are separate.");
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!("required tools missing: {}", missing.join(", ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn android_commands_select_the_requested_architecture_and_device() {
        let flags = [
            "--android",
            "--target",
            "x86_64-linux-android",
            "--device",
            "emulator-5554",
            "--release",
        ]
        .map(str::to_owned);
        assert_eq!(
            build_args("run", &flags).unwrap(),
            [
                "apk",
                "run",
                "--lib",
                "--release",
                "--target",
                "x86_64-linux-android",
                "--device",
                "emulator-5554"
            ]
        );
        assert!(build_args("build", &flags).is_err());
        assert!(build_args("run", &["--target".into()]).is_err());
        assert!(build_args("run", &["--device".into(), "emulator-5554".into()]).is_err());
        assert!(build_args("build", &["--unknown".into()]).is_err());
    }
    #[test]
    fn rejects_invalid_or_reserved_names() {
        for name in [
            "",
            "9app",
            "Bad",
            "a/b",
            "a\"b",
            "crate",
            "type",
            "gen",
            "rust-flutter",
        ] {
            assert!(!valid_name(name), "{name}");
        }
        for name in ["my-app", "my_app", "app123"] {
            assert!(valid_name(name));
        }
    }
    #[test]
    fn scaffold_is_shared_and_does_not_overwrite_existing_files() {
        let directory = env::temp_dir().join(format!(
            "rust-flutter-create-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        create(&directory, "my-app", Some(Path::new("/some/framework"))).unwrap();
        let manifest = fs::read_to_string(directory.join("Cargo.toml")).unwrap();
        assert!(manifest.contains("path = \"/some/framework\""));
        assert!(manifest.contains("dev.rustflutter.myapp"));
        assert!(manifest.contains("fontScale|layoutDirection|locale|uiMode"));
        assert!(
            fs::read_to_string(directory.join("src/main.rs"))
                .unwrap()
                .contains("my_app::App")
        );
        assert!(
            fs::read_to_string(directory.join("src/lib.rs"))
                .unwrap()
                .contains("android_main")
        );
        assert!(!manifest.contains("{{"));
        assert!(manifest.contains("rust-flutter-assets"));
        assert!(directory.join("build.rs").is_file());
        assert!(directory.join("assets/welcome.txt").is_file());
        assert!(create(&directory, "other-app", Some(Path::new("/other"))).is_err());
        assert_eq!(
            fs::read_to_string(directory.join("Cargo.toml")).unwrap(),
            manifest
        );
        fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn escapes_paths_for_toml_without_shell_interpretation() {
        assert_eq!(toml_string("C:\\a\"b\n"), "\"C:\\\\a\\\"b\\n\"");
    }
}

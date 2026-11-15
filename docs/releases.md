# Maintaining releases

The GitHub repository is the source distribution. Packages are not currently
published to crates.io. Install the CLI using the README command. Its generated
apps pin dependencies to the CLI's source commit; existing apps do not silently
advance when main changes. Update the rev fields deliberately and regenerate
Cargo.lock when upgrading an app. Advance its root renderer patch to the same
revision; see renderer-patches.md.

For a reproducible CLI installation, add `--rev <full-commit>` to `cargo install`.
Local contributor builds can generate path dependencies with --framework-path.
Source archives without Git metadata require RUST_FLUTTER_FRAMEWORK_REV set to
an existing full source commit when building the CLI. The fallback version tag
must exist before distributing such a binary.

## Release procedure

1. Review the parity audit and known platform limitations. Keep experimental
   status until the documented guarantees are verified.
2. Update workspace.package.version in Cargo.toml, Cargo.lock and CHANGELOG.md.
   Describe breaking API changes, including during 0.x development.
3. Run the CONTRIBUTING.md checks and generate an app outside the checkout.
   Verify desktop and Android as applicable, including exact RGB fills and native
   surface formats after recreation. Run both renderer-patch verification scripts. Inspect successful GitHub Actions
   runs on Linux, macOS and Android; document runtime gaps separately.
4. Merge the reviewed change and tag its commit as v<version>. Push the tag and
   create a GitHub release describing changes and verification. Do not attach
   private keys or claim unsigned debug APKs are production releases.
5. Verify installation with `cargo install --git
   https://github.com/rishwith2306/rust-flutter --tag v<version> --locked
   rust-flutter-cli`, then create and compile an independent app.

Publishing crates.io packages is a separate future release task: confirm package
ownership and names, include source licenses, add versioned internal dependencies,
and verify package archives before publishing. A GitHub push does not publish
registry packages.

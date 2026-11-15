# Pinned renderer and Android RGB888 presentation

The framework carries a small patch to wgpu-hal 27.0.4 in `vendor/wgpu-hal`.
Android GLES/EGL configuration selection now requests at least eight bits for
red, green, blue and alpha, including drivers without sRGB window-surface support.
The unpatched backend can choose RGB565 even with an Rgba8Unorm render target.
That caused visible channel quantization on the tested API 37 emulator.

The renderer remains wgpu on Android and desktop. This patch changes Android EGL
configuration selection; the other behavior change is an explicit elided lifetime
in one upstream signature to avoid a warning when compiling the source locally.
The vendored package also has its own workspace boundary. No other upstream
source changes are included. Minimum EGL color precision is not an HDR, wide-gamut
or full color-management implementation.

## Application manifests

New CLI projects include a root `[patch.crates-io]` entry automatically, pinned to
the same Git revision as the framework. Commit Cargo.lock. Local contributor
projects select `vendor/wgpu-hal` from the supplied checkout instead.

**Cargo patches are only honored at an application's workspace root.** A patch
in this framework's Cargo.toml does not propagate into a separate consumer. For
an existing/manual Git consumer, add this at the workspace root:

```toml
[dependencies]
rust-flutter = { git = "https://github.com/rishwith2306/rust-flutter", rev = "<framework-commit>" }

[patch.crates-io]
wgpu-hal = { git = "https://github.com/rishwith2306/rust-flutter", rev = "<same-framework-commit>" }
```

For local development use:

```toml
[patch.crates-io]
wgpu-hal = { path = "/path/to/rust-flutter/vendor/wgpu-hal" }
```

Use a real full commit SHA for both Git revisions. After changing the manifest,
run `cargo update -p wgpu-hal` and commit the resulting lockfile. The library also
pins wgpu-hal's exact version so a fresh resolution cannot silently prefer a newer
registry HAL over the reviewed source. Android rejects an RGB565 native window
with an error pointing to this manifest requirement; it does not silently
continue with reduced precision. Check that your app selects the patched source:

```sh
python3 /path/to/rust-flutter/scripts/check-renderer-selection.py \
  --manifest-path /path/to/app/Cargo.toml
```

The command requires an existing lockfile and resolves Cargo metadata with
`--locked`. It checks that exactly one wgpu-hal version is selected and that its
EGL source, manifest and licenses match the reviewed renderer in the checkout.
When upgrading the framework, advance its dependency and renderer patch together.
An already generated app remains pinned until you deliberately upgrade it.

## Origin and maintenance

The published crate archive has SHA-256
`5b21cb61c57ee198bc4aff71aeadff4cbb80b927beb912506af9c780d64313ce`.
Its upstream VCS metadata records gfx-rs/wgpu commit
`af91efa93da2af9366b252ed7b9a089175f4874e`.
The immutable archive URL and original file hashes are recorded in
`patches/wgpu-hal-27.0.4-origin.json`. The exact diff is
`patches/wgpu-hal-27.0.4-android-rgba8.patch`. Upstream copyright headers and both
[MIT](../vendor/wgpu-hal/LICENSE.MIT) and [Apache 2.0](../vendor/wgpu-hal/LICENSE.APACHE)
license texts are retained. Original framework code remains MIT licensed.

```sh
python3 scripts/check-renderer-patch.py
python3 scripts/check-renderer-selection.py
```

The first command reverses the patch in a temporary copy and checks every file
against the recorded original hashes. It also rejects extra/missing files. It
never rewrites the working tree. The second checks the resolved dependency graph.
CI runs both and checks a generated standalone app's selected source.

To update the renderer, download and verify the intended published archive before
extracting it. Review the upstream diff and determine whether the local fix is
still needed. Regenerate the original-file manifest from that verified archive,
retain licenses, and regenerate the patch against the pristine source. Update the
exact dependency pin, root patch, CLI selection verifier and these versioned
artifact paths together. Run ordinary workspace checks, generated-app checks,
Android RGB precision/input and activity-recreation tests before publishing.
Do not modify the vendored source without updating and reviewing the patch.

## Evidence and boundaries

The API 37 / x86_64 / 16 KB-page emulator now reports R8G8B8A8_UNORM instead of
R5G6B5_UNORM. The shared rendered probe samples exact interior RGB values for
blue, green and orange controls. It no longer accepts quantized counterparts.
Normal counter/aspect/observable input and restoration still pass. A separate
instrumentation run exercised four native hosts in the same process, each with
an RGBA8 surface, through three recreations and final activity destruction.
See [verification](verification.md) for evidence and packaging qualification.
Physical-device coverage, wide-gamut/HDR, gradients, alpha/color-space accuracy
and the complete painting/compositing API remain outstanding.

Primary references: Android's [native buffer formats](https://developer.android.com/ndk/reference/group/a-native-window),
and the pinned backend's [EGL configuration code](https://github.com/gfx-rs/wgpu/blob/af91efa93da2af9366b252ed7b9a089175f4874e/wgpu-hal/src/gles/egl.rs).

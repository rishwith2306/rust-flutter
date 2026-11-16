//! Android safe areas and keyboard samples. Java View access stays on the Java UI thread.
use crate::{
    AndroidApp,
    display::{KeyboardVisibility, PlatformInsets},
};
use egui::{SafeAreaInsets, epaint::MarginF32};
use jni::{
    jni_sig, jni_str,
    objects::{JObject, JValue},
    refs::Global,
};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq)]
struct Sample {
    platform: PlatformInsets,
    safe_area: Option<MarginF32>,
}
#[derive(Default)]
struct Snapshot {
    sample: Option<Sample>,
    pending: bool,
}
pub(crate) struct AndroidInsets {
    activity: AndroidApp,
    snapshot: Arc<Mutex<Snapshot>>,
    last_sample: Option<Instant>,
}
impl AndroidInsets {
    pub fn new(activity: AndroidApp) -> Self {
        Self {
            activity,
            snapshot: Arc::default(),
            last_sample: None,
        }
    }
    pub fn update(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        let scale = raw
            .viewports
            .get(&raw.viewport_id)
            .and_then(|v| v.native_pixels_per_point)
            .unwrap_or(1.0)
            * ctx.zoom_factor();
        if scale <= 0.0 || !scale.is_finite() {
            return;
        }
        // Legacy NativeActivity content bounds remain useful on non-edge-to-edge windows.
        if let Some(window) = self.activity.native_window() {
            let r = self.activity.content_rect();
            raw.safe_area_insets = crate::display::safe_area_insets(
                egui::vec2(window.width() as f32, window.height() as f32),
                egui::Rect::from_min_max(
                    egui::pos2(r.left as f32, r.top as f32),
                    egui::pos2(r.right as f32, r.bottom as f32),
                ),
                scale,
            );
        }
        let mut snapshot = self.snapshot.lock().unwrap();
        let sample = snapshot.sample;
        let platform = sample.map(|mut sample| {
            if let Some((size, _, _)) = sample.platform.geometry {
                let actual = self
                    .activity
                    .native_window()
                    .map(|w| egui::vec2(w.width() as f32, w.height() as f32));
                if actual.is_none_or(|actual| (size - actual).abs().max_elem() > 1.0) {
                    sample.platform.geometry = None;
                } else if let Some(insets) = sample.safe_area {
                    raw.safe_area_insets = Some(SafeAreaInsets(insets / scale));
                }
            }
            sample.platform
        });
        crate::display::set_platform_insets(ctx, platform);
        // No Java subclass/listener is required for a NativeActivity APK. Poll at
        // 1 Hz until a listener-backed platform host replaces this integration.
        ctx.request_repaint_after(Duration::from_secs(1));
        if snapshot.pending
            || self
                .last_sample
                .is_some_and(|t| t.elapsed() < Duration::from_millis(500))
        {
            return;
        }
        snapshot.pending = true;
        drop(snapshot);
        self.last_sample = Some(Instant::now());
        let activity = self.activity.clone();
        let cache = self.snapshot.clone();
        let context = ctx.clone();
        self.activity.run_on_java_main_thread(Box::new(move || {
            // SAFETY: AndroidApp supplies the VM pointer, valid for the app lifetime.
            let vm = unsafe { jni::JavaVM::from_raw(activity.vm_as_ptr().cast()) };
            let result: jni::errors::Result<Option<Sample>> = vm.attach_current_thread(|env| {
                let result = read_insets(env, &activity);
                if result.is_err() {
                    env.exception_clear();
                }
                result
            });
            let mut snapshot = cache.lock().unwrap();
            snapshot.pending = false;
            let sample = result.ok().flatten();
            if snapshot.sample != sample {
                snapshot.sample = sample;
                drop(snapshot);
                context.request_repaint();
            }
        }));
    }
}
fn read_insets(env: &mut jni::Env<'_>, app: &AndroidApp) -> jni::errors::Result<Option<Sample>> {
    if AndroidApp::sdk_version() < 30 {
        return Ok(None);
    }
    let raw_activity = app.activity_as_ptr() as jni::sys::jobject;
    // SAFETY: This borrows AndroidApp's global reference without taking ownership;
    // `app` outlives all JNI calls and the reference is never deleted here.
    let activity = unsafe { env.as_cast_raw::<Global<JObject>>(&raw_activity)? };
    let window = env
        .call_method(
            &activity,
            jni_str!("getWindow"),
            jni_sig!("()Landroid/view/Window;"),
            &[],
        )?
        .l()?;
    let decor = env
        .call_method(
            &window,
            jni_str!("getDecorView"),
            jni_sig!("()Landroid/view/View;"),
            &[],
        )?
        .l()?;
    let insets = env
        .call_method(
            &decor,
            jni_str!("getRootWindowInsets"),
            jni_sig!("()Landroid/view/WindowInsets;"),
            &[],
        )?
        .l()?;
    if insets.is_null() {
        return Ok(None);
    }
    let ime = env
        .call_static_method(
            jni_str!("android/view/WindowInsets$Type"),
            jni_str!("ime"),
            jni_sig!("()I"),
            &[],
        )?
        .i()?;
    let visible = env
        .call_method(
            &insets,
            jni_str!("isVisible"),
            jni_sig!("(I)Z"),
            &[JValue::Int(ime)],
        )?
        .z()?;
    let mut sample = Sample {
        platform: PlatformInsets {
            keyboard: if visible {
                KeyboardVisibility::Visible
            } else {
                KeyboardVisibility::Hidden
            },
            geometry: None,
        },
        safe_area: None,
    };
    let info = env
        .call_method(
            &activity,
            jni_str!("getApplicationInfo"),
            jni_sig!("()Landroid/content/pm/ApplicationInfo;"),
            &[],
        )?
        .l()?;
    // Only the enforced edge-to-edge host currently guarantees the native surface
    // shares the decor's coordinate space. Older hosts still report visibility.
    if AndroidApp::sdk_version() < 35
        || env
            .get_field(&info, jni_str!("targetSdkVersion"), jni_sig!("I"))?
            .i()?
            < 35
    {
        return Ok(Some(sample));
    }
    let width = env
        .call_method(&decor, jni_str!("getWidth"), jni_sig!("()I"), &[])?
        .i()?;
    let height = env
        .call_method(&decor, jni_str!("getHeight"), jni_sig!("()I"), &[])?
        .i()?;
    let bars = env
        .call_static_method(
            jni_str!("android/view/WindowInsets$Type"),
            jni_str!("systemBars"),
            jni_sig!("()I"),
            &[],
        )?
        .i()?;
    let cutout = env
        .call_static_method(
            jni_str!("android/view/WindowInsets$Type"),
            jni_str!("displayCutout"),
            jni_sig!("()I"),
            &[],
        )?
        .i()?;
    let safe_area = read_dimensions(env, &insets, bars | cutout, false)?;
    let padding = read_dimensions(env, &insets, bars | cutout, true)?;
    // IME must never be passed to getInsetsIgnoringVisibility: Android rejects it.
    let obscured = read_dimensions(env, &insets, ime, false)?;
    sample.platform.geometry = Some((egui::vec2(width as f32, height as f32), padding, obscured));
    sample.safe_area = Some(safe_area);
    Ok(Some(sample))
}

fn read_dimensions(
    env: &mut jni::Env<'_>,
    insets: &JObject<'_>,
    mask: i32,
    ignoring_visibility: bool,
) -> jni::errors::Result<MarginF32> {
    let method = if ignoring_visibility {
        jni_str!("getInsetsIgnoringVisibility")
    } else {
        jni_str!("getInsets")
    };
    let dimensions = env
        .call_method(
            insets,
            method,
            jni_sig!("(I)Landroid/graphics/Insets;"),
            &[JValue::Int(mask)],
        )?
        .l()?;
    Ok(MarginF32 {
        left: env
            .get_field(&dimensions, jni_str!("left"), jni_sig!("I"))?
            .i()?
            .max(0) as f32,
        top: env
            .get_field(&dimensions, jni_str!("top"), jni_sig!("I"))?
            .i()?
            .max(0) as f32,
        right: env
            .get_field(&dimensions, jni_str!("right"), jni_sig!("I"))?
            .i()?
            .max(0) as f32,
        bottom: env
            .get_field(&dimensions, jni_str!("bottom"), jni_sig!("I"))?
            .i()?
            .max(0) as f32,
    })
}

//! Android edge-to-edge insets. Java View access stays on the Java UI thread.
use crate::AndroidApp;
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

#[derive(Default)]
struct Snapshot {
    insets: Option<MarginF32>,
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
        if let Some(insets) = snapshot.insets {
            raw.safe_area_insets = Some(SafeAreaInsets(MarginF32 {
                left: insets.left / scale,
                top: insets.top / scale,
                right: insets.right / scale,
                bottom: insets.bottom / scale,
            }));
        }
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
            let result: jni::errors::Result<Option<MarginF32>> = vm.attach_current_thread(|env| {
                let result = read_insets(env, &activity);
                if result.is_err() {
                    env.exception_clear();
                }
                result
            });
            let mut snapshot = cache.lock().unwrap();
            snapshot.pending = false;
            if let Ok(Some(insets)) = result
                && snapshot.insets != Some(insets)
            {
                snapshot.insets = Some(insets);
                drop(snapshot);
                context.request_repaint();
            }
        }));
    }
}
fn read_insets(env: &mut jni::Env<'_>, app: &AndroidApp) -> jni::errors::Result<Option<MarginF32>> {
    // Edge-to-edge enforcement is active for target SDK 35+ on Android 35+.
    if AndroidApp::sdk_version() < 35 {
        return Ok(None);
    }
    let raw_activity = app.activity_as_ptr() as jni::sys::jobject;
    // SAFETY: This borrows AndroidApp's global reference without taking ownership;
    // `app` outlives all JNI calls and the reference is never deleted here.
    let activity = unsafe { env.as_cast_raw::<Global<JObject>>(&raw_activity)? };
    let info = env
        .call_method(
            &activity,
            jni_str!("getApplicationInfo"),
            jni_sig!("()Landroid/content/pm/ApplicationInfo;"),
            &[],
        )?
        .l()?;
    if env
        .get_field(&info, jni_str!("targetSdkVersion"), jni_sig!("I"))?
        .i()?
        < 35
    {
        return Ok(None);
    }
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
    let dimensions = env
        .call_method(
            &insets,
            jni_str!("getInsets"),
            jni_sig!("(I)Landroid/graphics/Insets;"),
            &[JValue::Int(bars | cutout)],
        )?
        .l()?;
    Ok(Some(MarginF32 {
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
    }))
}

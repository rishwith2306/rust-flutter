//! NativeActivity host. Each android_main invocation owns its context, GPU and
//! event loop, so a destroyed Activity can be recreated in the same JVM process.
mod accessibility;
mod input;
mod renderer;
use crate::frame_schedule::FrameSchedule;
use crate::{AppEvent, Application, Error, LifecycleEvent, NativeOptions, Runtime};
use accessibility::AndroidAccessibility;
use android_activity::{AndroidApp, MainEvent, PollEvent};
use egui::ViewportId;
use input::Input;
use jni::{
    jni_sig, jni_str,
    objects::{JObject, JValue},
    refs::Global,
};
use renderer::{Renderer, failure};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

static HOST_SERIAL: AtomicU64 = AtomicU64::new(0);

pub(super) fn run<A: Application>(
    mut options: NativeOptions,
    app: A,
    services: crate::services::Services,
) -> Result<(), Error> {
    if options.event_loop_builder.is_some() || options.window_builder.is_some() {
        return Err(failure(
            "winit builder hooks are desktop-only; Android owns a NativeActivity event loop",
        ));
    }
    if options.multisampling > 1 || options.depth_buffer != 0 || options.stencil_buffer != 0 {
        return Err(failure(
            "Android host currently supports the default single-sample, color-only UI surface",
        ));
    }
    let activity = options
        .android_app
        .take()
        .ok_or_else(|| failure("NativeOptions is missing android_app"))?;
    let ctx = egui::Context::default();
    let schedule = Arc::new(Mutex::new(FrameSchedule::default()));
    let wake = activity.create_waker();
    let repaint_wake = wake.clone();
    let requests = schedule.clone();
    ctx.set_request_repaint_callback(move |info| {
        if requests.lock().unwrap().request(Instant::now(), info.delay) {
            repaint_wake.wake();
        }
    });
    let accessibility = AndroidAccessibility::new(&activity, wake).map_err(failure)?;
    let runtime = Runtime::with_services(app, activity.internal_data_path(), services);
    let serial = HOST_SERIAL.fetch_add(1, Ordering::Relaxed) + 1;
    let mut host = Host {
        runtime,
        insets: crate::android_insets::AndroidInsets::new(activity.clone()),
        ctx,
        activity: activity.clone(),
        options,
        gpu: None,
        input: Input::default(),
        accessibility,
        dirty: true,
        schedule,
        resumed: false,
        focused: false,
        destroyed: false,
        closing: false,
        keyboard: false,
        beginning: Instant::now(),
        serial,
        #[cfg(debug_assertions)]
        presented_geometry: None,
    };
    host.trace("created");
    let mut error = None;
    while !host.destroyed && error.is_none() {
        let timeout = if host.can_draw() {
            if host.dirty {
                Some(Duration::ZERO)
            } else {
                host.schedule
                    .lock()
                    .unwrap()
                    .deadline()
                    .map(|at| at.saturating_duration_since(Instant::now()))
            }
        } else {
            None
        };
        activity.poll_events(timeout, |event| {
            if error.is_some() || host.destroyed {
                return;
            }
            let result = match event {
                PollEvent::Main(event) => host.main_event(event),
                PollEvent::Wake | PollEvent::Timeout => Ok(()),
                _ => Ok(()),
            };
            if let Err(e) = result {
                error = Some(e);
            }
        });
        if error.is_none()
            && !host.destroyed
            && host.can_draw()
            && (host.dirty
                || host
                    .schedule
                    .lock()
                    .unwrap()
                    .deadline()
                    .is_some_and(|at| at <= Instant::now()))
            && let Err(e) = host.draw()
        {
            error = Some(e);
        }
    }
    // Release the ANativeWindow before returning to android-activity's destruction
    // handshake. In particular, never retain a winit event loop in thread-local state.
    host.gpu = None;
    host.runtime.shutdown();
    host.ctx.set_request_repaint_callback(|_| {});
    host.trace("returned");
    match error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

struct Host<A: Application> {
    runtime: Runtime<A>,
    insets: crate::android_insets::AndroidInsets,
    ctx: egui::Context,
    activity: AndroidApp,
    options: NativeOptions,
    gpu: Option<Renderer>,
    input: Input,
    accessibility: AndroidAccessibility,
    dirty: bool,
    schedule: Arc<Mutex<FrameSchedule>>,
    resumed: bool,
    focused: bool,
    destroyed: bool,
    closing: bool,
    keyboard: bool,
    beginning: Instant,
    serial: u64,
    #[cfg(debug_assertions)]
    presented_geometry: Option<String>,
}
impl<A: Application> Host<A> {
    fn can_draw(&self) -> bool {
        self.resumed && !self.destroyed && self.gpu.as_ref().is_some_and(Renderer::has_surface)
    }
    fn trace(&self, event: &str) {
        #[cfg(debug_assertions)]
        eprintln!(
            "rust-flutter android host={} pid={} {event}",
            self.serial,
            std::process::id()
        );
        #[cfg(not(debug_assertions))]
        let _ = (event, self.serial);
    }
    fn lifecycle(&mut self, event: LifecycleEvent) {
        self.runtime
            .platform_event(&self.ctx, AppEvent::Lifecycle(event));
    }
    fn main_event(&mut self, event: MainEvent<'_>) -> Result<(), Error> {
        match event {
            MainEvent::Start => self.lifecycle(LifecycleEvent::Started),
            MainEvent::Resume { loader, .. } => {
                self.resumed = true;
                #[cfg(debug_assertions)]
                {
                    self.presented_geometry = None;
                }
                if let Some(state) = loader.load() {
                    self.runtime.restore_state(&self.ctx, &state);
                }
                self.lifecycle(LifecycleEvent::Resumed);
                self.trace("resumed");
                self.dirty = true;
            }
            MainEvent::Pause => {
                self.resumed = false;
                self.input.cancel();
                self.lifecycle(LifecycleEvent::Paused);
                self.trace("paused");
            }
            MainEvent::Stop => self.lifecycle(LifecycleEvent::Stopped),
            MainEvent::InitWindow { .. } => {
                #[cfg(debug_assertions)]
                {
                    self.presented_geometry = None;
                }
                let window = self
                    .activity
                    .native_window()
                    .ok_or_else(|| failure("InitWindow without ANativeWindow"))?;
                if let Some(gpu) = &mut self.gpu {
                    gpu.attach(window)?;
                } else {
                    self.gpu = Some(Renderer::new(window, &self.options)?);
                }
                self.lifecycle(LifecycleEvent::SurfaceCreated);
                self.trace("surface-created");
                self.dirty = true;
            }
            MainEvent::TerminateWindow { .. } => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.detach();
                }
                self.input.cancel();
                self.lifecycle(LifecycleEvent::SurfaceDestroyed);
                self.trace("surface-destroyed");
            }
            MainEvent::Destroy => {
                self.gpu = None;
                self.lifecycle(LifecycleEvent::Destroyed);
                self.runtime.shutdown();
                self.destroyed = true;
                self.trace("destroyed");
            }
            MainEvent::InputAvailable => {
                let activity = self.activity.clone();
                let scale = self.scale();
                let mut events = activity.input_events_iter().map_err(failure)?;
                while events.next(|event| self.input.read(&activity, event, scale)) {}
                self.dirty = true;
            }
            MainEvent::GainedFocus | MainEvent::LostFocus => {
                self.focused = matches!(event, MainEvent::GainedFocus);
                #[cfg(debug_assertions)]
                {
                    self.presented_geometry = None;
                }
                self.trace(if self.focused {
                    "focus-gained"
                } else {
                    "focus-lost"
                });
                if !self.focused {
                    self.input.cancel();
                }
                self.runtime.focus_changed(&self.ctx, self.focused);
                self.dirty = true;
            }
            MainEvent::WindowResized { .. }
            | MainEvent::ContentRectChanged { .. }
            | MainEvent::InsetsChanged { .. } => {
                self.dirty = true;
            }
            MainEvent::ConfigChanged { .. } => {
                self.input.cancel();
                self.lifecycle(LifecycleEvent::ConfigurationChanged);
                self.dirty = true;
            }
            MainEvent::RedrawNeeded { .. } => {
                self.draw()?;
            }
            MainEvent::LowMemory => self.lifecycle(LifecycleEvent::LowMemory),
            MainEvent::SaveState { saver, .. } => {
                self.lifecycle(LifecycleEvent::SaveStateRequested);
                if let Some(state) = self.runtime.save_state(&self.ctx) {
                    saver.store(&state);
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn density(&self) -> f32 {
        self.activity
            .config()
            .density()
            .filter(|d| *d > 0)
            .map_or(1.0, |d| d as f32 / 160.0)
    }
    fn scale(&self) -> f32 {
        self.density() * self.ctx.zoom_factor()
    }
    fn draw(&mut self) -> Result<(), Error> {
        if !self.can_draw() {
            return Ok(());
        }
        let Some(window) = self.activity.native_window() else {
            return Ok(());
        };
        let size = [window.width(), window.height()];
        if size[0] <= 0 || size[1] <= 0 {
            return Ok(());
        }
        let scale = self.scale();
        let density = self.density();
        let mut raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(size[0] as f32 / scale, size[1] as f32 / scale),
            )),
            time: Some(self.beginning.elapsed().as_secs_f64()),
            focused: self.focused,
            modifiers: self.input.modifiers,
            events: std::mem::take(&mut self.input.events),
            ..Default::default()
        };
        raw.system_theme = match self.activity.config().ui_mode_night() {
            android_activity::ndk::configuration::UiModeNight::Yes => Some(egui::Theme::Dark),
            android_activity::ndk::configuration::UiModeNight::No => Some(egui::Theme::Light),
            _ => None,
        };
        raw.viewports
            .entry(ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(density);
        raw.events.extend(self.accessibility.take_events());
        self.dirty = false;
        self.schedule.lock().unwrap().clear();
        // Clear the request consumed by this frame before platform hooks request
        // future frames. Otherwise a delayed insets refresh would be discarded.
        self.insets.update(&self.ctx, &mut raw);
        #[cfg(debug_assertions)]
        let mut geometry = format!(
            "{}x{} scale={scale} safe_top_px={} focused={}",
            size[0],
            size[1],
            raw.safe_area_insets.unwrap_or_default().0.top * scale,
            self.focused
        );
        let mut output = self.ctx.run(raw, |ctx| {
            self.runtime.frame(ctx);
            #[cfg(debug_assertions)]
            {
                // Exercise the same public API an application's build reads.
                let metrics = crate::display::view_metrics(ctx);
                let occlusion = metrics.occlusion;
                geometry.push_str(&format!(
                    " keyboard={:?} view_padding_bottom_px={} view_insets_bottom_px={} padding_bottom_px={}",
                    metrics.keyboard,
                    occlusion.map_or(-1.0, |o| o.view_padding.bottom * scale),
                    occlusion.map_or(-1.0, |o| o.view_insets.bottom * scale),
                    occlusion.map_or(-1.0, |o| o.padding.bottom * scale),
                ));
            }
        });

        if let Some(update) = output.platform_output.accesskit_update.take() {
            self.accessibility.update(update);
        }

        for command in &output.platform_output.commands {
            match command {
                egui::OutputCommand::CopyText(text) => {
                    copy_text_to_clipboard(self.activity.clone(), text.clone());
                }
                egui::OutputCommand::OpenUrl(url) => {
                    open_url(self.activity.clone(), url.url.clone());
                }
                egui::OutputCommand::CopyImage(_) => {}
            }
        }

        if let Some(viewport) = output.viewport_output.get(&ViewportId::ROOT) {
            self.schedule
                .lock()
                .unwrap()
                .request(Instant::now(), viewport.repaint_delay);
            let close = viewport.commands.contains(&egui::ViewportCommand::Close);
            let cancelled = viewport
                .commands
                .contains(&egui::ViewportCommand::CancelClose);
            if close && !cancelled && !self.closing {
                self.closing = true;
                request_finish(&self.activity);
            }
        }
        let keyboard = output.platform_output.ime.is_some();
        if keyboard != self.keyboard {
            self.keyboard = keyboard;
            if keyboard {
                self.activity.show_soft_input(false);
            } else {
                self.activity.hide_soft_input(false);
            }
        }
        if let Some(gpu) = &mut self.gpu {
            gpu.resize(size[0] as u32, size[1] as u32);
            if !gpu.draw(&self.ctx, &output)? {
                self.schedule
                    .lock()
                    .unwrap()
                    .request(Instant::now(), Duration::from_millis(16));
            } else {
                #[cfg(debug_assertions)]
                if self.presented_geometry.as_ref() != Some(&geometry) {
                    self.trace(&format!("presented {geometry}"));
                    self.presented_geometry = Some(geometry);
                }
            }
        }
        Ok(())
    }
}

fn copy_text_to_clipboard(activity: AndroidApp, text: String) {
    let callback_activity = activity.clone();
    activity.run_on_java_main_thread(Box::new(move || {
        // SAFETY: AndroidApp owns a live JVM handle through this queued callback.
        let vm = unsafe { jni::JavaVM::from_raw(callback_activity.vm_as_ptr().cast()) };
        let result = vm.attach_current_thread(|env| -> jni::errors::Result<()> {
            let raw_activity = callback_activity.activity_as_ptr() as jni::sys::jobject;
            // SAFETY: Borrow the global reference owned by AndroidApp; do not delete it.
            let activity = unsafe { env.as_cast_raw::<Global<JObject>>(&raw_activity)? };
            let service_name = env.new_string("clipboard")?;
            let clipboard = env
                .call_method(
                    &activity,
                    jni_str!("getSystemService"),
                    jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
                    &[JValue::Object(&service_name)],
                )?
                .l()?;
            let label = env.new_string("rust-flutter")?;
            let content = env.new_string(text)?;
            let clip_data_class = env.find_class(jni_str!("android/content/ClipData"))?;
            let clip = env
                .call_static_method(
                    &clip_data_class,
                    jni_str!("newPlainText"),
                    jni_sig!("(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;"),
                    &[JValue::Object(&label), JValue::Object(&content)],
                )?
                .l()?;
            env.call_method(
                &clipboard,
                jni_str!("setPrimaryClip"),
                jni_sig!("(Landroid/content/ClipData;)V"),
                &[JValue::Object(&clip)],
            )?;
            Ok(())
        });
        if let Err(error) = result {
            eprintln!("rust-flutter Android clipboard write failed: {error}");
            let _ = vm.attach_current_thread(|env| -> jni::errors::Result<()> {
                env.exception_clear();
                Ok(())
            });
        }
    }));
}

fn open_url(activity: AndroidApp, url: String) {
    let callback_activity = activity.clone();
    activity.run_on_java_main_thread(Box::new(move || {
        // SAFETY: AndroidApp owns a live JVM handle through this queued callback.
        let vm = unsafe { jni::JavaVM::from_raw(callback_activity.vm_as_ptr().cast()) };
        let result = vm.attach_current_thread(|env| -> jni::errors::Result<()> {
            let raw_activity = callback_activity.activity_as_ptr() as jni::sys::jobject;
            // SAFETY: Borrow the global reference owned by AndroidApp; do not delete it.
            let activity = unsafe { env.as_cast_raw::<Global<JObject>>(&raw_activity)? };
            let action = env.new_string("android.intent.action.VIEW")?;
            let intent_class = env.find_class(jni_str!("android/content/Intent"))?;
            let intent = env.new_object(
                &intent_class,
                jni_sig!("(Ljava/lang/String;)V"),
                &[JValue::Object(&action)],
            )?;
            let address = env.new_string(url)?;
            let uri_class = env.find_class(jni_str!("android/net/Uri"))?;
            let uri = env
                .call_static_method(
                    &uri_class,
                    jni_str!("parse"),
                    jni_sig!("(Ljava/lang/String;)Landroid/net/Uri;"),
                    &[JValue::Object(&address)],
                )?
                .l()?;
            env.call_method(
                &intent,
                jni_str!("setData"),
                jni_sig!("(Landroid/net/Uri;)Landroid/content/Intent;"),
                &[JValue::Object(&uri)],
            )?;
            env.call_method(
                &activity,
                jni_str!("startActivity"),
                jni_sig!("(Landroid/content/Intent;)V"),
                &[JValue::Object(&intent)],
            )?;
            Ok(())
        });
        if let Err(error) = result {
            eprintln!("rust-flutter Android URL open failed: {error}");
            let _ = vm.attach_current_thread(|env| -> jni::errors::Result<()> {
                env.exception_clear();
                Ok(())
            });
        }
    }));
}
fn request_finish(activity: &AndroidApp) {
    let app = activity.clone();
    activity.run_on_java_main_thread(Box::new(move || {
        use jni::{jni_sig, jni_str, objects::JObject, refs::Global};
        // SAFETY: AndroidApp owns the VM reference and the unowned Activity global
        // reference, and is retained until this UI-thread callback returns.
        let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) };
        let result: jni::errors::Result<()> = vm.attach_current_thread(|env| {
            let raw = app.activity_as_ptr() as jni::sys::jobject;
            let activity = unsafe { env.as_cast_raw::<Global<JObject>>(&raw)? };
            let result = env.call_method(&activity, jni_str!("finish"), jni_sig!("()V"), &[]);
            if result.is_err() {
                env.exception_clear();
            }
            result.map(|_| ())
        });
        if let Err(error) = result {
            eprintln!("Could not finish Android activity: {error}");
        }
    }));
}

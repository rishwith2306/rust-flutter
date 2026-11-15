//! NativeActivity host. Each android_main invocation owns its context, GPU and
//! event loop, so a destroyed Activity can be recreated in the same JVM process.
mod input;
mod renderer;
use crate::frame_schedule::FrameSchedule;
use crate::{AppEvent, Application, Error, LifecycleEvent, NativeOptions, Runtime};
use android_activity::{AndroidApp, MainEvent, PollEvent};
use egui::ViewportId;
use input::Input;
use renderer::{Renderer, failure};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

static HOST_SERIAL: AtomicU64 = AtomicU64::new(0);

pub(super) fn run<A: Application>(mut options: NativeOptions, app: A) -> Result<(), Error> {
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
    let requests = schedule.clone();
    ctx.set_request_repaint_callback(move |info| {
        if requests.lock().unwrap().request(Instant::now(), info.delay) {
            wake.wake();
        }
    });
    let runtime = Runtime::new(app, activity.internal_data_path());
    let serial = HOST_SERIAL.fetch_add(1, Ordering::Relaxed) + 1;
    let mut host = Host {
        runtime,
        insets: crate::android_insets::AndroidInsets::new(activity.clone()),
        ctx,
        activity: activity.clone(),
        options,
        gpu: None,
        input: Input::default(),
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
            MainEvent::Resume { .. } => {
                self.resumed = true;
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
            MainEvent::SaveState { .. } => self.lifecycle(LifecycleEvent::SaveStateRequested),
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
        raw.viewports
            .entry(ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(density);
        self.dirty = false;
        self.schedule.lock().unwrap().clear();
        // Clear the request consumed by this frame before platform hooks request
        // future frames. Otherwise a delayed insets refresh would be discarded.
        self.insets.update(&self.ctx, &mut raw);
        #[cfg(debug_assertions)]
        let geometry = format!(
            "{}x{} scale={scale} safe_top_px={}",
            size[0],
            size[1],
            raw.safe_area_insets.unwrap_or_default().0.top * scale
        );
        let output = self.ctx.run(raw, |ctx| self.runtime.frame(ctx));

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

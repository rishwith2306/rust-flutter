use rust_flutter::{AppContext, AppEvent, Application, EventResponse, Runtime, egui};

#[derive(Default)]
struct Trace {
    starts: usize,
    frames: usize,
    stops: usize,
    events: Vec<AppEvent>,
    path: Option<std::path::PathBuf>,
}
impl Application for Trace {
    fn initialize(&mut self, context: &AppContext) {
        self.starts += 1;
        self.path = context.data_dir.clone();
    }
    fn on_event(&mut self, _: &egui::Context, event: AppEvent) -> EventResponse {
        self.events.push(event);
        match event {
            AppEvent::BackRequested | AppEvent::CloseRequested => EventResponse::Consumed,
            _ => EventResponse::Continue,
        }
    }
    fn build(&mut self, ui: &mut egui::Ui) {
        self.frames += 1;
        ui.label("Runtime test");
    }
    fn shutdown(&mut self) {
        self.stops += 1;
    }
}
#[test]
fn runtime_initializes_once_reports_focus_changes_and_stops_once() {
    let ctx = egui::Context::default();
    let path = std::path::PathBuf::from("/test/private-app-data");
    let mut runtime = Runtime::new(Trace::default(), Some(path.clone()));
    for focused in [true, true, false, false, true] {
        let _ = ctx.run(
            egui::RawInput {
                focused,
                ..Default::default()
            },
            |ctx| runtime.frame(ctx),
        );
    }
    assert_eq!(runtime.app().starts, 1);
    assert_eq!(runtime.app().path, Some(path));
    assert_eq!(
        runtime.app().events,
        [
            AppEvent::FocusChanged(true),
            AppEvent::FocusChanged(false),
            AppEvent::FocusChanged(true)
        ]
    );
    let frames = runtime.app().frames;
    runtime.shutdown();
    runtime.shutdown();
    let _ = ctx.run(Default::default(), |ctx| runtime.frame(ctx));
    assert_eq!(runtime.app().stops, 1);
    assert_eq!(runtime.app().frames, frames);
}
#[test]
fn runtime_dispatches_android_back_key_and_can_veto_window_close() {
    let ctx = egui::Context::default();
    let mut runtime = Runtime::new(Trace::default(), None);
    let mut raw = egui::RawInput {
        events: vec![egui::Event::Key {
            key: egui::Key::BrowserBack,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
        ..Default::default()
    };
    raw.viewports
        .get_mut(&egui::ViewportId::ROOT)
        .unwrap()
        .events
        .push(egui::ViewportEvent::Close);
    let output = ctx.run(raw, |ctx| runtime.frame(ctx));
    assert_eq!(
        runtime
            .app()
            .events
            .iter()
            .filter(|&&e| e == AppEvent::BackRequested)
            .count(),
        1
    );
    assert_eq!(
        runtime
            .app()
            .events
            .iter()
            .filter(|&&e| e == AppEvent::CloseRequested)
            .count(),
        1
    );
    assert!(
        output.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .contains(&egui::ViewportCommand::CancelClose)
    );
}

#[test]
fn shutdown_before_first_frame_does_not_initialize_the_app() {
    let ctx = egui::Context::default();
    let mut runtime = Runtime::new(Trace::default(), None);
    runtime.shutdown();
    let _ = ctx.run(Default::default(), |ctx| runtime.frame(ctx));
    assert_eq!(runtime.app().starts, 0);
    assert_eq!(runtime.app().frames, 0);
    assert_eq!(runtime.app().stops, 0);
}

#[test]
fn native_events_work_without_a_surface_and_destroy_is_terminal() {
    use rust_flutter::LifecycleEvent as L;
    let ctx = egui::Context::default();
    let mut runtime = Runtime::new(Trace::default(), None);
    for event in [
        L::Started,
        L::Resumed,
        L::SurfaceCreated,
        L::SurfaceDestroyed,
        L::Paused,
        L::Stopped,
    ] {
        runtime.platform_event(&ctx, AppEvent::Lifecycle(event));
    }
    assert_eq!(runtime.app().starts, 1);
    assert_eq!(
        runtime.app().frames,
        0,
        "native events must not need a renderable window"
    );
    assert_eq!(
        runtime.app().stops,
        0,
        "surface loss and stop must allow resume"
    );
    runtime.platform_event(&ctx, AppEvent::Lifecycle(L::Resumed));
    let _ = ctx.run(Default::default(), |ctx| runtime.frame(ctx));
    assert!(runtime.app().frames > 0);
    runtime.platform_event(&ctx, AppEvent::Lifecycle(L::Destroyed));
    let frames = runtime.app().frames;
    let events = runtime.app().events.clone();
    runtime.platform_event(&ctx, AppEvent::Lifecycle(L::Resumed));
    let _ = ctx.run(Default::default(), |ctx| runtime.frame(ctx));
    runtime.shutdown();
    assert_eq!(runtime.app().starts, 1);
    assert_eq!(runtime.app().frames, frames);
    assert_eq!(runtime.app().events, events);
    assert_eq!(runtime.app().stops, 1);
}

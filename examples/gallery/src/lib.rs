use rust_flutter::{
    assets::{AssetBundle, AssetImages, EmbeddedBundle},
    layout::{
        self, Alignment, Axis, BoxConstraints, BoxNode, CrossAxisAlignment, Flex, FlexChild,
        Insets, Positioned, Stack, StackChild, StackFit,
    },
    localization::{FluentArgs, Localizations},
    prelude::*,
    state::{TaskState, spawn_task},
    storage::JsonStore,
    widgets::Theme,
};
use serde::{Deserialize, Serialize};
mod components;
mod forms;
mod lazy_lists;

// Use the same explicit pixel profile on desktop and Android. Dithering is
// useful for gradients, but introduces noise into exact-color gallery probes.
pub fn native_options() -> rust_flutter::NativeOptions {
    let backend = if cfg!(target_os = "android") {
        rust_flutter::GraphicsBackend::Gles
    } else {
        rust_flutter::GraphicsBackend::Auto
    };
    let mut options = rust_flutter::native_options(backend);
    options.dithering = false;
    println!(
        "rust-flutter gallery renderer dithering={}",
        options.dithering
    );
    options
}

static ASSETS: EmbeddedBundle =
    EmbeddedBundle::new(include!(concat!(env!("OUT_DIR"), "/assets.rs")));

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Route {
    #[default]
    Home,
    Settings,
    Assets,
    Layout,
    Components,
    LazyList,
    Form,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Snapshot {
    count: i32,
    name: String,
    dark: bool,
    language: String,
    #[serde(default)]
    route: Route,
}
pub struct Gallery {
    count: i32,
    name: String,
    dark: bool,
    language: String,
    l10n: Localizations,
    navigator: Navigator<Route>,
    task: Option<Signal<TaskState<String, String>>>,
    store: Option<JsonStore>,
    saved: Option<Snapshot>,
    storage_error: Option<String>,
    images: AssetImages<&'static EmbeddedBundle>,
    layout_demo: BoxNode,
    component_demo: Option<components::ComponentDemo>,
    form_demo: Option<forms::FormDemo>,
    lazy_list_demo: Option<lazy_lists::LazyListDemo>,
}
impl Default for Gallery {
    fn default() -> Self {
        let mut l10n = Localizations::new("en", include_str!("../locales/en.ftl"))
            .expect("valid English resource");
        l10n.add_locale("es", include_str!("../locales/es.ftl"))
            .expect("valid Spanish resource");
        Self {
            count: 0,
            name: String::new(),
            dark: true,
            language: "en".into(),
            l10n,
            navigator: Navigator::new(Route::Home),
            task: None,
            store: None,
            saved: None,
            storage_error: None,
            images: AssetImages::new(&ASSETS),
            layout_demo: layout_demo(),
            component_demo: None,
            form_demo: None,
            lazy_list_demo: None,
        }
    }
}
impl Gallery {
    fn tr(&self, id: &str) -> String {
        self.l10n
            .text(id)
            .unwrap_or_else(|e| format!("Translation error: {e}"))
    }
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            count: self.count,
            name: self.name.clone(),
            dark: self.dark,
            language: self.language.clone(),
            route: *self.navigator.current(),
        }
    }
    fn persist(&mut self) {
        let snapshot = self.snapshot();
        if self.saved.as_ref() == Some(&snapshot) {
            return;
        }
        if let Some(store) = &self.store {
            match store.save(&snapshot) {
                Ok(()) => {
                    self.saved = Some(snapshot);
                    self.storage_error = None;
                }
                Err(error) => self.storage_error = Some(error.to_string()),
            }
        }
    }
}
impl Application for Gallery {
    fn initialize(&mut self, context: &AppContext) {
        let Some(path) = &context.data_dir else {
            return;
        };
        let store = JsonStore::new(path.join("gallery.json"), 1);
        match store.load::<Snapshot>() {
            Ok(value) => {
                if let Some(snapshot) = value {
                    self.count = snapshot.count;
                    self.name = snapshot.name;
                    self.dark = snapshot.dark;
                    self.language = snapshot.language;
                    if snapshot.route != Route::Home {
                        self.navigator.push(snapshot.route);
                    }
                    let _ = self.l10n.select(&[&self.language]);
                    self.saved = Some(self.snapshot());
                }
                self.store = Some(store);
            }
            // Preserve unreadable/incompatible snapshots instead of replacing them with defaults.
            Err(error) => {
                self.storage_error = Some(format!(
                    "Restoration failed; original file preserved: {error}"
                ))
            }
        }
    }
    fn on_event(&mut self, _: &egui::Context, event: AppEvent) -> EventResponse {
        match event {
            AppEvent::BackRequested if self.navigator.can_pop() => {
                self.navigator.pop();
                EventResponse::Consumed
            }
            AppEvent::FocusChanged(false)
            | AppEvent::Lifecycle(LifecycleEvent::Paused | LifecycleEvent::SaveStateRequested) => {
                self.persist();
                EventResponse::Continue
            }
            _ => EventResponse::Continue,
        }
    }
    fn shutdown(&mut self) {
        self.component_demo.take();
        self.persist();
    }
    fn configure(&mut self, ctx: &egui::Context) {
        Theme {
            dark: self.dark,
            ..Theme::default()
        }
        .apply(ctx);
    }
    fn build(&mut self, ui: &mut Ui) {
        heading(ui, self.tr("title"));
        text(
            ui,
            format!(
                "Responsive layout: {:?}",
                Breakpoint::for_width(ui.available_width())
            ),
        );
        if let Some(error) = &self.storage_error {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
        ui.separator();
        if *self.navigator.current() != Route::Components
            && let Some(demo) = &self.component_demo
        {
            demo.cancel_input();
        }
        if *self.navigator.current() != Route::LazyList
            && let Some(demo) = &self.lazy_list_demo
        {
            demo.cancel_input();
        }
        match *self.navigator.current() {
            Route::Home => {
                scroll(ui, "home", |ui| {
                    card(ui, |ui| {
                        heading(ui, self.tr("intro"));
                        text(ui, self.tr("subtitle"));
                        let hint = self.tr("name-hint");
                        text_field(ui, "name", &mut self.name, &hint);
                        if !self.name.is_empty() {
                            let mut args = FluentArgs::new();
                            args.set("name", self.name.as_str());
                            text(ui, self.l10n.format("hello", Some(&args)).unwrap());
                        }
                        row(ui, |ui| {
                            if button(ui, "−").clicked() {
                                self.count = self.count.saturating_sub(1);
                            }
                            let mut args = FluentArgs::new();
                            args.set("n", self.count);
                            text(ui, self.l10n.format("count", Some(&args)).unwrap());
                            if button(ui, "+").clicked() {
                                self.count = self.count.saturating_add(1);
                            }
                        });
                        if button(ui, self.tr("settings")).clicked() {
                            self.navigator.push(Route::Settings);
                        }
                        if button(ui, self.tr("assets")).clicked() {
                            self.navigator.push(Route::Assets);
                        }
                        if button(ui, self.tr("layout")).clicked() {
                            self.navigator.push(Route::Layout);
                        }
                        if button(ui, self.tr("components")).clicked() {
                            self.navigator.push(Route::Components);
                        }
                        if ui
                            .add(
                                egui::Button::new(self.tr("lazy-list"))
                                    .fill(egui::Color32::from_rgb(66, 101, 139)),
                            )
                            .clicked()
                        {
                            self.navigator.push(Route::LazyList);
                        }
                        if ui
                            .add(
                                egui::Button::new(self.tr("forms"))
                                    .fill(egui::Color32::from_rgb(107, 91, 149)),
                            )
                            .clicked()
                        {
                            self.navigator.push(Route::Form);
                        }
                    });
                    card(ui, |ui| {
                        heading(ui, self.tr("background"));
                        if button(ui, self.tr("run-task")).clicked() {
                            let result = self.tr("worker-result");
                            self.task = Some(spawn_task(ui.ctx(), move || Ok(result)));
                        }
                        if let Some(task) = &self.task {
                            match task.get() {
                                TaskState::Pending => {
                                    ui.spinner();
                                }
                                TaskState::Ready(Ok(value)) => {
                                    text(ui, value);
                                }
                                TaskState::Ready(Err(error)) => {
                                    text(ui, error);
                                }
                                TaskState::Panicked => {
                                    text(ui, self.tr("worker-panic"));
                                }
                            }
                        }
                    });
                });
            }
            Route::Form => {
                heading(ui, self.tr("forms"));
                self.form_demo
                    .get_or_insert_with(|| forms::FormDemo::new(ui.ctx()))
                    .show(ui);
                if button(ui, self.tr("back")).clicked() {
                    self.navigator.pop();
                }
            }
            Route::LazyList => {
                heading(ui, self.tr("lazy-list"));
                self.lazy_list_demo
                    .get_or_insert_with(|| lazy_lists::LazyListDemo::new(ui.ctx()))
                    .show(ui);
                if button(ui, self.tr("back")).clicked() {
                    self.navigator.pop();
                }
            }
            Route::Components => {
                heading(ui, self.tr("components"));
                self.component_demo
                    .get_or_insert_with(|| components::ComponentDemo::new(ui.ctx()))
                    .show(ui);
                if button(ui, self.tr("back")).clicked() {
                    self.navigator.pop();
                }
            }
            Route::Layout => {
                heading(ui, self.tr("layout"));
                text(ui, "60 pt fixed · flex 1 · flex 2 · positioned overlay");
                let constraints = BoxConstraints::tight(egui::vec2(ui.available_width(), 180.0));
                match constraints.and_then(|c| self.layout_demo.layout(c)) {
                    Ok(geometry) => {
                        layout::show_layout(ui, &geometry, |key, ui| {
                            let rect = ui.max_rect();
                            if key == egui::Id::new("demo-fixed") {
                                if ui
                                    .put(rect, egui::Button::new(format!("Tap\n{}", self.count)))
                                    .clicked()
                                {
                                    self.count = self.count.saturating_add(1);
                                }
                            } else {
                                let (color, label) = if key == egui::Id::new("demo-one") {
                                    (egui::Color32::from_rgb(23, 105, 170), "1")
                                } else if key == egui::Id::new("demo-two") {
                                    (egui::Color32::from_rgb(25, 125, 85), "2")
                                } else {
                                    (egui::Color32::from_rgb(220, 150, 30), "top")
                                };
                                ui.painter().rect_filled(rect, 4.0, color);
                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    label,
                                    egui::FontId::proportional(18.0),
                                    egui::Color32::WHITE,
                                );
                            }
                        });
                    }
                    Err(error) => {
                        ui.colored_label(egui::Color32::LIGHT_RED, error.to_string());
                    }
                }
                text(
                    ui,
                    "Resize the window: the flexible widths keep their 1:2 ratio.",
                );
                if button(ui, self.tr("back")).clicked() {
                    self.navigator.pop();
                }
            }
            Route::Assets => {
                heading(ui, self.tr("assets"));
                match ASSETS.load_string("assets/message.txt") {
                    Ok(message) => {
                        text(ui, message);
                    }
                    Err(error) => {
                        text(ui, error.to_string());
                    }
                }
                match self.images.load(ui.ctx(), "assets/badge.png") {
                    Ok(image) => {
                        image.show(ui);
                        text(
                            ui,
                            format!("{}× image, displayed at 48 logical points", image.scale),
                        );
                    }
                    Err(error) => {
                        text(ui, error.to_string());
                    }
                }
                if button(ui, self.tr("back")).clicked() {
                    self.navigator.pop();
                }
            }
            Route::Settings => {
                heading(ui, self.tr("settings"));
                let dark_label = self.tr("dark-theme");
                ui.checkbox(&mut self.dark, dark_label);
                text(ui, self.tr("language"));
                row(ui, |ui| {
                    for (code, label) in [("en", "English"), ("es", "Español")] {
                        if ui.selectable_label(self.language == code, label).clicked() {
                            self.language = code.into();
                            self.l10n.select(&[code]).unwrap();
                        }
                    }
                });
                text(ui, self.tr("retained"));
                if button(ui, self.tr("back")).clicked() {
                    self.navigator.pop();
                }
            }
        }
        self.persist();
    }
}

fn layout_demo() -> BoxNode {
    let mut stack = Stack::new(vec![
        StackChild::new(BoxNode::leaf("demo-one", egui::Vec2::ZERO)),
        StackChild::positioned(
            BoxNode::leaf("demo-overlay", egui::Vec2::ZERO),
            Positioned {
                right: Some(4.0),
                bottom: Some(4.0),
                width: Some(48.0),
                height: Some(32.0),
                ..Default::default()
            },
        ),
    ]);
    stack.alignment = Alignment::CENTER;
    stack.fit = StackFit::Expand;
    let mut flex = Flex::new(
        Axis::Horizontal,
        vec![
            FlexChild::fixed(BoxNode::leaf("demo-fixed", egui::vec2(60.0, 40.0))),
            FlexChild::expanded(BoxNode::stack("demo-stack", stack), 1),
            FlexChild::expanded(BoxNode::leaf("demo-two", egui::Vec2::ZERO), 2),
        ],
    );
    flex.spacing = 8.0;
    flex.cross_alignment = CrossAxisAlignment::Stretch;
    BoxNode::padding(
        "demo-root",
        Insets::all(8.0),
        BoxNode::flex("demo-flex", flex),
    )
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: rust_flutter::AndroidApp) {
    rust_flutter::run_with_options(
        "Rust Flutter Gallery",
        rust_flutter::NativeOptions {
            android_app: Some(app),
            ..native_options()
        },
        Gallery::default(),
    )
    .expect("Android runner failed");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gallery_restores_preferences_and_handles_system_back_without_losing_model() {
        let dir = tempfile::tempdir().unwrap();
        let context = AppContext {
            egui: egui::Context::default(),
            data_dir: Some(dir.path().to_owned()),
        };
        let mut first = Gallery::default();
        first.initialize(&context);
        first.count = 22;
        first.name = "Ada".into();
        first.dark = false;
        first.language = "es".into();
        first.persist();
        let mut restored = Gallery::default();
        restored.initialize(&context);
        assert_eq!(restored.snapshot(), first.snapshot());
        assert_eq!(restored.tr("settings"), "Ajustes");
        restored.navigator.push(Route::Settings);
        assert_eq!(
            restored.on_event(&context.egui, AppEvent::BackRequested),
            EventResponse::Consumed
        );
        assert!(matches!(restored.navigator.current(), Route::Home));
        assert_eq!(restored.count, 22);
        assert_eq!(
            restored.on_event(&context.egui, AppEvent::BackRequested),
            EventResponse::Continue
        );
    }
    #[test]
    fn gallery_preserves_corrupt_snapshots() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("gallery.json"), "{broken}").unwrap();
        let context = AppContext {
            egui: egui::Context::default(),
            data_dir: Some(dir.path().to_owned()),
        };
        let mut app = Gallery::default();
        app.initialize(&context);
        app.persist();
        assert!(app.storage_error.is_some());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("gallery.json")).unwrap(),
            "{broken}"
        );
    }
}

#[cfg(test)]
mod route_restoration_tests {
    use super::*;
    #[test]
    fn settings_route_restores_with_a_home_route_underneath() {
        let dir = tempfile::tempdir().unwrap();
        let context = AppContext {
            egui: egui::Context::default(),
            data_dir: Some(dir.path().to_owned()),
        };
        let mut first = Gallery::default();
        first.initialize(&context);
        first.navigator.push(Route::Settings);
        first.persist();
        let mut restored = Gallery::default();
        restored.initialize(&context);
        assert!(matches!(restored.navigator.current(), Route::Settings));
        assert_eq!(
            restored.on_event(&context.egui, AppEvent::BackRequested),
            EventResponse::Consumed
        );
        assert!(matches!(restored.navigator.current(), Route::Home));
        restored.persist();
        let mut again = Gallery::default();
        again.initialize(&context);
        assert!(matches!(again.navigator.current(), Route::Home));
        assert!(!again.navigator.can_pop());
    }
}

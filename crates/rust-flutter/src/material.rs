//! Initial Material color and inherited theme foundations.
//!
//! This module provides semantic colors and a subtree theme. It is not yet a
//! Material component catalog or a complete Material 3 color-role inventory.
use crate::{
    egui::{self, Color32},
    element::{BuildContext, DependencyError, Widget},
    navigation::NavigatorController,
};

mod button;
mod checkbox;
mod list_tile;
mod navigation_bar;
mod navigation_drawer;
mod radio;
mod scaffold;
mod slider;
mod switch;
mod text_field;
pub use button::{FilledButton, OutlinedButton, TextButton};
pub use checkbox::Checkbox;
pub use list_tile::ListTile;
pub use navigation_bar::{NavigationBar, NavigationDestination};
pub use navigation_drawer::{NavigationDrawer, NavigationDrawerDestination};
pub use radio::Radio;
pub use scaffold::{AppBar, Drawer, Scaffold};
pub use slider::Slider;
pub(crate) use slider::SliderColors;
pub use switch::Switch;
pub use text_field::TextField;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Brightness {
    Light,
    Dark,
}

/// Core semantic roles used by the first Material controls and surfaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColorScheme {
    pub brightness: Brightness,
    pub primary: Color32,
    pub on_primary: Color32,
    pub primary_container: Color32,
    pub on_primary_container: Color32,
    pub secondary: Color32,
    pub on_secondary: Color32,
    pub secondary_container: Color32,
    pub on_secondary_container: Color32,
    pub tertiary: Color32,
    pub on_tertiary: Color32,
    pub tertiary_container: Color32,
    pub on_tertiary_container: Color32,
    pub error: Color32,
    pub on_error: Color32,
    pub error_container: Color32,
    pub on_error_container: Color32,
    pub surface: Color32,
    pub on_surface: Color32,
    pub surface_container: Color32,
    pub on_surface_variant: Color32,
    pub outline: Color32,
    pub outline_variant: Color32,
    pub inverse_surface: Color32,
    pub on_inverse_surface: Color32,
    pub inverse_primary: Color32,
    pub shadow: Color32,
    pub scrim: Color32,
}

impl ColorScheme {
    /// A static purple-seeded starting palette. Supply a custom scheme for
    /// brand colors; dynamic Material color generation is not implemented.
    pub fn light() -> Self {
        Self {
            brightness: Brightness::Light,
            primary: Color32::from_rgb(0x67, 0x50, 0xA4),
            on_primary: Color32::WHITE,
            primary_container: Color32::from_rgb(0xE9, 0xDD, 0xFF),
            on_primary_container: Color32::from_rgb(0x21, 0x00, 0x5D),
            secondary: Color32::from_rgb(0x62, 0x5B, 0x71),
            on_secondary: Color32::WHITE,
            secondary_container: Color32::from_rgb(0xE8, 0xDE, 0xF8),
            on_secondary_container: Color32::from_rgb(0x1E, 0x19, 0x2B),
            tertiary: Color32::from_rgb(0x7E, 0x52, 0x69),
            on_tertiary: Color32::WHITE,
            tertiary_container: Color32::from_rgb(0xFF, 0xD8, 0xE8),
            on_tertiary_container: Color32::from_rgb(0x31, 0x10, 0x22),
            error: Color32::from_rgb(0xB3, 0x26, 0x1E),
            on_error: Color32::WHITE,
            error_container: Color32::from_rgb(0xF9, 0xDE, 0xDC),
            on_error_container: Color32::from_rgb(0x41, 0x00, 0x02),
            surface: Color32::from_rgb(0xFF, 0xF7, 0xFF),
            on_surface: Color32::from_rgb(0x1D, 0x1B, 0x20),
            surface_container: Color32::from_rgb(0xF3, 0xED, 0xF7),
            on_surface_variant: Color32::from_rgb(0x49, 0x45, 0x4F),
            outline: Color32::from_rgb(0x79, 0x75, 0x7F),
            outline_variant: Color32::from_rgb(0xCA, 0xC4, 0xD0),
            inverse_surface: Color32::from_rgb(0x32, 0x30, 0x35),
            on_inverse_surface: Color32::from_rgb(0xF4, 0xEF, 0xF7),
            inverse_primary: Color32::from_rgb(0xD0, 0xBC, 0xFF),
            shadow: Color32::BLACK,
            scrim: Color32::BLACK,
        }
    }

    /// A static purple-seeded dark palette. This is a baseline, not dynamic
    /// Material You tonal palette generation.
    pub fn dark() -> Self {
        Self {
            brightness: Brightness::Dark,
            primary: Color32::from_rgb(0xD0, 0xBC, 0xFF),
            on_primary: Color32::from_rgb(0x38, 0x1E, 0x72),
            primary_container: Color32::from_rgb(0x4F, 0x37, 0x8B),
            on_primary_container: Color32::from_rgb(0xE9, 0xDD, 0xFF),
            secondary: Color32::from_rgb(0xCC, 0xC2, 0xDC),
            on_secondary: Color32::from_rgb(0x33, 0x2D, 0x41),
            secondary_container: Color32::from_rgb(0x4A, 0x44, 0x58),
            on_secondary_container: Color32::from_rgb(0xE8, 0xDE, 0xF8),
            tertiary: Color32::from_rgb(0xEF, 0xB8, 0xD2),
            on_tertiary: Color32::from_rgb(0x49, 0x20, 0x35),
            tertiary_container: Color32::from_rgb(0x63, 0x38, 0x4B),
            on_tertiary_container: Color32::from_rgb(0xFF, 0xD8, 0xE8),
            error: Color32::from_rgb(0xF2, 0xB8, 0xB5),
            on_error: Color32::from_rgb(0x60, 0x14, 0x10),
            error_container: Color32::from_rgb(0x8C, 0x1D, 0x18),
            on_error_container: Color32::from_rgb(0xF9, 0xDE, 0xDC),
            surface: Color32::from_rgb(0x14, 0x12, 0x18),
            on_surface: Color32::from_rgb(0xE6, 0xE0, 0xE9),
            surface_container: Color32::from_rgb(0x21, 0x1F, 0x26),
            on_surface_variant: Color32::from_rgb(0xCA, 0xC4, 0xD0),
            outline: Color32::from_rgb(0x93, 0x8F, 0x99),
            outline_variant: Color32::from_rgb(0x49, 0x45, 0x4F),
            inverse_surface: Color32::from_rgb(0xE6, 0xE0, 0xE9),
            on_inverse_surface: Color32::from_rgb(0x32, 0x30, 0x35),
            inverse_primary: Color32::from_rgb(0x67, 0x50, 0xA4),
            shadow: Color32::BLACK,
            scrim: Color32::BLACK,
        }
    }
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self::light()
    }
}

/// Inherited Material theme data and an optional egui global-style adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemeData {
    pub color_scheme: ColorScheme,
    pub use_material3: bool,
}

/// A Material root widget that provides an inherited theme to its home subtree.
/// Native window setup and route management remain the responsibility of the
/// host runner and navigator APIs.
pub struct MaterialApp {
    home: Widget,
    theme: ThemeData,
}

impl MaterialApp {
    pub fn new(home: Widget) -> Self {
        Self {
            home,
            theme: ThemeData::default(),
        }
    }

    /// Create an app root whose home page follows a retained route controller.
    pub fn with_navigator<R: Clone + PartialEq + 'static>(
        navigator: &NavigatorController<R>,
        builder: impl Fn(&BuildContext, &R) -> Widget + 'static,
    ) -> Self {
        Self::new(navigator.view(builder))
    }

    pub fn theme(mut self, theme: ThemeData) -> Self {
        self.theme = theme;
        self
    }

    pub fn into_widget(self) -> Widget {
        self.theme.provide(self.home)
    }
}

impl Default for ThemeData {
    fn default() -> Self {
        Self {
            color_scheme: ColorScheme::default(),
            use_material3: true,
        }
    }
}

impl ThemeData {
    pub fn of(context: &BuildContext) -> Result<Self, DependencyError> {
        Ok(context
            .depend_on::<Self>()?
            .map(|theme| *theme)
            .unwrap_or_default())
    }

    pub fn provide(self, child: Widget) -> Widget {
        Widget::inherited(self, child)
    }

    /// Apply the core scheme to egui defaults for apps that use backend
    /// controls. Retained framework controls can also read the inherited theme.
    pub fn apply(self, context: &egui::Context) {
        let scheme = self.color_scheme;
        let mut visuals = match scheme.brightness {
            Brightness::Light => egui::Visuals::light(),
            Brightness::Dark => egui::Visuals::dark(),
        };
        visuals.panel_fill = scheme.surface;
        visuals.window_fill = scheme.surface_container;
        visuals.extreme_bg_color = scheme.surface;
        visuals.faint_bg_color = scheme.surface_container;
        visuals.override_text_color = Some(scheme.on_surface);
        visuals.hyperlink_color = scheme.primary;
        visuals.selection.bg_fill = scheme.primary_container;
        visuals.selection.stroke.color = scheme.on_primary_container;
        context.set_visuals(visuals);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{element::*, layout::BoxConstraints};
    use std::{cell::RefCell, rc::Rc};

    struct ReadTheme(Rc<RefCell<Option<ThemeData>>>);
    impl StatelessWidget for ReadTheme {
        fn build(&self, context: &BuildContext) -> Widget {
            *self.0.borrow_mut() = Some(ThemeData::of(context).unwrap());
            Widget::empty()
        }
    }

    #[test]
    fn inherited_material_theme_resolves_nearest_scope_and_global_style() {
        let theme = ThemeData {
            color_scheme: ColorScheme::dark(),
            ..ThemeData::default()
        };
        let observed = Rc::new(RefCell::new(None));
        let widget = theme.provide(Widget::stateless(ReadTheme(observed.clone())));
        let context = egui::Context::default();
        let mut tree = ElementTree::new(&context, widget).unwrap();
        let _ = context.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                tree.show(ui, BoxConstraints::tight(egui::vec2(20.0, 20.0)).unwrap())
                    .unwrap();
            });
        });
        assert_eq!(*observed.borrow(), Some(theme));

        theme.apply(&context);
        let visuals = context.style().visuals.clone();
        assert_eq!(visuals.panel_fill, theme.color_scheme.surface);
        assert_eq!(
            visuals.override_text_color,
            Some(theme.color_scheme.on_surface)
        );
        assert_eq!(
            visuals.selection.bg_fill,
            theme.color_scheme.primary_container
        );
    }

    #[test]
    fn static_color_schemes_pair_content_roles_and_brightness() {
        for scheme in [ColorScheme::light(), ColorScheme::dark()] {
            assert_ne!(scheme.primary, scheme.on_primary);
            assert_ne!(scheme.surface, scheme.on_surface);
            assert_ne!(scheme.error, scheme.on_error);
        }
        assert_eq!(ColorScheme::default().brightness, Brightness::Light);
        assert_eq!(ColorScheme::dark().brightness, Brightness::Dark);
    }

    #[test]
    fn material_buttons_build_from_the_inherited_scheme() {
        let binding = crate::gestures::GestureBinding::new();
        let widgets = vec![
            crate::material::FilledButton::new(&binding, "Save", || {}).into_widget(),
            crate::material::OutlinedButton::new(&binding, "Cancel", || {}).into_widget(),
            crate::material::TextButton::new(&binding, "More", || {}).into_widget(),
        ];
        let child = Widget::flex(
            FlexStyle::new(crate::layout::Axis::Vertical),
            widgets.into_iter().map(FlexItem::fixed).collect(),
        );
        let context = egui::Context::default();
        let mut tree = ElementTree::new(&context, ThemeData::default().provide(child)).unwrap();
        let _ = context.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                tree.show(ui, BoxConstraints::tight(egui::vec2(240.0, 150.0)).unwrap())
                    .unwrap();
            });
        });
    }

    #[test]
    fn scaffold_lays_out_app_bar_body_and_bottom_slot() {
        let context = egui::Context::default();
        let body_painted = Rc::new(std::cell::Cell::new(false));
        let bottom_painted = Rc::new(std::cell::Cell::new(false));
        let body_flag = body_painted.clone();
        let bottom_flag = bottom_painted.clone();
        let theme = ThemeData {
            color_scheme: ColorScheme::dark(),
            ..ThemeData::default()
        };
        let scaffold =
            crate::material::Scaffold::new(Widget::leaf(egui::vec2(100.0, 100.0), move |_| {
                body_flag.set(true)
            }))
            .app_bar(
                crate::material::AppBar::new("Home")
                    .theme(theme)
                    .into_widget(),
            )
            .bottom_navigation_bar(Widget::leaf(egui::vec2(300.0, 60.0), move |_| {
                bottom_flag.set(true);
            }))
            .theme(theme)
            .into_widget();
        let mut tree = ElementTree::new(&context, scaffold).unwrap();
        let _ = context.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                tree.show(ui, BoxConstraints::tight(egui::vec2(320.0, 480.0)).unwrap())
                    .unwrap();
            });
        });
        assert!(body_painted.get());
        assert!(bottom_painted.get());
    }

    #[test]
    fn navigation_bar_builds_controlled_destinations_inside_theme_scope() {
        let context = egui::Context::default();
        let binding = crate::gestures::GestureBinding::new();
        let nav = crate::material::NavigationBar::new(
            &binding,
            vec![
                crate::material::NavigationDestination::new("⌂", "Home"),
                crate::material::NavigationDestination::new("⌕", "Search").selected_icon("⌕"),
                crate::material::NavigationDestination::new("⚙", "Settings"),
            ],
            1,
            |_| {},
        )
        .into_widget();
        let root = ThemeData::default().provide(
            crate::material::Scaffold::new(Widget::empty())
                .bottom_navigation_bar(nav)
                .into_widget(),
        );
        let mut tree = ElementTree::new(&context, root).unwrap();
        let _ = context.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                tree.show(ui, BoxConstraints::tight(egui::vec2(360.0, 720.0)).unwrap())
                    .unwrap();
            });
        });
    }
}

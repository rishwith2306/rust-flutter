//! Initial Material color and inherited theme foundations.
//!
//! This module provides semantic colors and a subtree theme. It is not yet a
//! Material component catalog or a complete Material 3 color-role inventory.
use crate::{
    animation::AnimationController,
    egui::{self, Color32},
    element::{BuildContext, DependencyError, Directionality, Widget},
    layout::TextDirection,
    localization::Localizations,
    navigation::NavigatorController,
    notifier::Listenable,
    text::Text,
};
use material_color_utils::{
    dynamic::{
        color_spec::SpecVersion, dynamic_scheme::DynamicScheme,
        material_dynamic_colors::MaterialDynamicColors,
    },
    hct::hct_color::Hct,
    scheme::{
        SchemeContent, SchemeExpressive, SchemeFidelity, SchemeFruitSalad, SchemeMonochrome,
        SchemeNeutral, SchemeRainbow, SchemeTonalSpot, SchemeVibrant,
    },
    utils::color_utils::Argb,
};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt,
    rc::Rc,
};

mod bottom_sheet;
mod button;
mod card;
mod checkbox;
mod dialog;
mod divider;
mod floating_action_button;
mod icon_button;
mod list_tile;
mod messenger;
mod navigation_bar;
mod navigation_drawer;
mod progress_indicator;
mod radio;
mod scaffold;
mod slider;
mod snack_bar;
mod switch;
mod text_field;
pub use bottom_sheet::{ModalBottomSheetController, ModalBottomSheetHost, ModalBottomSheetState};
pub use button::{FilledButton, OutlinedButton, TextButton};
pub use card::{Card, CardVariant};
pub use checkbox::Checkbox;
pub use dialog::{AlertDialog, MaterialDialogController, MaterialDialogHost, MaterialDialogState};
pub use divider::{Divider, VerticalDivider};
pub use floating_action_button::FloatingActionButton;
pub use icon_button::{IconButton, IconButtonVariant};
pub use list_tile::ListTile;
pub use messenger::{ScaffoldMessengerController, ScaffoldMessengerHost};
pub use navigation_bar::{NavigationBar, NavigationDestination};
pub use navigation_drawer::{NavigationDrawer, NavigationDrawerDestination};
pub use progress_indicator::{
    CircularProgressIndicator, LinearProgressIndicator, ProgressIndicatorTheme,
};
pub use radio::Radio;
pub use scaffold::{AppBar, Drawer, Scaffold};
pub use slider::Slider;
pub(crate) use slider::SliderColors;
pub use snack_bar::{SnackBar, SnackBarAction};
pub use switch::Switch;
pub use text_field::TextField;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Brightness {
    Light,
    Dark,
}

/// Palette strategy used by [`ColorScheme::from_seed_with_options`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DynamicSchemeVariant {
    Monochrome,
    Neutral,
    #[default]
    TonalSpot,
    Vibrant,
    Expressive,
    Fidelity,
    Content,
    Rainbow,
    FruitSalad,
}

/// Material semantic color roles used by controls, surfaces and app themes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColorScheme {
    pub brightness: Brightness,
    pub primary: Color32,
    pub on_primary: Color32,
    pub primary_container: Color32,
    pub on_primary_container: Color32,
    pub primary_fixed: Color32,
    pub primary_fixed_dim: Color32,
    pub on_primary_fixed: Color32,
    pub on_primary_fixed_variant: Color32,
    pub secondary: Color32,
    pub on_secondary: Color32,
    pub secondary_container: Color32,
    pub on_secondary_container: Color32,
    pub secondary_fixed: Color32,
    pub secondary_fixed_dim: Color32,
    pub on_secondary_fixed: Color32,
    pub on_secondary_fixed_variant: Color32,
    pub tertiary: Color32,
    pub on_tertiary: Color32,
    pub tertiary_container: Color32,
    pub on_tertiary_container: Color32,
    pub tertiary_fixed: Color32,
    pub tertiary_fixed_dim: Color32,
    pub on_tertiary_fixed: Color32,
    pub on_tertiary_fixed_variant: Color32,
    pub error: Color32,
    pub on_error: Color32,
    pub error_container: Color32,
    pub on_error_container: Color32,
    pub surface: Color32,
    pub on_surface: Color32,
    pub surface_dim: Color32,
    pub surface_bright: Color32,
    pub surface_container_lowest: Color32,
    pub surface_container_low: Color32,
    pub surface_container: Color32,
    pub surface_container_high: Color32,
    pub surface_container_highest: Color32,
    pub on_surface_variant: Color32,
    pub outline: Color32,
    pub outline_variant: Color32,
    pub inverse_surface: Color32,
    pub on_inverse_surface: Color32,
    pub inverse_primary: Color32,
    pub surface_tint: Color32,
    pub shadow: Color32,
    pub scrim: Color32,
    /// Compatibility alias for `surface`, deprecated by Material 3.
    pub background: Color32,
    /// Compatibility alias for `on_surface`, deprecated by Material 3.
    pub on_background: Color32,
    /// Compatibility alias for `surface_container_highest`, deprecated by Material 3.
    pub surface_variant: Color32,
}

impl ColorScheme {
    /// Linearly interpolate all semantic colors; brightness switches at the
    /// midpoint. Values outside `0..=1` extrapolate the color channels.
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        macro_rules! colors {
            ($($field:ident),+ $(,)?) => {
                Self {
                    brightness: if t < 0.5 { self.brightness } else { other.brightness },
                    $( $field: self.$field.lerp_to_gamma(other.$field, t), )+
                }
            };
        }
        colors!(
            primary,
            on_primary,
            primary_container,
            on_primary_container,
            primary_fixed,
            primary_fixed_dim,
            on_primary_fixed,
            on_primary_fixed_variant,
            secondary,
            on_secondary,
            secondary_container,
            on_secondary_container,
            secondary_fixed,
            secondary_fixed_dim,
            on_secondary_fixed,
            on_secondary_fixed_variant,
            tertiary,
            on_tertiary,
            tertiary_container,
            on_tertiary_container,
            tertiary_fixed,
            tertiary_fixed_dim,
            on_tertiary_fixed,
            on_tertiary_fixed_variant,
            error,
            on_error,
            error_container,
            on_error_container,
            surface,
            on_surface,
            surface_dim,
            surface_bright,
            surface_container_lowest,
            surface_container_low,
            surface_container,
            surface_container_high,
            surface_container_highest,
            on_surface_variant,
            outline,
            outline_variant,
            inverse_surface,
            on_inverse_surface,
            inverse_primary,
            surface_tint,
            shadow,
            scrim,
            background,
            on_background,
            surface_variant,
        )
    }

    /// Generate a Material 3 light or dark palette from a seed color.
    ///
    /// Uses the `tonalSpot` variant, zero contrast adjustment, and the Material
    /// Color Utilities 2026 specification.
    pub fn from_seed(seed_color: Color32, brightness: Brightness) -> Self {
        Self::from_seed_with_options(seed_color, brightness, DynamicSchemeVariant::TonalSpot, 0.0)
    }

    /// Generate a Material 3 palette with an explicit variant and contrast level.
    /// Contrast levels follow Material Color Utilities and are clamped to `-1..=1`.
    pub fn from_seed_with_options(
        seed_color: Color32,
        brightness: Brightness,
        variant: DynamicSchemeVariant,
        contrast_level: f64,
    ) -> Self {
        let seed = Hct::from_argb(Argb::from_rgb(
            seed_color.r(),
            seed_color.g(),
            seed_color.b(),
        ));
        let contrast_level = if contrast_level.is_finite() {
            contrast_level.clamp(-1.0, 1.0)
        } else {
            0.0
        };
        let scheme = make_dynamic_scheme(
            seed,
            brightness == Brightness::Dark,
            variant,
            contrast_level,
        );
        let roles = MaterialDynamicColors::new_with_spec(SpecVersion::Spec2026);
        let color = |role| dynamic_role(&scheme, role);

        let primary = color(roles.primary());
        let on_primary = color(roles.on_primary());
        let primary_container = color(roles.primary_container());
        let on_primary_container = color(roles.on_primary_container());
        let primary_fixed = color(roles.primary_fixed());
        let primary_fixed_dim = color(roles.primary_fixed_dim());
        let on_primary_fixed = color(roles.on_primary_fixed());
        let on_primary_fixed_variant = color(roles.on_primary_fixed_variant());
        let secondary = color(roles.secondary());
        let on_secondary = color(roles.on_secondary());
        let secondary_container = color(roles.secondary_container());
        let on_secondary_container = color(roles.on_secondary_container());
        let secondary_fixed = color(roles.secondary_fixed());
        let secondary_fixed_dim = color(roles.secondary_fixed_dim());
        let on_secondary_fixed = color(roles.on_secondary_fixed());
        let on_secondary_fixed_variant = color(roles.on_secondary_fixed_variant());
        let tertiary = color(roles.tertiary());
        let on_tertiary = color(roles.on_tertiary());
        let tertiary_container = color(roles.tertiary_container());
        let on_tertiary_container = color(roles.on_tertiary_container());
        let tertiary_fixed = color(roles.tertiary_fixed());
        let tertiary_fixed_dim = color(roles.tertiary_fixed_dim());
        let on_tertiary_fixed = color(roles.on_tertiary_fixed());
        let on_tertiary_fixed_variant = color(roles.on_tertiary_fixed_variant());
        let error = color(roles.error());
        let on_error = color(roles.on_error());
        let error_container = color(roles.error_container());
        let on_error_container = color(roles.on_error_container());
        let surface = color(roles.surface());
        let on_surface = color(roles.on_surface());
        let surface_dim = color(roles.surface_dim());
        let surface_bright = color(roles.surface_bright());
        let surface_container_lowest = color(roles.surface_container_lowest());
        let surface_container_low = color(roles.surface_container_low());
        let surface_container = color(roles.surface_container());
        let surface_container_high = color(roles.surface_container_high());
        let surface_container_highest = color(roles.surface_container_highest());
        let on_surface_variant = color(roles.on_surface_variant());
        let outline = color(roles.outline());
        let outline_variant = color(roles.outline_variant());
        let inverse_surface = color(roles.inverse_surface());
        let on_inverse_surface = color(roles.inverse_on_surface());
        let inverse_primary = color(roles.inverse_primary());
        let surface_tint = color(roles.surface_tint());
        let shadow = color(roles.shadow());
        let scrim = color(roles.scrim());

        Self {
            brightness,
            primary,
            on_primary,
            primary_container,
            on_primary_container,
            primary_fixed,
            primary_fixed_dim,
            on_primary_fixed,
            on_primary_fixed_variant,
            secondary,
            on_secondary,
            secondary_container,
            on_secondary_container,
            secondary_fixed,
            secondary_fixed_dim,
            on_secondary_fixed,
            on_secondary_fixed_variant,
            tertiary,
            on_tertiary,
            tertiary_container,
            on_tertiary_container,
            tertiary_fixed,
            tertiary_fixed_dim,
            on_tertiary_fixed,
            on_tertiary_fixed_variant,
            error,
            on_error,
            error_container,
            on_error_container,
            surface,
            on_surface,
            surface_dim,
            surface_bright,
            surface_container_lowest,
            surface_container_low,
            surface_container,
            surface_container_high,
            surface_container_highest,
            on_surface_variant,
            outline,
            outline_variant,
            inverse_surface,
            on_inverse_surface,
            inverse_primary,
            surface_tint,
            shadow,
            scrim,
            background: surface,
            on_background: on_surface,
            surface_variant: surface_container_highest,
        }
    }

    /// A static purple-seeded starting palette. Use [`Self::from_seed`] to
    /// derive a scheme from an application brand color.
    pub fn light() -> Self {
        Self {
            brightness: Brightness::Light,
            primary: Color32::from_rgb(0x67, 0x50, 0xA4),
            on_primary: Color32::WHITE,
            primary_container: Color32::from_rgb(0xE9, 0xDD, 0xFF),
            on_primary_container: Color32::from_rgb(0x21, 0x00, 0x5D),
            primary_fixed: Color32::from_rgb(0xE9, 0xDD, 0xFF),
            primary_fixed_dim: Color32::from_rgb(0xD0, 0xBC, 0xFF),
            on_primary_fixed: Color32::from_rgb(0x21, 0x00, 0x5D),
            on_primary_fixed_variant: Color32::from_rgb(0x4F, 0x37, 0x8A),
            secondary: Color32::from_rgb(0x62, 0x5B, 0x71),
            on_secondary: Color32::WHITE,
            secondary_container: Color32::from_rgb(0xE8, 0xDE, 0xF8),
            on_secondary_container: Color32::from_rgb(0x1E, 0x19, 0x2B),
            secondary_fixed: Color32::from_rgb(0xE8, 0xDE, 0xF8),
            secondary_fixed_dim: Color32::from_rgb(0xCC, 0xC2, 0xDC),
            on_secondary_fixed: Color32::from_rgb(0x1E, 0x19, 0x2B),
            on_secondary_fixed_variant: Color32::from_rgb(0x49, 0x45, 0x4F),
            tertiary: Color32::from_rgb(0x7E, 0x52, 0x69),
            on_tertiary: Color32::WHITE,
            tertiary_container: Color32::from_rgb(0xFF, 0xD8, 0xE8),
            on_tertiary_container: Color32::from_rgb(0x31, 0x10, 0x22),
            tertiary_fixed: Color32::from_rgb(0xFF, 0xD8, 0xE8),
            tertiary_fixed_dim: Color32::from_rgb(0xEF, 0xB8, 0xD2),
            on_tertiary_fixed: Color32::from_rgb(0x31, 0x10, 0x22),
            on_tertiary_fixed_variant: Color32::from_rgb(0x63, 0x38, 0x4B),
            error: Color32::from_rgb(0xB3, 0x26, 0x1E),
            on_error: Color32::WHITE,
            error_container: Color32::from_rgb(0xF9, 0xDE, 0xDC),
            on_error_container: Color32::from_rgb(0x41, 0x00, 0x02),
            surface: Color32::from_rgb(0xFF, 0xF7, 0xFF),
            on_surface: Color32::from_rgb(0x1D, 0x1B, 0x20),
            surface_dim: Color32::from_rgb(0xDE, 0xD8, 0xE1),
            surface_bright: Color32::from_rgb(0xFE, 0xF7, 0xFF),
            surface_container_lowest: Color32::WHITE,
            surface_container_low: Color32::from_rgb(0xF7, 0xF2, 0xFA),
            surface_container: Color32::from_rgb(0xF3, 0xED, 0xF7),
            surface_container_high: Color32::from_rgb(0xEC, 0xE6, 0xF0),
            surface_container_highest: Color32::from_rgb(0xE6, 0xE0, 0xE9),
            on_surface_variant: Color32::from_rgb(0x49, 0x45, 0x4F),
            outline: Color32::from_rgb(0x79, 0x75, 0x7F),
            outline_variant: Color32::from_rgb(0xCA, 0xC4, 0xD0),
            inverse_surface: Color32::from_rgb(0x32, 0x30, 0x35),
            on_inverse_surface: Color32::from_rgb(0xF4, 0xEF, 0xF7),
            inverse_primary: Color32::from_rgb(0xD0, 0xBC, 0xFF),
            surface_tint: Color32::from_rgb(0x67, 0x50, 0xA4),
            shadow: Color32::BLACK,
            scrim: Color32::BLACK,
            background: Color32::from_rgb(0xFF, 0xF7, 0xFF),
            on_background: Color32::from_rgb(0x1D, 0x1B, 0x20),
            surface_variant: Color32::from_rgb(0xE6, 0xE0, 0xE9),
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
            primary_fixed: Color32::from_rgb(0xE9, 0xDD, 0xFF),
            primary_fixed_dim: Color32::from_rgb(0xD0, 0xBC, 0xFF),
            on_primary_fixed: Color32::from_rgb(0x21, 0x00, 0x5D),
            on_primary_fixed_variant: Color32::from_rgb(0x4F, 0x37, 0x8A),
            secondary: Color32::from_rgb(0xCC, 0xC2, 0xDC),
            on_secondary: Color32::from_rgb(0x33, 0x2D, 0x41),
            secondary_container: Color32::from_rgb(0x4A, 0x44, 0x58),
            on_secondary_container: Color32::from_rgb(0xE8, 0xDE, 0xF8),
            secondary_fixed: Color32::from_rgb(0xE8, 0xDE, 0xF8),
            secondary_fixed_dim: Color32::from_rgb(0xCC, 0xC2, 0xDC),
            on_secondary_fixed: Color32::from_rgb(0x1E, 0x19, 0x2B),
            on_secondary_fixed_variant: Color32::from_rgb(0x49, 0x45, 0x4F),
            tertiary: Color32::from_rgb(0xEF, 0xB8, 0xD2),
            on_tertiary: Color32::from_rgb(0x49, 0x20, 0x35),
            tertiary_container: Color32::from_rgb(0x63, 0x38, 0x4B),
            on_tertiary_container: Color32::from_rgb(0xFF, 0xD8, 0xE8),
            tertiary_fixed: Color32::from_rgb(0xFF, 0xD8, 0xE8),
            tertiary_fixed_dim: Color32::from_rgb(0xEF, 0xB8, 0xD2),
            on_tertiary_fixed: Color32::from_rgb(0x31, 0x10, 0x22),
            on_tertiary_fixed_variant: Color32::from_rgb(0x63, 0x38, 0x4B),
            error: Color32::from_rgb(0xF2, 0xB8, 0xB5),
            on_error: Color32::from_rgb(0x60, 0x14, 0x10),
            error_container: Color32::from_rgb(0x8C, 0x1D, 0x18),
            on_error_container: Color32::from_rgb(0xF9, 0xDE, 0xDC),
            surface: Color32::from_rgb(0x14, 0x12, 0x18),
            on_surface: Color32::from_rgb(0xE6, 0xE0, 0xE9),
            surface_dim: Color32::from_rgb(0x14, 0x12, 0x18),
            surface_bright: Color32::from_rgb(0x3B, 0x38, 0x3E),
            surface_container_lowest: Color32::from_rgb(0x0F, 0x0D, 0x13),
            surface_container_low: Color32::from_rgb(0x1D, 0x1B, 0x20),
            surface_container: Color32::from_rgb(0x21, 0x1F, 0x26),
            surface_container_high: Color32::from_rgb(0x2B, 0x29, 0x2F),
            surface_container_highest: Color32::from_rgb(0x36, 0x34, 0x3A),
            on_surface_variant: Color32::from_rgb(0xCA, 0xC4, 0xD0),
            outline: Color32::from_rgb(0x93, 0x8F, 0x99),
            outline_variant: Color32::from_rgb(0x49, 0x45, 0x4F),
            inverse_surface: Color32::from_rgb(0xE6, 0xE0, 0xE9),
            on_inverse_surface: Color32::from_rgb(0x32, 0x30, 0x35),
            inverse_primary: Color32::from_rgb(0x67, 0x50, 0xA4),
            surface_tint: Color32::from_rgb(0xD0, 0xBC, 0xFF),
            shadow: Color32::BLACK,
            scrim: Color32::BLACK,
            background: Color32::from_rgb(0x14, 0x12, 0x18),
            on_background: Color32::from_rgb(0xE6, 0xE0, 0xE9),
            surface_variant: Color32::from_rgb(0x36, 0x34, 0x3A),
        }
    }
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self::light()
    }
}

fn make_dynamic_scheme(
    seed: Hct,
    is_dark: bool,
    variant: DynamicSchemeVariant,
    contrast_level: f64,
) -> DynamicScheme {
    macro_rules! generate {
        ($scheme:ty) => {
            <$scheme>::builder(seed.to_argb(), is_dark, contrast_level)
                .spec_version(SpecVersion::Spec2026)
                .build()
        };
    }
    match variant {
        DynamicSchemeVariant::Monochrome => generate!(SchemeMonochrome),
        DynamicSchemeVariant::Neutral => generate!(SchemeNeutral),
        DynamicSchemeVariant::TonalSpot => generate!(SchemeTonalSpot),
        DynamicSchemeVariant::Vibrant => generate!(SchemeVibrant),
        DynamicSchemeVariant::Expressive => generate!(SchemeExpressive),
        DynamicSchemeVariant::Fidelity => generate!(SchemeFidelity),
        DynamicSchemeVariant::Content => generate!(SchemeContent),
        DynamicSchemeVariant::Rainbow => generate!(SchemeRainbow),
        DynamicSchemeVariant::FruitSalad => generate!(SchemeFruitSalad),
    }
}

fn dynamic_role(
    scheme: &DynamicScheme,
    role: std::sync::Arc<material_color_utils::dynamic::dynamic_color::DynamicColor>,
) -> Color32 {
    let argb = scheme.get_argb(&role).0;
    Color32::from_rgba_unmultiplied(
        ((argb >> 16) & 0xff) as u8,
        ((argb >> 8) & 0xff) as u8,
        (argb & 0xff) as u8,
        (argb >> 24) as u8,
    )
}

/// Inherited Material theme data and an optional egui global-style adapter.
pub struct ThemeData {
    pub color_scheme: ColorScheme,
    /// Optional complete theme selected when dark mode is active.
    pub dark_theme: Option<Box<ThemeData>>,
    pub theme_mode: ThemeMode,
    pub use_material3: bool,
    pub progress_indicator_theme: ProgressIndicatorThemeData,
    pub extensions: ThemeExtensions,
}

/// Typed custom values attached to a [`ThemeData`]. Use `ThemeData`'s
/// `with_extension` and `extension` methods to modify and retrieve values.
#[derive(Default)]
pub struct ThemeExtensions(HashMap<TypeId, Box<dyn ErasedThemeExtension>>);

impl Clone for ThemeExtensions {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl fmt::Debug for ThemeExtensions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_map().entries(self.0.iter()).finish()
    }
}

impl PartialEq for ThemeExtensions {
    fn eq(&self, other: &Self) -> bool {
        self.0.len() == other.0.len()
            && self.0.iter().all(|(id, extension)| {
                other
                    .0
                    .get(id)
                    .is_some_and(|other| extension.equals(other.as_ref()))
            })
    }
}

/// A typed addition to [`ThemeData`]. Implement `lerp` to define how the value
/// transitions between themes; extension values must also support cloning and
/// equality so inherited theme updates can be tracked.
pub trait ThemeExtension: Clone + fmt::Debug + PartialEq + Send + Sync + 'static {
    fn lerp(&self, other: &Self, t: f32) -> Self;
}

trait ErasedThemeExtension: fmt::Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn clone_box(&self) -> Box<dyn ErasedThemeExtension>;
    fn equals(&self, other: &dyn ErasedThemeExtension) -> bool;
    fn lerp_box(&self, other: &dyn ErasedThemeExtension, t: f32) -> Box<dyn ErasedThemeExtension>;
}

#[derive(Clone, Debug)]
struct ThemeExtensionValue<T: ThemeExtension>(T);

impl<T: ThemeExtension> ErasedThemeExtension for ThemeExtensionValue<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn ErasedThemeExtension> {
        Box::new(self.clone())
    }

    fn equals(&self, other: &dyn ErasedThemeExtension) -> bool {
        other
            .as_any()
            .downcast_ref::<Self>()
            .is_some_and(|other| self.0 == other.0)
    }

    fn lerp_box(&self, other: &dyn ErasedThemeExtension, t: f32) -> Box<dyn ErasedThemeExtension> {
        other.as_any().downcast_ref::<Self>().map_or_else(
            || self.clone_box(),
            |other| Box::new(Self(self.0.lerp(&other.0, t))),
        )
    }
}

impl Clone for Box<dyn ErasedThemeExtension> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

impl Clone for ThemeData {
    fn clone(&self) -> Self {
        Self {
            color_scheme: self.color_scheme,
            dark_theme: self.dark_theme.clone(),
            theme_mode: self.theme_mode,
            use_material3: self.use_material3,
            progress_indicator_theme: self.progress_indicator_theme.clone(),
            extensions: self.extensions.clone(),
        }
    }
}

impl fmt::Debug for ThemeData {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ThemeData")
            .field("color_scheme", &self.color_scheme)
            .field("dark_theme", &self.dark_theme)
            .field("theme_mode", &self.theme_mode)
            .field("use_material3", &self.use_material3)
            .field("progress_indicator_theme", &self.progress_indicator_theme)
            .field("extensions", &self.extensions)
            .finish()
    }
}

impl PartialEq for ThemeData {
    fn eq(&self, other: &Self) -> bool {
        self.color_scheme == other.color_scheme
            && self.dark_theme == other.dark_theme
            && self.theme_mode == other.theme_mode
            && self.use_material3 == other.use_material3
            && self.progress_indicator_theme == other.progress_indicator_theme
            && self.extensions == other.extensions
    }
}

/// Select a fixed Material brightness or follow the platform appearance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

/// Shared visual defaults for descendant Material progress indicators.
#[derive(Clone, Default)]
pub struct ProgressIndicatorThemeData {
    pub color: Option<Color32>,
    pub linear_track_color: Option<Color32>,
    pub linear_min_height: Option<f32>,
    pub circular_track_color: Option<Color32>,
    pub stroke_width: Option<f32>,
    pub stroke_align: Option<f32>,
    pub stroke_cap: Option<StrokeCap>,
    pub border_radius: Option<f32>,
    pub track_gap: Option<f32>,
    pub stop_indicator_color: Option<Color32>,
    pub stop_indicator_radius: Option<f32>,
    pub controller: Option<AnimationController>,
}

impl std::fmt::Debug for ProgressIndicatorThemeData {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProgressIndicatorThemeData")
            .field("color", &self.color)
            .field("linear_track_color", &self.linear_track_color)
            .field("linear_min_height", &self.linear_min_height)
            .field("circular_track_color", &self.circular_track_color)
            .field("stroke_width", &self.stroke_width)
            .field("stroke_align", &self.stroke_align)
            .field("stroke_cap", &self.stroke_cap)
            .field("border_radius", &self.border_radius)
            .field("track_gap", &self.track_gap)
            .field("stop_indicator_color", &self.stop_indicator_color)
            .field("stop_indicator_radius", &self.stop_indicator_radius)
            .field("controller", &self.controller.is_some())
            .finish()
    }
}

impl PartialEq for ProgressIndicatorThemeData {
    fn eq(&self, other: &Self) -> bool {
        self.color == other.color
            && self.linear_track_color == other.linear_track_color
            && self.linear_min_height == other.linear_min_height
            && self.circular_track_color == other.circular_track_color
            && self.stroke_width == other.stroke_width
            && self.stroke_align == other.stroke_align
            && self.stroke_cap == other.stroke_cap
            && self.border_radius == other.border_radius
            && self.track_gap == other.track_gap
            && self.stop_indicator_color == other.stop_indicator_color
            && self.stop_indicator_radius == other.stop_indicator_radius
            && match (&self.controller, &other.controller) {
                (Some(left), Some(right)) => left.same_source(right),
                (None, None) => true,
                _ => false,
            }
    }
}

/// End shape used when painting a stroked circular progress arc.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StrokeCap {
    #[default]
    Butt,
    Round,
    Square,
}

/// A Material root widget that provides an inherited theme to its home subtree.
/// Native window setup and route management remain the responsibility of the
/// host runner and navigator APIs.
pub struct MaterialApp {
    home: Widget,
    theme: ThemeData,
    dark_theme: Option<ThemeData>,
    theme_mode: ThemeMode,
    localizations: Option<Localizations>,
    text_direction: TextDirection,
}

type UnknownRouteBuilder = dyn Fn(&BuildContext, &str) -> Widget;

/// A named route and the page builder used for it by [`MaterialApp`].
pub struct NamedRoute {
    name: String,
    builder: Rc<dyn Fn(&BuildContext) -> Widget>,
}

impl NamedRoute {
    pub fn new(
        name: impl Into<String>,
        builder: impl Fn(&BuildContext) -> Widget + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            builder: Rc::new(builder),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RouteConfigurationError {
    EmptyName,
    DuplicateName(String),
    InitialRouteMissing(String),
}

impl std::fmt::Display for RouteConfigurationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyName => write!(formatter, "Material route name must not be empty"),
            Self::DuplicateName(name) => {
                write!(formatter, "duplicate Material route name '{name}'")
            }
            Self::InitialRouteMissing(name) => {
                write!(
                    formatter,
                    "initial Material route '{name}' is not registered"
                )
            }
        }
    }
}

impl std::error::Error for RouteConfigurationError {}

impl MaterialApp {
    pub fn new(home: Widget) -> Self {
        Self {
            home,
            theme: ThemeData::default(),
            dark_theme: None,
            theme_mode: ThemeMode::System,
            localizations: None,
            text_direction: TextDirection::default(),
        }
    }

    /// Create an app root whose home page follows a retained route controller.
    pub fn with_navigator<R: Clone + PartialEq + 'static>(
        navigator: &NavigatorController<R>,
        builder: impl Fn(&BuildContext, &R) -> Widget + 'static,
    ) -> Self {
        Self::new(navigator.retained_view(builder))
    }

    /// Create an app backed by a string route table and an existing route
    /// controller. Unknown pushed names render a visible fallback page.
    pub fn with_named_routes(
        navigator: &NavigatorController<String>,
        routes: impl IntoIterator<Item = NamedRoute>,
    ) -> Result<Self, RouteConfigurationError> {
        Self::with_named_routes_and_fallback(navigator, routes, |_, name| {
            Text::new(format!("No page is registered for route '{name}'")).into_widget()
        })
    }

    /// Create an app with a named route table and a custom unknown-route page.
    pub fn with_named_routes_and_fallback(
        navigator: &NavigatorController<String>,
        routes: impl IntoIterator<Item = NamedRoute>,
        unknown_route: impl Fn(&BuildContext, &str) -> Widget + 'static,
    ) -> Result<Self, RouteConfigurationError> {
        let mut table = HashMap::new();
        for route in routes {
            if route.name.trim().is_empty() {
                return Err(RouteConfigurationError::EmptyName);
            }
            if table.insert(route.name.clone(), route.builder).is_some() {
                return Err(RouteConfigurationError::DuplicateName(route.name));
            }
        }
        let initial = navigator
            .current()
            .ok_or_else(|| RouteConfigurationError::InitialRouteMissing(String::new()))?;
        if !table.contains_key(&initial) {
            return Err(RouteConfigurationError::InitialRouteMissing(initial));
        }
        let table = Rc::new(table);
        let unknown_route: Rc<UnknownRouteBuilder> = Rc::new(unknown_route);
        let home = navigator.retained_view(move |context, name| {
            table
                .get(name)
                .map_or_else(|| unknown_route(context, name), |builder| builder(context))
        });
        Ok(Self::new(home))
    }

    pub fn theme(mut self, theme: ThemeData) -> Self {
        self.theme = theme;
        self
    }

    /// Set the theme selected when the app resolves to dark mode.
    pub fn dark_theme(mut self, theme: ThemeData) -> Self {
        self.dark_theme = Some(theme);
        self
    }

    /// Select whether the app follows the platform or forces a brightness.
    pub fn theme_mode(mut self, mode: ThemeMode) -> Self {
        self.theme_mode = mode;
        self
    }

    /// Provide a Fluent catalog to descendants through `Localizations::of`.
    /// Locale selection and platform locale discovery remain application-owned.
    pub fn localizations(mut self, localizations: Localizations) -> Self {
        self.localizations = Some(localizations);
        self
    }

    /// Set the inherited direction used by direction-aware descendants.
    pub fn text_direction(mut self, direction: TextDirection) -> Self {
        self.text_direction = direction;
        self
    }

    pub fn into_widget(self) -> Widget {
        let mut theme = self.theme;
        if let Some(dark_theme) = self.dark_theme {
            theme.dark_theme = Some(Box::new(dark_theme));
        }
        theme.theme_mode = self.theme_mode;
        let child = match self.localizations {
            Some(catalog) => catalog.provide(self.home),
            None => self.home,
        };
        theme.provide(Directionality::provide(self.text_direction, child))
    }
}

impl Default for ThemeData {
    fn default() -> Self {
        Self {
            color_scheme: ColorScheme::default(),
            dark_theme: None,
            theme_mode: ThemeMode::System,
            use_material3: true,
            progress_indicator_theme: ProgressIndicatorThemeData::default(),
            extensions: ThemeExtensions::default(),
        }
    }
}

impl ProgressIndicatorThemeData {
    /// Merge subtree values over app-level defaults, preferring `self` when set.
    pub fn merge(self, defaults: Self) -> Self {
        Self {
            color: self.color.or(defaults.color),
            linear_track_color: self.linear_track_color.or(defaults.linear_track_color),
            linear_min_height: self.linear_min_height.or(defaults.linear_min_height),
            circular_track_color: self.circular_track_color.or(defaults.circular_track_color),
            stroke_width: self.stroke_width.or(defaults.stroke_width),
            stroke_align: self.stroke_align.or(defaults.stroke_align),
            stroke_cap: self.stroke_cap.or(defaults.stroke_cap),
            border_radius: self.border_radius.or(defaults.border_radius),
            track_gap: self.track_gap.or(defaults.track_gap),
            stop_indicator_color: self.stop_indicator_color.or(defaults.stop_indicator_color),
            stop_indicator_radius: self
                .stop_indicator_radius
                .or(defaults.stop_indicator_radius),
            controller: self.controller.or(defaults.controller),
        }
    }
}

impl ThemeData {
    /// Return a copy of this theme with one typed extension inserted or replaced.
    pub fn with_extension<T: ThemeExtension>(mut self, extension: T) -> Self {
        self.extensions
            .0
            .insert(TypeId::of::<T>(), Box::new(ThemeExtensionValue(extension)));
        self
    }

    /// Read an extension previously inserted with [`ThemeData::with_extension`].
    pub fn extension<T: ThemeExtension>(&self) -> Option<&T> {
        self.extensions
            .0
            .get(&TypeId::of::<T>())?
            .as_any()
            .downcast_ref::<ThemeExtensionValue<T>>()
            .map(|extension| &extension.0)
    }

    /// Interpolate an extension shared by both themes. Return `None` when either
    /// theme lacks that extension; callers can then use their transition fallback.
    pub fn lerp_extension<T: ThemeExtension>(&self, other: &Self, t: f32) -> Option<T> {
        let left = self.extensions.0.get(&TypeId::of::<T>())?;
        let right = other.extensions.0.get(&TypeId::of::<T>())?;
        left.lerp_box(right.as_ref(), t)
            .as_any()
            .downcast_ref::<ThemeExtensionValue<T>>()
            .map(|extension| extension.0.clone())
    }

    pub fn of(context: &BuildContext) -> Result<Self, DependencyError> {
        let mut theme = context
            .depend_on::<Self>()?
            .map(|theme| theme.as_ref().clone())
            .unwrap_or_default();
        let dark = match theme.theme_mode {
            ThemeMode::Dark => true,
            ThemeMode::Light => false,
            ThemeMode::System if theme.dark_theme.is_some() => context
                .depend_on::<crate::display::PlatformTheme>()?
                .is_some_and(|platform| *platform == crate::display::PlatformTheme::Dark),
            ThemeMode::System => false,
        };
        if dark && let Some(dark_theme) = theme.dark_theme.take() {
            theme = *dark_theme;
        }
        Ok(theme)
    }

    pub fn provide(&self, child: Widget) -> Widget {
        let controller = self.progress_indicator_theme.controller.clone();
        let child = Widget::inherited(self.clone(), child);
        match controller {
            Some(controller) => Widget::inherited_notifier(Some(controller), child),
            None => child,
        }
    }

    /// Apply the core scheme to egui defaults for apps that use backend
    /// controls. Retained framework controls can also read the inherited theme.
    pub fn apply(&self, context: &egui::Context) {
        let dark = match self.theme_mode {
            ThemeMode::Dark => true,
            ThemeMode::Light => false,
            ThemeMode::System => {
                crate::display::platform_theme(context) == crate::display::PlatformTheme::Dark
            }
        };
        let scheme = if dark {
            self.dark_theme
                .as_ref()
                .map_or(self.color_scheme, |theme| theme.color_scheme)
        } else {
            self.color_scheme
        };
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
        assert_eq!(*observed.borrow(), Some(theme.clone()));

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
                    .theme(theme.clone())
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

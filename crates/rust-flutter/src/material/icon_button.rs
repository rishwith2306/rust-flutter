use super::ThemeData;
use crate::{
    decoration::{BoxDecoration, DecoratedBox},
    egui::{self, Color32, Stroke, Vec2},
    element::{Semantics, Tooltip, Widget},
    gestures::{GestureBinding, GestureDetector, GestureTargetConfig, TapCallbacks, TapConfig},
    layout::{Alignment, BoxConstraints, HitTestBehavior},
    text::{DefaultTextStyle, TextStyle},
};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IconButtonVariant {
    #[default]
    Standard,
    Filled,
    FilledTonal,
    Outlined,
}

/// A Material icon action with standard, filled, tonal, and outlined styles.
pub struct IconButton {
    binding: GestureBinding,
    icon: Widget,
    selected_icon: Option<Widget>,
    on_pressed: Rc<dyn Fn()>,
    enabled: bool,
    selected: bool,
    semantics_label: Option<String>,
    tooltip: Option<String>,
    size: Vec2,
    variant: IconButtonVariant,
    theme: ThemeData,
    background: Option<Color32>,
    foreground: Option<Color32>,
}

impl IconButton {
    pub fn new(binding: &GestureBinding, icon: Widget, on_pressed: impl Fn() + 'static) -> Self {
        Self {
            binding: binding.clone(),
            icon,
            selected_icon: None,
            on_pressed: Rc::new(on_pressed),
            enabled: true,
            selected: false,
            semantics_label: None,
            tooltip: None,
            size: egui::vec2(48.0, 48.0),
            variant: IconButtonVariant::Standard,
            theme: ThemeData::default(),
            background: None,
            foreground: None,
        }
    }

    pub fn variant(mut self, variant: IconButtonVariant) -> Self {
        self.variant = variant;
        self
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
    pub fn selected_icon(mut self, icon: Widget) -> Self {
        self.selected_icon = Some(icon);
        self
    }
    /// Sets the screen reader name for an icon-only action.
    pub fn semantics_label(mut self, label: impl Into<String>) -> Self {
        self.semantics_label = Some(label.into());
        self
    }
    pub fn tooltip(mut self, message: impl Into<String>) -> Self {
        self.tooltip = Some(message.into());
        self
    }
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = size;
        self
    }
    pub fn theme(mut self, theme: ThemeData) -> Self {
        self.theme = theme;
        self
    }
    pub fn background_color(mut self, color: Color32) -> Self {
        self.background = Some(color);
        self
    }
    pub fn foreground_color(mut self, color: Color32) -> Self {
        self.foreground = Some(color);
        self
    }

    pub fn into_widget(self) -> Result<Widget, &'static str> {
        if !self.size.x.is_finite()
            || self.size.x <= 0.0
            || !self.size.y.is_finite()
            || self.size.y <= 0.0
        {
            return Err("Material icon button size must be positive and finite");
        }
        let scheme = self.theme.color_scheme;
        let (background, foreground, border) = match (self.variant, self.enabled, self.selected) {
            (IconButtonVariant::Standard, true, false) => (
                Color32::TRANSPARENT,
                scheme.on_surface_variant,
                Stroke::NONE,
            ),
            (IconButtonVariant::Standard, true, true) => (
                scheme.secondary_container,
                scheme.on_secondary_container,
                Stroke::NONE,
            ),
            (IconButtonVariant::Filled, true, _) => {
                (scheme.primary, scheme.on_primary, Stroke::NONE)
            }
            (IconButtonVariant::FilledTonal, true, _) => (
                scheme.secondary_container,
                scheme.on_secondary_container,
                Stroke::NONE,
            ),
            (IconButtonVariant::Outlined, true, _) => (
                Color32::TRANSPARENT,
                scheme.on_surface_variant,
                Stroke::new(1.0_f32, scheme.outline),
            ),
            (IconButtonVariant::Standard, false, _) => (
                Color32::TRANSPARENT,
                scheme.on_surface.gamma_multiply(0.38),
                Stroke::NONE,
            ),
            (IconButtonVariant::Filled | IconButtonVariant::FilledTonal, false, _) => (
                scheme.on_surface.gamma_multiply(0.12),
                scheme.on_surface.gamma_multiply(0.38),
                Stroke::NONE,
            ),
            (IconButtonVariant::Outlined, false, _) => (
                Color32::TRANSPARENT,
                scheme.on_surface.gamma_multiply(0.38),
                Stroke::new(1.0_f32, scheme.on_surface.gamma_multiply(0.12)),
            ),
        };
        let background = self.background.unwrap_or(background);
        let foreground = self.foreground.unwrap_or(foreground);
        let constraints = BoxConstraints::tight_for(Some(self.size.x), Some(self.size.y))
            .expect("validated Material icon button size");
        let icon = if self.selected {
            self.selected_icon.unwrap_or(self.icon)
        } else {
            self.icon
        };
        let icon = DefaultTextStyle {
            style: TextStyle {
                color: Some(foreground),
                ..Default::default()
            },
            ..Default::default()
        }
        .provide(icon);
        let radius = (self.size.x.min(self.size.y) / 2.0)
            .round()
            .clamp(0.0, 255.0) as u8;
        let surface = DecoratedBox::new(BoxDecoration {
            color: background,
            border,
            corner_radius: egui::CornerRadius::same(radius),
        })
        .into_widget(Widget::constrained(
            constraints,
            Widget::align(Alignment::CENTER, icon),
        ));
        let callback = self.on_pressed;
        let visual = if self.enabled {
            let pointer_callback = callback.clone();
            GestureDetector::new(
                &self.binding,
                GestureTargetConfig {
                    tap: Some((
                        TapConfig::default(),
                        TapCallbacks {
                            on_tap: Some(Rc::new(move |_| pointer_callback())),
                            ..Default::default()
                        },
                    )),
                    ..Default::default()
                },
            )
            .map_err(|_| "Material icon button gesture binding is unavailable")?
            .behavior(HitTestBehavior::Opaque)
            .into_widget(surface)
        } else {
            surface
        };
        let mut semantics = Semantics::new(egui::accesskit::Role::Button)
            .enabled(self.enabled)
            .toggled(if self.selected {
                egui::accesskit::Toggled::True
            } else {
                egui::accesskit::Toggled::False
            });
        if let Some(label) = self.semantics_label {
            semantics = semantics.label(label);
        }
        if self.enabled {
            semantics = semantics.on_click(move || callback());
        }
        let widget = semantics.into_widget(visual);
        Ok(match self.tooltip {
            Some(message) => Tooltip::new(widget, message).into_widget(),
            None => widget,
        })
    }
}

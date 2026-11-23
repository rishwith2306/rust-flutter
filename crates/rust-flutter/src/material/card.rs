use super::ThemeData;
use crate::{
    decoration::{BoxDecoration, DecoratedBox},
    egui::{self, Color32, Stroke},
    element::{Semantics, Widget},
    gestures::{GestureBinding, GestureDetector, GestureTargetConfig, TapCallbacks, TapConfig},
    layout::{HitTestBehavior, Insets},
};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CardVariant {
    #[default]
    Elevated,
    Filled,
    Outlined,
}

/// A rounded Material surface for grouping related content.
pub struct Card {
    child: Widget,
    variant: CardVariant,
    margin: Insets,
    color: Option<Color32>,
    theme: ThemeData,
    tap: Option<(GestureBinding, Rc<dyn Fn()>)>,
    semantics_label: Option<String>,
}

impl Card {
    pub fn new(child: Widget) -> Self {
        Self {
            child,
            variant: CardVariant::Elevated,
            margin: Insets::all(4.0),
            color: None,
            theme: ThemeData::default(),
            tap: None,
            semantics_label: None,
        }
    }

    pub fn variant(mut self, variant: CardVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn margin(mut self, margin: Insets) -> Self {
        self.margin = margin;
        self
    }

    pub fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }

    pub fn theme(mut self, theme: ThemeData) -> Self {
        self.theme = theme;
        self
    }

    pub fn on_tap(mut self, binding: &GestureBinding, callback: impl Fn() + 'static) -> Self {
        self.tap = Some((binding.clone(), Rc::new(callback)));
        self
    }

    /// Sets the accessible name for a tappable card.
    pub fn semantics_label(mut self, label: impl Into<String>) -> Self {
        self.semantics_label = Some(label.into());
        self
    }

    pub fn into_widget(self) -> Result<Widget, &'static str> {
        if [
            self.margin.left,
            self.margin.top,
            self.margin.right,
            self.margin.bottom,
        ]
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err("Material card margin must be finite and nonnegative");
        }
        let scheme = self.theme.color_scheme;
        let variant = if self.theme.use_material3 {
            self.variant
        } else {
            CardVariant::Elevated
        };
        let (background, border) = match variant {
            CardVariant::Elevated => (scheme.surface_container_low, Stroke::NONE),
            CardVariant::Filled => (scheme.surface_container_highest, Stroke::NONE),
            CardVariant::Outlined => (scheme.surface, Stroke::new(1.0_f32, scheme.outline_variant)),
        };
        let surface = DecoratedBox::new(BoxDecoration {
            color: self.color.unwrap_or(background),
            border,
            corner_radius: egui::CornerRadius::same(12),
        })
        .into_widget(self.child);
        let card = Widget::padding(self.margin, surface);
        let interactive = if let Some((binding, callback)) = self.tap {
            let semantics_callback = callback.clone();
            let visual = GestureDetector::new(
                &binding,
                GestureTargetConfig {
                    tap: Some((
                        TapConfig::default(),
                        TapCallbacks {
                            on_tap: Some(Rc::new(move |_| callback())),
                            ..Default::default()
                        },
                    )),
                    ..Default::default()
                },
            )
            .map_err(|_| "Material card gesture binding is unavailable")?
            .behavior(HitTestBehavior::Opaque)
            .into_widget(card);
            let mut semantics = Semantics::new(egui::accesskit::Role::Button)
                .on_click(move || semantics_callback());
            if let Some(label) = self.semantics_label {
                semantics = semantics.label(label);
            }
            semantics.into_widget(visual)
        } else {
            card
        };
        Ok(interactive)
    }
}

use super::ThemeData;
use crate::{
    egui::{self, Color32, Vec2},
    element::{FlexItem, FlexStyle, Widget},
    gestures::{GestureBinding, GestureButton, GestureSemanticRole},
    layout::{Axis, Insets},
};
use std::rc::Rc;

/// An icon glyph and label for [`NavigationDrawer`].
pub struct NavigationDrawerDestination {
    icon: String,
    selected_icon: Option<String>,
    label: String,
    enabled: bool,
}

impl NavigationDrawerDestination {
    pub fn new(icon: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            icon: icon.into(),
            selected_icon: None,
            label: label.into(),
            enabled: true,
        }
    }

    pub fn selected_icon(mut self, icon: impl Into<String>) -> Self {
        self.selected_icon = Some(icon.into());
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Material 3 destination group for placing inside a [`super::Drawer`].
pub struct NavigationDrawer {
    binding: GestureBinding,
    destinations: Vec<NavigationDrawerDestination>,
    selected_index: Option<usize>,
    on_destination_selected: Rc<dyn Fn(usize)>,
    header: Option<Widget>,
    footer: Option<Widget>,
    width: f32,
    theme: ThemeData,
}

impl NavigationDrawer {
    pub fn new(
        binding: &GestureBinding,
        destinations: Vec<NavigationDrawerDestination>,
        selected_index: Option<usize>,
        on_destination_selected: impl Fn(usize) + 'static,
    ) -> Self {
        Self {
            binding: binding.clone(),
            destinations,
            selected_index,
            on_destination_selected: Rc::new(on_destination_selected),
            header: None,
            footer: None,
            width: 280.0,
            theme: ThemeData::default(),
        }
    }

    pub fn header(mut self, header: Widget) -> Self {
        self.header = Some(header);
        self
    }

    pub fn footer(mut self, footer: Widget) -> Self {
        self.footer = Some(footer);
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn theme(mut self, theme: ThemeData) -> Self {
        self.theme = theme;
        self
    }

    pub fn into_widget(self) -> Result<Widget, &'static str> {
        if !self.width.is_finite()
            || self.width <= 24.0
            || self
                .selected_index
                .is_some_and(|index| index >= self.destinations.len())
        {
            return Err("Material navigation drawer has invalid width or selected index");
        }

        let colors = self.theme.color_scheme;
        let mut children = Vec::with_capacity(self.destinations.len() + 3);
        if let Some(header) = self.header {
            children.push(FlexItem::fixed(Widget::padding(
                Insets {
                    left: 16.0,
                    top: 20.0,
                    right: 16.0,
                    bottom: 16.0,
                },
                header,
            )));
        }
        let on_selected = self.on_destination_selected;
        for (index, destination) in self.destinations.into_iter().enumerate() {
            let selected = self.selected_index == Some(index);
            let icon = if selected {
                destination
                    .selected_icon
                    .as_ref()
                    .unwrap_or(&destination.icon)
            } else {
                &destination.icon
            };
            let foreground = if !destination.enabled {
                colors.on_surface.gamma_multiply(0.38)
            } else if selected {
                colors.on_secondary_container
            } else {
                colors.on_surface_variant
            };
            let label =
                egui::RichText::new(format!("{icon}    {}", destination.label)).color(foreground);
            let callback = on_selected.clone();
            let button = GestureButton::new(&self.binding, label, move || callback(index))
                .size(Vec2::new(self.width - 24.0, 56.0))
                .enabled(destination.enabled)
                .accessibility_role(GestureSemanticRole::Button)
                .accessibility_label(destination.label.clone())
                .accessibility_selected(selected)
                .fill(if selected {
                    colors.secondary_container
                } else {
                    Color32::TRANSPARENT
                })
                .corner_radius(egui::CornerRadius::same(16))
                .into_widget();
            children.push(FlexItem::fixed(button));
        }
        if let Some(footer) = self.footer {
            children.push(FlexItem::expanded(Widget::empty(), 1));
            children.push(FlexItem::fixed(Widget::padding(
                Insets {
                    left: 16.0,
                    top: 12.0,
                    right: 16.0,
                    bottom: 16.0,
                },
                footer,
            )));
        }
        let mut style = FlexStyle::new(Axis::Vertical);
        style.spacing = 4.0;
        style.cross_alignment = crate::layout::CrossAxisAlignment::Center;
        let contents = Widget::flex(style, children);
        let constraints = crate::layout::BoxConstraints::tight_for(Some(self.width), None)
            .expect("validated finite navigation drawer width");
        Ok(Widget::constrained(
            constraints,
            Widget::padding(Insets::all(8.0), contents),
        ))
    }
}

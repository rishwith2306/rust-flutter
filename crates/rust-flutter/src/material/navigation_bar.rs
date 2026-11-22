use super::ThemeData;
use crate::{
    decoration::{BoxDecoration, DecoratedBox},
    egui::{self, CornerRadius},
    element::{BuildContext, FlexItem, FlexStyle, StatelessWidget, Widget},
    gestures::{GestureBinding, GestureButton},
    layout::{Axis, BoxConstraints, CrossAxisAlignment},
};
use std::rc::Rc;

/// Text glyphs supplied by the app stand in for Material icons until the icon
/// and asset catalog is available. The label remains part of the button text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavigationDestination {
    pub icon: String,
    pub selected_icon: Option<String>,
    pub label: String,
}

impl NavigationDestination {
    pub fn new(icon: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            icon: icon.into(),
            selected_icon: None,
            label: label.into(),
        }
    }

    pub fn selected_icon(mut self, icon: impl Into<String>) -> Self {
        self.selected_icon = Some(icon.into());
        self
    }
}

/// A controlled Material bottom navigation bar.
/// The owner updates `selected_index` when `on_destination_selected` fires.
pub struct NavigationBar {
    binding: GestureBinding,
    destinations: Vec<NavigationDestination>,
    selected_index: usize,
    on_destination_selected: Rc<dyn Fn(usize)>,
}

impl NavigationBar {
    pub fn new(
        binding: &GestureBinding,
        destinations: Vec<NavigationDestination>,
        selected_index: usize,
        on_destination_selected: impl Fn(usize) + 'static,
    ) -> Self {
        Self {
            binding: binding.clone(),
            destinations,
            selected_index,
            on_destination_selected: Rc::new(on_destination_selected),
        }
    }

    pub fn into_widget(self) -> Widget {
        Widget::stateless(self)
    }
}

impl StatelessWidget for NavigationBar {
    fn build(&self, context: &BuildContext) -> Widget {
        let scheme = ThemeData::of(context)
            .expect("Material navigation bar resolves theme during build")
            .color_scheme;
        let mut children = Vec::with_capacity(self.destinations.len());
        for (index, destination) in self.destinations.iter().enumerate() {
            let selected = index == self.selected_index;
            let icon = if selected {
                destination
                    .selected_icon
                    .as_deref()
                    .unwrap_or(&destination.icon)
            } else {
                &destination.icon
            };
            let (fill, foreground) = if selected {
                (scheme.secondary_container, scheme.on_secondary_container)
            } else {
                (scheme.surface_container, scheme.on_surface_variant)
            };
            let activate = self.on_destination_selected.clone();
            let button = GestureButton::new(
                &self.binding,
                egui::RichText::new(format!("{icon}\n{}", destination.label)).color(foreground),
                move || activate(index),
            )
            .size(egui::vec2(96.0, 64.0))
            .fill(fill)
            .stroke(egui::Stroke::NONE)
            .corner_radius(CornerRadius::same(16));
            children.push(FlexItem::expanded(button.into_widget(), 1));
        }
        let mut style = FlexStyle::new(Axis::Horizontal);
        style.cross_alignment = CrossAxisAlignment::Center;
        let row = Widget::flex(style, children);
        let height = BoxConstraints::tight_for(None, Some(80.0))
            .expect("finite navigation bar height is valid");
        DecoratedBox::new(BoxDecoration::solid(scheme.surface_container))
            .into_widget(Widget::constrained(height, row))
    }
}

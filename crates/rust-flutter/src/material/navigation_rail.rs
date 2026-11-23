use super::{NavigationDestination, ThemeData};
use crate::{
    decoration::{BoxDecoration, DecoratedBox},
    egui::{self, CornerRadius},
    element::{BuildContext, FlexItem, FlexStyle, StatelessWidget, Widget},
    gestures::{GestureBinding, GestureButton, GestureSemanticRole},
    layout::{Axis, BoxConstraints, CrossAxisAlignment},
};
use std::rc::Rc;

/// A controlled Material navigation rail for side-by-side desktop layouts.
/// The owner updates `selected_index` when `on_destination_selected` fires.
pub struct NavigationRail {
    binding: GestureBinding,
    destinations: Vec<NavigationDestination>,
    selected_index: usize,
    extended: bool,
    on_destination_selected: Rc<dyn Fn(usize)>,
}

impl NavigationRail {
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
            extended: false,
            on_destination_selected: Rc::new(on_destination_selected),
        }
    }

    /// Show each destination label beside its icon and widen the rail.
    pub fn extended(mut self, extended: bool) -> Self {
        self.extended = extended;
        self
    }

    pub fn into_widget(self) -> Widget {
        Widget::stateless(self)
    }
}

impl StatelessWidget for NavigationRail {
    fn build(&self, context: &BuildContext) -> Widget {
        let scheme = ThemeData::of(context)
            .expect("Material navigation rail resolves theme during build")
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
            let label = if self.extended {
                format!("{icon}  {}", destination.label)
            } else {
                format!("{icon}\n{}", destination.label)
            };
            let activate = self.on_destination_selected.clone();
            let button = GestureButton::new(
                &self.binding,
                egui::RichText::new(label).color(foreground),
                move || activate(index),
            )
            .size(egui::vec2(if self.extended { 184.0 } else { 72.0 }, 64.0))
            .accessibility_role(GestureSemanticRole::Tab)
            .accessibility_label(destination.label.clone())
            .accessibility_selected(selected)
            .fill(fill)
            .stroke(egui::Stroke::NONE)
            .corner_radius(CornerRadius::same(16));
            children.push(FlexItem::fixed(button.into_widget()));
        }
        let mut style = FlexStyle::new(Axis::Vertical);
        style.cross_alignment = CrossAxisAlignment::Center;
        let rail = Widget::flex(style, children);
        let width = if self.extended { 200.0 } else { 80.0 };
        let constraints = BoxConstraints::tight_for(Some(width), None)
            .expect("finite navigation rail width is valid");
        DecoratedBox::new(BoxDecoration::solid(scheme.surface_container))
            .into_widget(Widget::constrained(constraints, rail))
    }
}

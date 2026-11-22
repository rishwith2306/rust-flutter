//! Material themed text entry backed by the retained form field implementation.
use crate::{
    element::{BuildContext, StatelessWidget, Widget},
    forms::{TextEditingController, TextFormField},
};

/// A Material text input. The editable text, selection and focus are retained
/// by the underlying form field; colors resolve from the nearest [`ThemeData`].
///
/// This first version supports the common label, hint and text-entry behavior.
/// Full Flutter `InputDecoration` features such as prefix/suffix widgets,
/// counter rows, floating-label animation and density options are not included.
pub struct TextField {
    field: TextFormField,
}

impl TextField {
    pub fn new(initial: impl Into<String>) -> Self {
        Self {
            field: TextFormField::new(initial),
        }
    }

    pub fn with_controller(controller: TextEditingController) -> Self {
        Self {
            field: TextFormField::with_controller(controller),
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.field.decoration.label = Some(label.into());
        self
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.field.decoration.hint = hint.into();
        self
    }

    pub fn on_changed(mut self, callback: impl Fn(&str) + 'static) -> Self {
        self.field = self.field.on_changed(callback);
        self
    }

    pub fn on_submitted(mut self, callback: impl Fn(&str) + 'static) -> Self {
        self.field = self.field.on_submitted(callback);
        self
    }

    pub fn read_only(mut self, value: bool) -> Self {
        self.field = self.field.read_only(value);
        self
    }

    pub fn enabled(mut self, value: bool) -> Self {
        self.field = self.field.enabled(value);
        self
    }

    pub fn obscure_text(mut self, value: bool) -> Self {
        self.field = self.field.obscure_text(value);
        self
    }

    pub fn multiline(mut self, min_lines: usize, max_lines: Option<usize>) -> Self {
        self.field = self.field.multiline(min_lines, max_lines);
        self
    }

    pub fn max_chars(mut self, value: Option<usize>) -> Self {
        self.field = self.field.max_chars(value);
        self
    }

    pub fn preferred_width(mut self, width: f32) -> Self {
        self.field = self.field.preferred_width(width);
        self
    }

    pub fn into_widget(self) -> Result<Widget, crate::forms::TextEditingError> {
        // Validate at the API boundary. The adapter can then safely rebuild the
        // child whenever inherited theme data changes.
        self.field.clone().into_widget()?;
        Ok(Widget::stateless(MaterialTextField(self.field)))
    }
}

struct MaterialTextField(TextFormField);

impl StatelessWidget for MaterialTextField {
    fn build(&self, context: &BuildContext) -> Widget {
        let theme = ThemeData::of(context).unwrap_or_default();
        let scheme = theme.color_scheme;
        let mut field = self.0.clone();
        if field.decoration.text_color.is_none() {
            field.decoration.text_color = Some(scheme.on_surface);
        }
        if field.decoration.background_color.is_none() {
            field.decoration.background_color = Some(scheme.surface_container);
        }
        if field.decoration.error_color == crate::egui::Color32::LIGHT_RED {
            field.decoration.error_color = scheme.error;
        }
        // The original configuration was validated in `into_widget`; only the
        // theme-derived colors differ here.
        field.into_widget().unwrap_or_else(|_| Widget::empty())
    }
}

use super::ThemeData;

use super::*;
use crate::{
    egui,
    element::{BuildContext, InheritedNotifier, State, StateContext, StatefulWidget, Widget},
    focus::FocusNode,
    notifier::ValueListenable,
};
mod view;
#[derive(Clone, PartialEq)]
pub struct TextFieldDecoration {
    pub label: Option<String>,
    pub hint: String,
    pub text_color: Option<egui::Color32>,
    pub background_color: Option<egui::Color32>,
    pub error_color: egui::Color32,
}
impl Default for TextFieldDecoration {
    fn default() -> Self {
        Self {
            label: None,
            hint: String::new(),
            text_color: None,
            background_color: None,
            error_color: egui::Color32::LIGHT_RED,
        }
    }
}
#[derive(Clone, PartialEq)]
pub(super) struct InputStyle {
    pub decoration: TextFieldDecoration,
    pub read_only: bool,
    pub obscure: bool,
    pub multiline: bool,
    pub min_lines: usize,
    pub max_lines: Option<usize>,
    pub max_chars: Option<usize>,
    pub preferred_width: f32,
    pub autofocus: bool,
}
impl Default for InputStyle {
    fn default() -> Self {
        Self {
            decoration: TextFieldDecoration::default(),
            read_only: false,
            obscure: false,
            multiline: false,
            min_lines: 1,
            max_lines: Some(1),
            max_chars: None,
            preferred_width: 240.0,
            autofocus: false,
        }
    }
}
impl InputStyle {
    fn validate(&self) -> Result<(), TextEditingError> {
        if !self.preferred_width.is_finite()
            || self.preferred_width < 0.0
            || self.min_lines == 0
            || self.max_lines.is_some_and(|n| n < self.min_lines)
            || (!self.multiline && (self.min_lines != 1 || self.max_lines != Some(1)))
            || (self.multiline && self.obscure)
        {
            return Err(TextEditingError::InvalidConfiguration(
                "invalid text field width, line limits or obscured multiline configuration",
            ));
        }
        Ok(())
    }
}
type TextCallback = Rc<dyn Fn(&str)>;
/// Backend-neutral form integration rendered by egui; this is not Material's
/// InputDecorator or a complete Flutter EditableText implementation.
pub struct TextFormField {
    pub controller: Option<TextEditingController>,
    pub initial_value: Option<String>,
    pub decoration: TextFieldDecoration,
    pub enabled: bool,
    pub read_only: bool,
    pub obscure_text: bool,
    pub multiline: bool,
    pub min_lines: usize,
    pub max_lines: Option<usize>,
    pub max_chars: Option<usize>,
    pub preferred_width: f32,
    pub autofocus: bool,
    pub autovalidate_mode: AutovalidateMode,
    pub force_error_text: Option<String>,
    validator: Option<Validator<String>>,
    on_saved: Option<TextCallback>,
    on_changed: Option<TextCallback>,
    on_submitted: Option<TextCallback>,
    on_reset: Option<Listener>,
}
impl Default for TextFormField {
    fn default() -> Self {
        Self {
            controller: None,
            initial_value: None,
            decoration: TextFieldDecoration::default(),
            enabled: true,
            read_only: false,
            obscure_text: false,
            multiline: false,
            min_lines: 1,
            max_lines: Some(1),
            max_chars: None,
            preferred_width: 240.0,
            autofocus: false,
            autovalidate_mode: AutovalidateMode::Disabled,
            force_error_text: None,
            validator: None,
            on_saved: None,
            on_changed: None,
            on_submitted: None,
            on_reset: None,
        }
    }
}
impl TextFormField {
    pub fn new(initial: impl Into<String>) -> Self {
        Self {
            initial_value: Some(initial.into()),
            ..Self::default()
        }
    }
    pub fn with_controller(controller: TextEditingController) -> Self {
        Self {
            controller: Some(controller),
            ..Self::default()
        }
    }
    pub fn validator(mut self, validator: impl Fn(&String) -> Option<String> + 'static) -> Self {
        self.validator = Some(Rc::new(validator));
        self
    }
    pub fn on_saved(mut self, callback: impl Fn(&str) + 'static) -> Self {
        self.on_saved = Some(Rc::new(callback));
        self
    }
    pub fn on_changed(mut self, callback: impl Fn(&str) + 'static) -> Self {
        self.on_changed = Some(Rc::new(callback));
        self
    }
    pub fn on_submitted(mut self, callback: impl Fn(&str) + 'static) -> Self {
        self.on_submitted = Some(Rc::new(callback));
        self
    }
    pub fn on_reset(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_reset = Some(Rc::new(callback));
        self
    }
    pub fn decoration(mut self, decoration: TextFieldDecoration) -> Self {
        self.decoration = decoration;
        self
    }
    pub fn preferred_width(mut self, width: f32) -> Self {
        self.preferred_width = width;
        self
    }
    pub fn multiline(mut self, min_lines: usize, max_lines: Option<usize>) -> Self {
        self.multiline = true;
        self.min_lines = min_lines;
        self.max_lines = max_lines;
        self
    }
    pub fn read_only(mut self, value: bool) -> Self {
        self.read_only = value;
        self
    }
    pub fn enabled(mut self, value: bool) -> Self {
        self.enabled = value;
        self
    }
    pub fn obscure_text(mut self, value: bool) -> Self {
        self.obscure_text = value;
        self
    }
    pub fn max_chars(mut self, value: Option<usize>) -> Self {
        self.max_chars = value;
        self
    }
    pub fn autofocus(mut self, value: bool) -> Self {
        self.autofocus = value;
        self
    }
    fn style(&self) -> InputStyle {
        InputStyle {
            decoration: self.decoration.clone(),
            read_only: self.read_only,
            obscure: self.obscure_text,
            multiline: self.multiline,
            min_lines: self.min_lines,
            max_lines: self.max_lines,
            max_chars: self.max_chars,
            preferred_width: self.preferred_width,
            autofocus: self.autofocus,
        }
    }
    pub fn into_widget(self) -> Result<Widget, TextEditingError> {
        self.style().validate()?;
        if self.controller.is_some() && self.initial_value.is_some() {
            return Err(TextEditingError::InvalidConfiguration(
                "specify a controller or an initial value, not both",
            ));
        }
        if self
            .controller
            .as_ref()
            .is_some_and(TextEditingController::is_disposed)
        {
            return Err(TextEditingError::Disposed);
        }
        Ok(Widget::stateful(self))
    }
}
#[derive(Clone, Copy)]
enum Direction {
    Editor(bool),
    Field,
}
pub(super) struct Binding {
    editor: RefCell<TextEditingController>,
    field: RefCell<Option<FormFieldController<String>>>,
    subscriptions: RefCell<Vec<Subscription>>,
    seen_editor: RefCell<String>,
    seen_field: RefCell<String>,
    pending: Cell<Option<Direction>>,
    busy: Cell<bool>,
    failure: RefCell<Option<TextEditingError>>,
    changed: ChangeNotifier,
}
impl Binding {
    fn new(editor: TextEditingController) -> Rc<Self> {
        Rc::new(Self {
            seen_editor: RefCell::new(editor.text()),
            editor: RefCell::new(editor),
            field: RefCell::new(None),
            subscriptions: RefCell::new(vec![]),
            seen_field: RefCell::new(String::new()),
            pending: Cell::new(None),
            busy: Cell::new(false),
            failure: RefCell::new(None),
            changed: ChangeNotifier::new(),
        })
    }
    fn editor(&self) -> TextEditingController {
        self.editor.borrow().clone()
    }
    fn fail(&self, error: Option<TextEditingError>) {
        if *self.failure.borrow() != error {
            self.failure.replace(error);
            let _ = self.changed.notify_listeners();
        }
    }
    fn bind(self: &Rc<Self>, field: FormFieldController<String>) {
        let same = self
            .field
            .borrow()
            .as_ref()
            .is_some_and(|old| old.same_source(&field));
        if same {
            return;
        }
        self.detach();
        self.seen_field.replace(field.current_value());
        self.field.replace(Some(field));
        self.subscribe();
        self.request(Direction::Editor(false));
    }
    fn detach(&self) {
        let old = std::mem::take(&mut *self.subscriptions.borrow_mut());
        drop(old);
        self.field.borrow_mut().take();
        self.pending.set(None);
    }
    fn subscribe(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        let editor = self.editor();
        let sub = editor.subscribe(Rc::new(move || {
            if let Some(b) = weak.upgrade() {
                if b.editor().is_disposed() {
                    b.fail(Some(TextEditingError::Disposed));
                    return;
                }
                let text = b.editor().text();
                if *b.seen_editor.borrow() != text {
                    b.seen_editor.replace(text);
                    b.request(Direction::Editor(true));
                }
            }
        }));
        if let Ok(sub) = sub {
            self.subscriptions.borrow_mut().push(sub);
        } else {
            self.fail(Some(TextEditingError::Disposed));
        }
        let field = self.field.borrow().clone();
        if let Some(field) = field {
            let weak = Rc::downgrade(self);
            let sub = field
                .subscribe(Rc::new(move || {
                    if let Some(b) = weak.upgrade() {
                        let field = b.field.borrow().clone();
                        if let Some(field) = field {
                            let text = field.current_value();
                            if *b.seen_field.borrow() != text {
                                b.seen_field.replace(text);
                                b.request(Direction::Field);
                            }
                        }
                    }
                }))
                .expect("live form model");
            self.subscriptions.borrow_mut().push(sub);
        }
    }
    fn replace_editor(self: &Rc<Self>, editor: TextEditingController) {
        let old = std::mem::take(&mut *self.subscriptions.borrow_mut());
        drop(old);
        self.seen_editor.replace(editor.text());
        self.editor.replace(editor);
        self.subscribe();
        self.request(Direction::Editor(false));
        let _ = self.changed.notify_listeners();
    }
    fn request(&self, direction: Direction) {
        self.pending.set(Some(direction));
        if self.busy.get() {
            return;
        }
        let _guard = Guard::enter(&self.busy).expect("single binding dispatcher");
        self.fail(None);
        for _ in 0..32 {
            let Some(direction) = self.pending.take() else {
                return;
            };
            let field = self.field.borrow().clone();
            let Some(field) = field else {
                return;
            };
            let editor = self.editor();
            if editor.is_disposed() {
                self.fail(Some(TextEditingError::Disposed));
                return;
            }
            match direction {
                Direction::Editor(interacted) => {
                    let text = editor.text();
                    if field.current_value() != text {
                        self.seen_field.replace(text.clone());
                        if interacted {
                            let _ = field.did_change(text);
                        } else {
                            field.replace_bound_value(text);
                        }
                    }
                }
                Direction::Field => {
                    let text = field.current_value();
                    if editor.text() != text {
                        self.seen_editor.replace(text.clone());
                        if let Err(error) = editor.set_text(text) {
                            self.fail(Some(error));
                            return;
                        }
                    }
                }
            }
        }
        if self.pending.take().is_some() {
            self.fail(Some(TextEditingError::Synchronization));
        }
    }
    fn reset_editor(&self, fallback: &str) {
        self.fail(None);
        let field = self.field.borrow().clone();
        let text = field.map_or_else(|| fallback.to_owned(), |f| f.current_value());
        self.seen_editor.replace(text.clone());
        if let Err(error) = self.editor().set_text(text) {
            self.fail(Some(error));
        }
    }
    fn error(&self) -> Option<String> {
        if let Some(error) = self.failure.borrow().as_ref() {
            return Some(error.to_string());
        }
        let field = self.field.borrow().clone()?;
        let value = field.snapshot();
        value
            .error_text
            .or_else(|| value.validation_failure.map(|e| e.to_string()))
    }
    fn call(&self, name: &'static str, callback_fn: &Option<TextCallback>) {
        if let Some(cb) = callback_fn {
            let text = self.editor().text();
            if let Err(error) = callback(name, || cb(&text)) {
                self.fail(Some(TextEditingError::Form(error)));
            }
        }
    }
}
pub struct TextFormFieldState {
    binding: Rc<Binding>,
    owned: bool,
}
impl TextFormFieldState {
    pub fn editing_controller(&self) -> TextEditingController {
        self.binding.editor()
    }
    pub fn field_controller(&self) -> Option<FormFieldController<String>> {
        self.binding.field.borrow().clone()
    }
}
impl StatefulWidget for TextFormField {
    type State = TextFormFieldState;
    fn create_state(&self) -> Self::State {
        TextFormFieldState {
            binding: Binding::new(self.controller.clone().unwrap_or_else(|| {
                TextEditingController::new(self.initial_value.clone().unwrap_or_default())
            })),
            owned: self.controller.is_none(),
        }
    }
}
impl State<TextFormField> for TextFormFieldState {
    fn did_update_widget(
        &mut self,
        old: &TextFormField,
        new: &TextFormField,
        _: &StateContext<Self>,
    ) {
        let previous = self.binding.editor();
        let next = if let Some(editor) = &new.controller {
            (!previous.same_source(editor)).then(|| editor.clone())
        } else if old.controller.is_some() {
            Some(
                TextEditingController::from_value(previous.value())
                    .expect("existing ranges are valid"),
            )
        } else {
            None
        };
        if let Some(next) = next {
            self.binding.replace_editor(next);
            if self.owned {
                let _ = previous.dispose();
            }
        }
        self.owned = new.controller.is_none();
    }
    fn build(&mut self, widget: &TextFormField, context: &StateContext<Self>) -> Widget {
        let initial = widget.controller.as_ref().map_or_else(
            || widget.initial_value.clone().unwrap_or_default(),
            TextEditingController::text,
        );
        let binding = self.binding.clone();
        let style = widget.style();
        let changed = widget.on_changed.clone();
        let submitted = widget.on_submitted.clone();
        let focus = context
            .depend_on::<InheritedNotifier<FocusNode>>()
            .ok()
            .flatten()
            .and_then(|provider| provider.notifier.clone());
        let focus_scope = context
            .depend_on::<InheritedNotifier<crate::focus::FocusScopeNode>>()
            .ok()
            .flatten()
            .and_then(|provider| provider.notifier.clone());
        let mut field = FormField::new(initial.clone(), move |_: &BuildContext, field| {
            binding.bind(field.clone());
            Widget::render_object(
                view::TextView {
                    binding: binding.clone(),
                    field,
                    style: style.clone(),
                    focus: focus.clone(),
                    focus_scope: focus_scope.clone(),
                    on_changed: changed.clone(),
                    on_submitted: submitted.clone(),
                },
                vec![],
            )
        });
        field.enabled = widget.enabled;
        field.autovalidate_mode = widget.autovalidate_mode;
        field.force_error_text = widget.force_error_text.clone();
        let weak = Rc::downgrade(&self.binding);
        let validator = widget.validator.clone();
        field = field.validator(move |v| {
            if let Some(binding) = weak.upgrade()
                && let Some(error) = binding.failure.borrow().as_ref()
            {
                return Some(error.to_string());
            }
            validator.as_ref().and_then(|validate| validate(v))
        });
        if let Some(save) = widget.on_saved.clone() {
            field = field.on_saved(move |text| save(text));
        }
        let weak = Rc::downgrade(&self.binding);
        let on_reset = widget.on_reset.clone();
        let on_changed = widget.on_changed.clone();
        field = field.on_reset(move || {
            if let Some(binding) = weak.upgrade() {
                binding.reset_editor(&initial);
                binding.call("text change", &on_changed);
            }
            if let Some(reset) = &on_reset {
                reset();
            }
        });
        field.into_widget()
    }
    fn dispose(&mut self) {
        self.binding.detach();
        if self.owned {
            let _ = self.binding.editor().dispose();
        }
    }
}

use super::{field::Settings, *};
use crate::element::{BuildContext, DependencyError, State, StateContext, StatefulWidget, Widget};
#[derive(Clone)]
struct FormScope(FormController);
/// A retained scope whose descendant fields register with this controller.
#[derive(Clone, Default)]
pub struct Form {
    pub controller: FormController,
    pub autovalidate_mode: Option<AutovalidateMode>,
}
impl Form {
    pub fn new(controller: FormController) -> Self {
        Self {
            controller,
            autovalidate_mode: None,
        }
    }
    pub fn into_widget(self, child: Widget) -> Widget {
        if let Some(mode) = self.autovalidate_mode {
            self.controller.set_autovalidate_mode(mode);
        }
        Widget::inherited_with(
            FormScope(self.controller),
            |old, new| !old.0.same_source(&new.0),
            child,
        )
    }
    pub fn maybe_of(context: &BuildContext) -> Result<Option<FormController>, DependencyError> {
        Ok(context
            .depend_on::<FormScope>()?
            .map(|scope| scope.0.clone()))
    }
}
type Builder<T> = Rc<dyn Fn(&BuildContext, FormFieldController<T>) -> Widget>;
/// Retains typed field state; the builder adapts it to an application's control.
pub struct FormField<T: Clone + 'static> {
    pub initial_value: T,
    pub autovalidate_mode: AutovalidateMode,
    pub enabled: bool,
    pub force_error_text: Option<String>,
    validator: Option<Validator<T>>,
    on_saved: Option<field::Save<T>>,
    on_reset: Option<Listener>,
    builder: Builder<T>,
}
impl<T: Clone + 'static> FormField<T> {
    pub fn new(
        initial_value: T,
        builder: impl Fn(&BuildContext, FormFieldController<T>) -> Widget + 'static,
    ) -> Self {
        Self {
            initial_value,
            autovalidate_mode: AutovalidateMode::Disabled,
            enabled: true,
            force_error_text: None,
            validator: None,
            on_saved: None,
            on_reset: None,
            builder: Rc::new(builder),
        }
    }
    pub fn validator(mut self, validator: impl Fn(&T) -> Option<String> + 'static) -> Self {
        self.validator = Some(Rc::new(validator));
        self
    }
    pub fn on_saved(mut self, save: impl Fn(&T) + 'static) -> Self {
        self.on_saved = Some(Rc::new(save));
        self
    }
    pub fn on_reset(mut self, reset: impl Fn() + 'static) -> Self {
        self.on_reset = Some(Rc::new(reset));
        self
    }
    pub fn into_widget(self) -> Widget {
        Widget::stateful(self)
    }
    fn settings(&self) -> Settings<T> {
        Settings {
            initial: self.initial_value.clone(),
            validator: self.validator.clone(),
            on_saved: self.on_saved.clone(),
            on_reset: self.on_reset.clone(),
            mode: self.autovalidate_mode,
            enabled: self.enabled,
            force_error: self.force_error_text.clone(),
        }
    }
}
pub struct FormFieldState<T: Clone + 'static> {
    controller: FormFieldController<T>,
    form: Option<FormController>,
    registration: Option<FormRegistration>,
}
impl<T: Clone + 'static> StatefulWidget for FormField<T> {
    type State = FormFieldState<T>;
    fn create_state(&self) -> Self::State {
        FormFieldState {
            controller: FormFieldController::from_settings(self.settings()),
            form: None,
            registration: None,
        }
    }
}
impl<T: Clone + 'static> State<FormField<T>> for FormFieldState<T> {
    fn did_update_widget(&mut self, _: &FormField<T>, new: &FormField<T>, _: &StateContext<Self>) {
        self.controller.configure(new.settings());
    }
    fn build(&mut self, widget: &FormField<T>, context: &StateContext<Self>) -> Widget {
        let form = Form::maybe_of(context).expect("form lookup during build");
        let same = match (&self.form, &form) {
            (None, None) => true,
            (Some(a), Some(b)) => a.same_source(b),
            _ => false,
        };
        if !same {
            self.registration.take();
            self.registration = form.as_ref().map(|form| form.register(&self.controller));
            self.form = form;
        }
        let builder = widget.builder.clone();
        Widget::listenable_builder(self.controller.clone(), move |context, field| {
            let _ = field.validate_on_build();
            builder(context, field.clone())
        })
    }
    fn dispose(&mut self) {
        self.registration.take();
        self.form.take();
    }
}

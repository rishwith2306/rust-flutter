use super::*;
use crate::notifier::ValueListenable;

pub type Validator<T> = Rc<dyn Fn(&T) -> Option<String>>;
pub(super) type Save<T> = Rc<dyn Fn(&T)>;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AutovalidateMode {
    #[default]
    Disabled,
    Always,
    OnUserInteraction,
    OnUnfocus,
    OnUserInteractionIfError,
}
#[derive(Clone, Debug, PartialEq)]
pub struct FormFieldValue<T> {
    pub value: T,
    pub error_text: Option<String>,
    pub has_interacted_by_user: bool,
    pub enabled: bool,
    pub validation_failure: Option<FormError>,
}
#[derive(Clone)]
pub(super) struct Settings<T> {
    pub initial: T,
    pub validator: Option<Validator<T>>,
    pub on_saved: Option<Save<T>>,
    pub on_reset: Option<Listener>,
    pub mode: AutovalidateMode,
    pub enabled: bool,
    pub force_error: Option<String>,
}
pub(super) struct FieldCore<T> {
    data: RefCell<FormFieldValue<T>>,
    settings: RefCell<Settings<T>>,
    revision: Cell<u64>,
    busy: Cell<bool>,
    notifier: ChangeNotifier,
    events: ChangeNotifier,
    event_stack: RefCell<Vec<FieldEvent>>,
}
#[derive(Clone)]
pub struct FormFieldController<T: Clone + 'static>(pub(super) Rc<FieldCore<T>>);
#[derive(Clone)]
pub struct WeakFormFieldController<T: Clone + 'static>(Weak<FieldCore<T>>);
impl<T: Clone + 'static> WeakFormFieldController<T> {
    pub fn upgrade(&self) -> Option<FormFieldController<T>> {
        self.0.upgrade().map(FormFieldController)
    }
}
impl<T: Clone + 'static> FormFieldController<T> {
    pub fn new(initial: T) -> Self {
        Self::from_settings(Settings {
            initial,
            validator: None,
            on_saved: None,
            on_reset: None,
            mode: AutovalidateMode::Disabled,
            enabled: true,
            force_error: None,
        })
    }
    pub(super) fn from_settings(settings: Settings<T>) -> Self {
        Self(Rc::new(FieldCore {
            data: RefCell::new(FormFieldValue {
                value: settings.initial.clone(),
                error_text: settings.force_error.clone(),
                has_interacted_by_user: false,
                enabled: settings.enabled,
                validation_failure: None,
            }),
            settings: RefCell::new(settings),
            revision: Cell::new(0),
            busy: Cell::new(false),
            notifier: ChangeNotifier::new(),
            events: ChangeNotifier::new(),
            event_stack: RefCell::new(vec![]),
        }))
    }
    pub(super) fn subscribe_form_events(&self, listener: Rc<dyn Fn(FieldEvent)>) -> Subscription {
        let weak = Rc::downgrade(&self.0);
        self.0
            .events
            .subscribe(Rc::new(move || {
                if let Some(core) = weak.upgrade() {
                    let event = core.event_stack.borrow().last().copied();
                    if let Some(event) = event {
                        listener(event);
                    }
                }
            }))
            .expect("live field event source")
    }
    pub fn downgrade(&self) -> WeakFormFieldController<T> {
        WeakFormFieldController(Rc::downgrade(&self.0))
    }
    pub fn snapshot(&self) -> FormFieldValue<T> {
        self.0.data.borrow().clone()
    }
    pub fn current_value(&self) -> T {
        self.0.data.borrow().value.clone()
    }
    pub fn error_text(&self) -> Option<String> {
        self.0.data.borrow().error_text.clone()
    }
    pub fn has_interacted_by_user(&self) -> bool {
        self.0.data.borrow().has_interacted_by_user
    }
    pub fn set_validator(&self, validator: Option<Validator<T>>) {
        let old = std::mem::replace(&mut self.0.settings.borrow_mut().validator, validator);
        self.0.bump();
        drop(old);
        self.0.event(FieldEvent::Configured);
    }
    pub(super) fn configure(&self, settings: Settings<T>) {
        let force_changed = self.0.settings.borrow().force_error != settings.force_error;
        self.0.data.borrow_mut().enabled = settings.enabled;
        if force_changed {
            self.0.data.borrow_mut().error_text = settings.force_error.clone();
        }
        let old = self.0.settings.replace(settings);
        self.0.bump();
        drop(old);
        self.0.notify();
        self.0.event(FieldEvent::Configured);
    }
    pub(super) fn replace_bound_value(&self, value: T) {
        let old = std::mem::replace(&mut self.0.data.borrow_mut().value, value);
        self.0.bump();
        drop(old);
        self.0.notify();
        self.0.event(FieldEvent::Configured);
    }
    /// A user edit marks interaction even when the new value compares equal.
    pub fn did_change(&self, value: T) -> Result<(), FormError> {
        let old = {
            let mut data = self.0.data.borrow_mut();
            data.has_interacted_by_user = true;
            data.validation_failure = None;
            std::mem::replace(&mut data.value, value)
        };
        self.0.bump();
        drop(old);
        self.0.event(FieldEvent::Edited);
        if self.0.auto_validate(false) {
            let result = self.validate().map(|_| ());
            if matches!(result, Err(FormError::Reentrant | FormError::Superseded)) {
                self.0.notify();
            }
            result
        } else {
            self.0.notify();
            Ok(())
        }
    }
    /// Adapters call this when their control loses focus.
    pub fn did_unfocus(&self) -> Result<(), FormError> {
        self.0.event(FieldEvent::Unfocused);
        if self.0.auto_validate(true) {
            self.validate().map(|_| ())
        } else {
            Ok(())
        }
    }
    pub fn validate(&self) -> Result<bool, FormError> {
        self.0.validate()
    }
    /// Checks validity without publishing an error or changing interaction state.
    pub fn is_valid(&self) -> Result<bool, FormError> {
        self.0.check(false, false)
    }
    pub fn save(&self) -> Result<(), FormError> {
        self.0.save()
    }
    pub fn reset(&self) -> Result<(), FormError> {
        self.0.reset(false)
    }
    pub fn clear_error(&self) {
        self.0.clear_error();
    }
    pub(super) fn validate_on_build(&self) -> Result<(), FormError> {
        if self.0.auto_validate(false) {
            self.0.check(true, false).map(|_| ())
        } else {
            Ok(())
        }
    }
}
impl<T: Clone + 'static> FieldCore<T> {
    fn event(&self, event: FieldEvent) {
        self.event_stack.borrow_mut().push(event);
        let _ = self.events.notify_listeners();
        self.event_stack.borrow_mut().pop();
    }
    fn bump(&self) {
        self.revision.set(self.revision.get().wrapping_add(1));
    }
    fn notify(&self) {
        let _ = self.notifier.notify_listeners();
    }
    fn auto_validate(&self, unfocus: bool) -> bool {
        let s = self.settings.borrow();
        let d = self.data.borrow();
        s.enabled
            && match s.mode {
                AutovalidateMode::Disabled => false,
                AutovalidateMode::Always => !unfocus,
                AutovalidateMode::OnUserInteraction => !unfocus && d.has_interacted_by_user,
                AutovalidateMode::OnUnfocus => unfocus,
                AutovalidateMode::OnUserInteractionIfError => {
                    !unfocus && d.has_interacted_by_user && d.error_text.is_some()
                }
            }
    }
    fn check(&self, publish: bool, notify_unchanged: bool) -> Result<bool, FormError> {
        let _guard = Guard::enter(&self.busy)?;
        let revision = self.revision.get();
        let (forced, validator) = {
            let s = self.settings.borrow();
            (s.force_error.clone(), s.validator.clone())
        };
        let value = self.data.borrow().value.clone();
        let error = if forced.is_some() {
            forced
        } else if let Some(validator) = validator {
            match callback("validator", || validator(&value)) {
                Ok(error) => error,
                Err(error) => {
                    if publish && revision == self.revision.get() {
                        let changed = self.data.borrow().validation_failure != Some(error);
                        self.data.borrow_mut().validation_failure = Some(error);
                        if changed || notify_unchanged {
                            self.notify();
                        }
                    }
                    return Err(error);
                }
            }
        } else {
            None
        };
        if revision != self.revision.get() {
            return Err(FormError::Superseded);
        }
        let valid = error.is_none();
        if publish {
            let changed = {
                let mut data = self.data.borrow_mut();
                let changed = data.error_text != error || data.validation_failure.is_some();
                data.error_text = error;
                data.validation_failure = None;
                changed
            };
            if changed || notify_unchanged {
                self.notify();
            }
            if revision != self.revision.get() {
                return Err(FormError::Superseded);
            }
        }
        Ok(valid)
    }
}
impl<T: Clone + 'static> FieldOps for FieldCore<T> {
    fn validate_auto(&self) -> Result<bool, FormError> {
        self.check(true, false)
    }
    fn interacted(&self) -> bool {
        self.data.borrow().has_interacted_by_user
    }
    fn has_error(&self) -> bool {
        self.data.borrow().error_text.is_some()
    }
    fn enabled(&self) -> bool {
        self.settings.borrow().enabled
    }
    fn revision(&self) -> u64 {
        self.revision.get()
    }
    fn validate(&self) -> Result<bool, FormError> {
        self.check(true, true)
    }
    fn save(&self) -> Result<(), FormError> {
        let _guard = Guard::enter(&self.busy)?;
        let save = self.settings.borrow().on_saved.clone();
        if let Some(save) = save {
            let value = self.data.borrow().value.clone();
            callback("save", || save(&value))?;
        }
        Ok(())
    }
    fn reset(&self, defer_auto: bool) -> Result<(), FormError> {
        let _guard = Guard::enter(&self.busy)?;
        let (initial, on_reset) = {
            let s = self.settings.borrow();
            (s.initial.clone(), s.on_reset.clone())
        };
        let old = {
            let mut d = self.data.borrow_mut();
            d.error_text = None;
            d.validation_failure = None;
            d.has_interacted_by_user = false;
            std::mem::replace(&mut d.value, initial)
        };
        self.bump();
        drop(old);
        let revision = self.revision.get();
        let result = if let Some(on_reset) = on_reset {
            callback("reset", || on_reset())
        } else {
            Ok(())
        };
        self.notify();
        result?;
        if revision != self.revision.get() {
            return Err(FormError::Superseded);
        }
        drop(_guard);
        self.event(FieldEvent::Reset);
        if defer_auto {
            Ok(())
        } else {
            self.after_reset()
        }
    }
    fn after_reset(&self) -> Result<(), FormError> {
        if self.auto_validate(false) {
            self.validate().map(|_| ())
        } else {
            Ok(())
        }
    }
    fn clear_error(&self) {
        let mut data = self.data.borrow_mut();
        data.error_text = None;
        data.validation_failure = None;
        data.has_interacted_by_user = false;
        drop(data);
        self.notify();
        self.event(FieldEvent::Cleared);
    }
}
impl<T: Clone + 'static> Listenable for FormFieldController<T> {
    fn subscribe(&self, listener: Listener) -> Result<Subscription, Disposed> {
        self.0.notifier.subscribe(listener)
    }
    fn same_source(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl<T: Clone + 'static> ValueListenable for FormFieldController<T> {
    type Value = FormFieldValue<T>;
    fn value(&self) -> Self::Value {
        self.snapshot()
    }
}

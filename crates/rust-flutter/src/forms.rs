//! Typed form state, owned field registrations and retained form scopes.
mod automatic;
use automatic::{AutoRequest, FieldEvent};
mod field;
mod widgets;
use crate::notifier::{ChangeNotifier, Disposed, Listenable, Listener, Subscription};
pub use field::{
    AutovalidateMode, FormFieldController, FormFieldValue, Validator, WeakFormFieldController,
};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
pub use widgets::{Form, FormField, FormFieldState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormError {
    Reentrant,
    Superseded,
    CallbackPanicked(&'static str),
}
impl std::fmt::Display for FormError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reentrant => f.write_str("a form operation is already running"),
            Self::Superseded => f.write_str("form data or membership changed during the operation"),
            Self::CallbackPanicked(name) => write!(f, "form {name} callback panicked"),
        }
    }
}
impl std::error::Error for FormError {}
pub(super) struct Guard<'a>(&'a Cell<bool>);
impl<'a> Guard<'a> {
    pub fn enter(busy: &'a Cell<bool>) -> Result<Self, FormError> {
        if busy.replace(true) {
            Err(FormError::Reentrant)
        } else {
            Ok(Self(busy))
        }
    }
}
impl Drop for Guard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
pub(super) fn callback<T>(name: &'static str, f: impl FnOnce() -> T) -> Result<T, FormError> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f))
        .map_err(|_| FormError::CallbackPanicked(name))
}
trait FieldOps {
    fn validate(&self) -> Result<bool, FormError>;
    fn validate_auto(&self) -> Result<bool, FormError>;
    fn interacted(&self) -> bool;
    fn has_error(&self) -> bool;
    fn enabled(&self) -> bool;
    fn save(&self) -> Result<(), FormError>;
    fn reset(&self, defer_auto: bool) -> Result<(), FormError>;
    fn after_reset(&self) -> Result<(), FormError>;
    fn clear_error(&self);
    fn revision(&self) -> u64;
}
enum FormCommand {
    Save,
    Reset,
    ClearErrors,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FormFieldId(u64);
struct Entry {
    id: FormFieldId,
    field: Weak<dyn FieldOps>,
    requests: usize,
    _subscription: Subscription,
    _events: Subscription,
}
struct FormInner {
    entries: RefCell<Vec<Entry>>,
    next: Cell<u64>,
    epoch: Cell<u64>,
    busy: Cell<bool>,
    notifier: ChangeNotifier,
    mode: Cell<AutovalidateMode>,
    interacted: Cell<bool>,
    pending: RefCell<Option<AutoRequest>>,
    auto_running: Cell<bool>,
    auto_failure: Cell<Option<FormError>>,
}
#[derive(Clone)]
pub struct FormController(Rc<FormInner>);
impl Default for FormController {
    fn default() -> Self {
        Self::new()
    }
}
impl FormController {
    pub fn new() -> Self {
        Self(Rc::new(FormInner {
            entries: RefCell::new(vec![]),
            next: Cell::new(0),
            epoch: Cell::new(0),
            busy: Cell::new(false),
            notifier: ChangeNotifier::new(),
            mode: Cell::new(AutovalidateMode::Disabled),
            interacted: Cell::new(false),
            pending: RefCell::new(None),
            auto_running: Cell::new(false),
            auto_failure: Cell::new(None),
        }))
    }
    pub fn field_count(&self) -> usize {
        self.0
            .entries
            .borrow()
            .iter()
            .filter(|e| e.field.strong_count() > 0)
            .count()
    }
    pub fn field_id<T: Clone + 'static>(
        &self,
        field: &FormFieldController<T>,
    ) -> Option<FormFieldId> {
        let erased: Rc<dyn FieldOps> = field.0.clone();
        let source = Rc::downgrade(&erased);
        self.0
            .entries
            .borrow()
            .iter()
            .find(|entry| entry.field.ptr_eq(&source))
            .map(|entry| entry.id)
    }
    /// The returned guard owns membership; the form itself does not retain the field.
    pub fn register<T: Clone + 'static>(&self, field: &FormFieldController<T>) -> FormRegistration {
        let erased: Rc<dyn FieldOps> = field.0.clone();
        let source = Rc::downgrade(&erased);
        {
            let mut entries = self.0.entries.borrow_mut();
            if let Some(entry) = entries.iter_mut().find(|e| e.field.ptr_eq(&source)) {
                entry.requests = entry
                    .requests
                    .checked_add(1)
                    .expect("form registration count exhausted");
                return FormRegistration {
                    form: Rc::downgrade(&self.0),
                    id: entry.id,
                };
            }
        }
        let id = FormFieldId(self.0.next.get());
        self.0
            .next
            .set(id.0.checked_add(1).expect("form field ID space exhausted"));
        let weak = Rc::downgrade(&self.0);
        let subscription = field
            .subscribe(Rc::new(move || {
                if let Some(form) = weak.upgrade() {
                    let _ = form.notifier.notify_listeners();
                }
            }))
            .expect("form field source is live");
        let weak = Rc::downgrade(&self.0);
        let events = field.subscribe_form_events(Rc::new(move |event| {
            if let Some(form) = weak.upgrade() {
                FormController(form).field_event(Some(id), event);
            }
        }));
        self.0.entries.borrow_mut().push(Entry {
            id,
            field: source,
            requests: 1,
            _subscription: subscription,
            _events: events,
        });
        self.0.epoch.set(self.0.epoch.get().wrapping_add(1));
        self.field_event(Some(id), FieldEvent::Membership);
        let _ = self.0.notifier.notify_listeners();
        FormRegistration {
            form: Rc::downgrade(&self.0),
            id,
        }
    }
    fn snapshot(&self) -> Vec<(FormFieldId, Rc<dyn FieldOps>)> {
        self.0
            .entries
            .borrow()
            .iter()
            .filter_map(|e| e.field.upgrade().map(|f| (e.id, f)))
            .collect()
    }
    fn contains(&self, id: FormFieldId) -> bool {
        self.0.entries.borrow().iter().any(|e| e.id == id)
    }
    pub fn validate(&self) -> Result<bool, FormError> {
        Ok(self.validate_granularly()?.is_empty())
    }
    pub fn validate_granularly(&self) -> Result<Vec<FormFieldId>, FormError> {
        if self.0.busy.get() || self.0.auto_running.get() {
            return Err(FormError::Reentrant);
        }
        self.0.interacted.set(true);
        self.validate_internal(false, None)
    }
    fn validate_internal(
        &self,
        automatic: bool,
        targets: Option<&[FormFieldId]>,
    ) -> Result<Vec<FormFieldId>, FormError> {
        let _guard = Guard::enter(&self.0.busy)?;
        let epoch = self.0.epoch.get();
        let fields = self.snapshot();
        let revisions: Vec<_> = fields.iter().map(|(_, field)| field.revision()).collect();
        let mut invalid = vec![];
        for (id, field) in &fields {
            if self.contains(*id)
                && (!automatic || field.enabled())
                && targets.is_none_or(|ids| ids.contains(id))
            {
                let valid = if automatic {
                    field.validate_auto()?
                } else {
                    field.validate()?
                };
                if !valid {
                    invalid.push(*id);
                }
            }
        }
        if epoch != self.0.epoch.get()
            || fields
                .iter()
                .zip(revisions)
                .any(|((_, f), r)| f.revision() != r)
        {
            return Err(FormError::Superseded);
        }
        Ok(invalid)
    }
    fn command(&self, command: FormCommand) -> Result<(), FormError> {
        if self.0.auto_running.get() {
            return Err(FormError::Reentrant);
        }
        let _guard = Guard::enter(&self.0.busy)?;
        let epoch = self.0.epoch.get();
        let fields = self.snapshot();
        for (id, field) in &fields {
            if self.contains(*id) {
                match command {
                    FormCommand::Save => field.save()?,
                    FormCommand::Reset => field.reset(true)?,
                    FormCommand::ClearErrors => field.clear_error(),
                }
            }
        }
        if matches!(command, FormCommand::Reset) {
            for (id, field) in &fields {
                if self.contains(*id) {
                    field.after_reset()?;
                }
            }
        }
        if epoch != self.0.epoch.get() {
            return Err(FormError::Superseded);
        }
        drop(_guard);
        self.flush_validation()
    }
    /// Saving does not request validation; deferred automatic requests may flush.
    pub fn save(&self) -> Result<(), FormError> {
        self.command(FormCommand::Save)
    }
    pub fn reset(&self) -> Result<(), FormError> {
        self.command(FormCommand::Reset)
    }
    pub fn clear_errors(&self) -> Result<(), FormError> {
        self.command(FormCommand::ClearErrors)
    }
}
impl Listenable for FormController {
    fn subscribe(&self, listener: Listener) -> Result<Subscription, Disposed> {
        self.0.notifier.subscribe(listener)
    }
    fn same_source(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
#[must_use = "dropping the registration removes the field from its form"]
pub struct FormRegistration {
    form: Weak<FormInner>,
    id: FormFieldId,
}
impl FormRegistration {
    pub fn id(&self) -> FormFieldId {
        self.id
    }
}
impl Drop for FormRegistration {
    fn drop(&mut self) {
        if let Some(form) = self.form.upgrade() {
            let removed = {
                let mut entries = form.entries.borrow_mut();
                if let Some(index) = entries.iter().position(|e| e.id == self.id) {
                    entries[index].requests -= 1;
                    if entries[index].requests == 0 {
                        Some(entries.remove(index))
                    } else {
                        None
                    }
                } else {
                    None
                }
            };
            if removed.is_some() {
                form.epoch.set(form.epoch.get().wrapping_add(1));
            }
            let changed = removed.is_some();
            drop(removed);
            if changed {
                FormController(form.clone()).field_event(Some(self.id), FieldEvent::Membership);
                let _ = form.notifier.notify_listeners();
            }
        }
    }
}

use super::*;
use crate::notifier::ValueListenable;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextAffinity {
    Upstream,
    #[default]
    Downstream,
}
/// Offsets are UTF-16 code units; ranges cannot split surrogate pairs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextRange {
    pub start: usize,
    pub end: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextSelection {
    pub base_offset: usize,
    pub extent_offset: usize,
    pub affinity: TextAffinity,
}
impl TextSelection {
    pub fn collapsed(offset: usize) -> Self {
        Self {
            base_offset: offset,
            extent_offset: offset,
            affinity: TextAffinity::Downstream,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextEditingValue {
    pub text: String,
    pub selection: Option<TextSelection>,
    pub composing: Option<TextRange>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextEditingError {
    Disposed,
    InvalidOffset,
    InvalidComposingRange,
    InvalidConfiguration(&'static str),
    Form(FormError),
    Synchronization,
}
impl std::fmt::Display for TextEditingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disposed => f.write_str("text editing controller is disposed"),
            Self::InvalidOffset => {
                f.write_str("text offset is outside the string or splits a UTF-16 surrogate pair")
            }
            Self::InvalidComposingRange => {
                f.write_str("composing range must be ordered and within the text")
            }
            Self::InvalidConfiguration(message) => f.write_str(message),
            Self::Form(error) => error.fmt(f),
            Self::Synchronization => f.write_str("text synchronization did not settle"),
        }
    }
}
impl std::error::Error for TextEditingError {}
pub(super) fn utf16_to_char(text: &str, offset: usize) -> Result<usize, TextEditingError> {
    let mut utf16 = 0;
    for (i, c) in text.chars().enumerate() {
        if utf16 == offset {
            return Ok(i);
        }
        utf16 += c.len_utf16();
        if utf16 > offset {
            return Err(TextEditingError::InvalidOffset);
        }
    }
    if utf16 == offset {
        Ok(text.chars().count())
    } else {
        Err(TextEditingError::InvalidOffset)
    }
}
pub(super) fn char_to_utf16(text: &str, offset: usize) -> usize {
    text.chars().take(offset).map(char::len_utf16).sum()
}
impl TextEditingValue {
    pub fn validate(&self) -> Result<(), TextEditingError> {
        if let Some(s) = self.selection {
            utf16_to_char(&self.text, s.base_offset)?;
            utf16_to_char(&self.text, s.extent_offset)?;
        }
        if let Some(r) = self.composing
            && (r.start > r.end
                || utf16_to_char(&self.text, r.start).is_err()
                || utf16_to_char(&self.text, r.end).is_err())
        {
            return Err(TextEditingError::InvalidComposingRange);
        }
        Ok(())
    }
}
struct EditingCore {
    value: RefCell<TextEditingValue>,
    revision: Cell<u64>,
    disposed: Cell<bool>,
    disposing: Cell<bool>,
    notifier: ChangeNotifier,
}
#[derive(Clone)]
pub struct TextEditingController(Rc<EditingCore>);
#[derive(Clone)]
pub struct WeakTextEditingController(Weak<EditingCore>);
impl WeakTextEditingController {
    pub fn upgrade(&self) -> Option<TextEditingController> {
        self.0.upgrade().map(TextEditingController)
    }
}
impl Default for TextEditingController {
    fn default() -> Self {
        Self::new(String::new())
    }
}
impl TextEditingController {
    pub fn new(text: impl Into<String>) -> Self {
        Self::from_value(TextEditingValue {
            text: text.into(),
            ..Default::default()
        })
        .expect("string with no ranges is valid")
    }
    pub fn from_value(value: TextEditingValue) -> Result<Self, TextEditingError> {
        value.validate()?;
        Ok(Self(Rc::new(EditingCore {
            value: RefCell::new(value),
            revision: Cell::new(0),
            disposed: Cell::new(false),
            disposing: Cell::new(false),
            notifier: ChangeNotifier::new(),
        })))
    }
    pub fn text(&self) -> String {
        self.0.value.borrow().text.clone()
    }
    pub fn revision(&self) -> u64 {
        self.0.revision.get()
    }
    pub fn is_disposed(&self) -> bool {
        self.0.disposed.get()
    }
    pub fn downgrade(&self) -> WeakTextEditingController {
        WeakTextEditingController(Rc::downgrade(&self.0))
    }
    pub fn set_value(&self, value: TextEditingValue) -> Result<bool, TextEditingError> {
        if self.is_disposed() {
            return Err(TextEditingError::Disposed);
        }
        value.validate()?;
        if *self.0.value.borrow() == value {
            return Ok(false);
        }
        let old = self.0.value.replace(value);
        self.0.revision.set(self.revision().wrapping_add(1));
        drop(old);
        let _ = self.0.notifier.notify_listeners();
        if self.is_disposed() {
            let _ = self.0.notifier.dispose();
            return Err(TextEditingError::Disposed);
        }
        Ok(true)
    }
    /// Replace text and clear selection/composition. Use set_value for an atomic edit.
    pub fn set_text(&self, text: impl Into<String>) -> Result<bool, TextEditingError> {
        self.set_value(TextEditingValue {
            text: text.into(),
            ..Default::default()
        })
    }
    pub fn set_selection(
        &self,
        selection: Option<TextSelection>,
    ) -> Result<bool, TextEditingError> {
        let mut v = self.value();
        v.selection = selection;
        self.set_value(v)
    }
    pub fn clear_composing(&self) -> Result<bool, TextEditingError> {
        let mut v = self.value();
        v.composing = None;
        self.set_value(v)
    }
    pub fn clear(&self) -> Result<bool, TextEditingError> {
        self.set_text(String::new())
    }
    /// Final notification observes disposed=true. Cleanup can finish after an
    /// in-flight dispatch; remaining ordinary listeners are skipped.
    pub fn dispose(&self) -> Result<(), TextEditingError> {
        if self.0.disposed.replace(true) {
            return Err(TextEditingError::Disposed);
        }
        let guard = Guard::enter(&self.0.disposing).expect("one disposal dispatch");
        let _ = self.0.notifier.notify_listeners();
        drop(guard);
        let _ = self.0.notifier.dispose();
        Ok(())
    }
}
impl Listenable for TextEditingController {
    fn subscribe(&self, listener: Listener) -> Result<Subscription, Disposed> {
        if self.is_disposed() {
            return Err(Disposed);
        }
        let weak = Rc::downgrade(&self.0);
        self.0.notifier.subscribe(Rc::new(move || {
            if let Some(core) = weak.upgrade()
                && (!core.disposed.get() || core.disposing.get())
            {
                listener();
            }
        }))
    }
    fn same_source(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl ValueListenable for TextEditingController {
    type Value = TextEditingValue;
    fn value(&self) -> Self::Value {
        self.0.value.borrow().clone()
    }
}

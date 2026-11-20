use super::*;
#[derive(Clone, Copy)]
pub(super) enum FieldEvent {
    Edited,
    Unfocused,
    Reset,
    Cleared,
    Configured,
    Membership,
}
pub(super) enum AutoRequest {
    All,
    Fields(Vec<FormFieldId>),
}
impl FormController {
    pub fn autovalidate_mode(&self) -> AutovalidateMode {
        self.0.mode.get()
    }
    pub fn has_interacted_by_user(&self) -> bool {
        self.0.interacted.get()
    }
    pub fn last_auto_failure(&self) -> Option<FormError> {
        self.0.auto_failure.get()
    }
    pub fn set_autovalidate_mode(&self, mode: AutovalidateMode) {
        let changed = self.0.mode.replace(mode) != mode;
        if changed {
            self.0.epoch.set(self.0.epoch.get().wrapping_add(1));
        }
        self.0.pending.borrow_mut().take();
        if !matches!(
            mode,
            AutovalidateMode::Disabled | AutovalidateMode::OnUnfocus
        ) {
            self.queue_auto(AutoRequest::All);
            let _ = self.flush_validation();
        }
        if changed {
            let _ = self.0.notifier.notify_listeners();
        }
    }
    fn queue_auto(&self, request: AutoRequest) {
        let mut pending = self.0.pending.borrow_mut();
        match (&mut *pending, request) {
            (Some(AutoRequest::All), _) => {}
            (_, AutoRequest::All) => *pending = Some(AutoRequest::All),
            (Some(AutoRequest::Fields(ids)), AutoRequest::Fields(next)) => {
                for id in next {
                    if !ids.contains(&id) {
                        ids.push(id);
                    }
                }
            }
            (slot, request) => *slot = Some(request),
        }
    }
    pub(super) fn field_event(&self, id: Option<FormFieldId>, event: FieldEvent) {
        if matches!(
            event,
            FieldEvent::Edited | FieldEvent::Reset | FieldEvent::Cleared
        ) {
            self.0
                .interacted
                .set(self.snapshot().iter().any(|(_, field)| field.interacted()));
        }
        if matches!(event, FieldEvent::Membership) {
            if self.field_count() == 0 {
                self.0.interacted.set(false);
            } else if self.snapshot().iter().any(|(_, field)| field.interacted()) {
                self.0.interacted.set(true);
            }
        }
        match self.0.mode.get() {
            AutovalidateMode::Disabled => return,
            AutovalidateMode::OnUnfocus => {
                if !matches!(event, FieldEvent::Unfocused) {
                    return;
                }
                if let Some(id) = id {
                    self.queue_auto(AutoRequest::Fields(vec![id]));
                }
            }
            _ => {
                if matches!(event, FieldEvent::Unfocused) {
                    return;
                }
                self.queue_auto(AutoRequest::All);
            }
        }
        if !self.0.busy.get() && !self.0.auto_running.get() {
            let _ = self.flush_validation();
        }
    }
    /// Run at most one pending automatic pass. Reentrant edits stay pending for
    /// the next event or explicit flush; validators cannot cause a synchronous loop.
    pub fn flush_validation(&self) -> Result<(), FormError> {
        if self.0.busy.get() || self.0.auto_running.get() {
            return Err(FormError::Reentrant);
        }
        let Some(request) = self.0.pending.borrow_mut().take() else {
            return Ok(());
        };
        let mode = self.0.mode.get();
        let eligible = match mode {
            AutovalidateMode::Disabled => false,
            AutovalidateMode::Always | AutovalidateMode::OnUnfocus => true,
            AutovalidateMode::OnUserInteraction => self.0.interacted.get(),
            AutovalidateMode::OnUserInteractionIfError => {
                self.0.interacted.get() && self.snapshot().iter().any(|(_, f)| f.has_error())
            }
        };
        if !eligible {
            return Ok(());
        }
        let _running = Guard::enter(&self.0.auto_running)?;
        let targets = match &request {
            AutoRequest::All => None,
            AutoRequest::Fields(ids) => Some(ids.as_slice()),
        };
        let result = self.validate_internal(true, targets).map(|_| ());
        let failure = result.as_ref().err().copied();
        if self.0.auto_failure.replace(failure) != failure {
            let _ = self.0.notifier.notify_listeners();
        }
        result
    }
}

use super::SnackBar;
use crate::element::{State, StateContext, StateHandle, StatefulWidget, Widget};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    time::Duration,
};

type MessengerBuilder = dyn Fn(ScaffoldMessengerController, Option<SnackBar>) -> Widget + 'static;

/// Retained host for transient Material snackbars. The builder receives the
/// current snackbar and can place it in a `Scaffold`.
pub struct ScaffoldMessengerHost {
    builder: Rc<MessengerBuilder>,
    current: Rc<RefCell<Option<SnackBar>>>,
    duration: Duration,
}

impl ScaffoldMessengerHost {
    pub fn new(
        builder: impl Fn(ScaffoldMessengerController, Option<SnackBar>) -> Widget + 'static,
    ) -> Self {
        Self {
            builder: Rc::new(builder),
            current: Rc::new(RefCell::new(None)),
            duration: Duration::from_secs(4),
        }
    }

    /// Set the display duration for messages shown by the controller.
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    pub fn into_widget(self) -> Widget {
        Widget::stateful(self)
    }
}

#[doc(hidden)]
pub struct MessengerState {
    visible: bool,
    expires_at: Option<f64>,
}

#[derive(Clone)]
pub struct ScaffoldMessengerController {
    state: StateHandle<MessengerState>,
    current: Weak<RefCell<Option<SnackBar>>>,
    context: egui::Context,
    duration: Duration,
}

impl ScaffoldMessengerController {
    /// Display a snackbar, replacing the currently visible message.
    pub fn show_snack_bar(&self, snack_bar: SnackBar) -> Result<(), crate::element::Unmounted> {
        let current = self.current.upgrade().ok_or(crate::element::Unmounted)?;
        let previous = current.borrow_mut().replace(snack_bar);
        let now = self.context.input(|input| input.time);
        let duration = self.duration;
        if let Err(error) = self.state.update(move |state| {
            state.visible = true;
            state.expires_at = Some(now + duration.as_secs_f64());
        }) {
            *current.borrow_mut() = previous;
            return Err(error);
        }
        self.context.request_repaint_after(duration);
        Ok(())
    }

    pub fn hide_current_snack_bar(&self) -> Result<(), crate::element::Unmounted> {
        let current = self.current.upgrade().ok_or(crate::element::Unmounted)?;
        let previous = current.borrow_mut().take();
        if let Err(error) = self.state.update(|state| {
            state.visible = false;
            state.expires_at = None;
        }) {
            *current.borrow_mut() = previous;
            return Err(error);
        }
        Ok(())
    }

    pub fn is_snack_bar_visible(&self) -> bool {
        self.current
            .upgrade()
            .is_some_and(|current| current.borrow().is_some())
    }
}

impl StatefulWidget for ScaffoldMessengerHost {
    type State = MessengerState;

    fn create_state(&self) -> Self::State {
        MessengerState {
            visible: false,
            expires_at: None,
        }
    }
}

impl State<ScaffoldMessengerHost> for MessengerState {
    fn build(&mut self, widget: &ScaffoldMessengerHost, context: &StateContext<Self>) -> Widget {
        let now = context.egui().input(|input| input.time);
        if self.expires_at.is_some_and(|deadline| now >= deadline) {
            self.visible = false;
            self.expires_at = None;
            widget.current.borrow_mut().take();
        }
        let controller = ScaffoldMessengerController {
            state: context.handle(),
            current: Rc::downgrade(&widget.current),
            context: context.egui().clone(),
            duration: widget.duration,
        };
        let current = self
            .visible
            .then(|| widget.current.borrow().clone())
            .flatten();
        (widget.builder)(controller, current)
    }
}

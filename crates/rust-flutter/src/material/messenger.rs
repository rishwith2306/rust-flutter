use super::SnackBar;
use crate::element::{State, StateContext, StateHandle, StatefulWidget, Widget};
use std::{
    cell::RefCell,
    collections::VecDeque,
    rc::{Rc, Weak},
    time::Duration,
};

type MessengerBuilder = dyn Fn(ScaffoldMessengerController, Option<SnackBar>) -> Widget + 'static;

#[derive(Default)]
struct MessengerData {
    current: Option<SnackBar>,
    queue: VecDeque<SnackBar>,
}

/// Retained host for transient Material snackbars. The builder receives the
/// current snackbar and can place it in a `Scaffold`.
pub struct ScaffoldMessengerHost {
    builder: Rc<MessengerBuilder>,
    data: Rc<RefCell<MessengerData>>,
    duration: Duration,
}

impl ScaffoldMessengerHost {
    pub fn new(
        builder: impl Fn(ScaffoldMessengerController, Option<SnackBar>) -> Widget + 'static,
    ) -> Self {
        Self {
            builder: Rc::new(builder),
            data: Rc::new(RefCell::new(MessengerData::default())),
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
    data: Weak<RefCell<MessengerData>>,
    context: egui::Context,
    duration: Duration,
}

impl ScaffoldMessengerController {
    /// Queue a snackbar after the currently visible and already queued messages.
    pub fn show_snack_bar(&self, snack_bar: SnackBar) -> Result<(), crate::element::Unmounted> {
        let data = self.data.upgrade().ok_or(crate::element::Unmounted)?;
        let has_current = data.borrow().current.is_some();
        let now = self.context.input(|input| input.time);
        let duration = self.duration;
        self.state.update(move |state| {
            if !has_current {
                state.visible = true;
                state.expires_at = Some(now + duration.as_secs_f64());
            }
        })?;

        let mut data = data.borrow_mut();
        if data.current.is_some() {
            data.queue.push_back(snack_bar);
        } else {
            data.current = Some(snack_bar);
            self.context.request_repaint_after(duration);
        }
        Ok(())
    }

    pub fn hide_current_snack_bar(&self) -> Result<(), crate::element::Unmounted> {
        let data = self.data.upgrade().ok_or(crate::element::Unmounted)?;
        let has_next = !data.borrow().queue.is_empty();
        let now = self.context.input(|input| input.time);
        let duration = self.duration;
        self.state.update(move |state| {
            state.visible = has_next;
            state.expires_at = has_next.then_some(now + duration.as_secs_f64());
        })?;

        let mut data = data.borrow_mut();
        data.current = data.queue.pop_front();
        if has_next {
            self.context.request_repaint_after(duration);
        }
        Ok(())
    }

    pub fn is_snack_bar_visible(&self) -> bool {
        self.data
            .upgrade()
            .is_some_and(|data| data.borrow().current.is_some())
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
            let mut data = widget.data.borrow_mut();
            data.current = data.queue.pop_front();
            self.visible = data.current.is_some();
            self.expires_at = self.visible.then_some(now + widget.duration.as_secs_f64());
            if self.visible {
                context.egui().request_repaint_after(widget.duration);
            }
        }
        let controller = ScaffoldMessengerController {
            state: context.handle(),
            data: Rc::downgrade(&widget.data),
            context: context.egui().clone(),
            duration: widget.duration,
        };
        let current = self
            .visible
            .then(|| widget.data.borrow().current.clone())
            .flatten();
        (widget.builder)(controller, current)
    }
}

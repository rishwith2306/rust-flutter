use super::{BuildContext, State, StateContext, StatefulWidget, Widget};
use crate::state::{Signal, SignalSubscription};

pub(super) struct SignalBuilder<T, F> {
    pub signal: Signal<T>,
    pub builder: F,
}

impl<T, F> StatefulWidget for SignalBuilder<T, F>
where
    T: Clone + Send + 'static,
    F: Fn(&BuildContext, T) -> Widget + 'static,
{
    type State = SignalBuilderState;

    fn create_state(&self) -> Self::State {
        SignalBuilderState { subscription: None }
    }
}

pub(super) struct SignalBuilderState {
    subscription: Option<SignalSubscription>,
}

impl<T, F> State<SignalBuilder<T, F>> for SignalBuilderState
where
    T: Clone + Send + 'static,
    F: Fn(&BuildContext, T) -> Widget + 'static,
{
    fn init_state(&mut self, widget: &SignalBuilder<T, F>, context: &StateContext<Self>) {
        self.listen(widget, context);
    }

    fn did_update_widget(
        &mut self,
        old: &SignalBuilder<T, F>,
        new: &SignalBuilder<T, F>,
        context: &StateContext<Self>,
    ) {
        if !old.signal.same_source(&new.signal) {
            self.listen(new, context);
        }
    }

    fn build(&mut self, widget: &SignalBuilder<T, F>, context: &StateContext<Self>) -> Widget {
        (widget.builder)(context, widget.signal.get())
    }

    fn dispose(&mut self) {
        self.subscription.take();
    }
}

impl SignalBuilderState {
    fn listen<T, F>(&mut self, widget: &SignalBuilder<T, F>, context: &StateContext<Self>)
    where
        T: Clone + Send + 'static,
        F: Fn(&BuildContext, T) -> Widget + 'static,
    {
        let handle = context.handle();
        self.subscription = Some(widget.signal.subscribe(move || {
            let _ = handle.request_rebuild();
        }));
    }
}

use super::{BuildContext, InheritedNotifier, StatelessWidget, Widget};
use crate::notifier::{Listenable, ValueListenable};
use std::marker::PhantomData;

struct ListenableBuild<N, F> {
    builder: F,
    marker: PhantomData<fn() -> N>,
}
impl<N: Listenable, F: Fn(&BuildContext, &N) -> Widget + 'static> StatelessWidget
    for ListenableBuild<N, F>
{
    fn build(&self, context: &BuildContext) -> Widget {
        let inherited = context
            .depend_on::<InheritedNotifier<N>>()
            .expect("mounted builder outside init_state")
            .expect("builder source provider");
        inherited
            .notifier
            .as_ref()
            .map_or_else(Widget::empty, |source| (self.builder)(context, source))
    }
}
impl Widget {
    /// Rebuild this subtree when source emits. Events coalesce per tree pass.
    pub fn listenable_builder<N: Listenable>(
        source: N,
        builder: impl Fn(&BuildContext, &N) -> Widget + 'static,
    ) -> Self {
        Self::inherited_notifier(
            Some(source),
            Self::stateless(ListenableBuild {
                builder,
                marker: PhantomData,
            }),
        )
    }
    /// Build with the latest value, supplied when the subtree actually rebuilds.
    pub fn value_listenable_builder<N: ValueListenable>(
        source: N,
        builder: impl Fn(&BuildContext, N::Value) -> Widget + 'static,
    ) -> Self {
        Self::listenable_builder(source, move |ctx, source| builder(ctx, source.value()))
    }
}

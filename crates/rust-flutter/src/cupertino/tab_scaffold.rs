use super::{CupertinoPageScaffold, CupertinoTabBar, CupertinoTabDestination, CupertinoThemeData};
use crate::{
    element::{BuildContext, StackItem, StackStyle, State, StateContext, StatefulWidget, Widget},
    gestures::GestureBinding,
    layout::{Clip, StackFit},
};
use std::rc::Rc;

type TabBuilder = dyn Fn(&BuildContext, usize) -> Widget;

/// A Cupertino tab root that retains each tab subtree while switching pages.
/// The owner supplies a builder for each tab; hidden tabs remain mounted and
/// their tickers are muted until selected again.
pub struct CupertinoTabScaffold {
    binding: GestureBinding,
    destinations: Vec<CupertinoTabDestination>,
    initial_index: usize,
    theme: CupertinoThemeData,
    tab_builder: Rc<TabBuilder>,
}

impl CupertinoTabScaffold {
    pub fn new(
        binding: &GestureBinding,
        destinations: Vec<CupertinoTabDestination>,
        initial_index: usize,
        tab_builder: impl Fn(&BuildContext, usize) -> Widget + 'static,
    ) -> Result<Self, &'static str> {
        if !(2..=5).contains(&destinations.len()) || initial_index >= destinations.len() {
            return Err(
                "Cupertino tab scaffold requires two to five tabs and a valid initial index",
            );
        }
        Ok(Self {
            binding: binding.clone(),
            destinations,
            initial_index,
            theme: CupertinoThemeData::default(),
            tab_builder: Rc::new(tab_builder),
        })
    }

    pub fn theme(mut self, theme: CupertinoThemeData) -> Self {
        self.theme = theme;
        self
    }

    pub fn into_widget(self) -> Widget {
        let theme = self.theme;
        theme.provide(Widget::stateful(self))
    }
}

pub struct CupertinoTabScaffoldState {
    current_index: usize,
}

impl StatefulWidget for CupertinoTabScaffold {
    type State = CupertinoTabScaffoldState;

    fn create_state(&self) -> Self::State {
        CupertinoTabScaffoldState {
            current_index: self.initial_index,
        }
    }
}

impl State<CupertinoTabScaffold> for CupertinoTabScaffoldState {
    fn did_update_widget(
        &mut self,
        _: &CupertinoTabScaffold,
        widget: &CupertinoTabScaffold,
        _: &StateContext<Self>,
    ) {
        if self.current_index >= widget.destinations.len() {
            self.current_index = widget.initial_index.min(widget.destinations.len() - 1);
        }
    }

    fn build(&mut self, widget: &CupertinoTabScaffold, context: &StateContext<Self>) -> Widget {
        let mut pages = Vec::with_capacity(widget.destinations.len());
        for index in 0..widget.destinations.len() {
            let page = (widget.tab_builder)(context, index);
            let page = Widget::ticker_mode(index == self.current_index, page);
            let page = Widget::offstage(index != self.current_index, page).keyed(index as u64);
            pages.push(StackItem::new(page));
        }
        let stack_style = StackStyle {
            fit: StackFit::Expand,
            clip: Clip::HardEdge,
            ..Default::default()
        };
        let body = Widget::stack(stack_style, pages);
        let handle = context.handle();
        let selected = self.current_index;
        let tab_bar = CupertinoTabBar::new(
            &widget.binding,
            widget.destinations.clone(),
            selected,
            move |next| {
                let _ = handle.update(move |state: &mut CupertinoTabScaffoldState| {
                    state.current_index = next;
                });
            },
        )
        .into_widget()
        .expect("tab scaffold validates the tab bar configuration");
        CupertinoPageScaffold::new(body)
            .tab_bar(tab_bar)
            .theme(widget.theme)
            .into_widget()
    }
}

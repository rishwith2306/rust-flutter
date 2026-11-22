use super::{CupertinoPageScaffold, CupertinoTabBar, CupertinoTabDestination, CupertinoThemeData};
use crate::{
    element::{BuildContext, StackItem, StackStyle, State, StateContext, StatefulWidget, Widget},
    gestures::GestureBinding,
    layout::{Clip, StackFit},
    navigation::{BackDispatcher, BackSubscription, NavigatorController},
};
use std::{cell::Cell, rc::Rc};

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
    selected_index: Rc<Cell<usize>>,
    on_tab_changed: Option<Rc<dyn Fn(usize)>>,
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
            selected_index: Rc::new(Cell::new(initial_index)),
            on_tab_changed: None,
        })
    }

    /// Create a tab scaffold whose tabs each own an independent route stack.
    /// Returns the scaffold and a controller for pushing and popping tab routes.
    pub fn with_navigators<R>(
        binding: &GestureBinding,
        destinations: Vec<CupertinoTabDestination>,
        navigators: Vec<NavigatorController<R>>,
        initial_index: usize,
        page_builder: impl Fn(&BuildContext, usize, &R) -> Widget + 'static,
    ) -> Result<(Self, CupertinoTabNavigationController<R>), &'static str>
    where
        R: Clone + PartialEq + 'static,
    {
        if navigators.len() != destinations.len() {
            return Err("Cupertino tab scaffold requires one navigator per destination");
        }
        let navigation = CupertinoTabNavigationController::new(navigators, initial_index)?;
        let scaffold = Self::with_navigation_controller(
            binding,
            destinations,
            navigation.clone(),
            page_builder,
        )?;
        Ok((scaffold, navigation))
    }

    /// Build a per-tab route scaffold from a controller that can be retained
    /// across parent rebuilds.
    pub fn with_navigation_controller<R>(
        binding: &GestureBinding,
        destinations: Vec<CupertinoTabDestination>,
        navigation: CupertinoTabNavigationController<R>,
        page_builder: impl Fn(&BuildContext, usize, &R) -> Widget + 'static,
    ) -> Result<Self, &'static str>
    where
        R: Clone + PartialEq + 'static,
    {
        if navigation.navigators.len() != destinations.len() {
            return Err("Cupertino tab scaffold requires one navigator per destination");
        }
        let page_builder = Rc::new(page_builder);
        let tabs = navigation.navigators.clone();
        let pages = page_builder.clone();
        let scaffold = Self::new(
            binding,
            destinations,
            navigation.selected_index(),
            move |_context, index| {
                let Some(navigator) = tabs.get(index) else {
                    return Widget::empty();
                };
                let page_builder = pages.clone();
                navigator.retained_view(move |context, route| page_builder(context, index, route))
            },
        )?;
        let on_change_index = navigation.selected_index.clone();
        Ok(Self {
            selected_index: on_change_index.clone(),
            on_tab_changed: Some(Rc::new(move |index| on_change_index.set(index))),
            ..scaffold
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
        widget.selected_index.set(self.current_index);
        if self.current_index >= widget.destinations.len() {
            self.current_index = widget.initial_index.min(widget.destinations.len() - 1);
            widget.selected_index.set(self.current_index);
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
        let selected_index = widget.selected_index.clone();
        let on_tab_changed = widget.on_tab_changed.clone();
        let tab_bar = CupertinoTabBar::new(
            &widget.binding,
            widget.destinations.clone(),
            selected,
            move |next| {
                if handle
                    .update(move |state: &mut CupertinoTabScaffoldState| {
                        state.current_index = next;
                    })
                    .is_ok()
                {
                    selected_index.set(next);
                    if let Some(on_tab_changed) = &on_tab_changed {
                        on_tab_changed(next);
                    }
                }
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

/// Controls independent route stacks owned by a [`CupertinoTabScaffold`].
#[derive(Clone)]
pub struct CupertinoTabNavigationController<R: Clone + PartialEq + 'static> {
    navigators: Rc<Vec<NavigatorController<R>>>,
    selected_index: Rc<Cell<usize>>,
}

impl<R: Clone + PartialEq + 'static> CupertinoTabNavigationController<R> {
    pub fn new(
        navigators: Vec<NavigatorController<R>>,
        initial_index: usize,
    ) -> Result<Self, &'static str> {
        if !(2..=5).contains(&navigators.len()) || initial_index >= navigators.len() {
            return Err(
                "Cupertino tab navigation requires two to five navigators and a valid initial index",
            );
        }
        Ok(Self {
            navigators: Rc::new(navigators),
            selected_index: Rc::new(Cell::new(initial_index)),
        })
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index.get()
    }

    pub fn navigator(&self, tab_index: usize) -> Option<NavigatorController<R>> {
        self.navigators.get(tab_index).cloned()
    }

    pub fn current_navigator(&self) -> NavigatorController<R> {
        self.navigators[self.selected_index()].clone()
    }

    pub fn push(&self, route: R) -> Result<bool, crate::notifier::Disposed> {
        self.current_navigator().push(route)
    }

    pub fn push_on(
        &self,
        tab_index: usize,
        route: R,
    ) -> Option<Result<bool, crate::notifier::Disposed>> {
        self.navigators
            .get(tab_index)
            .map(|navigator| navigator.push(route))
    }

    pub fn pop(&self) -> Result<Option<R>, crate::notifier::Disposed> {
        self.current_navigator().pop()
    }

    /// Pop the active tab before other route handlers receive Back.
    pub fn register_back_handler(&self, dispatcher: &BackDispatcher) -> BackSubscription {
        let controller = self.clone();
        dispatcher.register_with_priority(BackDispatcher::ROUTE_PRIORITY, move || {
            controller.pop().is_ok_and(|popped| popped.is_some())
        })
    }
}

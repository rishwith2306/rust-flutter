use super::inherited::{InheritedModel, InheritedNotifier, Provided, ProviderDescription, Watch};
use super::render_object::{RenderDescription, RenderObjectWidget, RenderWidget};
use super::{Key, SafeAreaOptions, state::*};
use crate::{layout::*, notifier::Listenable};
use std::{cell::RefCell, rc::Rc};

pub(super) type Paint = Rc<RefCell<dyn FnMut(&mut egui::Ui)>>;
pub(super) type Measure = Rc<RefCell<dyn FnMut(BoxConstraints) -> Result<egui::Vec2, LayoutError>>>;
#[derive(Clone, Copy, Debug)]
pub struct FlexStyle {
    pub axis: Axis,
    pub main_size: MainAxisSize,
    pub main_alignment: MainAxisAlignment,
    pub cross_alignment: CrossAxisAlignment,
    pub text_direction: TextDirection,
    pub vertical_direction: VerticalDirection,
    pub spacing: f32,
    pub clip: Clip,
}
impl FlexStyle {
    pub fn new(axis: Axis) -> Self {
        Self {
            axis,
            main_size: MainAxisSize::Max,
            main_alignment: MainAxisAlignment::Start,
            cross_alignment: CrossAxisAlignment::Center,
            text_direction: TextDirection::Ltr,
            vertical_direction: VerticalDirection::Down,
            spacing: 0.0,
            clip: Clip::HardEdge,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct StackStyle {
    pub alignment: Alignment,
    pub fit: StackFit,
    pub clip: Clip,
}
impl Default for StackStyle {
    fn default() -> Self {
        Self {
            alignment: Alignment::TOP_LEFT,
            fit: StackFit::Loose,
            clip: Clip::HardEdge,
        }
    }
}
pub struct FlexItem {
    pub widget: Widget,
    pub sizing: FlexSizing,
}
impl FlexItem {
    pub fn fixed(widget: Widget) -> Self {
        Self {
            widget,
            sizing: FlexSizing::Fixed,
        }
    }
    pub fn expanded(widget: Widget, factor: u32) -> Self {
        Self {
            widget,
            sizing: FlexSizing::Flexible {
                factor,
                fit: FlexFit::Tight,
            },
        }
    }
    pub fn flexible(widget: Widget, factor: u32, fit: FlexFit) -> Self {
        Self {
            widget,
            sizing: FlexSizing::Flexible { factor, fit },
        }
    }
}
pub struct StackItem {
    pub widget: Widget,
    pub position: Positioned,
}
impl StackItem {
    pub fn new(widget: Widget) -> Self {
        Self {
            widget,
            position: Positioned::default(),
        }
    }
    pub fn positioned(widget: Widget, position: Positioned) -> Self {
        Self { widget, position }
    }
}
/// Declarative configuration. ElementTree owns persistent identity and state.
pub struct Widget {
    pub(super) key: Option<Key>,
    pub(super) kind: Kind,
    pub(super) children: Vec<Widget>,
}
pub(super) enum Kind {
    Leaf {
        preferred: egui::Vec2,
        measure: Option<Measure>,
        paint: Paint,
    },
    Padding(Insets),
    SafeArea(SafeAreaOptions),
    Constrained(BoxConstraints),
    Align {
        alignment: Alignment,
        width_factor: Option<f32>,
        height_factor: Option<f32>,
    },
    Flex(FlexStyle, Vec<FlexSizing>),
    Stack(StackStyle, Vec<Positioned>),
    Component(Box<dyn Description>),
    Inherited(Box<dyn ProviderDescription>),
    RenderObject(Box<dyn RenderDescription>),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Token {
    Leaf,
    Padding,
    SafeArea,
    Constrained,
    Align,
    Flex,
    Stack,
    Component(std::any::TypeId),
    Inherited(std::any::TypeId),
    RenderObject(std::any::TypeId),
}
impl Kind {
    pub fn token(&self) -> Token {
        match self {
            Self::Leaf { .. } => Token::Leaf,
            Self::Padding(_) => Token::Padding,
            Self::SafeArea(_) => Token::SafeArea,
            Self::Constrained(_) => Token::Constrained,
            Self::Align { .. } => Token::Align,
            Self::Flex(..) => Token::Flex,
            Self::Stack(..) => Token::Stack,
            Self::Component(d) => Token::Component(d.token()),
            Self::Inherited(d) => Token::Inherited(d.kind()),
            Self::RenderObject(d) => Token::RenderObject(d.token()),
        }
    }
    pub fn name(&self) -> &'static str {
        match self {
            Self::Leaf { .. } => "Leaf",
            Self::Padding(_) => "Padding",
            Self::SafeArea(_) => "SafeArea",
            Self::Constrained(_) => "Constrained",
            Self::Align { .. } => "Align",
            Self::Flex(..) => "Flex",
            Self::Stack(..) => "Stack",
            Self::Component(d) => d.name(),
            Self::Inherited(d) => d.name(),
            Self::RenderObject(d) => d.name(),
        }
    }
}
impl Widget {
    pub fn inherited<T: PartialEq + 'static>(value: T, child: Self) -> Self {
        Self::inherited_with(value, |old, new| old != new, child)
    }
    pub fn inherited_with<T: 'static>(
        value: T,
        should_notify: impl Fn(&T, &T) -> bool + 'static,
        child: Self,
    ) -> Self {
        Self::new(
            Kind::Inherited(Box::new(Provided {
                value,
                notify: Rc::new(should_notify),
                aspect_notify: None,
                watch: None,
            })),
            vec![child],
        )
    }
    /// Provide a model whose consumers can subscribe to selected aspects.
    pub fn inherited_model<T: InheritedModel>(value: T, child: Self) -> Self {
        Self::new(
            Kind::Inherited(Box::new(Provided {
                value,
                notify: Rc::new(|old, new: &T| new.update_should_notify(old)),
                aspect_notify: Some(Rc::new(|old, new: &T, aspects| {
                    new.update_should_notify_dependent(
                        old,
                        aspects.downcast_ref().expect("model aspect type"),
                    )
                })),
                watch: None,
            })),
            vec![child],
        )
    }
    /// Observe a UI-local source and coalesce its events into dependency rebuilds.
    pub fn inherited_notifier<N: Listenable>(notifier: Option<N>, child: Self) -> Self {
        Self::inherited_notifier_with(
            notifier,
            |old, new| match (old, new) {
                (Some(old), Some(new)) => !old.same_source(new),
                (None, None) => false,
                _ => true,
            },
            child,
        )
    }
    /// Customize identity-change notifications; source events always notify.
    pub fn inherited_notifier_with<N: Listenable>(
        notifier: Option<N>,
        should_notify: impl Fn(&Option<N>, &Option<N>) -> bool + 'static,
        child: Self,
    ) -> Self {
        Self::new(
            Kind::Inherited(Box::new(Provided {
                value: InheritedNotifier { notifier },
                notify: Rc::new(move |old: &InheritedNotifier<N>, new| {
                    should_notify(&old.notifier, &new.notifier)
                }),
                aspect_notify: None,
                watch: Some(Rc::new(Watch {
                    same: Rc::new(|old: &InheritedNotifier<N>, new| {
                        match (&old.notifier, &new.notifier) {
                            (Some(old), Some(new)) => old.same_source(new),
                            (None, None) => true,
                            _ => false,
                        }
                    }),
                    subscribe: Rc::new(|value: &InheritedNotifier<N>, listener| {
                        value
                            .notifier
                            .as_ref()
                            .map(|source| source.subscribe(listener))
                            .transpose()
                    }),
                })),
            })),
            vec![child],
        )
    }
    fn new(kind: Kind, children: Vec<Self>) -> Self {
        Self {
            key: None,
            kind,
            children,
        }
    }
    pub fn keyed(mut self, key: impl Into<Key>) -> Self {
        self.key = Some(key.into());
        self
    }
    pub fn leaf(preferred: egui::Vec2, paint: impl FnMut(&mut egui::Ui) + 'static) -> Self {
        Self::new(
            Kind::Leaf {
                preferred,
                measure: None,
                paint: Rc::new(RefCell::new(paint)),
            },
            vec![],
        )
    }
    pub fn empty() -> Self {
        Self::leaf(egui::Vec2::ZERO, |_| {})
    }
    /// Measure against the actual parent constraints during box layout.
    pub fn measured(
        measure: impl FnMut(BoxConstraints) -> Result<egui::Vec2, LayoutError> + 'static,
        paint: impl FnMut(&mut egui::Ui) + 'static,
    ) -> Self {
        Self::new(
            Kind::Leaf {
                preferred: egui::Vec2::ZERO,
                measure: Some(Rc::new(RefCell::new(measure))),
                paint: Rc::new(RefCell::new(paint)),
            },
            vec![],
        )
    }
    pub fn padding(insets: Insets, child: Self) -> Self {
        Self::new(Kind::Padding(insets), vec![child])
    }
    /// Pad around the inherited safe area and consume that padding below it.
    pub fn safe_area(child: Self) -> Self {
        Self::safe_area_with(SafeAreaOptions::default(), child)
    }
    pub fn safe_area_with(options: SafeAreaOptions, child: Self) -> Self {
        Self::new(Kind::SafeArea(options), vec![child])
    }
    pub fn constrained(constraints: BoxConstraints, child: Self) -> Self {
        Self::new(Kind::Constrained(constraints), vec![child])
    }
    pub fn align(alignment: Alignment, child: Self) -> Self {
        Self::align_factors(alignment, None, None, child)
    }
    pub fn align_factors(
        alignment: Alignment,
        width_factor: Option<f32>,
        height_factor: Option<f32>,
        child: Self,
    ) -> Self {
        Self::new(
            Kind::Align {
                alignment,
                width_factor,
                height_factor,
            },
            vec![child],
        )
    }
    pub fn flex(style: FlexStyle, items: Vec<FlexItem>) -> Self {
        let (children, sizing) = items
            .into_iter()
            .map(|item| (item.widget, item.sizing))
            .unzip();
        Self::new(Kind::Flex(style, sizing), children)
    }
    pub fn stack(style: StackStyle, items: Vec<StackItem>) -> Self {
        let (children, positions) = items
            .into_iter()
            .map(|item| (item.widget, item.position))
            .unzip();
        Self::new(Kind::Stack(style, positions), children)
    }
    /// Mount a persistent render box with ordered, keyed children. Empty and
    /// single-child lists support leaf and proxy render objects respectively.
    pub fn render_object<W: RenderObjectWidget>(widget: W, children: Vec<Self>) -> Self {
        Self::new(Kind::RenderObject(Box::new(RenderWidget(widget))), children)
    }
    pub fn stateful<W: StatefulWidget>(widget: W) -> Self {
        Self::new(Kind::Component(Box::new(Stateful(widget))), vec![])
    }
    pub fn stateless<W: StatelessWidget>(widget: W) -> Self {
        Self::new(Kind::Component(Box::new(Stateless(widget))), vec![])
    }
}

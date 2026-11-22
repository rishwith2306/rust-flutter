use super::ThemeData;
use crate::{
    decoration::{BoxDecoration, DecoratedBox},
    egui::{self, Color32, Stroke, Vec2},
    element::{
        FlexItem, FlexStyle, StackItem, StackStyle, State, StateContext, StateHandle,
        StatefulWidget, Widget,
    },
    gestures::{GestureBinding, GestureDetector, GestureTargetConfig, TapCallbacks, TapConfig},
    layout::{
        Alignment, Axis, BoxConstraints, Clip, CrossAxisAlignment, HitTestBehavior, Insets,
        MainAxisAlignment, Positioned, StackFit,
    },
    text::{DefaultTextStyle, TextStyle},
};
use std::{cell::Cell, rc::Rc};

/// Material alert dialog surface. Put it inside [`MaterialDialogHost`] to show
/// it modally with a dismissible scrim.
pub struct AlertDialog {
    title: Widget,
    content: Option<Widget>,
    actions: Vec<Widget>,
    width: f32,
    theme: ThemeData,
}

impl AlertDialog {
    pub fn new(title: Widget) -> Self {
        Self {
            title,
            content: None,
            actions: Vec::new(),
            width: 560.0,
            theme: ThemeData::default(),
        }
    }

    pub fn content(mut self, content: Widget) -> Self {
        self.content = Some(content);
        self
    }
    pub fn actions(mut self, actions: Vec<Widget>) -> Self {
        self.actions = actions;
        self
    }
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
    pub fn theme(mut self, theme: ThemeData) -> Self {
        self.theme = theme;
        self
    }

    pub fn into_widget(self) -> Result<Widget, &'static str> {
        if !self.width.is_finite() || self.width <= 0.0 {
            return Err("Material alert dialog width must be positive and finite");
        }
        let colors = self.theme.color_scheme;
        let mut children = vec![FlexItem::fixed(Widget::padding(
            Insets {
                left: 24.0,
                top: 24.0,
                right: 24.0,
                bottom: 16.0,
            },
            DefaultTextStyle {
                style: TextStyle {
                    font_size: Some(24.0),
                    color: Some(colors.on_surface),
                    ..Default::default()
                },
                ..Default::default()
            }
            .provide(self.title),
        ))];
        if let Some(content) = self.content {
            children.push(FlexItem::fixed(Widget::padding(
                Insets {
                    left: 24.0,
                    top: 0.0,
                    right: 24.0,
                    bottom: 24.0,
                },
                DefaultTextStyle {
                    style: TextStyle {
                        font_size: Some(14.0),
                        color: Some(colors.on_surface_variant),
                        ..Default::default()
                    },
                    ..Default::default()
                }
                .provide(content),
            )));
        }
        if !self.actions.is_empty() {
            let mut items = Vec::with_capacity(self.actions.len());
            for action in self.actions {
                items.push(FlexItem::fixed(action));
            }
            let mut actions = FlexStyle::new(Axis::Horizontal);
            actions.main_alignment = MainAxisAlignment::End;
            actions.cross_alignment = CrossAxisAlignment::Center;
            children.push(FlexItem::fixed(Widget::padding(
                Insets {
                    left: 16.0,
                    top: 0.0,
                    right: 16.0,
                    bottom: 16.0,
                },
                Widget::flex(actions, items),
            )));
        }
        let mut column = FlexStyle::new(Axis::Vertical);
        column.cross_alignment = CrossAxisAlignment::Stretch;
        let surface = DecoratedBox::new(BoxDecoration {
            color: colors.surface_container,
            border: Stroke::new(0.5_f32, colors.outline_variant),
            corner_radius: egui::CornerRadius::same(28),
        })
        .into_widget(Widget::flex(column, children));
        let constraints = BoxConstraints::loose(Vec2::new(self.width, 1200.0))
            .expect("validated finite Material alert dialog width");
        Ok(Widget::constrained(constraints, surface))
    }
}

type DialogBuilder = dyn Fn(MaterialDialogController, Option<u8>) -> (Widget, Option<Widget>);

/// Retained modal dialog host. The builder returns the page and an optional
/// dialog panel for the currently selected identifier.
pub struct MaterialDialogHost {
    binding: GestureBinding,
    builder: Rc<DialogBuilder>,
    selected: Rc<Cell<Option<u8>>>,
    barrier_color: Color32,
}

impl MaterialDialogHost {
    pub fn new(
        binding: &GestureBinding,
        builder: impl Fn(MaterialDialogController, Option<u8>) -> (Widget, Option<Widget>) + 'static,
    ) -> Self {
        Self {
            binding: binding.clone(),
            builder: Rc::new(builder),
            selected: Rc::new(Cell::new(None)),
            barrier_color: Color32::from_rgba_unmultiplied(0, 0, 0, 112),
        }
    }

    pub fn barrier_color(mut self, color: Color32) -> Self {
        self.barrier_color = color;
        self
    }
    pub fn into_widget(self) -> Widget {
        Widget::stateful(self)
    }
}

pub struct MaterialDialogState {
    selected: Option<u8>,
}

#[derive(Clone)]
pub struct MaterialDialogController {
    state: StateHandle<MaterialDialogState>,
    selected: Rc<Cell<Option<u8>>>,
}

impl MaterialDialogController {
    pub fn is_presented(&self) -> bool {
        self.selected.get().is_some()
    }
    pub fn present(&self, id: u8) -> Result<(), crate::element::Unmounted> {
        self.state.update(move |state| state.selected = Some(id))?;
        self.selected.set(Some(id));
        Ok(())
    }
    pub fn dismiss(&self) -> Result<(), crate::element::Unmounted> {
        self.state.update(|state| state.selected = None)?;
        self.selected.set(None);
        Ok(())
    }
}

impl StatefulWidget for MaterialDialogHost {
    type State = MaterialDialogState;
    fn create_state(&self) -> Self::State {
        MaterialDialogState { selected: None }
    }
}

impl State<MaterialDialogHost> for MaterialDialogState {
    fn build(&mut self, widget: &MaterialDialogHost, context: &StateContext<Self>) -> Widget {
        let controller = MaterialDialogController {
            state: context.handle(),
            selected: widget.selected.clone(),
        };
        widget.selected.set(self.selected);
        let (page, dialog) = (widget.builder)(controller.clone(), self.selected);
        let mut layers = vec![StackItem::new(page)];
        if let Some(dialog) = dialog {
            let dismiss = controller;
            let barrier = GestureDetector::new(
                &widget.binding,
                GestureTargetConfig {
                    tap: Some((
                        TapConfig::default(),
                        TapCallbacks {
                            on_tap: Some(Rc::new(move |_| {
                                let _ = dismiss.dismiss();
                            })),
                            ..Default::default()
                        },
                    )),
                    ..Default::default()
                },
            )
            .expect("live Material dialog gesture binding")
            .behavior(HitTestBehavior::Opaque)
            .into_widget(
                DecoratedBox::new(BoxDecoration::solid(widget.barrier_color))
                    .into_widget(Widget::empty()),
            );
            layers.push(StackItem::positioned(barrier, Positioned::fill()));
            layers.push(StackItem::new(Widget::align(
                Alignment::CENTER,
                Widget::safe_area(dialog),
            )));
        }
        Widget::stack(
            StackStyle {
                alignment: Alignment::TOP_LEFT,
                fit: StackFit::Expand,
                clip: Clip::HardEdge,
            },
            layers,
        )
    }
}

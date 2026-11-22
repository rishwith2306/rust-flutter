use super::ThemeData;
use crate::{
    decoration::{BoxDecoration, DecoratedBox},
    egui::{self, Color32, Stroke, Vec2},
    element::{
        FlexItem, FlexStyle, StackItem, StackStyle, State, StateContext, StateHandle,
        StatefulWidget, Widget,
    },
    gestures::{
        DragAxis, GestureBinding, GestureDetector, GestureTargetConfig, PanCallbacks, PanConfig,
        TapCallbacks, TapConfig,
    },
    layout::{
        Alignment, Axis, BoxConstraints, Clip, HitTestBehavior, Insets, Positioned, StackFit,
    },
};
use std::{any::Any, cell::Cell, cell::RefCell, rc::Rc};

type SheetBuilder = dyn Fn(ModalBottomSheetController, bool) -> (Widget, Option<Widget>);

/// Retained modal bottom-sheet host. Its builder returns the page and, when
/// presented, the sheet content. The host draws a dismissible scrim and a
/// themed Material surface at the bottom of the available viewport.
pub struct ModalBottomSheetHost {
    binding: GestureBinding,
    builder: Rc<SheetBuilder>,
    is_presented: Rc<Cell<bool>>,
    result: Rc<RefCell<Option<Box<dyn Any>>>>,
    barrier_dismissible: bool,
    barrier_color: Color32,
    max_height: f32,
    enable_drag: bool,
    show_drag_handle: bool,
    theme: ThemeData,
}

impl ModalBottomSheetHost {
    pub fn new(
        binding: &GestureBinding,
        builder: impl Fn(ModalBottomSheetController, bool) -> (Widget, Option<Widget>) + 'static,
    ) -> Self {
        Self {
            binding: binding.clone(),
            builder: Rc::new(builder),
            is_presented: Rc::new(Cell::new(false)),
            result: Rc::new(RefCell::new(None)),
            barrier_dismissible: true,
            barrier_color: Color32::from_rgba_unmultiplied(0, 0, 0, 112),
            max_height: 640.0,
            enable_drag: true,
            show_drag_handle: true,
            theme: ThemeData::default(),
        }
    }

    pub fn barrier_dismissible(mut self, dismissible: bool) -> Self {
        self.barrier_dismissible = dismissible;
        self
    }

    pub fn barrier_color(mut self, color: Color32) -> Self {
        self.barrier_color = color;
        self
    }

    pub fn max_height(mut self, height: f32) -> Result<Self, &'static str> {
        if !height.is_finite() || height <= 0.0 {
            return Err("Material modal bottom-sheet max height must be positive and finite");
        }
        self.max_height = height;
        Ok(self)
    }

    pub fn enable_drag(mut self, enabled: bool) -> Self {
        self.enable_drag = enabled;
        self
    }

    pub fn show_drag_handle(mut self, visible: bool) -> Self {
        self.show_drag_handle = visible;
        self
    }

    pub fn theme(mut self, theme: ThemeData) -> Self {
        self.theme = theme;
        self
    }

    pub fn into_widget(self) -> Widget {
        Widget::stateful(self)
    }
}

pub struct ModalBottomSheetState {
    is_presented: bool,
}

#[derive(Clone)]
pub struct ModalBottomSheetController {
    state: StateHandle<ModalBottomSheetState>,
    is_presented: Rc<Cell<bool>>,
    result: Rc<RefCell<Option<Box<dyn Any>>>>,
}

impl ModalBottomSheetController {
    pub fn is_presented(&self) -> bool {
        self.is_presented.get()
    }

    pub fn present(&self) -> Result<(), crate::element::Unmounted> {
        let previous = self.result.borrow_mut().take();
        if let Err(error) = self.state.update(|state| state.is_presented = true) {
            *self.result.borrow_mut() = previous;
            return Err(error);
        }
        self.is_presented.set(true);
        Ok(())
    }

    pub fn dismiss(&self) -> Result<(), crate::element::Unmounted> {
        self.state.update(|state| state.is_presented = false)?;
        self.is_presented.set(false);
        Ok(())
    }

    pub fn dismiss_with_result<T: 'static>(
        &self,
        result: T,
    ) -> Result<(), crate::element::Unmounted> {
        let previous = self.result.borrow_mut().replace(Box::new(result));
        if let Err(error) = self.state.update(|state| state.is_presented = false) {
            *self.result.borrow_mut() = previous;
            return Err(error);
        }
        self.is_presented.set(false);
        Ok(())
    }

    pub fn take_result<T: 'static>(&self) -> Result<Option<T>, &'static str> {
        let Some(result) = self.result.borrow_mut().take() else {
            return Ok(None);
        };
        match result.downcast::<T>() {
            Ok(result) => Ok(Some(*result)),
            Err(result) => {
                *self.result.borrow_mut() = Some(result);
                Err("Material bottom-sheet result type does not match the requested type")
            }
        }
    }
}

impl StatefulWidget for ModalBottomSheetHost {
    type State = ModalBottomSheetState;

    fn create_state(&self) -> Self::State {
        ModalBottomSheetState {
            is_presented: false,
        }
    }
}

impl State<ModalBottomSheetHost> for ModalBottomSheetState {
    fn build(&mut self, widget: &ModalBottomSheetHost, context: &StateContext<Self>) -> Widget {
        let controller = ModalBottomSheetController {
            state: context.handle(),
            is_presented: widget.is_presented.clone(),
            result: widget.result.clone(),
        };
        widget.is_presented.set(self.is_presented);
        let (page, sheet) = (widget.builder)(controller.clone(), self.is_presented);
        let mut layers = vec![StackItem::new(page)];
        if let Some(sheet) = sheet {
            let dismiss = controller.clone();
            let scrim = DecoratedBox::new(BoxDecoration::solid(widget.barrier_color))
                .into_widget(Widget::empty());
            let barrier = if widget.barrier_dismissible {
                GestureDetector::new(
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
                .expect("live Material modal bottom-sheet gesture binding")
                .behavior(HitTestBehavior::Opaque)
                .into_widget(scrim)
            } else {
                scrim
            };
            layers.push(StackItem::positioned(barrier, Positioned::fill()));
            let max = Vec2::new(f32::INFINITY, widget.max_height);
            let constraints =
                BoxConstraints::loose(max).expect("validated finite Material bottom-sheet height");
            let colors = widget.theme.color_scheme;
            let mut content = vec![];
            if widget.show_drag_handle {
                let drag_distance = Rc::new(Cell::new(0.0_f32));
                let mut config = GestureTargetConfig::default();
                if widget.enable_drag {
                    let start_distance = drag_distance.clone();
                    let update_distance = drag_distance.clone();
                    let end_distance = drag_distance.clone();
                    let dismiss = controller.clone();
                    config.pan = Some((
                        PanConfig {
                            axis: Some(DragAxis::Vertical),
                            ..Default::default()
                        },
                        PanCallbacks {
                            on_start: Some(Rc::new(move |_| start_distance.set(0.0))),
                            on_update: Some(Rc::new(move |details| {
                                update_distance.set(
                                    update_distance.get() + details.primary_delta.unwrap_or(0.0),
                                );
                            })),
                            on_end: Some(Rc::new(move |details| {
                                let distance = end_distance.get();
                                if distance >= 72.0
                                    || details.primary_velocity.unwrap_or(0.0) >= 900.0
                                {
                                    let _ = dismiss.dismiss();
                                }
                            })),
                            ..Default::default()
                        },
                    ));
                }
                let handle = DecoratedBox::new(BoxDecoration {
                    color: colors.outline_variant,
                    corner_radius: egui::CornerRadius::same(2),
                    ..Default::default()
                })
                .into_widget(Widget::empty());
                let handle_size = BoxConstraints::tight(Vec2::new(32.0, 4.0))
                    .expect("finite Material drag-handle dimensions");
                let handle = Widget::padding(
                    Insets {
                        left: 0.0,
                        top: 14.0,
                        right: 0.0,
                        bottom: 10.0,
                    },
                    Widget::align(Alignment::CENTER, Widget::constrained(handle_size, handle)),
                );
                let handle = if widget.enable_drag {
                    GestureDetector::new(&widget.binding, config)
                        .expect("live Material bottom-sheet gesture binding")
                        .behavior(HitTestBehavior::Opaque)
                        .into_widget(handle)
                } else {
                    handle
                };
                content.push(FlexItem::fixed(handle));
            }
            content.push(FlexItem::fixed(sheet));
            let mut column = FlexStyle::new(Axis::Vertical);
            column.cross_alignment = crate::layout::CrossAxisAlignment::Stretch;
            let content = Widget::flex(column, content);
            let surface = DecoratedBox::new(BoxDecoration {
                color: colors.surface_container_low,
                border: Stroke::new(0.5_f32, colors.outline_variant),
                corner_radius: egui::CornerRadius::same(28),
            })
            .into_widget(Widget::safe_area(Widget::constrained(constraints, content)));
            layers.push(StackItem::new(Widget::align(
                Alignment { x: 0.0, y: 1.0 },
                surface,
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

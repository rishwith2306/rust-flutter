use crate::{
    decoration::{BoxDecoration, DecoratedBox},
    egui::Color32,
    element::{StackItem, StackStyle, State, StateContext, StateHandle, StatefulWidget, Widget},
    gestures::{GestureBinding, GestureDetector, GestureTargetConfig, TapCallbacks, TapConfig},
    layout::{Alignment, Clip, HitTestBehavior, Positioned, StackFit},
};
use std::rc::Rc;

type ModalBuilder = dyn Fn(
    CupertinoModalPopupController,
    Option<u8>,
) -> (Widget, Option<(Widget, Option<Alignment>)>);

/// Retained modal popup host with a dismissible scrim.
///
/// The builder receives the current popup identifier and a controller on every
/// build. Return the ordinary page and an optional popup panel with its
/// alignment. The caller owns navigation and any system-back integration.
pub struct CupertinoModalPopup {
    binding: GestureBinding,
    builder: Rc<ModalBuilder>,
    alignment: Alignment,
    barrier_color: Color32,
}

impl CupertinoModalPopup {
    pub fn new(
        binding: &GestureBinding,
        builder: impl Fn(
            CupertinoModalPopupController,
            Option<u8>,
        ) -> (Widget, Option<(Widget, Option<Alignment>)>)
        + 'static,
    ) -> Self {
        Self {
            binding: binding.clone(),
            builder: Rc::new(builder),
            alignment: Alignment::CENTER,
            barrier_color: Color32::from_rgba_unmultiplied(0, 0, 0, 112),
        }
    }

    /// Position the popup panel in the full host bounds. Action sheets commonly
    /// use `Alignment::BOTTOM_CENTER`; alert dialogs commonly use `CENTER`.
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn barrier_color(mut self, color: Color32) -> Self {
        self.barrier_color = color;
        self
    }

    pub fn into_widget(self) -> Widget {
        Widget::stateful(self)
    }
}

pub struct ModalState {
    selected: Option<u8>,
}

/// Handle used by page buttons and popup actions to present or dismiss a panel.
#[derive(Clone)]
pub struct CupertinoModalPopupController {
    state: StateHandle<ModalState>,
}

impl CupertinoModalPopupController {
    pub fn present(&self, popup_id: u8) -> Result<(), crate::element::Unmounted> {
        self.state
            .update(move |state| state.selected = Some(popup_id))
    }

    pub fn dismiss(&self) -> Result<(), crate::element::Unmounted> {
        self.state.update(|state| state.selected = None)
    }
}

impl StatefulWidget for CupertinoModalPopup {
    type State = ModalState;

    fn create_state(&self) -> Self::State {
        ModalState { selected: None }
    }
}

impl State<CupertinoModalPopup> for ModalState {
    fn build(&mut self, widget: &CupertinoModalPopup, context: &StateContext<Self>) -> Widget {
        let controller = CupertinoModalPopupController {
            state: context.handle(),
        };
        let (page, popup) = (widget.builder)(controller.clone(), self.selected);
        let mut layers = vec![StackItem::new(page)];
        if let Some((popup, alignment)) = popup {
            let dismiss = controller.clone();
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
            .expect("live Cupertino modal popup gesture binding")
            .behavior(HitTestBehavior::Opaque)
            .into_widget(
                DecoratedBox::new(BoxDecoration {
                    color: widget.barrier_color,
                    ..Default::default()
                })
                .into_widget(Widget::empty()),
            );
            layers.push(StackItem::positioned(barrier, Positioned::fill()));
            layers.push(StackItem::new(Widget::align(
                alignment.unwrap_or(widget.alignment),
                Widget::safe_area(popup),
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

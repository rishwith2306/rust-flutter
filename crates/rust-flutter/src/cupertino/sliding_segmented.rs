use super::CupertinoThemeData;
use crate::{
    egui::{self, Align2, Color32, FontId, Rect, Vec2},
    element::{BuildContext, StatelessWidget, Widget},
    gestures::{
        DragAxis, GestureBinding, GestureDetector, GestureTargetConfig, PanCallbacks, PanConfig,
        PointerEvent, TapCallbacks, TapConfig, TapDetails,
    },
    layout::HitTestBehavior,
};
use std::{cell::Cell, rc::Rc};

/// A typed option for [`CupertinoSlidingSegmentedControl`].
pub struct CupertinoSlidingSegment<T> {
    pub value: T,
    pub label: String,
    pub enabled: bool,
}

impl<T> CupertinoSlidingSegment<T> {
    pub fn new(value: T, label: impl Into<String>) -> Self {
        Self {
            value,
            label: label.into(),
            enabled: true,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// A controlled iOS 13 style segmented control that supports tap and drag.
pub struct CupertinoSlidingSegmentedControl<T: Clone + PartialEq + 'static> {
    binding: GestureBinding,
    segments: Vec<CupertinoSlidingSegment<T>>,
    selected: T,
    width: f32,
    height: f32,
    on_changed: Rc<dyn Fn(T)>,
}

impl<T: Clone + PartialEq + 'static> CupertinoSlidingSegmentedControl<T> {
    pub fn new(
        binding: &GestureBinding,
        segments: Vec<CupertinoSlidingSegment<T>>,
        selected: T,
        on_changed: impl Fn(T) + 'static,
    ) -> Result<Self, &'static str> {
        if segments.is_empty()
            || !segments.iter().any(|segment| segment.value == selected)
            || segments.iter().enumerate().any(|(index, segment)| {
                segments[..index]
                    .iter()
                    .any(|previous| previous.value == segment.value)
            })
        {
            return Err("sliding segmented control requires unique options and a selected value");
        }
        Ok(Self {
            binding: binding.clone(),
            segments,
            selected,
            width: 240.0,
            height: 36.0,
            on_changed: Rc::new(on_changed),
        })
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn into_widget(self) -> Result<Widget, &'static str> {
        if !self.width.is_finite()
            || self.width <= 0.0
            || !self.height.is_finite()
            || self.height <= 0.0
        {
            return Err("sliding segmented control dimensions must be positive and finite");
        }
        Ok(Widget::stateless(SlidingControlView(self)))
    }
}

struct SlidingControlView<T: Clone + PartialEq + 'static>(CupertinoSlidingSegmentedControl<T>);

impl<T: Clone + PartialEq + 'static> StatelessWidget for SlidingControlView<T> {
    fn build(&self, context: &BuildContext) -> Widget {
        let control = &self.0;
        let theme = CupertinoThemeData::of(context).unwrap_or_default();
        let bounds = Rc::new(Cell::new(Rect::NOTHING));
        let last_pan_index = Rc::new(Cell::new(None));
        let index_at = {
            let bounds = bounds.clone();
            let count = control.segments.len();
            Rc::new(move |event: PointerEvent| {
                let rect = bounds.get();
                if !rect.is_finite() || rect.width() <= 0.0 || !rect.contains(event.position) {
                    return None;
                }
                let fraction = ((event.position.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                Some(((fraction * count as f32).floor() as usize).min(count - 1))
            })
        };
        let mut gestures = GestureTargetConfig::default();
        let tap_index = index_at.clone();
        let on_tap = control.on_changed.clone();
        let selected = control.selected.clone();
        let values: Vec<_> = control
            .segments
            .iter()
            .map(|item| item.value.clone())
            .collect();
        let enabled: Vec<_> = control.segments.iter().map(|item| item.enabled).collect();
        gestures.tap = Some((
            TapConfig::default(),
            TapCallbacks {
                on_tap: Some(Rc::new(move |details: TapDetails| {
                    if let Some(index) = tap_index(details.up)
                        && enabled[index]
                        && values[index] != selected
                    {
                        on_tap(values[index].clone());
                    }
                })),
                ..Default::default()
            },
        ));
        let pan_start_index = index_at.clone();
        let pan_update_index = index_at;
        let pan_values: Vec<_> = control
            .segments
            .iter()
            .map(|item| item.value.clone())
            .collect();
        let pan_enabled: Vec<_> = control.segments.iter().map(|item| item.enabled).collect();
        let pan_callback = control.on_changed.clone();
        let last_start = last_pan_index.clone();
        let last_update = last_pan_index;
        gestures.pan = Some((
            PanConfig {
                axis: Some(DragAxis::Horizontal),
                slop: 5.0,
                ..Default::default()
            },
            PanCallbacks {
                on_start: Some(Rc::new(move |event| {
                    last_start.set(pan_start_index(event));
                })),
                on_update: Some(Rc::new(move |details| {
                    if let Some(index) = pan_update_index(details.event)
                        && pan_enabled[index]
                        && last_update.get() != Some(index)
                    {
                        last_update.set(Some(index));
                        pan_callback(pan_values[index].clone());
                    }
                })),
                ..Default::default()
            },
        ));
        let painter_bounds = bounds.clone();
        let labels: Vec<_> = control
            .segments
            .iter()
            .map(|item| item.label.clone())
            .collect();
        let enabled = control
            .segments
            .iter()
            .map(|item| item.enabled)
            .collect::<Vec<_>>();
        let semantic_values: Vec<_> = control
            .segments
            .iter()
            .map(|segment| segment.value.clone())
            .collect();
        let semantic_on_changed = control.on_changed.clone();
        let selected_index = control
            .segments
            .iter()
            .position(|item| item.value == control.selected)
            .expect("selected value is validated by the constructor");
        let selected_text = theme.text_color;
        let unselected_text = theme.disabled_color;
        let background = match theme.brightness {
            super::Brightness::Light => Color32::from_gray(225),
            super::Brightness::Dark => Color32::from_gray(55),
        };
        let child = Widget::leaf(Vec2::new(control.width, control.height), move |ui| {
            let rect = ui.max_rect();
            painter_bounds.set(rect);
            let painter = ui.painter();
            painter.rect_filled(rect, egui::CornerRadius::same(8), background);
            let segment_width = rect.width() / labels.len() as f32;
            let thumb = Rect::from_min_max(
                rect.min + Vec2::new(selected_index as f32 * segment_width + 2.0, 2.0),
                egui::pos2(
                    rect.left() + (selected_index + 1) as f32 * segment_width - 2.0,
                    rect.bottom() - 2.0,
                ),
            );
            painter.rect_filled(
                thumb,
                egui::CornerRadius::same(7),
                theme.scaffold_background_color,
            );
            for (index, label) in labels.iter().enumerate() {
                let segment_rect = Rect::from_min_max(
                    egui::pos2(rect.left() + index as f32 * segment_width, rect.top()),
                    egui::pos2(
                        rect.left() + (index + 1) as f32 * segment_width,
                        rect.bottom(),
                    ),
                );
                let center = egui::pos2(
                    rect.left() + (index as f32 + 0.5) * segment_width,
                    rect.center().y,
                );
                let color = if !enabled[index] {
                    theme.disabled_color
                } else if index == selected_index {
                    selected_text
                } else {
                    unselected_text
                };
                painter.text(
                    center,
                    Align2::CENTER_CENTER,
                    label,
                    FontId::proportional(13.0),
                    color,
                );
                let id = ui.id().with(("cupertino-sliding-segment", index));
                let response = ui.interact(segment_rect, id, egui::Sense::hover());
                ui.ctx().accesskit_node_builder(response.id, |node| {
                    node.set_role(egui::accesskit::Role::RadioButton);
                    node.set_label(label.clone());
                    node.set_selected(index == selected_index);
                    if !enabled[index] {
                        node.set_disabled();
                    } else {
                        node.add_action(egui::accesskit::Action::Click);
                    }
                });
                if enabled[index] {
                    let mut activate = false;
                    ui.input_mut(|input| {
                        input.consume_accesskit_action_requests(response.id, |request| {
                            if request.action == egui::accesskit::Action::Click {
                                activate = true;
                                true
                            } else {
                                false
                            }
                        });
                    });
                    if activate && index != selected_index {
                        semantic_on_changed(semantic_values[index].clone());
                    }
                }
            }
        });
        GestureDetector::new(&control.binding, gestures)
            .expect("validated Cupertino sliding gesture configuration")
            .behavior(HitTestBehavior::Opaque)
            .into_widget(child)
    }
}

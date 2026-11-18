use super::{ScrollPosition, viewport::ViewportTask};
use crate::{
    egui::{self, Id, Key, Modifiers},
    layout::Axis,
};
use std::cell::{Cell, RefCell};

#[derive(Clone)]
struct Painted {
    ctx: egui::Context,
    id: Id,
    layer: egui::LayerId,
    pass: u64,
    enabled: bool,
    axis: Axis,
    reverse: bool,
}
#[derive(Default)]
pub(super) struct ScrollKeyboard {
    painted: RefCell<Option<Painted>>,
    handled_frame: Cell<Option<u64>>,
}
#[derive(Clone, Copy)]
enum Command {
    Line(f64),
    Page(f64),
    Start,
    End,
}
fn command(key: Key, modifiers: Modifiers, axis: Axis) -> Option<Command> {
    if modifiers.ctrl || modifiers.command || modifiers.alt {
        return None;
    }
    if key == Key::Space {
        return Some(Command::Page(if modifiers.shift { -1.0 } else { 1.0 }));
    }
    if modifiers.shift {
        return None;
    }
    match key {
        Key::ArrowDown if axis == Axis::Vertical => Some(Command::Line(1.0)),
        Key::ArrowUp if axis == Axis::Vertical => Some(Command::Line(-1.0)),
        Key::ArrowRight if axis == Axis::Horizontal => Some(Command::Line(1.0)),
        Key::ArrowLeft if axis == Axis::Horizontal => Some(Command::Line(-1.0)),
        Key::PageDown => Some(Command::Page(1.0)),
        Key::PageUp => Some(Command::Page(-1.0)),
        Key::Home => Some(Command::Start),
        Key::End => Some(Command::End),
        _ => None,
    }
}
impl ScrollKeyboard {
    pub(super) fn inherit(&self, old: &Self) {
        *self.painted.borrow_mut() = old.painted.borrow().clone();
        self.handled_frame.set(old.handled_frame.get());
    }
    pub(super) fn disable(&self) {
        if let Some(node) = self.painted.borrow_mut().as_mut() {
            node.enabled = false;
            node.ctx.memory_mut(|m| m.surrender_focus(node.id));
        }
    }
    pub(super) fn paint(
        &self,
        ui: &mut egui::Ui,
        id: Id,
        axis: Axis,
        reverse: bool,
        enabled: bool,
    ) {
        let enabled = enabled
            && ui.is_enabled()
            && (ui.is_rect_visible(ui.max_rect()) || crate::layout::ui_is_offstage(ui));
        if enabled {
            ui.interact(ui.max_rect(), id, egui::Sense::focusable_noninteractive());
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    id,
                    egui::EventFilter {
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        ..Default::default()
                    },
                )
            });
        } else {
            ui.memory_mut(|m| m.surrender_focus(id));
        }
        *self.painted.borrow_mut() = Some(Painted {
            ctx: ui.ctx().clone(),
            id,
            layer: ui.layer_id(),
            pass: ui.ctx().cumulative_pass_nr(),
            enabled,
            axis,
            reverse,
        });
    }
    pub(super) fn focus_from_tap(&self, position: egui::Pos2) {
        let Some(node) = self.painted.borrow().clone().filter(|n| n.enabled) else {
            return;
        };
        // Ordinary backend controls can acquire focus independently of the arena.
        // Preserve a focused descendant under this click instead of stealing it.
        if let Some(focused) = node.ctx.memory(|m| m.focused()).filter(|id| *id != node.id)
            && let Some(response) = node.ctx.read_response(focused)
        {
            let local = node
                .ctx
                .layer_transform_from_global(response.layer_id)
                .map_or(position, |t| t * position);
            if response.rect.contains(local) {
                return;
            }
        }
        node.ctx.memory_mut(|m| m.request_focus(node.id));
        node.ctx.request_repaint();
    }
    fn eligible(&self, ctx: &egui::Context) -> Option<Painted> {
        self.painted.borrow().clone().filter(|n| {
            n.enabled
                && n.ctx == *ctx
                && n.pass == ctx.cumulative_pass_nr()
                && ctx.input(|i| i.focused)
                && ctx.memory(|m| m.has_focus(n.id) && m.allows_interaction(n.layer))
        })
    }
    pub(super) fn dispatch(
        &self,
        task: &ViewportTask,
        ctx: &egui::Context,
        position: &ScrollPosition,
    ) {
        if self.handled_frame.get() == Some(ctx.cumulative_frame_nr()) || position.is_disposed() {
            return;
        }
        let Some(node) = self.eligible(ctx) else {
            return;
        };
        let events = ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|event| {
                    if let egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } = event
                    {
                        command(*key, *modifiers, node.axis).map(|c| (event.clone(), c))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        });
        for (event, action) in events {
            if !task.keyboard_active() || self.eligible(ctx).is_none() || position.is_disposed() {
                break;
            }
            let Some(dimensions) = position.dimensions() else {
                break;
            };
            let consumed = ctx.input_mut(|i| {
                if let Some(index) = i.events.iter().position(|e| *e == event) {
                    i.events.remove(index);
                    true
                } else {
                    false
                }
            });
            if !consumed {
                continue;
            }
            self.handled_frame.set(Some(ctx.cumulative_frame_nr()));
            let sign = if node.reverse { -1.0 } else { 1.0 };
            let target = match action {
                Command::Start => f64::from(dimensions.min_scroll_extent()),
                Command::End => f64::from(dimensions.max_scroll_extent()),
                Command::Line(direction) => f64::from(position.pixels()) + direction * sign * 50.0,
                Command::Page(direction) => {
                    f64::from(position.pixels())
                        + direction * sign * f64::from(dimensions.viewport_dimension()) * 0.8
                }
            }
            .clamp(
                f64::from(dimensions.min_scroll_extent()),
                f64::from(dimensions.max_scroll_extent()),
            ) as f32;
            let _ = position.jump_to(target);
        }
    }
    pub(super) fn paint_focus(&self, ui: &mut egui::Ui) {
        let focused = self
            .painted
            .borrow()
            .as_ref()
            .is_some_and(|n| n.enabled && ui.memory(|m| m.has_focus(n.id)));
        if !focused {
            return;
        }
        // Corner brackets preserve visible fill between markers and have bounded
        // work even for very large layout extents.
        let rect = ui.max_rect().intersect(ui.clip_rect()).shrink(0.5);
        if !rect.is_positive() {
            return;
        }
        let length = 4.0_f32.min(rect.width() * 0.25).min(rect.height() * 0.25);
        let stroke = egui::Stroke::new(1.0_f32, ui.visuals().selection.stroke.color);
        for (corner, x, y) in [
            (rect.left_top(), 1.0, 1.0),
            (rect.right_top(), -1.0, 1.0),
            (rect.left_bottom(), 1.0, -1.0),
            (rect.right_bottom(), -1.0, -1.0),
        ] {
            ui.painter()
                .line_segment([corner, corner + egui::vec2(x * length, 0.0)], stroke);
            ui.painter()
                .line_segment([corner, corner + egui::vec2(0.0, y * length)], stroke);
        }
    }
}

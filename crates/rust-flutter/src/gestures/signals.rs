use super::*;
use crate::egui::{self, Vec2};
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    time::Duration,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollSignalUnit {
    Points,
    Lines,
    Pages,
}
/// Positive delta means forward content offset (opposite egui's content-motion
/// delta). Position and point deltas use global logical coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerScrollEvent {
    pub position: Pos2,
    pub scroll_delta: Vec2,
    pub unit: ScrollSignalUnit,
    pub time: Duration,
    pub modifiers: egui::Modifiers,
}
pub type ScrollCallback = Rc<dyn Fn(PointerScrollEvent) -> bool>;
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollClaim {
    pub event: PointerScrollEvent,
    pub target: Id,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollDispatchFailure {
    pub event: PointerScrollEvent,
    pub error: BindingError,
}
pub(super) struct ScrollPacket {
    pub event: PointerScrollEvent,
    pub path: Vec<(Id, Weak<Target>)>,
}
impl Inner {
    pub(super) fn dispatch_scroll_one(
        &self,
        packet: ScrollPacket,
    ) -> Result<Option<Id>, BindingError> {
        if self.arena.is_disposed() || self.router.is_disposed() {
            self.shutdown();
            return Err(BindingError::Disposed);
        }
        if self.state.borrow().disposed {
            return Err(BindingError::Disposed);
        }
        let event = packet.event;
        if !event.position.is_finite() {
            return Err(BindingError::InvalidPosition);
        }
        if !event.scroll_delta.is_finite() {
            return Err(BindingError::InvalidScrollDelta);
        }
        if event.scroll_delta == Vec2::ZERO {
            return Ok(None);
        }
        for (id, weak) in packet.path {
            let target = {
                let state = self.state.borrow();
                if state.disposed {
                    break;
                }
                state
                    .targets
                    .get(&id)
                    .filter(|t| Weak::ptr_eq(&Rc::downgrade(t), &weak))
                    .cloned()
            };
            let callback = target.and_then(|target| target.config.borrow().on_scroll.clone());
            if let Some(callback) = callback {
                // Clone first, then release configuration before application code.
                match catch_unwind(AssertUnwindSafe(|| callback(event))) {
                    Ok(true) => return Ok(Some(id)),
                    Ok(false) => {}
                    Err(_) => return Err(BindingError::ScrollCallbackPanicked(id)),
                }
            }
        }
        Ok(None)
    }
}
impl GestureBinding {
    /// First willing live target in hit-path order handles the entire signal.
    /// Reentrant pointer/signal packets share the binding's dispatch queue.
    pub fn dispatch_scroll(
        &self,
        event: PointerScrollEvent,
        hit_path: impl IntoIterator<Item = Id>,
    ) -> DispatchReport {
        let mut seen = HashSet::new();
        let ids: Vec<_> = hit_path.into_iter().filter(|id| seen.insert(*id)).collect();
        let path = {
            let state = self.0.state.borrow();
            ids.into_iter()
                .filter_map(|id| state.targets.get(&id).map(|t| (id, Rc::downgrade(t))))
                .collect()
        };
        self.0
            .state
            .borrow_mut()
            .queue
            .push_back(Packet::Scroll(ScrollPacket { event, path }));
        self.drain()
    }
    pub fn dispatch_scroll_layout(
        &self,
        event: PointerScrollEvent,
        geometry: &LayoutBox,
        origin: Pos2,
    ) -> DispatchReport {
        let path = geometry
            .hit_test_path(event.position - origin.to_vec2())
            .into_iter()
            .map(|e| e.id);
        self.dispatch_scroll(event, path)
    }
    pub(super) fn cursor(&self, ctx: &egui::Context, events: &[egui::Event]) -> Option<Pos2> {
        self.0
            .host
            .borrow()
            .as_ref()
            .and_then(|h| h.cursor)
            .or_else(|| {
                (!events.iter().any(|e| {
                    matches!(
                        e,
                        egui::Event::PointerMoved(_)
                            | egui::Event::PointerButton { .. }
                            | egui::Event::PointerGone
                    )
                }))
                .then(|| ctx.input(|i| i.pointer.hover_pos()))
                .flatten()
            })
    }
    pub(super) fn set_cursor(&self, position: Option<Pos2>) {
        if let Some(host) = self.0.host.borrow_mut().as_mut() {
            host.cursor = position;
        }
    }
}
pub(super) fn from_egui(
    position: Pos2,
    unit: egui::MouseWheelUnit,
    delta: Vec2,
    modifiers: egui::Modifiers,
    time: Duration,
) -> PointerScrollEvent {
    PointerScrollEvent {
        position,
        scroll_delta: -delta,
        unit: match unit {
            egui::MouseWheelUnit::Point => ScrollSignalUnit::Points,
            egui::MouseWheelUnit::Line => ScrollSignalUnit::Lines,
            egui::MouseWheelUnit::Page => ScrollSignalUnit::Pages,
        },
        time,
        modifiers,
    }
}

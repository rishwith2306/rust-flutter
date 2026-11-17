//! Gallery-only primary-pointer adapter for exercising the public arena kernel.
//! General widget hit-path routing and a recognizer catalog remain framework work.
use rust_flutter::{egui, gestures::*};
use std::rc::Rc;
const POINTER: PointerId = PointerId(0);
struct Candidate {
    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    kind: &'static str,
    accepted: Rc<dyn Fn()>,
}
impl GestureArenaMember for Candidate {
    fn accept_gesture(&self, _: PointerId) {
        #[cfg(debug_assertions)]
        eprintln!("rust-flutter gallery gesture-arena {} accepted", self.kind);
        (self.accepted)();
    }
    fn reject_gesture(&self, _: PointerId) {
        #[cfg(debug_assertions)]
        eprintln!("rust-flutter gallery gesture-arena {} rejected", self.kind);
    }
}
struct Sequence {
    start: egui::Pos2,
    tap: GestureArenaEntry,
    drag: GestureArenaEntry,
    dragged: bool,
}
impl Sequence {
    fn moved(&mut self, pos: egui::Pos2) {
        if !self.dragged && pos.distance(self.start) > 18.0 {
            self.dragged = true;
            self.tap.resolve(GestureDisposition::Rejected);
            self.drag.resolve(GestureDisposition::Accepted);
        }
    }
}
pub(super) struct PointerMarker {
    arena: GestureArenaManager,
    active: Option<Sequence>,
    last_frame: Option<u64>,
    tap: Rc<dyn Fn()>,
    drag: Rc<dyn Fn()>,
}
impl PointerMarker {
    pub fn new(tap: impl Fn() + 'static, drag: impl Fn() + 'static) -> Self {
        Self {
            arena: GestureArenaManager::new(),
            active: None,
            last_frame: None,
            tap: Rc::new(tap),
            drag: Rc::new(drag),
        }
    }
    pub fn cancel(&mut self) {
        self.active = None;
        self.arena.cancel(POINTER);
        self.arena.flush();
    }
    pub fn handle(&mut self, ui: &egui::Ui) {
        let frame = ui.ctx().cumulative_frame_nr();
        if self.last_frame == Some(frame) {
            return;
        }
        self.last_frame = Some(frame);
        let (focused, events) = ui.input(|i| (i.focused, i.events.clone()));
        if !focused {
            self.cancel();
            return;
        }
        for event in events {
            match event {
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    ..
                } => {
                    if ui.is_enabled()
                        && ui.max_rect().contains(pos)
                        && ui.clip_rect().contains(pos)
                    {
                        self.cancel();
                        let tap = self
                            .arena
                            .add(
                                POINTER,
                                Rc::new(Candidate {
                                    kind: "tap",
                                    accepted: self.tap.clone(),
                                }),
                            )
                            .expect("fresh tap arena");
                        let drag = self
                            .arena
                            .add(
                                POINTER,
                                Rc::new(Candidate {
                                    kind: "drag",
                                    accepted: self.drag.clone(),
                                }),
                            )
                            .expect("open drag arena");
                        self.active = Some(Sequence {
                            start: pos,
                            tap,
                            drag,
                            dragged: false,
                        });
                        self.arena
                            .close(POINTER)
                            .expect("registered down candidates");
                    }
                }
                egui::Event::PointerMoved(pos) => {
                    if let Some(sequence) = &mut self.active {
                        sequence.moved(pos);
                    }
                }
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    ..
                } => {
                    if let Some(mut sequence) = self.active.take() {
                        sequence.moved(pos);
                        if !sequence.dragged {
                            sequence.tap.resolve(GestureDisposition::Accepted);
                        }
                        self.arena.sweep(POINTER).expect("closed up arena");
                    }
                }
                egui::Event::PointerGone | egui::Event::WindowFocused(false) => self.cancel(),
                _ => {}
            }
            self.arena.flush();
        }
    }
}

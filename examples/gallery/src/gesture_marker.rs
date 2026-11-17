//! Gallery-only primary-pointer binding for the public recognizers.
//! General widget hit-path dispatch remains framework work.
use rust_flutter::{egui, gestures::*};
use std::{rc::Rc, time::Duration};
const POINTER: PointerId = PointerId(0);

pub(super) struct PointerMarker {
    arena: GestureArenaManager,
    router: PointerRouter<PointerEvent>,
    tap: TapGestureRecognizer,
    pan: PanGestureRecognizer,
    last_frame: Option<u64>,
}
impl PointerMarker {
    pub fn new(tap: impl Fn() + 'static, drag: impl Fn() + 'static) -> Self {
        let arena = GestureArenaManager::new();
        let router = PointerRouter::new();
        let tap = TapGestureRecognizer::new(
            &arena,
            &router,
            TapConfig::default(),
            TapCallbacks {
                on_tap: Some(Rc::new(move |_| {
                    #[cfg(debug_assertions)]
                    eprintln!("rust-flutter gallery gesture-arena tap accepted");
                    tap();
                })),
                on_cancel: Some(Rc::new(|_| {
                    #[cfg(debug_assertions)]
                    eprintln!("rust-flutter gallery gesture-arena tap rejected");
                })),
                ..Default::default()
            },
        )
        .expect("valid tap settings");
        let pan = PanGestureRecognizer::new(
            &arena,
            &router,
            PanConfig::default(),
            PanCallbacks {
                on_start: Some(Rc::new(move |_| {
                    #[cfg(debug_assertions)]
                    eprintln!("rust-flutter gallery gesture-arena drag accepted");
                    drag();
                })),
                on_cancel: Some(Rc::new(|_| {
                    #[cfg(debug_assertions)]
                    eprintln!("rust-flutter gallery gesture-arena drag rejected");
                })),
                ..Default::default()
            },
        )
        .expect("valid pan settings");
        Self {
            arena,
            router,
            tap,
            pan,
            last_frame: None,
        }
    }
    pub fn cancel(&mut self) {
        self.tap.cancel();
        self.pan.cancel();
        self.arena.cancel(POINTER);
        self.arena.flush();
    }
    pub fn handle(&mut self, ui: &egui::Ui) {
        let frame = ui.ctx().cumulative_frame_nr();
        if self.last_frame == Some(frame) {
            return;
        }
        self.last_frame = Some(frame);
        let (focused, time, events) = ui.input(|i| (i.focused, i.time, i.events.clone()));
        if !focused {
            self.cancel();
            return;
        }
        let time = Duration::try_from_secs_f64(time).unwrap_or_default();
        let sample = |phase, position, buttons| PointerEvent {
            pointer: POINTER,
            phase,
            position,
            time,
            buttons,
            // egui's normalized primary pointer alone does not identify its device.
            device: PointerDeviceKind::Unknown,
        };
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
                        let down = sample(PointerPhase::Down, pos, PointerButtons::PRIMARY);
                        self.tap.add_pointer(down).expect("live tap recognizer");
                        self.pan.add_pointer(down).expect("live pan recognizer");
                        self.arena
                            .close(POINTER)
                            .expect("registered down candidates");
                    }
                }
                egui::Event::PointerMoved(pos) => {
                    self.router.route(
                        POINTER,
                        &sample(PointerPhase::Move, pos, PointerButtons::PRIMARY),
                    );
                }
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    ..
                } => {
                    self.router.route(
                        POINTER,
                        &sample(PointerPhase::Up, pos, PointerButtons::NONE),
                    );
                    self.arena.sweep(POINTER).expect("closed up arena");
                }
                egui::Event::PointerGone | egui::Event::WindowFocused(false) => self.cancel(),
                _ => {}
            }
            self.arena.flush();
        }
    }
}

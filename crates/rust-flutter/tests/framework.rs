use rust_flutter::{
    animation::{Curve, Tween},
    navigation::Navigator,
    services::Services,
    state::{Signal, TaskState, spawn_task},
    widgets::*,
};
use std::{
    sync::{Arc, mpsc},
    time::Duration,
};

#[test]
fn navigation_preserves_root_and_returns_removed_routes() {
    let mut nav = Navigator::new("home");
    assert_eq!(nav.pop(), None);
    nav.push("details");
    nav.push("edit");
    assert_eq!(nav.replace("preview"), "edit");
    assert_eq!(nav.pop(), Some("preview"));
    assert_eq!(nav.current(), &"details");
    nav.reset("login");
    assert_eq!(nav.routes(), &["login"]);
    assert!(!nav.can_pop());
}

#[test]
fn shared_state_crosses_threads_and_wakes_ui() {
    let ctx = egui::Context::default();
    // Drain the backend's startup repaint requests before testing a new wakeup.
    for _ in 0..4 {
        let _ = ctx.run(Default::default(), |_| {});
    }
    let (tx, rx) = mpsc::channel();
    ctx.set_request_repaint_callback(move |_| {
        let _ = tx.send(());
    });
    let signal = Signal::new(0, &ctx);
    let worker = signal.clone();
    std::thread::spawn(move || worker.update(|n| *n += 7))
        .join()
        .unwrap();
    assert_eq!(signal.get(), 7);
    rx.recv_timeout(Duration::from_secs(2))
        .expect("mutation must wake UI");
}

#[test]
fn worker_publishes_errors_and_panics_without_staying_pending() {
    let ctx = egui::Context::default();
    let error = spawn_task(&ctx, || Err::<(), _>("offline"));
    let panic = spawn_task(&ctx, || -> Result<(), ()> { panic!("worker failure") });
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while error.get() == TaskState::Pending || panic.get() == TaskState::Pending {
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert_eq!(error.get(), TaskState::Ready(Err("offline")));
    assert_eq!(panic.get(), TaskState::Panicked);
}

#[test]
fn animations_handle_boundaries_curves_and_zero_duration() {
    let tween = Tween::new(10.0, 30.0, 2.0, Duration::from_secs(2), Curve::EaseInOut);
    assert_eq!(tween.sample(1.0), (10.0, false));
    assert_eq!(tween.sample(3.0), (20.0, false));
    assert_eq!(tween.sample(5.0), (30.0, true));
    assert_eq!(
        Tween::new(0.0, 1.0, 0.0, Duration::ZERO, Curve::Linear).sample(0.0),
        (1.0, true)
    );
    assert!(Curve::EaseOut.transform(0.5) > 0.5);
}

#[test]
fn registry_replaces_by_type_without_affecting_other_services() {
    let mut services = Services::default();
    services.insert(Arc::new("clipboard"));
    assert_eq!(services.insert(42_u32), None);
    assert_eq!(services.insert(9_u32), Some(42));
    assert_eq!(services.get::<u32>(), Some(&9));
    assert_eq!(**services.get::<Arc<&str>>().unwrap(), "clipboard");
    assert!(services.get::<String>().is_none());
}

#[test]
fn pointer_interaction_activates_button_and_produces_paint() {
    let ctx = egui::Context::default();
    let mut rect = egui::Rect::NOTHING;
    let mut count = 0;
    let output = {
        let mut frame = |events: Vec<egui::Event>| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(400.0, 600.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        column(ui, |ui| {
                            let response = button(ui, "Increment");
                            rect = response.rect;
                            if response.clicked() {
                                count += 1;
                            }
                        });
                    });
                },
            )
        };
        frame(vec![])
    };
    assert!(!output.shapes.is_empty());
    let position = rect.center();
    for pressed in [true, false] {
        let _ = ctx.run(
            egui::RawInput {
                events: vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    },
                ],
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    column(ui, |ui| {
                        if button(ui, "Increment").clicked() {
                            count += 1;
                        }
                    });
                });
            },
        );
    }
    assert_eq!(count, 1);
}

#[test]
fn responsive_boundaries_are_in_logical_points() {
    assert_eq!(Breakpoint::for_width(599.0), Breakpoint::Compact);
    assert_eq!(Breakpoint::for_width(600.0), Breakpoint::Medium);
    assert_eq!(Breakpoint::for_width(840.0), Breakpoint::Expanded);
}

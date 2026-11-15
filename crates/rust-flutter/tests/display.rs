use rust_flutter::{
    display::safe_area_insets,
    egui::{self, epaint::MarginF32},
};
#[test]
fn mobile_insets_account_for_density_rotation_and_out_of_bounds_content() {
    let portrait = safe_area_insets(
        egui::vec2(1080.0, 2400.0),
        egui::Rect::from_min_max(egui::pos2(0.0, 63.0), egui::pos2(1080.0, 2337.0)),
        2.625,
    )
    .unwrap();
    assert_eq!(
        portrait.0,
        MarginF32 {
            left: 0.0,
            right: 0.0,
            top: 24.0,
            bottom: 24.0
        }
    );
    let landscape = safe_area_insets(
        egui::vec2(2400.0, 1080.0),
        egui::Rect::from_min_max(egui::pos2(100.0, -5.0), egui::pos2(2300.0, 1082.0)),
        2.0,
    )
    .unwrap();
    assert_eq!(
        landscape.0,
        MarginF32 {
            left: 50.0,
            right: 50.0,
            top: 0.0,
            bottom: 0.0
        }
    );
}
#[test]
fn uninitialized_or_invalid_window_geometry_does_not_hide_the_ui() {
    for scale in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert!(
            safe_area_insets(
                egui::vec2(100.0, 200.0),
                egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(100.0, 200.0)),
                scale
            )
            .is_none()
        );
    }
    assert!(safe_area_insets(egui::Vec2::ZERO, egui::Rect::NOTHING, 1.0).is_none());
    assert!(safe_area_insets(egui::vec2(100.0, 200.0), egui::Rect::ZERO, 1.0).is_none());
}

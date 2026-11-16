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

#[test]
fn occlusion_keeps_stable_padding_separate_from_the_keyboard() {
    use rust_flutter::display::ViewOcclusion;
    let padding = MarginF32 {
        top: 63.0,
        bottom: 63.0,
        left: 21.0,
        right: 0.0,
    };
    let keyboard = MarginF32 {
        bottom: 840.0,
        ..MarginF32::ZERO
    };
    let shown = ViewOcclusion::from_physical(padding, keyboard, 2.625).unwrap();
    assert_eq!(shown.view_padding.bottom, 24.0);
    assert_eq!(shown.view_insets.bottom, 320.0);
    assert_eq!(shown.padding.bottom, 0.0);
    assert_eq!(shown.padding.top, 24.0);
    assert_eq!(shown.padding.left, 8.0);
    let hidden = ViewOcclusion::from_physical(padding, MarginF32::ZERO, 2.625).unwrap();
    assert_eq!(hidden.view_padding, shown.view_padding);
    assert_eq!(hidden.padding.bottom, 24.0);
    for scale in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert!(ViewOcclusion::from_physical(padding, keyboard, scale).is_none());
    }
    for bottom in [-1.0, f32::NAN, f32::INFINITY] {
        assert!(
            ViewOcclusion::from_physical(padding, MarginF32 { bottom, ..keyboard }, 1.0).is_none()
        );
    }
    assert!(ViewOcclusion::from_physical(padding, keyboard, f32::MIN_POSITIVE).is_none());
}

#[test]
fn metrics_use_active_viewport_density_zoom_and_preserve_unknown_keyboard() {
    use rust_flutter::display::{KeyboardVisibility, ViewMetrics, ViewOcclusion, view_metrics};
    let ctx = egui::Context::default();
    ctx.set_zoom_factor(1.5);
    let mut raw = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::pos2(10.0, 20.0),
            egui::vec2(400.0, 600.0),
        )),
        safe_area_insets: Some(egui::SafeAreaInsets(MarginF32 {
            top: 10.0,
            bottom: 20.0,
            ..MarginF32::ZERO
        })),
        ..Default::default()
    };
    raw.viewports
        .get_mut(&egui::ViewportId::ROOT)
        .unwrap()
        .native_pixels_per_point = Some(2.0);
    // egui applies a newly requested zoom at the first pass, rescaling old bounds.
    let _ = ctx.run(raw.clone(), |_| {});
    let _ = ctx.run(raw, |ctx| {
        let measured = view_metrics(ctx);
        assert_eq!(measured.pixels_per_point, 3.0);
        assert_eq!(measured.native_pixels_per_point, Some(2.0));
        assert_eq!(measured.keyboard, KeyboardVisibility::Unknown);
        assert_eq!(measured.occlusion, None);
        assert_eq!(
            measured.unobscured_rect(),
            egui::Rect::from_min_max(egui::pos2(10.0, 30.0), egui::pos2(410.0, 600.0))
        );
        let occlusion = ViewOcclusion::from_physical(
            MarginF32::ZERO,
            MarginF32 {
                bottom: 300.0,
                ..MarginF32::ZERO
            },
            3.0,
        )
        .unwrap();
        let shown = ViewMetrics {
            keyboard: KeyboardVisibility::Visible,
            occlusion: Some(occlusion),
            ..measured
        };
        assert_eq!(shown.unobscured_rect().bottom(), 520.0); // max(20, 100), not 20 + 100
        let covered = ViewMetrics {
            occlusion: Some(ViewOcclusion {
                view_insets: MarginF32::same(10_000.0),
                ..occlusion
            }),
            ..shown
        };
        assert_eq!(covered.unobscured_rect().size(), egui::Vec2::ZERO);
    });
}

//! Display geometry conversion used by mobile integrations.
use egui::{Rect, SafeAreaInsets, Vec2, epaint::MarginF32};

/// Convert a platform content rectangle in physical pixels to logical safe-area
/// margins. Empty/uninitialized geometry and invalid scale factors return `None`.
/// Content outside the surface is clamped, so inset values are never negative.
pub fn safe_area_insets(
    surface: Vec2,
    content: Rect,
    pixels_per_point: f32,
) -> Option<SafeAreaInsets> {
    if !surface.is_finite()
        || surface.x <= 0.0
        || surface.y <= 0.0
        || !content.is_finite()
        || !pixels_per_point.is_finite()
        || pixels_per_point <= 0.0
    {
        return None;
    }
    let bounds = Rect::from_min_size(egui::Pos2::ZERO, surface);
    let content = content.intersect(bounds);
    if content.width() <= 0.0 || content.height() <= 0.0 {
        return None;
    }
    Some(SafeAreaInsets(MarginF32 {
        left: content.left() / pixels_per_point,
        top: content.top() / pixels_per_point,
        right: (surface.x - content.right()) / pixels_per_point,
        bottom: (surface.y - content.bottom()) / pixels_per_point,
    }))
}

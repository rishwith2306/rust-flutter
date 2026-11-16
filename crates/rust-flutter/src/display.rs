//! View geometry and sampled platform keyboard metrics.
//!
//! Call [`view_metrics`] during an egui frame. Unsupported platform measurements
//! remain unknown; keyboard visibility is independent of keyboard occlusion (a
//! floating keyboard can be visible without covering a window edge).
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

/// Actual platform visibility, independent of an application's show/hide request.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeyboardVisibility {
    #[default]
    Unknown,
    Hidden,
    Visible,
}

/// Insets in logical points, measured against the full view bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewOcclusion {
    /// System bars and cutouts, independent of their current visibility.
    pub view_padding: MarginF32,
    /// Fully obscured edges, currently measured from Android's IME insets.
    pub view_insets: MarginF32,
    /// Each edge is `max(view_padding - view_insets, 0)`.
    pub padding: MarginF32,
}
impl ViewOcclusion {
    /// Convert physical measurements at the current egui scale, including zoom.
    /// Invalid or negative measurements return `None`.
    pub fn from_physical(
        view_padding: MarginF32,
        view_insets: MarginF32,
        pixels_per_point: f32,
    ) -> Option<Self> {
        if !pixels_per_point.is_finite() || pixels_per_point <= 0.0 {
            return None;
        }
        let convert = |m: MarginF32| -> Option<MarginF32> {
            let values = [m.left, m.right, m.top, m.bottom];
            if values.iter().any(|v| !v.is_finite() || *v < 0.0) {
                return None;
            }
            let result = m / pixels_per_point;
            [result.left, result.right, result.top, result.bottom]
                .iter()
                .all(|v| v.is_finite())
                .then_some(result)
        };
        let view_padding = convert(view_padding)?;
        let view_insets = convert(view_insets)?;
        Some(Self {
            view_padding,
            view_insets,
            padding: MarginF32 {
                left: (view_padding.left - view_insets.left).max(0.0),
                right: (view_padding.right - view_insets.right).max(0.0),
                top: (view_padding.top - view_insets.top).max(0.0),
                bottom: (view_padding.bottom - view_insets.bottom).max(0.0),
            },
        })
    }
}

/// Current viewport geometry and the latest platform measurements.
///
/// Android samples asynchronously at roughly one-second intervals. On API 30+
/// visibility is available; full occlusion is currently supported only on
/// edge-to-edge Android/target SDK 35+, with matching decor/surface dimensions.
/// Desktop and older Android hosts leave unsupported measurements unknown.
/// This is a geometry subset, not the complete Flutter MediaQuery API.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewMetrics {
    pub viewport: Rect,
    /// Effective physical pixels per egui point, including application zoom.
    pub pixels_per_point: f32,
    /// Native display density, excluding application zoom, if supplied by the host.
    pub native_pixels_per_point: Option<f32>,
    /// Current host safe area. Does not include IME occlusion on Android.
    pub safe_area: SafeAreaInsets,
    pub keyboard: KeyboardVisibility,
    pub occlusion: Option<ViewOcclusion>,
}
impl ViewMetrics {
    /// The rectangular region outside safe areas and known keyboard edge insets.
    /// Overlapping navigation-bar/keyboard insets use their maximum, not their sum.
    /// Unknown occlusion is not guessed; floating/interior obstructions are not represented.
    pub fn unobscured_rect(&self) -> Rect {
        let safe = self.safe_area.0;
        let ime = self.occlusion.map_or(MarginF32::ZERO, |o| o.view_insets);
        let min = egui::pos2(
            self.viewport.left() + safe.left.max(ime.left),
            self.viewport.top() + safe.top.max(ime.top),
        )
        .min(self.viewport.max);
        let max = egui::pos2(
            self.viewport.right() - safe.right.max(ime.right),
            self.viewport.bottom() - safe.bottom.max(ime.bottom),
        )
        .max(min);
        Rect::from_min_max(min, max)
    }
}

/// Selective inherited dependencies for [`ViewMetrics`]. An observation returns
/// a full immutable snapshot but only changes to the selected aspects rebuild it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ViewAspect {
    Viewport,
    Size,
    Orientation,
    PixelsPerPoint,
    NativePixelsPerPoint,
    SafeArea,
    Keyboard,
    ViewPadding,
    ViewInsets,
    Padding,
    Occlusion,
    UnobscuredRect,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    Portrait,
    Landscape,
}
impl ViewMetrics {
    /// Square viewports are portrait, matching Flutter's orientation convention.
    pub fn orientation(&self) -> Orientation {
        if self.viewport.width() > self.viewport.height() {
            Orientation::Landscape
        } else {
            Orientation::Portrait
        }
    }
}
impl crate::element::InheritedModel for ViewMetrics {
    type Aspect = ViewAspect;
    fn update_should_notify(&self, old: &Self) -> bool {
        self != old
    }
    fn update_should_notify_dependent(
        &self,
        old: &Self,
        aspects: &std::collections::HashSet<ViewAspect>,
    ) -> bool {
        aspects.iter().any(|aspect| match aspect {
            ViewAspect::Viewport => self.viewport != old.viewport,
            ViewAspect::Size => self.viewport.size() != old.viewport.size(),
            ViewAspect::Orientation => self.orientation() != old.orientation(),
            ViewAspect::PixelsPerPoint => self.pixels_per_point != old.pixels_per_point,
            ViewAspect::NativePixelsPerPoint => {
                self.native_pixels_per_point != old.native_pixels_per_point
            }
            ViewAspect::SafeArea => self.safe_area != old.safe_area,
            ViewAspect::Keyboard => self.keyboard != old.keyboard,
            ViewAspect::ViewPadding => {
                self.occlusion.map(|o| o.view_padding) != old.occlusion.map(|o| o.view_padding)
            }
            ViewAspect::ViewInsets => {
                self.occlusion.map(|o| o.view_insets) != old.occlusion.map(|o| o.view_insets)
            }
            ViewAspect::Padding => {
                self.occlusion.map(|o| o.padding) != old.occlusion.map(|o| o.padding)
            }
            ViewAspect::Occlusion => self.occlusion != old.occlusion,
            ViewAspect::UnobscuredRect => self.unobscured_rect() != old.unobscured_rect(),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PlatformInsets {
    pub keyboard: KeyboardVisibility,
    /// Physical view padding and IME insets, when decor and native surface agree.
    pub geometry: Option<(Vec2, MarginF32, MarginF32)>,
}

fn platform_key(viewport: egui::ViewportId) -> egui::Id {
    egui::Id::new(("rust-flutter platform insets", viewport))
}

#[cfg(target_os = "android")]
pub(crate) fn set_platform_insets(ctx: &egui::Context, value: Option<PlatformInsets>) {
    let key = platform_key(egui::ViewportId::ROOT);
    ctx.data_mut(|data| {
        if let Some(value) = value {
            data.insert_temp(key, value);
        } else {
            data.remove::<PlatformInsets>(key);
        }
    });
}

/// Read metrics for the active egui viewport. Use during a frame; before the first
/// frame, egui's default viewport geometry is not a platform measurement.
/// Reading this function does not subscribe a retained element. To subscribe,
/// use `BuildContext::depend_on::<ViewMetrics>()` or
/// `BuildContext::depend_on_aspect::<ViewMetrics>(ViewAspect::Size)` instead.
pub fn view_metrics(ctx: &egui::Context) -> ViewMetrics {
    let (viewport, safe_area, native_pixels_per_point, id) = ctx.input(|i| {
        (
            i.viewport_rect(),
            i.safe_area_insets(),
            i.viewport().native_pixels_per_point,
            i.raw.viewport_id,
        )
    });
    let pixels_per_point = ctx.pixels_per_point();
    let platform = ctx.data_mut(|d| d.get_temp::<PlatformInsets>(platform_key(id)));
    let occlusion = platform
        .and_then(|p| p.geometry)
        .and_then(|(size, padding, insets)| {
            let actual = viewport.size() * pixels_per_point;
            if (size - actual).abs().max_elem() > 1.0 {
                return None;
            }
            ViewOcclusion::from_physical(padding, insets, pixels_per_point)
        });
    ViewMetrics {
        viewport,
        pixels_per_point,
        native_pixels_per_point,
        safe_area,
        keyboard: platform.map_or(KeyboardVisibility::Unknown, |p| p.keyboard),
        occlusion,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_keyboard_samples_notify_only_interested_retained_components() {
        use crate::element::*;
        use std::{cell::Cell, rc::Rc};
        struct Observer(ViewAspect, Rc<Cell<usize>>);
        impl StatelessWidget for Observer {
            fn build(&self, ctx: &BuildContext) -> Widget {
                ctx.depend_on_aspect::<ViewMetrics>(self.0)
                    .unwrap()
                    .unwrap();
                self.1.set(self.1.get() + 1);
                Widget::empty()
            }
        }
        let ctx = egui::Context::default();
        let counts: Vec<_> = (0..6).map(|_| Rc::new(Cell::new(0))).collect();
        let aspects = [
            ViewAspect::Size,
            ViewAspect::Keyboard,
            ViewAspect::ViewInsets,
            ViewAspect::ViewPadding,
            ViewAspect::Padding,
            ViewAspect::UnobscuredRect,
        ];
        let raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(400.0, 600.0),
            )),
            ..Default::default()
        };
        let mut tree = None;
        let _ = ctx.run(raw.clone(), |ctx| {
            tree = Some(
                ElementTree::new(
                    ctx,
                    Widget::flex(
                        FlexStyle::new(crate::layout::Axis::Vertical),
                        aspects
                            .iter()
                            .zip(&counts)
                            .map(|(a, c)| {
                                FlexItem::fixed(Widget::stateless(Observer(*a, c.clone())))
                            })
                            .collect(),
                    ),
                )
                .unwrap(),
            );
        });
        let mut tree = tree.unwrap();
        let sample = |keyboard, bottom| PlatformInsets {
            keyboard,
            geometry: Some((
                egui::vec2(400.0, 600.0),
                MarginF32 {
                    bottom: 24.0,
                    ..MarginF32::ZERO
                },
                MarginF32 {
                    bottom,
                    ..MarginF32::ZERO
                },
            )),
        };
        let mut publish = |sample: Option<PlatformInsets>| {
            ctx.data_mut(|d| {
                let key = platform_key(egui::ViewportId::ROOT);
                if let Some(sample) = sample {
                    d.insert_temp(key, sample);
                } else {
                    d.remove::<PlatformInsets>(key);
                }
            });
            let _ = ctx.run(raw.clone(), |_| {
                tree.flush().unwrap();
            });
        };
        publish(Some(sample(KeyboardVisibility::Hidden, 0.0)));
        assert_eq!(
            counts.iter().map(|c| c.get()).collect::<Vec<_>>(),
            [1, 2, 2, 2, 2, 1]
        );
        publish(Some(sample(KeyboardVisibility::Visible, 300.0)));
        assert_eq!(
            counts.iter().map(|c| c.get()).collect::<Vec<_>>(),
            [1, 3, 3, 2, 3, 2]
        );
        publish(Some(sample(KeyboardVisibility::Visible, 300.0))); // repeated polling is quiet
        assert_eq!(
            counts.iter().map(|c| c.get()).collect::<Vec<_>>(),
            [1, 3, 3, 2, 3, 2]
        );
        publish(Some(sample(KeyboardVisibility::Visible, 0.0))); // visible floating keyboard
        assert_eq!(
            counts.iter().map(|c| c.get()).collect::<Vec<_>>(),
            [1, 3, 4, 2, 4, 3]
        );
        publish(Some(sample(KeyboardVisibility::Hidden, 0.0)));
        assert_eq!(
            counts.iter().map(|c| c.get()).collect::<Vec<_>>(),
            [1, 4, 4, 2, 4, 3]
        );
        publish(None); // host can no longer measure the keyboard
        assert_eq!(
            counts.iter().map(|c| c.get()).collect::<Vec<_>>(),
            [1, 5, 5, 3, 5, 3]
        );
    }

    #[test]
    fn sampled_geometry_rejects_resize_mismatches_and_is_scoped_to_a_viewport() {
        let ctx = egui::Context::default();
        ctx.data_mut(|d| {
            d.insert_temp(
                platform_key(egui::ViewportId::ROOT),
                PlatformInsets {
                    keyboard: KeyboardVisibility::Visible,
                    geometry: Some((
                        egui::vec2(800.0, 1200.0),
                        MarginF32::same(40.0),
                        MarginF32 {
                            bottom: 300.0,
                            ..MarginF32::ZERO
                        },
                    )),
                },
            )
        });
        let mut raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(400.0, 600.0),
            )),
            ..Default::default()
        };
        raw.viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .native_pixels_per_point = Some(2.0);
        let _ = ctx.run(raw.clone(), |ctx| {
            let metrics = view_metrics(ctx);
            assert_eq!(metrics.keyboard, KeyboardVisibility::Visible);
            assert_eq!(metrics.occlusion.unwrap().view_insets.bottom, 150.0);
        });
        raw.screen_rect = Some(Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(600.0, 400.0),
        ));
        let _ = ctx.run(raw.clone(), |ctx| {
            let metrics = view_metrics(ctx);
            assert_eq!(metrics.occlusion, None);
            assert_eq!(metrics.keyboard, KeyboardVisibility::Visible);
        });
        let other = egui::ViewportId::from_hash_of("second window");
        raw.viewport_id = other;
        raw.viewports.insert(other, egui::ViewportInfo::default());
        let _ = ctx.run(raw, |ctx| {
            let metrics = view_metrics(ctx);
            assert_eq!(metrics.keyboard, KeyboardVisibility::Unknown);
            assert_eq!(metrics.occlusion, None);
        });
    }
}

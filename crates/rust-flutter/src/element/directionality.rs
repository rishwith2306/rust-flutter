use super::{BuildContext, DependencyError, Widget};
use crate::layout::TextDirection;

/// An inherited text-direction scope for layout-aware widget builders.
///
/// Descendants subscribe with [`Directionality::of`] and apply the returned
/// direction to direction-sensitive layout properties such as
/// [`crate::element::FlexStyle::text_direction`]. Text shaping and automatic
/// localization-based direction selection are separate features.
pub struct Directionality;

impl Directionality {
    /// Makes `direction` available to descendants and rebuilds them when it
    /// changes.
    pub fn provide(direction: TextDirection, child: Widget) -> Widget {
        Widget::inherited(direction, child)
    }

    /// Reads the nearest direction scope and subscribes the current build.
    /// When no scope exists, the default is left-to-right.
    pub fn of(context: &BuildContext) -> Result<TextDirection, DependencyError> {
        Ok(context
            .depend_on::<TextDirection>()?
            .map_or(TextDirection::default(), |direction| *direction))
    }
}

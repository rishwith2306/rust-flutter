//! UI-local scroll positions, metrics and controller attachments.
//! Retained viewports and platform scroll physics are still under development.
mod controller;
mod position;
use crate::animation::AnimationError;
pub use controller::{ScrollAttachment, ScrollController, WeakScrollController};
pub use position::{
    ScrollActivity, ScrollDimensions, ScrollMetrics, ScrollPosition, WeakScrollPosition,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollError {
    Disposed,
    DuringNotification,
    InvalidOffset,
    InvalidDimensions,
    InvalidDuration,
    NoDimensions,
    NotDragging,
    Superseded,
    NoClients,
    MultipleClients,
    AlreadyAttached,
    Animation(AnimationError),
}
impl std::fmt::Display for ScrollError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "scroll: {self:?}")
    }
}
impl std::error::Error for ScrollError {}

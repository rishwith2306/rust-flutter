//! UI-local scroll positions, metrics and controller attachments.
//! Eager retained single-child viewports support captured primary-pointer pans.
//! Wheel/point-scroll signals and interactive scrollbars use the shared binding.
//! Focused viewports support keyboard scrolling. Lazy slivers and platform
//! scroll physics remain unfinished.
mod controller;
mod keyboard;
mod position;
mod scrollbar;
mod viewport;
use crate::animation::AnimationError;
pub use controller::{ScrollAttachment, ScrollController, WeakScrollController};
pub use position::{
    ScrollActivity, ScrollDimensions, ScrollMetrics, ScrollPosition, WeakScrollPosition,
};
pub use scrollbar::{Scrollbar, ScrollbarStyle};
pub use viewport::SingleChildScrollView;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollError {
    Disposed,
    DuringNotification,
    InvalidOffset,
    InvalidDimensions,
    InvalidDuration,
    InvalidScrollbarStyle,
    NoDimensions,
    NotDragging,
    Superseded,
    NoClients,
    MultipleClients,
    AlreadyAttached,
    ViewportAlreadyAttached,
    Binding(crate::gestures::BindingError),
    Animation(AnimationError),
}
impl std::fmt::Display for ScrollError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "scroll: {self:?}")
    }
}
impl std::error::Error for ScrollError {}

//! UI-local scroll positions, metrics and controller attachments.
//! Eager retained single-child viewports support captured primary-pointer pans.
//! Wheel/point-scroll signals and interactive scrollbars use the shared binding.
//! Focused viewports support keyboard scrolling. Fixed-extent lists build only
//! their viewport/cache range. Lazy slivers and platform
//! scroll physics remain unfinished.
mod controller;
mod keep_alive;
mod keyboard;
mod list;
pub use keep_alive::{KeepAlive, KeepAliveHandle};
mod position;
mod scrollbar;
mod viewport;
use crate::animation::AnimationError;
pub use controller::{ScrollAttachment, ScrollController, WeakScrollController};
pub use list::ListView;
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
    InvalidItemExtent,
    InvalidCacheExtent,
    InvalidRealizedItemLimit,
    ContentExtentOverflow,
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

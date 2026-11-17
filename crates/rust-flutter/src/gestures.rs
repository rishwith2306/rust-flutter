//! UI-local gesture arbitration and explicit pointer routing.
mod arena;
mod binding;
mod button;
pub use button::{GestureButton, GestureButtonRenderBox};
mod detector;
pub use binding::{
    BindingError, DispatchFailure, DispatchReport, GestureBinding, GestureFrame, GestureTarget,
    GestureTargetConfig, PointerScrollEvent, ScrollCallback, ScrollClaim, ScrollDispatchFailure,
    ScrollSignalUnit, WeakGestureBinding,
};
pub use detector::{GestureDetector, GestureDetectorRenderBox};
mod recognizers;
mod router;
mod velocity;
pub use arena::{
    ArenaError, ArenaSnapshot, GestureArenaEntry, GestureArenaManager, GestureArenaMember,
    GestureDisposition, PointerId, WeakGestureArenaManager,
};
pub use recognizers::{
    DragEndDetails, DragStartBehavior, DragUpdateDetails, PanCallbacks, PanConfig,
    PanGestureRecognizer, PointerButtons, PointerDeviceKind, PointerEvent, PointerPhase,
    RecognizerError, TapCallbacks, TapConfig, TapDetails, TapGestureRecognizer,
    WeakPanGestureRecognizer, WeakTapGestureRecognizer,
};
pub use router::{PointerRoute, PointerRouter, PointerRouterError, WeakPointerRouter};
pub use velocity::{
    Fling, FlingConfig, Velocity, VelocityError, VelocityEstimate, VelocityEstimateStatus,
    VelocityTracker, VelocityTrackerConfig,
};

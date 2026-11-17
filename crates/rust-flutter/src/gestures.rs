//! UI-local gesture arbitration and explicit pointer routing.
mod arena;
mod binding;
mod detector;
pub use binding::{
    BindingError, DispatchFailure, DispatchReport, GestureBinding, GestureTarget,
    GestureTargetConfig, WeakGestureBinding,
};
pub use detector::{GestureDetector, GestureDetectorRenderBox};
mod recognizers;
mod router;
pub use arena::{
    ArenaError, ArenaSnapshot, GestureArenaEntry, GestureArenaManager, GestureArenaMember,
    GestureDisposition, PointerId, WeakGestureArenaManager,
};
pub use recognizers::{
    DragStartBehavior, DragUpdateDetails, PanCallbacks, PanConfig, PanGestureRecognizer,
    PointerButtons, PointerDeviceKind, PointerEvent, PointerPhase, RecognizerError, TapCallbacks,
    TapConfig, TapDetails, TapGestureRecognizer, WeakPanGestureRecognizer,
    WeakTapGestureRecognizer,
};
pub use router::{PointerRoute, PointerRouter, PointerRouterError, WeakPointerRouter};

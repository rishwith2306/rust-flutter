//! UI-local gesture arbitration and explicit pointer routing.
mod arena;
mod router;
pub use arena::{
    ArenaError, ArenaSnapshot, GestureArenaEntry, GestureArenaManager, GestureArenaMember,
    GestureDisposition, PointerId, WeakGestureArenaManager,
};
pub use router::{PointerRoute, PointerRouter, PointerRouterError, WeakPointerRouter};

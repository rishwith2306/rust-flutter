//! UI-local gesture arbitration. Hosts/recognizers supply pointer routing.
mod arena;
pub use arena::{
    ArenaError, ArenaSnapshot, GestureArenaEntry, GestureArenaManager, GestureArenaMember,
    GestureDisposition, PointerId, WeakGestureArenaManager,
};

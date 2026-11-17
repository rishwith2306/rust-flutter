use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, VecDeque},
    rc::{Rc, Weak},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PointerId(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GestureDisposition {
    Accepted,
    Rejected,
}
/// Notifications are synchronous, occur without manager borrows and are final
/// per registration. A claim to accept is not itself a victory notification.
pub trait GestureArenaMember: 'static {
    fn accept_gesture(&self, pointer: PointerId);
    fn reject_gesture(&self, pointer: PointerId);
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArenaError {
    Closed,
    StillOpen,
    Disposed,
    IdExhausted,
}
impl std::fmt::Display for ArenaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Closed => "cannot add to a closed gesture arena",
            Self::StillOpen => "cannot sweep an open gesture arena",
            Self::Disposed => "gesture arena manager is disposed",
            Self::IdExhausted => "gesture arena ID space exhausted",
        })
    }
}
impl std::error::Error for ArenaError {}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArenaSnapshot {
    pub members: usize,
    pub is_open: bool,
    pub is_held: bool,
    pub pending_sweep: bool,
    pub has_eager_winner: bool,
}
struct Member {
    id: u64,
    member: Rc<dyn GestureArenaMember>,
}
struct Arena {
    generation: u64,
    members: Vec<Member>,
    claims: Vec<u64>,
    open: bool,
    held: bool,
    pending_sweep: bool,
    default_queued: bool,
}
#[derive(Default)]
struct State {
    arenas: HashMap<PointerId, Arena>,
    defaults: VecDeque<(PointerId, u64)>,
    next: u64,
    disposed: bool,
}
struct Inner {
    state: RefCell<State>,
    flushing: Cell<bool>,
}
impl Default for Inner {
    fn default() -> Self {
        Self {
            state: RefCell::new(State::default()),
            flushing: Cell::new(false),
        }
    }
}
/// Cloneable arbitration service. The first active claimant, or the last
/// remaining member, wins. Pointer-up sweep selects the first registration.
#[derive(Clone, Default)]
pub struct GestureArenaManager(Rc<Inner>);
#[derive(Clone)]
pub struct WeakGestureArenaManager(Weak<Inner>);
impl WeakGestureArenaManager {
    pub fn upgrade(&self) -> Option<GestureArenaManager> {
        self.0.upgrade().map(GestureArenaManager)
    }
}
/// Weak, generation-checked participation token. Dropping it does not withdraw
/// the member; explicitly reject it when a recognizer stops tracking a pointer.
#[derive(Clone)]
pub struct GestureArenaEntry {
    manager: Weak<Inner>,
    pointer: PointerId,
    generation: u64,
    member: u64,
}
impl GestureArenaEntry {
    pub fn pointer(&self) -> PointerId {
        self.pointer
    }
    /// False means stale/missing/disposed or an already-recorded accept claim.
    /// A participant can withdraw an eager claim before final resolution.
    pub fn resolve(&self, disposition: GestureDisposition) -> bool {
        self.manager
            .upgrade()
            .is_some_and(|inner| GestureArenaManager(inner).resolve(self, disposition))
    }
}
fn notify(member: &Rc<dyn GestureArenaMember>, pointer: PointerId, accepted: bool) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if accepted {
            member.accept_gesture(pointer)
        } else {
            member.reject_gesture(pointer)
        }
    }));
}
fn reject_all(pointer: PointerId, members: Vec<Member>) {
    for member in members {
        notify(&member.member, pointer, false);
    }
}
fn winner(pointer: PointerId, arena: Arena, id: u64, sweep: bool) {
    let mut selected = None;
    let mut others = vec![];
    for member in arena.members {
        if member.id == id {
            selected = Some(member.member);
        } else {
            others.push(member.member);
        }
    }
    if sweep && let Some(member) = &selected {
        notify(member, pointer, true);
    }
    for member in others {
        notify(&member, pointer, false);
    }
    if !sweep && let Some(member) = &selected {
        notify(member, pointer, true);
    }
}
impl GestureArenaManager {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn downgrade(&self) -> WeakGestureArenaManager {
        WeakGestureArenaManager(Rc::downgrade(&self.0))
    }
    pub fn is_disposed(&self) -> bool {
        self.0.state.borrow().disposed
    }
    pub fn active_pointers(&self) -> usize {
        self.0.state.borrow().arenas.len()
    }
    pub fn arena(&self, pointer: PointerId) -> Option<ArenaSnapshot> {
        self.0
            .state
            .borrow()
            .arenas
            .get(&pointer)
            .map(|a| ArenaSnapshot {
                members: a.members.len(),
                is_open: a.open,
                is_held: a.held,
                pending_sweep: a.pending_sweep,
                has_eager_winner: !a.claims.is_empty(),
            })
    }
    pub fn add(
        &self,
        pointer: PointerId,
        member: Rc<dyn GestureArenaMember>,
    ) -> Result<GestureArenaEntry, ArenaError> {
        let mut state = self.0.state.borrow_mut();
        if state.disposed {
            return Err(ArenaError::Disposed);
        }
        if state.arenas.get(&pointer).is_some_and(|arena| !arena.open) {
            return Err(ArenaError::Closed);
        }
        // Reserve both IDs atomically before mutating an arena.
        let last = state.next.checked_add(2).ok_or(ArenaError::IdExhausted)?;
        let candidate_generation = state.next;
        let id = state.next + 1;
        state.next = last;
        let arena = state.arenas.entry(pointer).or_insert_with(|| Arena {
            generation: candidate_generation,
            members: vec![],
            claims: vec![],
            open: true,
            held: false,
            pending_sweep: false,
            default_queued: false,
        });
        let generation = arena.generation;
        arena.members.push(Member { id, member });
        Ok(GestureArenaEntry {
            manager: Rc::downgrade(&self.0),
            pointer,
            generation,
            member: id,
        })
    }
    pub fn close(&self, pointer: PointerId) -> Result<(), ArenaError> {
        let generation = {
            let mut state = self.0.state.borrow_mut();
            if state.disposed {
                return Err(ArenaError::Disposed);
            }
            let Some(arena) = state.arenas.get_mut(&pointer) else {
                return Ok(());
            };
            arena.open = false;
            arena.generation
        };
        self.try_resolve(pointer, generation);
        Ok(())
    }
    fn try_resolve(&self, pointer: PointerId, generation: u64) {
        let resolution = {
            let mut state = self.0.state.borrow_mut();
            let Some(arena) = state.arenas.get_mut(&pointer) else {
                return;
            };
            if arena.generation != generation || arena.open {
                return;
            }
            if arena.members.is_empty() {
                state.arenas.remove(&pointer);
                None
            } else if arena.members.len() == 1 {
                if !arena.default_queued {
                    arena.default_queued = true;
                    state.defaults.push_back((pointer, generation));
                }
                None
            } else if let Some(id) = arena.claims.first().copied() {
                state.arenas.remove(&pointer).map(|arena| (arena, id))
            } else {
                None
            }
        };
        if let Some((arena, id)) = resolution {
            winner(pointer, arena, id, false);
        }
    }
    fn resolve(&self, entry: &GestureArenaEntry, disposition: GestureDisposition) -> bool {
        match disposition {
            GestureDisposition::Rejected => {
                let member = {
                    let mut state = self.0.state.borrow_mut();
                    if state.disposed {
                        return false;
                    }
                    let Some(arena) = state.arenas.get_mut(&entry.pointer) else {
                        return false;
                    };
                    if arena.generation != entry.generation {
                        return false;
                    }
                    let Some(index) = arena.members.iter().position(|m| m.id == entry.member)
                    else {
                        return false;
                    };
                    arena.claims.retain(|id| *id != entry.member);
                    arena.members.remove(index).member
                };
                notify(&member, entry.pointer, false);
                self.try_resolve(entry.pointer, entry.generation);
                true
            }
            GestureDisposition::Accepted => {
                let resolution = {
                    let mut state = self.0.state.borrow_mut();
                    if state.disposed {
                        return false;
                    }
                    let Some(arena) = state.arenas.get_mut(&entry.pointer) else {
                        return false;
                    };
                    if arena.generation != entry.generation
                        || !arena.members.iter().any(|m| m.id == entry.member)
                        || arena.claims.contains(&entry.member)
                    {
                        return false;
                    }
                    if arena.open {
                        arena.claims.push(entry.member);
                        None
                    } else {
                        state.arenas.remove(&entry.pointer)
                    }
                };
                if let Some(arena) = resolution {
                    winner(entry.pointer, arena, entry.member, false);
                }
                true
            }
        }
    }
    /// Process the defaults queued before this call. Newly queued work waits for
    /// a later flush. Reentrant flush is a no-op; explicit accepts remain immediate.
    pub fn flush(&self) -> usize {
        if self.0.flushing.replace(true) {
            return 0;
        }
        struct Guard<'a>(&'a Cell<bool>);
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _guard = Guard(&self.0.flushing);
        let queued = std::mem::take(&mut self.0.state.borrow_mut().defaults);
        let mut resolved = 0;
        for (pointer, generation) in queued {
            let arena =
                {
                    let mut state = self.0.state.borrow_mut();
                    if state.disposed {
                        break;
                    }
                    if state.arenas.get(&pointer).is_some_and(|a| {
                        a.generation == generation && !a.open && a.members.len() == 1
                    }) {
                        state.arenas.remove(&pointer)
                    } else {
                        None
                    }
                };
            if let Some(arena) = arena {
                let id = arena.members[0].id;
                winner(pointer, arena, id, false);
                resolved += 1;
            }
        }
        resolved
    }
    pub fn hold(&self, pointer: PointerId) -> Result<(), ArenaError> {
        let mut state = self.0.state.borrow_mut();
        if state.disposed {
            return Err(ArenaError::Disposed);
        }
        if let Some(arena) = state.arenas.get_mut(&pointer) {
            arena.held = true;
        }
        Ok(())
    }
    pub fn release(&self, pointer: PointerId) -> Result<(), ArenaError> {
        let sweep = {
            let mut state = self.0.state.borrow_mut();
            if state.disposed {
                return Err(ArenaError::Disposed);
            }
            if let Some(arena) = state.arenas.get_mut(&pointer) {
                arena.held = false;
                arena.pending_sweep
            } else {
                false
            }
        };
        if sweep {
            self.sweep(pointer)?;
        }
        Ok(())
    }
    pub fn sweep(&self, pointer: PointerId) -> Result<(), ArenaError> {
        let arena = {
            let mut state = self.0.state.borrow_mut();
            if state.disposed {
                return Err(ArenaError::Disposed);
            }
            let Some(arena) = state.arenas.get_mut(&pointer) else {
                return Ok(());
            };
            if arena.open {
                return Err(ArenaError::StillOpen);
            }
            if arena.held {
                arena.pending_sweep = true;
                return Ok(());
            }
            state.arenas.remove(&pointer)
        };
        if let Some(arena) = arena
            && let Some(first) = arena.members.first()
        {
            let id = first.id;
            winner(pointer, arena, id, true);
        }
        Ok(())
    }
    pub fn cancel(&self, pointer: PointerId) -> bool {
        let arena = self.0.state.borrow_mut().arenas.remove(&pointer);
        if let Some(arena) = arena {
            reject_all(pointer, arena.members);
            true
        } else {
            false
        }
    }
    pub fn dispose(&self) -> Result<(), ArenaError> {
        let arenas = {
            let mut state = self.0.state.borrow_mut();
            if state.disposed {
                return Err(ArenaError::Disposed);
            }
            state.disposed = true;
            state.defaults.clear();
            std::mem::take(&mut state.arenas)
        };
        let mut arenas = arenas.into_iter().collect::<Vec<_>>();
        arenas.sort_by_key(|(pointer, _)| *pointer);
        for (pointer, arena) in arenas {
            reject_all(pointer, arena.members);
        }
        Ok(())
    }
}
impl Drop for Inner {
    fn drop(&mut self) {
        let state = self.state.get_mut();
        let mut arenas = std::mem::take(&mut state.arenas)
            .into_iter()
            .collect::<Vec<_>>();
        arenas.sort_by_key(|(pointer, _)| *pointer);
        for (pointer, arena) in arenas {
            reject_all(pointer, arena.members);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Silent;
    impl GestureArenaMember for Silent {
        fn accept_gesture(&self, _: PointerId) {}
        fn reject_gesture(&self, _: PointerId) {}
    }
    #[test]
    fn exhausted_ids_do_not_leave_a_partially_created_arena() {
        let manager = GestureArenaManager::new();
        manager.0.state.borrow_mut().next = u64::MAX - 1;
        assert!(matches!(
            manager.add(PointerId(0), Rc::new(Silent)),
            Err(ArenaError::IdExhausted)
        ));
        assert_eq!(manager.active_pointers(), 0);
        assert_eq!(manager.0.state.borrow().next, u64::MAX - 1);
        assert_eq!(manager.flush(), 0);
    }
}

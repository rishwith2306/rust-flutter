use super::PointerId;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::{Rc, Weak},
};

/// Registration failed before changing the router.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerRouterError {
    Disposed,
    IdExhausted,
}
impl std::fmt::Display for PointerRouterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Disposed => "pointer router is disposed",
            Self::IdExhausted => "pointer route ID space exhausted",
        })
    }
}
impl std::error::Error for PointerRouterError {}

type Callback<E> = Rc<dyn Fn(PointerId, &E)>;
struct Registration<E> {
    pointer: Option<PointerId>,
    callback: Callback<E>,
}
struct State<E> {
    registrations: BTreeMap<u64, Registration<E>>,
    next: u64,
    disposed: bool,
}

/// UI-local routing for host-defined events. Routing does not create arenas or
/// interpret event payloads. Retain each returned token for the tracking lifetime.
///
/// Each dispatch snapshots matching routes, then invokes pointer-specific routes
/// before global routes, in registration order within each group. Removing a
/// route before its turn skips it. Additions wait for the next dispatch, including
/// an explicit nested dispatch. Callbacks run without router borrows and may route
/// recursively, cancel routes or dispose the router.
pub struct PointerRouter<E: 'static> {
    state: Rc<RefCell<State<E>>>,
}
/// A callback-safe reference that does not keep the router alive.
pub struct WeakPointerRouter<E: 'static> {
    state: Weak<RefCell<State<E>>>,
}
impl<E: 'static> Clone for WeakPointerRouter<E> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}
impl<E: 'static> WeakPointerRouter<E> {
    pub fn upgrade(&self) -> Option<PointerRouter<E>> {
        self.state.upgrade().map(|state| PointerRouter { state })
    }
}
impl<E: 'static> Clone for PointerRouter<E> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}
impl<E: 'static> Default for PointerRouter<E> {
    fn default() -> Self {
        Self::new()
    }
}
impl<E: 'static> PointerRouter<E> {
    /// Prefer weak captures inside routes to avoid reference cycles.
    pub fn downgrade(&self) -> WeakPointerRouter<E> {
        WeakPointerRouter {
            state: Rc::downgrade(&self.state),
        }
    }
    pub fn new() -> Self {
        Self {
            state: Rc::new(RefCell::new(State {
                registrations: BTreeMap::new(),
                next: 0,
                disposed: false,
            })),
        }
    }

    pub fn add_route(
        &self,
        pointer: PointerId,
        callback: impl Fn(PointerId, &E) + 'static,
    ) -> Result<PointerRoute<E>, PointerRouterError> {
        self.add(Some(pointer), Rc::new(callback))
    }

    pub fn add_global_route(
        &self,
        callback: impl Fn(PointerId, &E) + 'static,
    ) -> Result<PointerRoute<E>, PointerRouterError> {
        self.add(None, Rc::new(callback))
    }

    fn add(
        &self,
        pointer: Option<PointerId>,
        callback: Callback<E>,
    ) -> Result<PointerRoute<E>, PointerRouterError> {
        let mut state = self.state.borrow_mut();
        if state.disposed {
            return Err(PointerRouterError::Disposed);
        }
        let id = state.next;
        let next = id.checked_add(1).ok_or(PointerRouterError::IdExhausted)?;
        state.next = next;
        state
            .registrations
            .insert(id, Registration { pointer, callback });
        Ok(PointerRoute {
            state: Rc::downgrade(&self.state),
            id,
        })
    }

    /// Dispatch one borrowed payload. Returns the number of callbacks attempted;
    /// a panicking callback counts and does not stop others during Rust unwinding.
    /// Disposed routers ignore events. There is no implicit up/cancel cleanup.
    pub fn route(&self, pointer: PointerId, event: &E) -> usize {
        let routes: Vec<_> = {
            let state = self.state.borrow();
            let local = state
                .registrations
                .iter()
                .filter(|(_, r)| r.pointer == Some(pointer));
            let global = state
                .registrations
                .iter()
                .filter(|(_, r)| r.pointer.is_none());
            local
                .chain(global)
                .map(|(&id, r)| (id, r.callback.clone()))
                .collect()
        };
        let mut called = 0;
        for (id, callback) in routes {
            if !self.state.borrow().registrations.contains_key(&id) {
                continue;
            }
            called += 1;
            let _ =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback(pointer, event)));
        }
        called
    }

    /// Remove all routes for this pointer; global routes remain registered.
    pub fn remove_pointer(&self, pointer: PointerId) -> usize {
        let removed = {
            let mut state = self.state.borrow_mut();
            let ids: Vec<_> = state
                .registrations
                .iter()
                .filter_map(|(&id, r)| (r.pointer == Some(pointer)).then_some(id))
                .collect();
            ids.into_iter()
                .filter_map(|id| state.registrations.remove(&id))
                .collect::<Vec<_>>()
        };
        // Drop captures after releasing the router borrow: capture destructors
        // may themselves own route tokens or reenter the router.
        removed.len()
    }

    pub fn route_count(&self) -> usize {
        self.state.borrow().registrations.len()
    }
    pub fn is_disposed(&self) -> bool {
        self.state.borrow().disposed
    }

    /// Terminal and idempotent. Tokens become inactive; no cancellation event is
    /// synthesized. Hosts must dispatch any required cancel payload first.
    pub fn dispose(&self) {
        let removed = {
            let mut state = self.state.borrow_mut();
            state.disposed = true;
            std::mem::take(&mut state.registrations)
        };
        drop(removed);
    }
}

/// An owned route subscription. Drop or `cancel` unregisters it. This token does
/// not retain its router. Unlike an arena entry, dropping it stops participation.
#[must_use = "dropping the pointer route unregisters its callback"]
pub struct PointerRoute<E: 'static> {
    state: Weak<RefCell<State<E>>>,
    id: u64,
}
impl<E: 'static> PointerRoute<E> {
    pub fn is_active(&self) -> bool {
        self.state
            .upgrade()
            .is_some_and(|s| s.borrow().registrations.contains_key(&self.id))
    }

    /// Returns true only if this call removed a live registration.
    pub fn cancel(&self) -> bool {
        let removed = self
            .state
            .upgrade()
            .and_then(|s| s.borrow_mut().registrations.remove(&self.id));
        removed.is_some()
    }
}
impl<E: 'static> Drop for PointerRoute<E> {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exhausted_ids_do_not_modify_registrations() {
        let router = PointerRouter::<()>::new();
        let token = router.add_global_route(|_, _| {}).unwrap();
        router.state.borrow_mut().next = u64::MAX;
        assert!(matches!(
            router.add_route(PointerId(1), |_, _| {}),
            Err(PointerRouterError::IdExhausted)
        ));
        assert_eq!(router.route_count(), 1);
        assert!(token.is_active());
    }
}

use rust_flutter::gestures::*;
use std::{cell::RefCell, rc::Rc};
type Log = Rc<RefCell<Vec<(&'static str, bool, PointerId)>>>;
struct Member {
    name: &'static str,
    log: Log,
    on_accept: Option<Rc<dyn Fn()>>,
    on_reject: Option<Rc<dyn Fn()>>,
}
impl GestureArenaMember for Member {
    fn accept_gesture(&self, p: PointerId) {
        self.log.borrow_mut().push((self.name, true, p));
        if let Some(f) = &self.on_accept {
            f();
        }
    }
    fn reject_gesture(&self, p: PointerId) {
        self.log.borrow_mut().push((self.name, false, p));
        if let Some(f) = &self.on_reject {
            f();
        }
    }
}
fn member(name: &'static str, log: &Log) -> Rc<Member> {
    Rc::new(Member {
        name,
        log: log.clone(),
        on_accept: None,
        on_reject: None,
    })
}
fn p() -> PointerId {
    PointerId(9)
}
fn outcomes(log: &Log) -> Vec<(&'static str, bool)> {
    log.borrow()
        .iter()
        .map(|(name, won, _)| (*name, *won))
        .collect()
}
#[test]
fn eager_accept_waits_for_close_and_first_claimant_wins() {
    let m = GestureArenaManager::new();
    let log: Log = Rc::default();
    let _a = m.add(p(), member("a", &log)).unwrap();
    let b = m.add(p(), member("b", &log)).unwrap();
    let c = m.add(p(), member("c", &log)).unwrap();
    assert!(b.resolve(GestureDisposition::Accepted));
    assert!(c.resolve(GestureDisposition::Accepted));
    assert!(log.borrow().is_empty());
    assert!(m.arena(p()).unwrap().has_eager_winner);
    m.close(p()).unwrap();
    assert_eq!(
        outcomes(&log),
        vec![("a", false), ("c", false), ("b", true)]
    );
    assert_eq!(m.active_pointers(), 0);
}
#[test]
fn explicit_accept_after_close_resolves_immediately_before_deferred_default() {
    let m = GestureArenaManager::new();
    let log: Log = Rc::default();
    let a = m.add(p(), member("a", &log)).unwrap();
    m.close(p()).unwrap();
    assert!(log.borrow().is_empty());
    assert!(a.resolve(GestureDisposition::Accepted));
    assert_eq!(outcomes(&log), vec![("a", true)]);
    assert_eq!(m.flush(), 0);
    assert!(!a.resolve(GestureDisposition::Accepted));
}
#[test]
fn last_remaining_member_is_deferred_and_can_reject_before_flush() {
    for reject in [false, true] {
        let m = GestureArenaManager::new();
        let log: Log = Rc::default();
        let a = m.add(p(), member("a", &log)).unwrap();
        let b = m.add(p(), member("b", &log)).unwrap();
        m.close(p()).unwrap();
        a.resolve(GestureDisposition::Rejected);
        assert_eq!(outcomes(&log), vec![("a", false)]);
        if reject {
            b.resolve(GestureDisposition::Rejected);
            assert_eq!(m.flush(), 0);
            assert_eq!(outcomes(&log), vec![("a", false), ("b", false)]);
        } else {
            assert_eq!(m.flush(), 1);
            assert_eq!(outcomes(&log), vec![("a", false), ("b", true)]);
        }
    }
}
#[test]
fn hold_delays_sweep_and_release_awards_first_registration_in_sweep_order() {
    let m = GestureArenaManager::new();
    let log: Log = Rc::default();
    m.add(p(), member("a", &log)).unwrap();
    m.add(p(), member("b", &log)).unwrap();
    m.close(p()).unwrap();
    m.hold(p()).unwrap();
    m.sweep(p()).unwrap();
    assert!(m.arena(p()).unwrap().pending_sweep);
    assert!(log.borrow().is_empty());
    m.release(p()).unwrap();
    assert_eq!(outcomes(&log), vec![("a", true), ("b", false)]);
    m.release(p()).unwrap();
    assert_eq!(m.flush(), 0);
}
#[test]
fn hold_does_not_block_last_member_default_or_explicit_victory() {
    let m = GestureArenaManager::new();
    let log: Log = Rc::default();
    m.add(p(), member("a", &log)).unwrap();
    m.hold(p()).unwrap();
    m.close(p()).unwrap();
    assert_eq!(m.flush(), 1);
    assert_eq!(outcomes(&log), vec![("a", true)]);
    let a = m.add(p(), member("b", &log)).unwrap();
    m.add(p(), member("c", &log)).unwrap();
    m.close(p()).unwrap();
    m.hold(p()).unwrap();
    a.resolve(GestureDisposition::Accepted);
    assert_eq!(outcomes(&log), vec![("a", true), ("c", false), ("b", true)]);
}
#[test]
fn stale_entries_and_defaults_cannot_resolve_a_reused_pointer() {
    let m = GestureArenaManager::new();
    let log: Log = Rc::default();
    let old = m.add(p(), member("old", &log)).unwrap();
    m.close(p()).unwrap();
    assert!(m.cancel(p()));
    let new = m.add(p(), member("new", &log)).unwrap();
    m.close(p()).unwrap();
    assert!(!old.resolve(GestureDisposition::Accepted));
    assert_eq!(m.arena(p()).unwrap().members, 1);
    assert_eq!(m.flush(), 1);
    assert!(!new.resolve(GestureDisposition::Rejected));
    assert_eq!(outcomes(&log), vec![("old", false), ("new", true)]);
}
#[test]
fn rejection_callback_can_cancel_and_recreate_the_same_pointer() {
    let m = GestureArenaManager::new();
    let log: Log = Rc::default();
    let weak = m.downgrade();
    let recorded = log.clone();
    let a = m
        .add(
            p(),
            Rc::new(Member {
                name: "a",
                log: log.clone(),
                on_accept: None,
                on_reject: Some(Rc::new(move || {
                    let m = weak.upgrade().unwrap();
                    m.cancel(p());
                    m.add(p(), member("new", &recorded)).unwrap();
                    m.close(p()).unwrap();
                })),
            }),
        )
        .unwrap();
    m.add(p(), member("b", &log)).unwrap();
    m.close(p()).unwrap();
    a.resolve(GestureDisposition::Rejected);
    assert_eq!(m.flush(), 1);
    assert_eq!(
        outcomes(&log),
        vec![("a", false), ("b", false), ("new", true)]
    );
}
#[test]
fn flush_has_a_snapshot_boundary_and_reentrant_flush_does_not_drain_new_work() {
    let m = GestureArenaManager::new();
    let log: Log = Rc::default();
    let weak = m.downgrade();
    let recorded = log.clone();
    m.add(
        p(),
        Rc::new(Member {
            name: "a",
            log: log.clone(),
            on_reject: None,
            on_accept: Some(Rc::new(move || {
                let m = weak.upgrade().unwrap();
                m.add(PointerId(10), member("b", &recorded)).unwrap();
                m.close(PointerId(10)).unwrap();
                assert_eq!(m.flush(), 0);
            })),
        }),
    )
    .unwrap();
    m.close(p()).unwrap();
    assert_eq!(m.flush(), 1);
    assert_eq!(outcomes(&log), vec![("a", true)]);
    assert_eq!(m.flush(), 1);
    assert_eq!(outcomes(&log), vec![("a", true), ("b", true)]);
}
#[test]
fn dispose_from_loser_callback_finishes_committed_resolution_without_borrow_conflicts() {
    let m = GestureArenaManager::new();
    let log: Log = Rc::default();
    let weak = m.downgrade();
    m.add(
        p(),
        Rc::new(Member {
            name: "a",
            log: log.clone(),
            on_accept: None,
            on_reject: Some(Rc::new(move || weak.upgrade().unwrap().dispose().unwrap())),
        }),
    )
    .unwrap();
    let b = m.add(p(), member("b", &log)).unwrap();
    m.add(PointerId(10), member("other", &log)).unwrap();
    m.close(p()).unwrap();
    assert!(b.resolve(GestureDisposition::Accepted));
    assert_eq!(
        outcomes(&log),
        vec![("a", false), ("other", false), ("b", true)]
    );
    assert!(m.is_disposed());
    assert_eq!(m.close(p()), Err(ArenaError::Disposed));
    assert!(matches!(
        m.add(p(), member("late", &log)),
        Err(ArenaError::Disposed)
    ));
    assert_eq!(m.flush(), 0);
}
#[test]
fn dropping_manager_rejects_unresolved_members_and_entries_do_not_keep_it_alive() {
    let m = GestureArenaManager::new();
    let log: Log = Rc::default();
    let weak = m.downgrade();
    let entry = m.add(p(), member("a", &log)).unwrap();
    m.close(p()).unwrap();
    drop(m);
    assert!(weak.upgrade().is_none());
    assert!(!entry.resolve(GestureDisposition::Accepted));
    assert_eq!(outcomes(&log), vec![("a", false)]);
}
#[test]
fn withdrawing_eager_claim_preserves_remaining_claim_order() {
    let m = GestureArenaManager::new();
    let log: Log = Rc::default();
    let a = m.add(p(), member("a", &log)).unwrap();
    let b = m.add(p(), member("b", &log)).unwrap();
    let c = m.add(p(), member("c", &log)).unwrap();
    a.resolve(GestureDisposition::Accepted);
    c.resolve(GestureDisposition::Accepted);
    b.resolve(GestureDisposition::Accepted);
    assert!(!a.resolve(GestureDisposition::Accepted));
    assert!(a.resolve(GestureDisposition::Rejected));
    m.close(p()).unwrap();
    assert_eq!(
        outcomes(&log),
        vec![("a", false), ("b", false), ("c", true)]
    );
}
#[test]
fn protocol_errors_empty_arenas_and_independent_pointers_preserve_state() {
    let m = GestureArenaManager::new();
    let log: Log = Rc::default();
    let a = m.add(p(), member("a", &log)).unwrap();
    assert_eq!(m.sweep(p()), Err(ArenaError::StillOpen));
    assert_eq!(m.arena(p()).unwrap().members, 1);
    a.resolve(GestureDisposition::Rejected);
    m.close(p()).unwrap();
    assert_eq!(m.active_pointers(), 0);
    m.add(PointerId(1), member("one", &log)).unwrap();
    m.close(PointerId(1)).unwrap();
    assert!(matches!(
        m.add(PointerId(1), member("late", &log)),
        Err(ArenaError::Closed)
    ));
    let other = m.add(PointerId(2), member("two", &log)).unwrap();
    assert_eq!(other.pointer(), PointerId(2));
    m.close(PointerId(2)).unwrap();
    m.cancel(PointerId(1));
    assert_eq!(m.flush(), 1);
    assert!(log.borrow().contains(&("two", true, PointerId(2))));
}
#[test]
fn callback_panics_do_not_prevent_other_final_notifications() {
    let m = GestureArenaManager::new();
    let log: Log = Rc::default();
    m.add(
        p(),
        Rc::new(Member {
            name: "loser",
            log: log.clone(),
            on_accept: None,
            on_reject: Some(Rc::new(|| panic!("isolated arena callback"))),
        }),
    )
    .unwrap();
    let win = m.add(p(), member("winner", &log)).unwrap();
    m.close(p()).unwrap();
    win.resolve(GestureDisposition::Accepted);
    assert_eq!(outcomes(&log), vec![("loser", false), ("winner", true)]);
    assert_eq!(m.active_pointers(), 0);
}

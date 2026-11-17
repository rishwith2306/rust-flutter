use rust_flutter::gestures::*;
use std::{cell::RefCell, rc::Rc};

#[test]
fn routes_match_pointer_and_local_precedes_global_in_registration_order() {
    let router = PointerRouter::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut tokens = Vec::new();
    for (name, pointer) in [
        ("g1", None),
        ("p1", Some(1)),
        ("other", Some(2)),
        ("g2", None),
        ("p2", Some(1)),
    ] {
        let log = log.clone();
        let callback = move |p, value: &i32| log.borrow_mut().push((name, p, *value));
        tokens.push(match pointer {
            Some(p) => router.add_route(PointerId(p), callback).unwrap(),
            None => router.add_global_route(callback).unwrap(),
        });
    }
    assert_eq!(router.route(PointerId(1), &7), 4);
    assert_eq!(
        *log.borrow(),
        vec![
            ("p1", PointerId(1), 7),
            ("p2", PointerId(1), 7),
            ("g1", PointerId(1), 7),
            ("g2", PointerId(1), 7)
        ]
    );
    log.borrow_mut().clear();
    assert_eq!(router.route(PointerId(2), &9), 3);
    assert_eq!(log.borrow()[0], ("other", PointerId(2), 9));
    assert_eq!(router.route_count(), 5);
    drop(tokens);
    assert_eq!(router.route_count(), 0);
}

#[test]
fn cancellation_during_dispatch_skips_pending_callbacks() {
    let router = PointerRouter::<()>::new();
    let pending = Rc::new(RefCell::new(None::<PointerRoute<()>>));
    let other = pending.clone();
    let first = router
        .add_route(PointerId(1), move |_, _| {
            assert!(other.borrow().as_ref().unwrap().cancel());
        })
        .unwrap();
    *pending.borrow_mut() = Some(
        router
            .add_global_route(|_, _| panic!("canceled callback ran"))
            .unwrap(),
    );
    assert_eq!(router.route(PointerId(1), &()), 1);
    assert!(!pending.borrow().as_ref().unwrap().is_active());
    assert!(first.cancel());
    assert!(!first.cancel());
}

#[test]
fn additions_wait_for_next_dispatch_and_nested_dispatch_has_its_own_snapshot() {
    let router = PointerRouter::<u8>::new();
    let weak = router.downgrade();
    let log = Rc::new(RefCell::new(Vec::new()));
    let added = Rc::new(RefCell::new(Vec::new()));
    let captures = (log.clone(), added.clone());
    let _first = router
        .add_route(PointerId(3), move |p, &value| {
            captures.0.borrow_mut().push(("first", value));
            if value == 0 {
                let router = weak.upgrade().unwrap();
                let log = captures.0.clone();
                captures.1.borrow_mut().push(
                    router
                        .add_global_route(move |_, &value| {
                            log.borrow_mut().push(("new", value));
                        })
                        .unwrap(),
                );
                assert_eq!(router.route(p, &1), 2);
            }
        })
        .unwrap();
    assert_eq!(router.route(PointerId(3), &0), 1);
    assert_eq!(*log.borrow(), vec![("first", 0), ("first", 1), ("new", 1)]);
    assert_eq!(router.route(PointerId(3), &2), 2);
}

#[test]
fn remove_pointer_keeps_global_routes_and_supports_pointer_reuse() {
    let router = PointerRouter::<()>::new();
    let old = router.add_route(PointerId(1), |_, _| {}).unwrap();
    let other = router.add_route(PointerId(2), |_, _| {}).unwrap();
    let global = router.add_global_route(|_, _| {}).unwrap();
    assert_eq!(router.remove_pointer(PointerId(1)), 1);
    assert_eq!(router.remove_pointer(PointerId(1)), 0);
    let new = router.add_route(PointerId(1), |_, _| {}).unwrap();
    assert!(!old.cancel());
    assert!(new.is_active() && other.is_active() && global.is_active());
    assert_eq!(router.route(PointerId(1), &()), 2);
}

#[test]
fn disposal_during_dispatch_skips_remaining_routes_and_is_terminal() {
    let router = PointerRouter::<()>::new();
    let weak = router.downgrade();
    let first = router
        .add_route(PointerId(1), move |_, _| weak.upgrade().unwrap().dispose())
        .unwrap();
    let second = router
        .add_route(PointerId(1), |_, _| panic!("disposed callback ran"))
        .unwrap();
    assert_eq!(router.route(PointerId(1), &()), 1);
    assert!(!first.is_active() && !second.is_active());
    assert_eq!(router.route(PointerId(1), &()), 0);
    assert!(router.is_disposed());
    assert!(matches!(
        router.add_global_route(|_, _| {}),
        Err(PointerRouterError::Disposed)
    ));
    router.dispose();
}

#[test]
fn callback_panics_do_not_prevent_remaining_notifications() {
    let router = PointerRouter::<()>::new();
    let _bad = router
        .add_route(PointerId(1), |_, _| panic!("expected callback panic"))
        .unwrap();
    let count = Rc::new(RefCell::new(0));
    let c = count.clone();
    let _good = router
        .add_global_route(move |_, _| *c.borrow_mut() += 1)
        .unwrap();
    assert_eq!(router.route(PointerId(1), &()), 2);
    assert_eq!(*count.borrow(), 1);
}

#[test]
fn captures_drop_outside_router_borrows_even_when_they_own_other_tokens() {
    let router = PointerRouter::<()>::new();
    let global = router.add_global_route(|_, _| {}).unwrap();
    let owner = router
        .add_route(PointerId(1), move |_, _| {
            assert!(global.is_active());
        })
        .unwrap();
    assert_eq!(router.route_count(), 2);
    assert!(owner.cancel());
    assert_eq!(router.route_count(), 0);
    let global = router.add_global_route(|_, _| {}).unwrap();
    let _owner = router
        .add_route(PointerId(1), move |_, _| {
            assert!(global.is_active());
        })
        .unwrap();
    assert_eq!(router.remove_pointer(PointerId(1)), 1);
    assert_eq!(router.route_count(), 0);
}

#[test]
fn tokens_and_weak_references_do_not_retain_router_or_captures() {
    let router = PointerRouter::<()>::new();
    let weak = router.downgrade();
    let capture = Rc::new(());
    let observed = Rc::downgrade(&capture);
    let token = router
        .add_global_route(move |_, _| {
            let _ = &capture;
        })
        .unwrap();
    drop(router);
    assert!(weak.upgrade().is_none());
    assert!(observed.upgrade().is_none());
    assert!(!token.is_active());
    assert!(!token.cancel());
}

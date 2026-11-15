use rust_flutter::notifier::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[test]
fn dispatch_skips_removed_callbacks_and_defers_added_callbacks() {
    let source = ChangeNotifier::new();
    let weak = source.downgrade();
    let second = Rc::new(Cell::new(None));
    let added = Rc::new(Cell::new(false));
    let trace = Rc::new(RefCell::new(Vec::new()));
    source
        .add_listener(Rc::new({
            let second = second.clone();
            let added = added.clone();
            let trace = trace.clone();
            move || {
                trace.borrow_mut().push("first");
                let source = weak.upgrade().unwrap();
                if !added.replace(true) {
                    assert!(source.remove_listener(second.get().unwrap()));
                    let trace = trace.clone();
                    source
                        .add_listener(Rc::new(move || trace.borrow_mut().push("late")))
                        .unwrap();
                }
            }
        }))
        .unwrap();
    second.set(Some(
        source
            .add_listener(Rc::new({
                let trace = trace.clone();
                move || trace.borrow_mut().push("removed")
            }))
            .unwrap(),
    ));
    assert_eq!(source.notify_listeners().unwrap().called, 1);
    assert_eq!(*trace.borrow(), ["first"]);
    assert_eq!(source.notify_listeners().unwrap().called, 2);
    assert_eq!(*trace.borrow(), ["first", "first", "late"]);
}

#[test]
fn recursive_dispatch_keeps_each_iteration_boundary_and_registration_order() {
    let source = ChangeNotifier::new();
    let weak = source.downgrade();
    let entered = Rc::new(Cell::new(false));
    let trace = Rc::new(RefCell::new(Vec::new()));
    source
        .add_listener(Rc::new({
            let trace = trace.clone();
            move || {
                trace.borrow_mut().push(1);
                if !entered.replace(true) {
                    assert_eq!(
                        weak.upgrade().unwrap().notify_listeners().unwrap().called,
                        2
                    );
                }
            }
        }))
        .unwrap();
    source
        .add_listener(Rc::new({
            let trace = trace.clone();
            move || trace.borrow_mut().push(2)
        }))
        .unwrap();
    assert_eq!(source.notify_listeners().unwrap().called, 2);
    assert_eq!(*trace.borrow(), [1, 1, 2, 2]);
}

#[test]
fn duplicate_callbacks_have_independent_registration_tokens() {
    let source = ChangeNotifier::new();
    let hits = Rc::new(Cell::new(0));
    let callback: Listener = Rc::new({
        let hits = hits.clone();
        move || hits.set(hits.get() + 1)
    });
    let one = source.add_listener(callback.clone()).unwrap();
    let two = source.add_listener(callback).unwrap();
    assert_ne!(one, two);
    assert!(source.remove_listener(one));
    source.notify_listeners().unwrap();
    assert_eq!(hits.get(), 1);
    assert!(!source.remove_listener(one));
    assert!(source.remove_listener(two));
    assert!(!source.has_listeners());
    let unrelated = ChangeNotifier::new();
    unrelated.add_listener(Rc::new(|| {})).unwrap();
    assert!(!unrelated.remove_listener(two));
}

#[test]
fn subscriptions_cancel_exactly_once_and_do_not_keep_the_source_alive() {
    let source = ChangeNotifier::new();
    let weak = source.downgrade();
    let mut subscription = source.subscribe(Rc::new(|| {})).unwrap();
    assert!(source.has_listeners());
    subscription.cancel();
    subscription.cancel();
    assert!(!source.has_listeners());
    let subscription = source.subscribe(Rc::new(|| {})).unwrap();
    drop(source);
    assert!(weak.upgrade().is_none());
    drop(subscription);
}

#[test]
fn panicking_listener_is_reported_without_starving_later_listeners() {
    let source = ChangeNotifier::new();
    let hits = Rc::new(Cell::new(0));
    source
        .add_listener(Rc::new(|| panic!("listener failure")))
        .unwrap();
    source
        .add_listener(Rc::new({
            let hits = hits.clone();
            move || hits.set(hits.get() + 1)
        }))
        .unwrap();
    assert_eq!(
        source.notify_listeners().unwrap(),
        NotificationReport {
            called: 2,
            panicked: 1
        }
    );
    assert_eq!(hits.get(), 1);
    assert!(source.dispose().is_ok());
}

#[test]
fn disposal_is_rejected_during_dispatch_and_checked_afterwards() {
    let source = ChangeNotifier::new();
    let weak = source.downgrade();
    source
        .add_listener(Rc::new(move || {
            assert_eq!(
                weak.upgrade().unwrap().dispose(),
                Err(DisposeError::DuringNotification)
            )
        }))
        .unwrap();
    source.notify_listeners().unwrap();
    source.dispose().unwrap();
    assert!(source.is_disposed());
    assert!(!source.has_listeners());
    assert_eq!(source.dispose(), Err(DisposeError::Disposed));
    assert_eq!(source.notify_listeners(), Err(Disposed));
    assert!(matches!(source.subscribe(Rc::new(|| {})), Err(Disposed)));
}

#[test]
fn callback_capture_destructors_run_outside_the_listener_borrow() {
    struct InspectOnDrop {
        source: WeakChangeNotifier,
        count: Rc<Cell<usize>>,
    }
    impl Drop for InspectOnDrop {
        fn drop(&mut self) {
            if let Some(source) = self.source.upgrade() {
                let _ = source.has_listeners();
            }
            self.count.set(self.count.get() + 1);
        }
    }
    let source = ChangeNotifier::new();
    let count = Rc::new(Cell::new(0));
    let probe = InspectOnDrop {
        source: source.downgrade(),
        count: count.clone(),
    };
    let id = source
        .add_listener(Rc::new(move || {
            let _ = &probe;
        }))
        .unwrap();
    source.remove_listener(id);
    assert_eq!(count.get(), 1);
    let probe = InspectOnDrop {
        source: source.downgrade(),
        count: count.clone(),
    };
    source
        .add_listener(Rc::new(move || {
            let _ = &probe;
        }))
        .unwrap();
    source.dispose().unwrap();
    assert_eq!(count.get(), 2);
}

#[test]
fn value_equality_suppresses_events_and_callbacks_can_read_and_replace_values() {
    let source = ValueNotifier::new(1);
    let weak = source.downgrade();
    let values = Rc::new(RefCell::new(Vec::new()));
    let _subscription = source
        .subscribe(Rc::new({
            let values = values.clone();
            move || {
                let source = weak.upgrade().unwrap();
                let value = source.get();
                values.borrow_mut().push(value);
                if value == 2 {
                    source.set(3).unwrap();
                }
            }
        }))
        .unwrap();
    assert!(!source.set(1).unwrap());
    assert!(source.set(2).unwrap());
    assert_eq!(*values.borrow(), [2, 3]);
    assert_eq!(ValueListenable::value(&source), 3);
    source.dispose().unwrap();
    assert_eq!(source.get(), 3);
    assert_eq!(source.set(4), Err(Disposed));
}

#[test]
fn interior_mutation_requires_explicit_notification() {
    let value = Rc::new(RefCell::new(vec![1]));
    let source = ValueNotifier::new(value.clone());
    let hits = Rc::new(Cell::new(0));
    let _subscription = source
        .subscribe(Rc::new({
            let hits = hits.clone();
            move || hits.set(hits.get() + 1)
        }))
        .unwrap();
    value.borrow_mut().push(2);
    assert!(!source.set(value).unwrap());
    assert_eq!(hits.get(), 0);
    source.notify_listeners().unwrap();
    assert_eq!(hits.get(), 1);
}

#[test]
fn merged_sources_forward_events_duplicates_and_remove_all_registrations() {
    let a = ChangeNotifier::new();
    let b = ChangeNotifier::new();
    let merged = MergedListenable::new(vec![
        Rc::new(a.clone()),
        Rc::new(b.clone()),
        Rc::new(a.clone()),
    ]);
    let hits = Rc::new(Cell::new(0));
    let subscription = merged
        .subscribe(Rc::new({
            let hits = hits.clone();
            move || hits.set(hits.get() + 1)
        }))
        .unwrap();
    a.notify_listeners().unwrap();
    b.notify_listeners().unwrap();
    assert_eq!(hits.get(), 3);
    drop(subscription);
    assert!(!a.has_listeners());
    assert!(!b.has_listeners());
    assert!(merged.same_source(&merged.clone()));
}

#[test]
fn a_failed_merged_registration_rolls_back_already_attached_sources() {
    let a = ChangeNotifier::new();
    let b = ChangeNotifier::new();
    b.dispose().unwrap();
    let merged = MergedListenable::new(vec![Rc::new(a.clone()), Rc::new(b)]);
    assert!(matches!(merged.subscribe(Rc::new(|| {})), Err(Disposed)));
    assert!(!a.has_listeners());
}

//! Owned forwarding links shared by derived animation sources. No application
//! cancellation captures are dropped while the registry or a slot is borrowed.
use crate::notifier::Subscription;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
struct Registration(RefCell<Option<Subscription>>);
impl Registration {
    fn cancel(&self) {
        let subscription = self.0.borrow_mut().take();
        drop(subscription);
    }
}
#[derive(Default)]
pub(super) struct Forwarded {
    registrations: RefCell<Vec<Weak<Registration>>>,
}
impl Forwarded {
    pub fn add(&self, subscription: Subscription) -> Subscription {
        let registration = Rc::new(Registration(RefCell::new(Some(subscription))));
        {
            let mut registrations = self.registrations.borrow_mut();
            registrations.retain(|r| r.strong_count() > 0);
            registrations.push(Rc::downgrade(&registration));
        }
        Subscription::new(move || registration.cancel())
    }
    pub fn cancel_all(&self) {
        let registrations = std::mem::take(&mut *self.registrations.borrow_mut());
        for registration in registrations.into_iter().filter_map(|r| r.upgrade()) {
            registration.cancel();
        }
    }
}
impl Drop for Forwarded {
    fn drop(&mut self) {
        let registrations = std::mem::take(self.registrations.get_mut());
        for registration in registrations.into_iter().filter_map(|r| r.upgrade()) {
            registration.cancel();
        }
    }
}

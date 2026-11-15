//! Monotonic frame requests shared by UI and worker threads in native hosts.
use std::time::{Duration, Instant};
#[derive(Default)]
pub(crate) struct FrameSchedule {
    next: Option<Instant>,
}
impl FrameSchedule {
    pub fn request(&mut self, now: Instant, delay: Duration) -> bool {
        let Some(deadline) = now.checked_add(delay) else {
            return false;
        };
        if self.next.is_none_or(|old| deadline < old) {
            self.next = Some(deadline);
            true
        } else {
            false
        }
    }
    pub fn deadline(&self) -> Option<Instant> {
        self.next
    }
    pub fn clear(&mut self) {
        self.next = None;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn delayed_requests_do_not_force_immediate_frames_or_postpone_earlier_requests() {
        let now = Instant::now();
        let mut schedule = FrameSchedule::default();
        assert!(schedule.request(now, Duration::from_secs(1)));
        assert_eq!(schedule.deadline(), Some(now + Duration::from_secs(1)));
        assert!(!schedule.request(now, Duration::from_secs(5)));
        assert!(schedule.request(now, Duration::from_millis(20)));
        assert_eq!(schedule.deadline(), Some(now + Duration::from_millis(20)));
        assert!(schedule.request(now, Duration::ZERO));
        assert_eq!(schedule.deadline(), Some(now));
    }
    #[test]
    fn frame_start_clear_allows_new_worker_requests_and_infinite_delays_do_not_overflow() {
        let now = Instant::now();
        let mut schedule = FrameSchedule::default();
        schedule.request(now, Duration::ZERO);
        schedule.clear();
        assert!(!schedule.request(now, Duration::MAX));
        assert_eq!(schedule.deadline(), None);
        assert!(schedule.request(now, Duration::from_millis(10)));
        assert_eq!(schedule.deadline(), Some(now + Duration::from_millis(10)));
    }
}

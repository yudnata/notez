use std::time::{Duration, Instant};

pub const IDLE_DELAY: Duration = Duration::from_millis(500);
const MAX_WAIT: Duration = Duration::from_secs(5);
const RETRY_DELAY: Duration = Duration::from_secs(2);

#[derive(Default)]
pub struct Autosave {
    pending_since: Option<Instant>,
    last_change: Option<Instant>,
    retry_at: Option<Instant>,
    urgent: bool,
}

impl Autosave {
    pub fn edited(&mut self, now: Instant) {
        self.pending_since.get_or_insert(now);
        self.last_change = Some(now);
    }

    pub fn immediately(&mut self, now: Instant) {
        self.edited(now);
        self.urgent = true;
    }

    pub fn deadline(&self) -> Option<Instant> {
        let first = self.pending_since?;
        let idle = self.last_change? + IDLE_DELAY;
        let due = if self.urgent {
            first
        } else {
            idle.min(first + MAX_WAIT)
        };
        Some(self.retry_at.map_or(due, |retry| due.max(retry)))
    }

    pub fn due(&self, now: Instant) -> bool {
        self.deadline().is_some_and(|deadline| now >= deadline)
    }

    pub fn failed(&mut self, now: Instant) {
        self.retry_at = Some(now + RETRY_DELAY);
    }

    pub fn retrying(&self) -> bool {
        self.retry_at.is_some()
    }

    pub fn saved(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_and_deleting_restart_the_idle_timer() {
        let start = Instant::now();
        let mut save = Autosave::default();
        save.edited(start);
        assert!(!save.due(start + Duration::from_millis(499)));
        save.edited(start + Duration::from_millis(450));
        assert!(!save.due(start + Duration::from_millis(949)));
        assert!(save.due(start + Duration::from_millis(950)));
        save.saved();
        assert_eq!(save.deadline(), None);
    }

    #[test]
    fn continuous_edits_are_saved_at_least_every_five_seconds() {
        let start = Instant::now();
        let mut save = Autosave::default();
        for milliseconds in (0..=4800).step_by(400) {
            save.edited(start + Duration::from_millis(milliseconds));
        }
        assert!(!save.due(start + Duration::from_millis(4999)));
        assert!(save.due(start + MAX_WAIT));
    }

    #[test]
    fn discrete_changes_save_immediately_and_failures_back_off() {
        let start = Instant::now();
        let mut save = Autosave::default();
        save.immediately(start);
        assert!(save.due(start));
        save.failed(start);
        save.edited(start + Duration::from_millis(100));
        assert!(save.retrying());
        assert!(!save.due(start + Duration::from_millis(1999)));
        assert!(save.due(start + RETRY_DELAY));
        save.saved();
        assert!(!save.retrying());
        assert!(!save.due(start + RETRY_DELAY));
    }
}

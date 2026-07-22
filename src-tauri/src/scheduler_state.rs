//! Mutable state owned by the notification scheduler, kept on `AppState`.
//!
//! Splitting this into its own module avoids a circular dependency between
//! `state` (which owns the scheduler state) and `scheduler` (which needs
//! `AppState`).

use std::collections::HashSet;
use std::time::Instant;

use chrono::NaiveDate;

#[derive(Default)]
pub struct SchedulerState {
    /// Local calendar day the counters belong to. Resets at local midnight.
    pub day: Option<NaiveDate>,
    /// Notifications fired during `day`.
    pub count_today: u32,
    /// Instant of the most recent fired notification.
    pub last_fired: Option<Instant>,
    /// Tool names notified during `day` (for dedupe).
    pub notified_tools: HashSet<String>,
}

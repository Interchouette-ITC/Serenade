//! Schedule triggers.

use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cron::Schedule as CronSchedule;

use crate::SchedulerError;

/// When a [`crate::Schedule`] should fire.
#[derive(Clone, Debug)]
pub enum Trigger {
    /// Fire every `every` after the previous due time (anchored from first arming).
    Interval {
        /// Delay between fires.
        every: Duration,
    },
    /// Unix cron (six fields including seconds), via the `cron` crate.
    Cron {
        /// Parsed cron schedule.
        schedule: Box<CronSchedule>,
    },
}

impl Trigger {
    /// Interval trigger.
    ///
    /// # Errors
    ///
    /// Returns [`SchedulerError::InvalidTrigger`] when `every` is zero.
    pub fn interval(every: Duration) -> Result<Self, SchedulerError> {
        if every.is_zero() {
            return Err(SchedulerError::InvalidTrigger {
                message: "interval must be greater than zero".to_owned(),
            });
        }
        Ok(Self::Interval { every })
    }

    /// Cron trigger from a six-field expression (`sec min hour day month dow`).
    ///
    /// # Errors
    ///
    /// Returns [`SchedulerError::InvalidTrigger`] when the expression is invalid.
    pub fn cron(expr: &str) -> Result<Self, SchedulerError> {
        let schedule =
            CronSchedule::from_str(expr).map_err(|error| SchedulerError::InvalidTrigger {
                message: error.to_string(),
            })?;
        Ok(Self::Cron {
            schedule: Box::new(schedule),
        })
    }

    pub(crate) fn first_due(&self, now: SystemTime) -> SystemTime {
        match self {
            Self::Interval { every } => now.checked_add(*every).unwrap_or_else(far_future),
            Self::Cron { schedule } => next_cron(schedule, now).unwrap_or_else(far_future),
        }
    }

    pub(crate) fn next_after(&self, after: SystemTime, now: SystemTime) -> SystemTime {
        match self {
            Self::Interval { every } => {
                let _ = after;
                // Re-arm from `now` so a long pause does not storm catch-up fires.
                now.checked_add(*every).unwrap_or_else(far_future)
            }
            Self::Cron { schedule } => {
                let _ = after;
                next_cron(schedule, now).unwrap_or_else(far_future)
            }
        }
    }
}

fn next_cron(schedule: &CronSchedule, now: SystemTime) -> Option<SystemTime> {
    let datetime = system_time_to_datetime(now)?;
    schedule
        .after(&datetime)
        .next()
        .map(datetime_to_system_time)
}

fn system_time_to_datetime(now: SystemTime) -> Option<chrono::DateTime<chrono::Utc>> {
    let duration = now.duration_since(UNIX_EPOCH).ok()?;
    chrono::DateTime::from_timestamp(
        i64::try_from(duration.as_secs()).ok()?,
        duration.subsec_nanos(),
    )
}

fn datetime_to_system_time(datetime: chrono::DateTime<chrono::Utc>) -> SystemTime {
    let secs = u64::try_from(datetime.timestamp()).unwrap_or(0);
    let nanos = datetime.timestamp_subsec_nanos();
    UNIX_EPOCH + Duration::new(secs, nanos)
}

fn far_future() -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(u64::from(u32::MAX) * 86_400)
}

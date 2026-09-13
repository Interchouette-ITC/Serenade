//! Symfony-shaped scheduling.
//!
//! - [`Trigger`]: interval or cron expression
//! - [`Schedule`]: named recurring definition
//! - [`Clock`] / [`ManualClock`] / [`SystemClock`]: injectable time
//! - [`Scheduler`]: `tick` returns due work without sleeping (no wall-clock flake)

mod clock;
mod error;
mod schedule;
mod scheduler;
mod trigger;

pub use clock::{Clock, ManualClock, SystemClock};
pub use error::SchedulerError;
pub use schedule::{DueJob, Schedule};
pub use scheduler::{Scheduler, validate_id};
pub use trigger::Trigger;

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;

//! Symfony-shaped scheduling.
//!
//! - [`Trigger`]: interval or cron expression
//! - [`Schedule`]: named recurring definition
//! - [`Clock`] / [`ManualClock`] / [`SystemClock`]: injectable time
//! - [`Scheduler`]: `tick` returns due work without sleeping (no wall-clock flake)
//! - [`HandlerMap`] / [`ScheduleHandler`]: sync dispatch for due jobs
//! - [`RunSchedulerCommand`] (`serenade:scheduler:run`)

mod clock;
mod compile_pass;
mod console;
mod error;
mod handler;
mod schedule;
mod scheduler;
mod trigger;

#[cfg(feature = "messenger")]
mod messenger;

pub use clock::{Clock, ManualClock, SystemClock};
pub use compile_pass::{
    DEFAULT_SCHEDULER_SERVICE, RegisterDefaultSchedulerPass, SCHEDULER_TAG, SchedulerService,
};
pub use console::RunSchedulerCommand;
pub use error::SchedulerError;
pub use handler::{FnScheduleHandler, HandlerMap, ScheduleHandler};
pub use schedule::{DueJob, Schedule};
pub use scheduler::{Scheduler, validate_id};
pub use trigger::Trigger;

#[cfg(feature = "messenger")]
pub use messenger::CommandOnDue;

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;

#[cfg(all(test, feature = "messenger"))]
#[path = "messenger_tests.rs"]
mod messenger_tests;

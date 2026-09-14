//! `serenade:scheduler:run` console command.

use std::thread;
use std::time::Duration;

use serenade_console::{Command, ConsoleError, Input};

use crate::{Clock, DEFAULT_SCHEDULER_SERVICE, SchedulerService, SystemClock};

/// Runs the default scheduler loop (or a single tick with `--once`).
#[derive(Clone, Copy, Debug, Default)]
pub struct RunSchedulerCommand;

impl Command for RunSchedulerCommand {
    fn name(&self) -> &'static str {
        "serenade:scheduler:run"
    }

    fn description(&self) -> &'static str {
        "Run the scheduler tick loop (use --once for a single tick)"
    }

    fn execute(&self, input: &Input) -> Result<(), ConsoleError> {
        let Some(container) = input.container() else {
            return Err(ConsoleError::Failed(
                "serenade:scheduler:run needs a container (Application::run_with)".to_owned(),
            ));
        };
        let service = container
            .get_as::<SchedulerService>(DEFAULT_SCHEDULER_SERVICE)
            .map_err(|error| ConsoleError::Failed(error.to_string()))?;
        let once = input.args().iter().any(|arg| arg == "--once");
        run_scheduler_loop(&service, &SystemClock, once)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum LoopStep {
    /// Sleep then tick again.
    Continue {
        /// How long to sleep before the next tick.
        sleep: Duration,
    },
    /// Stop the loop.
    Done,
}

/// One tick of the scheduler run loop (no sleep). Used by [`run_scheduler_loop`].
///
/// # Errors
///
/// Returns [`ConsoleError::Failed`] when dispatch fails.
pub fn scheduler_loop_step(
    service: &SchedulerService,
    clock: &dyn Clock,
    once: bool,
) -> Result<LoopStep, ConsoleError> {
    let due = service
        .tick_and_dispatch(clock)
        .map_err(|error| ConsoleError::Failed(error.to_string()))?;
    for job in &due {
        println!(
            "scheduler: dispatched `{}` (scheduled_for={:?})",
            job.id(),
            job.scheduled_for()
        );
    }
    if once {
        return Ok(LoopStep::Done);
    }
    if service.is_empty() {
        println!("scheduler: no schedules registered; exiting");
        return Ok(LoopStep::Done);
    }
    let wake = service
        .next_wake()
        .expect("non-empty scheduler always has a next wake");
    let now = clock.now();
    let sleep = wake
        .duration_since(now)
        .unwrap_or(Duration::ZERO)
        .min(Duration::from_secs(60));
    Ok(LoopStep::Continue { sleep })
}

/// Tick / dispatch loop used by [`RunSchedulerCommand`] (testable with [`crate::ManualClock`]).
///
/// # Errors
///
/// Returns [`ConsoleError::Failed`] when dispatch fails.
pub fn run_scheduler_loop(
    service: &SchedulerService,
    clock: &dyn Clock,
    once: bool,
) -> Result<(), ConsoleError> {
    run_scheduler_loop_limited(service, clock, once, None)
}

/// Like [`run_scheduler_loop`], but stops after `max_steps` iterations when set (tests).
///
/// # Errors
///
/// Returns [`ConsoleError::Failed`] when dispatch fails.
pub fn run_scheduler_loop_limited(
    service: &SchedulerService,
    clock: &dyn Clock,
    once: bool,
    max_steps: Option<usize>,
) -> Result<(), ConsoleError> {
    let limit = max_steps.unwrap_or(usize::MAX);
    for _ in 0..limit {
        match scheduler_loop_step(service, clock, once)? {
            LoopStep::Done => return Ok(()),
            LoopStep::Continue { sleep } => thread::sleep(sleep),
        }
    }
    Ok(())
}

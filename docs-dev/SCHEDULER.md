# Scheduler

Scheduling lives in **`serenade-scheduler`** ([#187](https://github.com/Interchouette-ITC/Serenade/issues/187), [#195](https://github.com/Interchouette-ITC/Serenade/issues/195), [#194](https://github.com/Interchouette-ITC/Serenade/issues/194)).

Symfony **Scheduler** analogue: cron / interval triggers that hand work to sync handlers (or Messenger when the `messenger` feature is enabled). Apps own job bodies; Serenade owns the runner.

## Types

| Type | Role |
| --- | --- |
| `Trigger` | Interval or six-field cron expression |
| `Schedule` | Named recurring definition |
| `Clock` / `ManualClock` / `SystemClock` | Injectable time (tests use `ManualClock`) |
| `Scheduler` | `tick` returns due jobs without sleeping |
| `DueJob` | One fired schedule id + armed instant |
| `ScheduleHandler` / `HandlerMap` / `FnScheduleHandler` | Sync dispatch for due jobs |
| `SchedulerService` | Shared scheduler + handlers for DI / console |
| `RunSchedulerCommand` | `serenade:scheduler:run` |

## Dispatch

Register a handler per schedule id, then call `SchedulerService::tick_and_dispatch` (or run the console command). Missing handlers return `SchedulerError::MissingHandler`.

With Cargo feature `messenger`, use `CommandOnDue` to build a typed Messenger command from each `DueJob` and `MessageBus::dispatch_command`.

## DI

`FrameworkExtension` adds [`RegisterDefaultSchedulerPass`], which registers service id `scheduler` (`DEFAULT_SCHEDULER_SERVICE`) tagged `scheduler` with an empty `SchedulerService` when missing. Resolve `SchedulerService`, `add` schedules, and `insert_handler` before running the loop.

## Console

| Command | Role |
| --- | --- |
| `serenade:scheduler:run` | Tick loop: dispatch due jobs, sleep until `next_wake` (capped at 60s). `--once` runs a single tick and exits. Exits when no schedules are registered. |

## Example

```rust
use std::sync::Arc;
use std::time::Duration;
use serenade_scheduler::{
    FnScheduleHandler, ManualClock, Schedule, SchedulerService, SystemClock, Trigger,
};

let service = SchedulerService::new();
let clock = ManualClock::new(std::time::UNIX_EPOCH);
service.add(
    Schedule::new("heartbeat", Trigger::interval(Duration::from_secs(60))?)?,
    &clock,
)?;
service.insert_handler(
    "heartbeat",
    Arc::new(FnScheduleHandler::new(|job| {
        println!("due {}", job.id());
        Ok(())
    })),
)?;
let _ = service.tick_and_dispatch(&SystemClock)?;
```

## Related

- Parent epic: [#187](https://github.com/Interchouette-ITC/Serenade/issues/187)
- Lock (single-runner safety): [LOCK.md](LOCK.md)
- Messenger overview: [KERNEL.md](KERNEL.md)

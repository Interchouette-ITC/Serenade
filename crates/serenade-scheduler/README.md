# serenade-scheduler

Symfony-shaped schedules: `Trigger` (interval / cron), `Schedule`,
injectable `Clock` (`ManualClock` / `SystemClock`), `Scheduler` ticks,
sync `ScheduleHandler` / `HandlerMap`, DI via `RegisterDefaultSchedulerPass`
(service id `scheduler`), and `serenade:scheduler:run`.

Optional Cargo feature `messenger` adds `CommandOnDue` for typed
`MessageBus` dispatch.

See `docs-dev/SCHEDULER.md`.

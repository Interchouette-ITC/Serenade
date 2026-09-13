# serenade-scheduler

Symfony-shaped schedules: `Trigger` (interval / cron), `Schedule`,
injectable `Clock` (`ManualClock` / `SystemClock`), and a `Scheduler`
that returns due jobs from `tick` without sleeping.

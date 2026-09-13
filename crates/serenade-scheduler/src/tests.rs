//! Unit tests for `serenade-scheduler`.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::{
    Clock, ManualClock, Schedule, Scheduler, SchedulerError, SystemClock, Trigger, version,
};

fn epoch_plus(secs: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(secs)
}

#[test]
fn version_is_nonempty() {
    assert_ne!(version(), "");
}

#[test]
fn system_clock_returns_wall_time() {
    let before = SystemTime::now();
    let now = SystemClock.now();
    let after = SystemTime::now();
    assert!(now >= before && now <= after);
}

#[test]
fn interval_rejects_zero() {
    assert!(matches!(
        Trigger::interval(Duration::ZERO),
        Err(SchedulerError::InvalidTrigger { .. })
    ));
}

#[test]
fn cron_rejects_bad_expression() {
    assert!(matches!(
        Trigger::cron("not a cron"),
        Err(SchedulerError::InvalidTrigger { .. })
    ));
}

#[test]
fn schedule_rejects_empty_id() {
    let trigger = Trigger::interval(Duration::from_secs(1)).expect("trigger");
    assert!(matches!(
        Schedule::new("", trigger),
        Err(SchedulerError::InvalidId { .. })
    ));
}

#[test]
fn interval_tick_is_deterministic_with_manual_clock() {
    let clock = ManualClock::new(epoch_plus(1_000));
    let mut scheduler = Scheduler::new();
    let schedule = Schedule::new(
        "heartbeat",
        Trigger::interval(Duration::from_secs(10)).expect("trigger"),
    )
    .expect("schedule");
    assert_eq!(schedule.id(), "heartbeat");
    assert!(matches!(schedule.trigger(), Trigger::Interval { .. }));
    scheduler.add(schedule, &clock).expect("add");
    assert_eq!(scheduler.len(), 1);
    assert!(!scheduler.is_empty());
    assert_eq!(scheduler.next_wake(), Some(epoch_plus(1_010)));

    assert_eq!(scheduler.tick(&clock), []);

    clock.advance(Duration::from_secs(10));
    let due = scheduler.tick(&clock);
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].id(), "heartbeat");
    assert_eq!(due[0].scheduled_for(), epoch_plus(1_010));
    assert_eq!(scheduler.next_wake(), Some(epoch_plus(1_020)));

    clock.advance(Duration::from_secs(5));
    assert_eq!(scheduler.tick(&clock), []);

    clock.advance(Duration::from_secs(5));
    let due = scheduler.tick(&clock);
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].scheduled_for(), epoch_plus(1_020));
}

#[test]
fn duplicate_id_rejected_and_remove_works() {
    let clock = ManualClock::new(epoch_plus(0));
    let mut scheduler = Scheduler::new();
    let trigger = Trigger::interval(Duration::from_secs(1)).expect("trigger");
    scheduler
        .add(Schedule::new("a", trigger.clone()).expect("s1"), &clock)
        .expect("add1");
    assert!(matches!(
        scheduler.add(Schedule::new("a", trigger).expect("s2"), &clock),
        Err(SchedulerError::InvalidId { .. })
    ));
    assert!(scheduler.remove("a"));
    assert!(!scheduler.remove("a"));
    assert!(scheduler.is_empty());
}

#[test]
fn cron_fires_on_manual_clock_jump() {
    // Every minute at second 0: `0 * * * * *`
    let trigger = Trigger::cron("0 * * * * *").expect("cron");
    let clock = ManualClock::new(epoch_plus(30));
    let mut scheduler = Scheduler::new();
    scheduler
        .add(
            Schedule::new("cron-job", trigger).expect("schedule"),
            &clock,
        )
        .expect("add");

    // First due is the next :00 after t=30 → t=60.
    assert_eq!(scheduler.next_wake(), Some(epoch_plus(60)));
    assert_eq!(scheduler.tick(&clock), []);

    clock.set(epoch_plus(60));
    let due = scheduler.tick(&clock);
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].id(), "cron-job");
    assert_eq!(due[0].scheduled_for(), epoch_plus(60));
    assert_eq!(scheduler.next_wake(), Some(epoch_plus(120)));
}

#[test]
fn long_pause_does_not_storm_interval_catch_up() {
    let clock = ManualClock::new(epoch_plus(0));
    let mut scheduler = Scheduler::new();
    scheduler
        .add(
            Schedule::new(
                "slow",
                Trigger::interval(Duration::from_secs(1)).expect("trigger"),
            )
            .expect("schedule"),
            &clock,
        )
        .expect("add");

    clock.advance(Duration::from_secs(100));
    let due = scheduler.tick(&clock);
    assert_eq!(due.len(), 1, "one fire per tick even after a long pause");
    assert_eq!(scheduler.next_wake(), Some(epoch_plus(101)));
}

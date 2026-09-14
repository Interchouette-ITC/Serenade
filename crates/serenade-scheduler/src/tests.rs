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

fn expected_far_future() -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(u64::from(u32::MAX) * 86_400)
}

#[test]
fn interval_overflow_arms_far_future() {
    let trigger = Trigger::interval(Duration::from_secs(10)).expect("trigger");
    let near_end = UNIX_EPOCH
        .checked_add(Duration::from_secs(i64::MAX as u64))
        .expect("platform supports near-max SystemTime");
    assert_eq!(trigger.first_due(near_end), expected_far_future());
    assert_eq!(
        trigger.next_after(near_end, near_end),
        expected_far_future()
    );
}

#[test]
fn cron_before_unix_epoch_arms_far_future() {
    let trigger = Trigger::cron("0 * * * * *").expect("cron");
    let before = UNIX_EPOCH - Duration::from_secs(1);
    assert_eq!(trigger.first_due(before), expected_far_future());
    assert_eq!(trigger.next_after(before, before), expected_far_future());
}

#[test]
fn handler_map_dispatches_and_reports_missing() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::{FnScheduleHandler, HandlerMap};

    let hits = Arc::new(AtomicUsize::new(0));
    let hits_clone = Arc::clone(&hits);
    let mut handlers = HandlerMap::new();
    assert!(handlers.is_empty());
    handlers
        .insert(
            "heartbeat",
            Arc::new(FnScheduleHandler::new(move |_job| {
                hits_clone.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })),
        )
        .expect("insert");
    assert_eq!(handlers.len(), 1);
    assert!(matches!(
        handlers.insert("heartbeat", Arc::new(FnScheduleHandler::new(|_| Ok(())))),
        Err(SchedulerError::InvalidId { .. })
    ));

    let clock = ManualClock::new(epoch_plus(0));
    let mut scheduler = Scheduler::new();
    scheduler
        .add(
            Schedule::new(
                "heartbeat",
                Trigger::interval(Duration::from_secs(1)).expect("trigger"),
            )
            .expect("schedule"),
            &clock,
        )
        .expect("add");
    clock.advance(Duration::from_secs(1));
    let due = scheduler.tick(&clock);
    assert_eq!(due.len(), 1);
    handlers.dispatch(&due[0]).expect("dispatch");
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    assert!(matches!(
        handlers.dispatch(&crate::DueJob::new("missing".to_owned(), epoch_plus(0))),
        Err(SchedulerError::MissingHandler { .. })
    ));
    assert!(handlers.remove("heartbeat"));
    assert!(!handlers.remove("heartbeat"));
}

#[test]
fn scheduler_service_tick_and_dispatch() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::{FnScheduleHandler, SchedulerService};

    let hits = Arc::new(AtomicUsize::new(0));
    let hits_clone = Arc::clone(&hits);
    let service = SchedulerService::new();
    let clock = ManualClock::new(epoch_plus(0));
    service
        .add(
            Schedule::new(
                "job",
                Trigger::interval(Duration::from_secs(5)).expect("trigger"),
            )
            .expect("schedule"),
            &clock,
        )
        .expect("add");
    service
        .insert_handler(
            "job",
            Arc::new(FnScheduleHandler::new(move |_job| {
                hits_clone.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })),
        )
        .expect("handler");
    assert!(!service.is_empty());
    assert_eq!(service.next_wake(), Some(epoch_plus(5)));
    assert_eq!(service.tick_and_dispatch(&clock).expect("idle").len(), 0);
    clock.advance(Duration::from_secs(5));
    let due = service.tick_and_dispatch(&clock).expect("due");
    assert_eq!(due.len(), 1);
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

#[test]
fn compile_pass_registers_scheduler_and_skips_when_present() {
    use serenade_di::{CompilePass, ContainerBuilder, ServiceDefinition};

    use crate::{
        DEFAULT_SCHEDULER_SERVICE, RegisterDefaultSchedulerPass, SCHEDULER_TAG, SchedulerService,
    };

    let pass = RegisterDefaultSchedulerPass;
    assert_eq!(pass.name(), "register_default_scheduler");
    let mut builder = ContainerBuilder::new();
    builder.add_compile_pass(RegisterDefaultSchedulerPass);
    let container = builder.compile().expect("compile");
    let service = container
        .get_as::<SchedulerService>(DEFAULT_SCHEDULER_SERVICE)
        .expect("scheduler");
    assert!(service.is_empty());

    let mut builder = ContainerBuilder::new();
    builder
        .register(
            ServiceDefinition::new(DEFAULT_SCHEDULER_SERVICE).with_tag(SCHEDULER_TAG),
            |_container| Ok(Box::new(SchedulerService::new())),
        )
        .expect("pre-register");
    builder.add_compile_pass(RegisterDefaultSchedulerPass);
    builder.compile().expect("compile with existing");
}

#[test]
fn run_command_once_with_empty_scheduler() {
    use std::sync::Arc;

    use serenade_console::{Command, Input};
    use serenade_di::ContainerBuilder;
    use serenade_kernel::Environment;

    use crate::{DEFAULT_SCHEDULER_SERVICE, RegisterDefaultSchedulerPass, RunSchedulerCommand};

    let mut builder = ContainerBuilder::new();
    builder.add_compile_pass(RegisterDefaultSchedulerPass);
    let container = Arc::new(builder.compile().expect("compile"));
    let _ = container
        .get_as::<crate::SchedulerService>(DEFAULT_SCHEDULER_SERVICE)
        .expect("scheduler");
    let input = Input::new(
        Environment::Test,
        true,
        vec!["--once".to_owned()],
        Some(container),
    );
    RunSchedulerCommand.execute(&input).expect("run --once");
}

#[test]
fn run_command_requires_container() {
    use serenade_console::{Command, ConsoleError, Input};
    use serenade_kernel::Environment;

    use crate::RunSchedulerCommand;

    let input = Input::new(Environment::Test, true, Vec::new(), None);
    assert!(matches!(
        RunSchedulerCommand.execute(&input),
        Err(ConsoleError::Failed(_))
    ));
}

#[test]
fn run_loop_exits_when_empty_without_once() {
    use crate::{SchedulerService, console::run_scheduler_loop};

    let service = SchedulerService::new();
    let clock = ManualClock::new(epoch_plus(0));
    run_scheduler_loop(&service, &clock, false).expect("empty exit");
}

#[test]
fn run_loop_dispatches_due_job_with_once() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::{FnScheduleHandler, SchedulerService, console::run_scheduler_loop};

    let hits = Arc::new(AtomicUsize::new(0));
    let hits_clone = Arc::clone(&hits);
    let service = SchedulerService::new();
    let clock = ManualClock::new(epoch_plus(0));
    service
        .add(
            Schedule::new(
                "pulse",
                Trigger::interval(Duration::from_secs(1)).expect("trigger"),
            )
            .expect("schedule"),
            &clock,
        )
        .expect("add");
    service
        .insert_handler(
            "pulse",
            Arc::new(FnScheduleHandler::new(move |_job| {
                hits_clone.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })),
        )
        .expect("handler");
    clock.advance(Duration::from_secs(1));
    run_scheduler_loop(&service, &clock, true).expect("once");
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

#[test]
fn run_loop_maps_handler_failure() {
    use std::sync::Arc;

    use serenade_console::ConsoleError;

    use crate::{FnScheduleHandler, SchedulerService, console::run_scheduler_loop};

    let service = SchedulerService::new();
    let clock = ManualClock::new(epoch_plus(0));
    service
        .add(
            Schedule::new(
                "boom",
                Trigger::interval(Duration::from_secs(1)).expect("trigger"),
            )
            .expect("schedule"),
            &clock,
        )
        .expect("add");
    service
        .insert_handler(
            "boom",
            Arc::new(FnScheduleHandler::new(|job| {
                Err(SchedulerError::Handler {
                    id: job.id().to_owned(),
                    message: "explode".to_owned(),
                })
            })),
        )
        .expect("handler");
    clock.advance(Duration::from_secs(1));
    assert!(matches!(
        run_scheduler_loop(&service, &clock, true),
        Err(ConsoleError::Failed(_))
    ));
}

#[test]
fn scheduler_loop_step_reports_continue_sleep() {
    use std::sync::Arc;

    use crate::{
        FnScheduleHandler, SchedulerService,
        console::{LoopStep, scheduler_loop_step},
    };

    let service = SchedulerService::new();
    let clock = ManualClock::new(epoch_plus(0));
    service
        .add(
            Schedule::new(
                "later",
                Trigger::interval(Duration::from_secs(30)).expect("trigger"),
            )
            .expect("schedule"),
            &clock,
        )
        .expect("add");
    service
        .insert_handler("later", Arc::new(FnScheduleHandler::new(|_| Ok(()))))
        .expect("handler");
    match scheduler_loop_step(&service, &clock, false).expect("step") {
        LoopStep::Continue { sleep } => assert_eq!(sleep, Duration::from_secs(30)),
        LoopStep::Done => panic!("expected continue"),
    }
}

#[test]
fn scheduler_loop_step_caps_sleep_at_sixty_seconds() {
    use std::sync::Arc;

    use crate::{
        FnScheduleHandler, SchedulerService,
        console::{LoopStep, scheduler_loop_step},
    };

    let service = SchedulerService::new();
    let clock = ManualClock::new(epoch_plus(0));
    service
        .add(
            Schedule::new(
                "far",
                Trigger::interval(Duration::from_secs(120)).expect("trigger"),
            )
            .expect("schedule"),
            &clock,
        )
        .expect("add");
    service
        .insert_handler("far", Arc::new(FnScheduleHandler::new(|_| Ok(()))))
        .expect("handler");
    match scheduler_loop_step(&service, &clock, false).expect("step") {
        LoopStep::Continue { sleep } => assert_eq!(sleep, Duration::from_secs(60)),
        LoopStep::Done => panic!("expected continue"),
    }
}

#[test]
fn run_loop_sleeps_briefly_then_stops() {
    use std::sync::Arc;

    use crate::{FnScheduleHandler, SchedulerService, console::run_scheduler_loop_limited};

    let service = SchedulerService::new();
    let clock = ManualClock::new(epoch_plus(0));
    service
        .add(
            Schedule::new(
                "brief",
                Trigger::interval(Duration::from_millis(5)).expect("trigger"),
            )
            .expect("schedule"),
            &clock,
        )
        .expect("add");
    service
        .insert_handler("brief", Arc::new(FnScheduleHandler::new(|_| Ok(()))))
        .expect("handler");
    run_scheduler_loop_limited(&service, &clock, false, Some(1)).expect("one sleep step");
}

#[test]
fn run_loop_zero_sleep_continues_with_limit() {
    use std::sync::Arc;

    use crate::{FnScheduleHandler, SchedulerService, console::run_scheduler_loop_limited};

    let service = SchedulerService::new();
    let clock = ManualClock::new(epoch_plus(0));
    service
        .add(
            Schedule::new(
                "ready",
                Trigger::interval(Duration::from_secs(1)).expect("trigger"),
            )
            .expect("schedule"),
            &clock,
        )
        .expect("add");
    service
        .insert_handler("ready", Arc::new(FnScheduleHandler::new(|_| Ok(()))))
        .expect("handler");
    // Jump past the first due so tick fires and re-arms to now+1s, then jump again
    // so wake <= now and sleep is zero.
    clock.advance(Duration::from_secs(1));
    let _ = service.tick_and_dispatch(&clock).expect("fire");
    clock.advance(Duration::from_secs(1));
    run_scheduler_loop_limited(&service, &clock, false, Some(1)).expect("zero sleep");
}

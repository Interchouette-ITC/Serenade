//! Messenger feature tests (enabled with `--features messenger`).

#![cfg(feature = "messenger")]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serenade_messenger::{Command, CommandHandler, Message, MessageBus, MessengerError};

use crate::{CommandOnDue, ManualClock, Schedule, ScheduleHandler, SchedulerService, Trigger};

fn epoch_plus(secs: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(secs)
}

struct Ping {
    id: String,
}

impl Message for Ping {
    const NAME: &'static str = "scheduler.ping";
}

impl Command for Ping {}

struct PingHandler {
    hits: Arc<AtomicUsize>,
}

impl CommandHandler<Ping> for PingHandler {
    fn handle(&self, command: &Ping) -> Result<(), MessengerError> {
        assert_eq!(command.id, "ping");
        self.hits.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[test]
fn command_on_due_maps_bus_errors() {
    use serenade_messenger::{Command, Message, MessageBus};

    struct Unregistered;

    impl Message for Unregistered {
        const NAME: &'static str = "scheduler.unregistered";
    }

    impl Command for Unregistered {}

    let bus = MessageBus::new();
    let handler = CommandOnDue::new(bus, |_job| Unregistered);
    let err = handler
        .on_due(&crate::DueJob::new("x".to_owned(), epoch_plus(0)))
        .expect_err("no handler on bus");
    assert!(matches!(err, crate::SchedulerError::Handler { .. }));
}

#[test]
fn command_on_due_dispatches_through_message_bus() {
    let hits = Arc::new(AtomicUsize::new(0));
    let mut bus = MessageBus::new();
    bus.register_command(PingHandler {
        hits: Arc::clone(&hits),
    })
    .expect("register");

    let handler: Arc<dyn ScheduleHandler> = Arc::new(CommandOnDue::new(bus, |job| Ping {
        id: job.id().to_owned(),
    }));

    let service = SchedulerService::new();
    let clock = ManualClock::new(epoch_plus(0));
    service
        .add(
            Schedule::new(
                "ping",
                Trigger::interval(Duration::from_secs(1)).expect("trigger"),
            )
            .expect("schedule"),
            &clock,
        )
        .expect("add");
    service
        .insert_handler("ping", handler)
        .expect("insert handler");
    clock.advance(Duration::from_secs(1));
    let due = service.tick_and_dispatch(&clock).expect("dispatch");
    assert_eq!(due.len(), 1);
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

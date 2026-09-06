use std::sync::{Arc, Mutex};

use super::{
    Command, CommandHandler, DispatchContext, DispatchKind, Event, EventHandler, LogSink,
    LoggingMiddleware, Message, MessageBus, MessengerError, Middleware, ValidateHook,
    ValidationMiddleware,
};

struct PlaceOrder {
    sku: &'static str,
}

impl Message for PlaceOrder {
    const NAME: &'static str = "order.place";
}

impl Command for PlaceOrder {}

struct PlaceOrderHandler {
    log: Arc<Mutex<Vec<&'static str>>>,
}

impl CommandHandler<PlaceOrder> for PlaceOrderHandler {
    fn handle(&self, command: &PlaceOrder) -> Result<(), MessengerError> {
        self.log.lock().expect("lock").push(command.sku);
        Ok(())
    }
}

struct OrderPlaced {
    id: &'static str,
}

impl Message for OrderPlaced {
    const NAME: &'static str = "order.placed";
}

impl Event for OrderPlaced {}

struct RecordOrderPlaced {
    log: Arc<Mutex<Vec<&'static str>>>,
    label: &'static str,
}

impl EventHandler<OrderPlaced> for RecordOrderPlaced {
    fn handle(&self, event: &OrderPlaced) -> Result<(), MessengerError> {
        let _ = event.id;
        self.log.lock().expect("lock").push(self.label);
        Ok(())
    }
}

struct FailingCommandHandler;

impl CommandHandler<PlaceOrder> for FailingCommandHandler {
    fn handle(&self, _command: &PlaceOrder) -> Result<(), MessengerError> {
        Err(MessengerError::Handler {
            name: PlaceOrder::NAME,
            message: "rejected".to_owned(),
        })
    }
}

struct FailingEventHandler;

impl EventHandler<OrderPlaced> for FailingEventHandler {
    fn handle(&self, _event: &OrderPlaced) -> Result<(), MessengerError> {
        Err(MessengerError::Handler {
            name: OrderPlaced::NAME,
            message: "boom".to_owned(),
        })
    }
}

#[test]
fn empty_bus_is_empty() {
    let bus = MessageBus::new();
    assert!(bus.is_empty());
    assert_eq!(bus.command_count(), 0);
    assert_eq!(bus.event_handler_count(), 0);
}

#[test]
fn dispatch_command_happy_path() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut bus = MessageBus::new();
    bus.register_command(PlaceOrderHandler {
        log: Arc::clone(&log),
    })
    .expect("register");
    bus.dispatch_command(&PlaceOrder { sku: "hoodie" })
        .expect("dispatch");
    assert_eq!(*log.lock().expect("lock"), ["hoodie"]);
    assert_eq!(bus.command_count(), 1);
    assert!(!bus.is_empty());
}

#[test]
fn unknown_command_errors() {
    let bus = MessageBus::new();
    let err = bus
        .dispatch_command(&PlaceOrder { sku: "x" })
        .expect_err("missing handler");
    assert_eq!(
        err,
        MessengerError::UnknownCommand {
            name: "order.place"
        }
    );
}

#[test]
fn duplicate_command_registration_errors() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut bus = MessageBus::new();
    bus.register_command(PlaceOrderHandler {
        log: Arc::clone(&log),
    })
    .expect("first");
    let err = bus
        .register_command(PlaceOrderHandler { log })
        .expect_err("duplicate");
    assert_eq!(
        err,
        MessengerError::DuplicateCommand {
            name: "order.place"
        }
    );
}

#[test]
fn dispatch_event_fans_out_in_registration_order() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut bus = MessageBus::new();
    bus.register_event(RecordOrderPlaced {
        log: Arc::clone(&log),
        label: "first",
    });
    bus.register_event(RecordOrderPlaced {
        log: Arc::clone(&log),
        label: "second",
    });
    bus.dispatch_event(&OrderPlaced { id: "1" })
        .expect("dispatch");
    assert_eq!(*log.lock().expect("lock"), ["first", "second"]);
    assert_eq!(bus.event_handler_count(), 2);
}

#[test]
fn dispatch_event_without_handlers_is_ok() {
    let bus = MessageBus::new();
    bus.dispatch_event(&OrderPlaced { id: "1" }).expect("noop");
}

#[test]
fn dispatch_event_continues_after_handler_error() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut bus = MessageBus::new();
    bus.register_event(FailingEventHandler);
    bus.register_event(RecordOrderPlaced {
        log: Arc::clone(&log),
        label: "after",
    });
    let err = bus
        .dispatch_event(&OrderPlaced { id: "1" })
        .expect_err("first failed");
    assert!(matches!(
        err,
        MessengerError::Handler {
            name: "order.placed",
            ..
        }
    ));
    assert_eq!(*log.lock().expect("lock"), ["after"]);
}

#[test]
fn event_handler_order_snapshot() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut bus = MessageBus::new();
    bus.register_event(RecordOrderPlaced {
        log: Arc::clone(&log),
        label: "mailer",
    });
    bus.register_event(RecordOrderPlaced {
        log: Arc::clone(&log),
        label: "analytics",
    });
    bus.dispatch_event(&OrderPlaced { id: "42" })
        .expect("dispatch");
    let recorded: Vec<&str> = log.lock().expect("lock").clone();
    insta::assert_yaml_snapshot!(recorded);
}

#[test]
fn message_name_defaults_to_associated_const() {
    assert_eq!(PlaceOrder { sku: "hoodie" }.name(), PlaceOrder::NAME);
    assert_eq!(OrderPlaced { id: "1" }.name(), OrderPlaced::NAME);
}

#[test]
fn dispatch_command_propagates_handler_error() {
    let mut bus = MessageBus::new();
    bus.register_command(FailingCommandHandler)
        .expect("register");
    let err = bus
        .dispatch_command(&PlaceOrder { sku: "x" })
        .expect_err("handler failed");
    assert_eq!(
        err,
        MessengerError::Handler {
            name: "order.place",
            message: "rejected".to_owned(),
        }
    );
}

#[test]
fn logging_middleware_records_command_and_event() {
    let sink = RecordingSink::default();
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut bus = MessageBus::new();
    bus.add_middleware(LoggingMiddleware::new(sink.clone()));
    bus.register_command(PlaceOrderHandler {
        log: Arc::clone(&log),
    })
    .expect("register");
    bus.register_event(RecordOrderPlaced {
        log: Arc::clone(&log),
        label: "listener",
    });
    bus.dispatch_command(&PlaceOrder { sku: "hoodie" })
        .expect("command");
    bus.dispatch_event(&OrderPlaced { id: "1" }).expect("event");
    assert_eq!(bus.middleware_count(), 1);
    assert_eq!(
        *sink.entries.lock().expect("lock"),
        [
            (PlaceOrder::NAME, DispatchKind::Command),
            (OrderPlaced::NAME, DispatchKind::Event),
        ]
    );
}

#[test]
fn validation_middleware_rejects_before_handler() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut bus = MessageBus::new();
    bus.add_middleware(ValidationMiddleware::new(RejectPlaceOrder));
    bus.register_command(PlaceOrderHandler {
        log: Arc::clone(&log),
    })
    .expect("register");
    let err = bus
        .dispatch_command(&PlaceOrder { sku: "hoodie" })
        .expect_err("rejected");
    assert_eq!(
        err,
        MessengerError::Rejected {
            name: "order.place",
            message: "blocked".to_owned(),
        }
    );
    assert!(log.lock().expect("lock").is_empty());
}

#[test]
fn validation_middleware_calls_next_when_allowed() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut bus = MessageBus::new();
    bus.add_middleware(ValidationMiddleware::new(AllowAll));
    bus.register_command(PlaceOrderHandler {
        log: Arc::clone(&log),
    })
    .expect("register");
    bus.dispatch_command(&PlaceOrder { sku: "hoodie" })
        .expect("allowed");
    assert_eq!(*log.lock().expect("lock"), ["hoodie"]);
}

#[test]
fn middleware_runs_outer_to_inner() {
    let order = Arc::new(Mutex::new(Vec::new()));
    let mut bus = MessageBus::new();
    bus.add_middleware(OrderProbe {
        order: Arc::clone(&order),
        label: "outer",
    });
    bus.add_middleware(OrderProbe {
        order: Arc::clone(&order),
        label: "inner",
    });
    bus.register_command(PlaceOrderHandler {
        log: Arc::new(Mutex::new(Vec::new())),
    })
    .expect("register");
    bus.dispatch_command(&PlaceOrder { sku: "x" })
        .expect("dispatch");
    assert_eq!(
        *order.lock().expect("lock"),
        ["outer:before", "inner:before", "inner:after", "outer:after"]
    );
}

#[test]
fn logging_still_runs_for_event_without_handlers() {
    let sink = RecordingSink::default();
    let mut bus = MessageBus::new();
    bus.add_middleware(LoggingMiddleware::new(sink.clone()));
    bus.dispatch_event(&OrderPlaced { id: "1" })
        .expect("noop handlers");
    assert_eq!(
        *sink.entries.lock().expect("lock"),
        [(OrderPlaced::NAME, DispatchKind::Event)]
    );
}

#[test]
fn version_is_non_empty() {
    assert_ne!(super::version(), "");
}

#[derive(Clone, Default)]
struct RecordingSink {
    entries: Arc<Mutex<Vec<(&'static str, DispatchKind)>>>,
}

impl LogSink for RecordingSink {
    fn record(&self, ctx: &DispatchContext) {
        self.entries
            .lock()
            .expect("lock")
            .push((ctx.message_name, ctx.kind));
    }
}

struct RejectPlaceOrder;

impl ValidateHook for RejectPlaceOrder {
    fn validate(&self, ctx: &DispatchContext) -> Result<(), MessengerError> {
        if ctx.message_name == PlaceOrder::NAME {
            return Err(MessengerError::Rejected {
                name: ctx.message_name,
                message: "blocked".to_owned(),
            });
        }
        Ok(())
    }
}

struct AllowAll;

impl ValidateHook for AllowAll {
    fn validate(&self, _ctx: &DispatchContext) -> Result<(), MessengerError> {
        Ok(())
    }
}

struct OrderProbe {
    order: Arc<Mutex<Vec<&'static str>>>,
    label: &'static str,
}

impl Middleware for OrderProbe {
    fn handle(
        &self,
        _ctx: &DispatchContext,
        next: &dyn Fn() -> Result<(), MessengerError>,
    ) -> Result<(), MessengerError> {
        self.order.lock().expect("lock").push(match self.label {
            "outer" => "outer:before",
            _ => "inner:before",
        });
        let result = next();
        self.order.lock().expect("lock").push(match self.label {
            "outer" => "outer:after",
            _ => "inner:after",
        });
        result
    }
}

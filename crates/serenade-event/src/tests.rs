use std::sync::{Arc, Mutex};

use serenade_di::{CompilePass, ContainerBuilder, ServiceDefinition};

use super::{
    assert_dispatched, Event, EventDispatcher, EventSubscriber, RecordingSubscriber,
    RegisterEventSubscribersPass, SubscriberService, DISPATCHER_SERVICE, SUBSCRIBER_TAG,
};

struct NamedEvent(&'static str);

impl Event for NamedEvent {
    fn name(&self) -> &'static str {
        self.0
    }
}

struct OrderListener {
    log: Arc<Mutex<Vec<&'static str>>>,
    priority: i32,
}

impl EventSubscriber for OrderListener {
    fn event_name(&self) -> &'static str {
        "order.placed"
    }

    fn priority(&self) -> i32 {
        self.priority
    }

    fn handle(&self, event: &dyn Event) -> Result<(), super::EventError> {
        self.log
            .lock()
            .expect("lock")
            .push(if self.priority > 0 { "high" } else { "low" });
        let _ = event.name();
        Ok(())
    }
}

#[test]
fn empty_dispatcher_is_empty_and_dispatch_ok() {
    let dispatcher = EventDispatcher::new();
    assert!(dispatcher.is_empty());
    dispatcher.dispatch(&NamedEvent("noop")).unwrap();
}

#[test]
fn dispatcher_runs_higher_priority_first() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut dispatcher = EventDispatcher::new();
    dispatcher.add(Arc::new(OrderListener {
        log: log.clone(),
        priority: 0,
    }));
    dispatcher.add(Arc::new(OrderListener {
        log: log.clone(),
        priority: 10,
    }));
    dispatcher.dispatch(&NamedEvent("order.placed")).unwrap();
    assert!(!dispatcher.is_empty());
    let recorded = log.lock().expect("lock").clone();
    assert_eq!(recorded, ["high", "low"]);
}

#[test]
fn harness_records_matching_events() {
    let names = Arc::new(Mutex::new(Vec::new()));
    let mut dispatcher = EventDispatcher::new();
    dispatcher.add(Arc::new(
        RecordingSubscriber::new("cart.updated", names.clone()).with_priority(1),
    ));
    dispatcher.dispatch(&NamedEvent("cart.updated")).unwrap();
    dispatcher.dispatch(&NamedEvent("order.placed")).unwrap();
    let recorded = names.lock().expect("lock").clone();
    assert_dispatched(&recorded, &["cart.updated"]);
}

#[test]
fn harness_dispatch_order_snapshot() {
    let names = Arc::new(Mutex::new(Vec::new()));
    let mut dispatcher = EventDispatcher::new();
    dispatcher.add(Arc::new(
        RecordingSubscriber::new("cart.updated", names.clone()).with_priority(1),
    ));
    dispatcher.add(Arc::new(
        RecordingSubscriber::new("order.placed", names.clone()).with_priority(0),
    ));
    dispatcher.dispatch(&NamedEvent("cart.updated")).unwrap();
    dispatcher.dispatch(&NamedEvent("order.placed")).unwrap();
    let recorded: Vec<&str> = names.lock().expect("lock").clone();
    insta::assert_yaml_snapshot!(recorded);
}

/// Sync collaborator double for bundle authors (mockall + `cargo test`).
#[mockall::automock]
trait LabelSource {
    fn label(&self) -> &'static str;
}

#[test]
fn mockall_label_source_returns_configured_value() {
    let mut source = MockLabelSource::new();
    source.expect_label().return_const("cart.updated");
    assert_eq!(source.label(), "cart.updated");
}

#[test]
fn compile_pass_collects_tagged_subscribers() {
    let names = Arc::new(Mutex::new(Vec::new()));
    let listener = RecordingSubscriber::new("cart.updated", names.clone());
    let mut builder = ContainerBuilder::new();
    builder
        .register(
            ServiceDefinition::new("cart_listener").with_tag(SUBSCRIBER_TAG),
            move |_| Ok(Box::new(SubscriberService(Arc::new(listener.clone())))),
        )
        .unwrap();
    builder.add_compile_pass(RegisterEventSubscribersPass);
    let container = builder.compile().unwrap();
    let dispatcher = container
        .get_as::<EventDispatcher>(DISPATCHER_SERVICE)
        .unwrap();
    dispatcher.dispatch(&NamedEvent("cart.updated")).unwrap();
    let actual = names.lock().expect("lock").clone();
    assert_dispatched(&actual, &["cart.updated"]);
}

#[test]
fn crate_version_is_non_empty() {
    assert_ne!(super::version(), "");
}

struct DefaultPriorityListener {
    log: Arc<Mutex<Vec<&'static str>>>,
}

impl EventSubscriber for DefaultPriorityListener {
    fn event_name(&self) -> &'static str {
        "noop.default"
    }

    fn handle(&self, event: &dyn Event) -> Result<(), super::EventError> {
        self.log.lock().expect("lock").push(event.name());
        Ok(())
    }
}

#[test]
fn subscriber_default_priority_is_zero() {
    let listener = DefaultPriorityListener {
        log: Arc::new(Mutex::new(Vec::new())),
    };
    assert_eq!(listener.priority(), 0);
    let mut dispatcher = EventDispatcher::new();
    dispatcher.add(Arc::new(listener));
    dispatcher.dispatch(&NamedEvent("noop.default")).unwrap();
}

struct FailingListener {
    message: &'static str,
}

impl EventSubscriber for FailingListener {
    fn event_name(&self) -> &'static str {
        "fail.once"
    }

    fn handle(&self, event: &dyn Event) -> Result<(), super::EventError> {
        Err(super::EventError::Subscriber {
            subscriber: self.event_name().to_owned(),
            event: event.name(),
            message: self.message.to_owned(),
        })
    }
}

#[test]
fn dispatch_returns_first_subscriber_error_and_reports_len() {
    let mut dispatcher = EventDispatcher::new();
    dispatcher.add(Arc::new(FailingListener { message: "first" }));
    dispatcher.add(Arc::new(FailingListener { message: "second" }));
    assert_eq!(dispatcher.len(), 2);
    let err = dispatcher
        .dispatch(&NamedEvent("fail.once"))
        .expect_err("first failure wins");
    assert!(matches!(
        err,
        super::EventError::Subscriber { message, .. } if message == "first"
    ));
}

#[test]
fn recording_subscriber_maps_poisoned_lock() {
    let names = Arc::new(Mutex::new(Vec::new()));
    let poison = Arc::clone(&names);
    let _ = std::thread::spawn(move || {
        let _guard = poison.lock().expect("lock");
        panic!("poison recording mutex");
    })
    .join();
    let subscriber = RecordingSubscriber::new("poison.event", names);
    let err = subscriber
        .handle(&NamedEvent("poison.event"))
        .expect_err("poisoned lock");
    assert!(matches!(
        err,
        super::EventError::Subscriber { message, .. } if message == "lock poisoned"
    ));
}

#[test]
fn register_pass_name_is_stable() {
    assert_eq!(
        RegisterEventSubscribersPass.name(),
        "register_event_subscribers"
    );
}

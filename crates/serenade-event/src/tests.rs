use std::sync::{Arc, Mutex};

use serenade_di::{ContainerBuilder, ServiceDefinition};

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

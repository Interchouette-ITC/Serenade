//! Unit tests for `serenade-notifier`.

use std::sync::Arc;

use serenade_di::{CompilePass, ContainerBuilder, ServiceDefinition};

use crate::{
    Channel, DEFAULT_NOTIFIER_SERVICE, MemoryTransport, NOTIFIER_TRANSPORT_TAG, Notification,
    NotifierError, NotifierService, NullTransport, RegisterDefaultNotifierPass, SmsOnlyTransport,
    Transport, version,
};

#[test]
fn version_is_nonempty() {
    assert_ne!(version(), "");
}

#[test]
fn channel_names() {
    assert_eq!(Channel::Sms.as_str(), "sms");
    assert_eq!(Channel::Push.as_str(), "push");
}

#[test]
fn notification_channel_and_accessors() {
    let sms = Notification::sms("+15551212", "hello");
    assert_eq!(sms.channel(), Channel::Sms);
    assert!(matches!(
        &sms,
        Notification::Sms(message)
            if message.to() == "+15551212" && message.body() == "hello"
    ));

    let push = Notification::push("token-1", "Hi", "body");
    assert_eq!(push.channel(), Channel::Push);
    assert!(matches!(
        &push,
        Notification::Push(message)
            if message.device_token() == "token-1"
                && message.title() == "Hi"
                && message.body() == "body"
    ));
}

#[test]
fn null_transport_validates_and_discards() {
    let transport = NullTransport::new();
    assert!(transport.supports(Channel::Sms));
    assert!(transport.supports(Channel::Push));
    transport.send(&Notification::sms("+1", "ok")).expect("sms");
    transport
        .send(&Notification::push("tok", "t", "b"))
        .expect("push");
    assert!(matches!(
        transport.send(&Notification::sms("", "x")),
        Err(NotifierError::MissingRecipient)
    ));
    assert!(matches!(
        transport.send(&Notification::sms("+1", "  ")),
        Err(NotifierError::MissingBody)
    ));
    assert!(matches!(
        transport.send(&Notification::push("", "t", "b")),
        Err(NotifierError::MissingRecipient)
    ));
    assert!(matches!(
        transport.send(&Notification::push("tok", "t", "")),
        Err(NotifierError::MissingBody)
    ));
}

#[test]
fn memory_transport_records_sends() {
    let transport = MemoryTransport::new();
    transport.send(&Notification::sms("+1", "a")).expect("sms");
    transport
        .send(&Notification::push("tok", "t", "b"))
        .expect("push");
    assert_eq!(transport.sent().len(), 2);
    transport.clear();
    assert_eq!(transport.sent().len(), 0);
}

#[test]
fn sms_only_rejects_push() {
    let transport = SmsOnlyTransport::new();
    assert!(transport.supports(Channel::Sms));
    assert!(!transport.supports(Channel::Push));
    transport.send(&Notification::sms("+1", "ok")).expect("sms");
    assert!(matches!(
        transport.send(&Notification::push("tok", "t", "b")),
        Err(NotifierError::UnsupportedChannel { channel }) if channel == "push"
    ));
}

#[test]
fn compile_pass_registers_null_notifier() {
    let pass = RegisterDefaultNotifierPass;
    assert_eq!(pass.name(), "register_default_notifier");
    let mut builder = ContainerBuilder::new();
    builder.add_compile_pass(RegisterDefaultNotifierPass);
    let container = builder.compile().expect("compile");
    let notifier = container
        .get_as::<NotifierService>(DEFAULT_NOTIFIER_SERVICE)
        .expect("notifier");
    notifier.send(&Notification::sms("+1", "hi")).expect("send");
}

#[test]
fn compile_pass_skips_when_default_already_registered() {
    let mut builder = ContainerBuilder::new();
    builder
        .register(
            ServiceDefinition::new(DEFAULT_NOTIFIER_SERVICE).with_tag(NOTIFIER_TRANSPORT_TAG),
            |_container| {
                Ok(Box::new(NotifierService(
                    Arc::new(MemoryTransport::new()) as Arc<dyn Transport>
                )))
            },
        )
        .expect("register");
    builder.add_compile_pass(RegisterDefaultNotifierPass);
    let container = builder.compile().expect("compile");
    let notifier = container
        .get_as::<NotifierService>(DEFAULT_NOTIFIER_SERVICE)
        .expect("notifier");
    notifier
        .send(&Notification::push("tok", "t", "b"))
        .expect("send");
}

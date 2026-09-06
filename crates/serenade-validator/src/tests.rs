use serenade_messenger::{
    Command, CommandHandler, Message, MessageBus, MessengerError, ValidationMiddleware,
};

use super::{
    version, Constraint, ConstraintViolationList, Length, MessengerValidateHook, NotBlank, Range,
    RecursiveValidator, Validatable, Validator, Violation,
};

struct EnqueueJob {
    code: String,
    qty: String,
}

impl Message for EnqueueJob {
    const NAME: &'static str = "job.enqueue";
}

impl Command for EnqueueJob {}

impl Validatable for EnqueueJob {
    fn validate(&self, validator: &dyn Validator) -> ConstraintViolationList {
        let mut list = validator.validate_value(
            &self.code,
            "code",
            &[&NotBlank as &dyn Constraint, &Length::new(1, 16)],
        );
        let qty = validator.validate_value(&self.qty, "qty", &[&Range::new(1, 100)]);
        for violation in qty.as_slice() {
            list.add(violation.clone());
        }
        list
    }
}

struct EnqueueJobHandler;

impl CommandHandler<EnqueueJob> for EnqueueJobHandler {
    fn handle(&self, _command: &EnqueueJob) -> Result<(), MessengerError> {
        Ok(())
    }
}

#[test]
fn version_is_non_empty() {
    assert_ne!(version(), "");
}

#[test]
fn not_blank_and_length_collect_violations() {
    let validator = RecursiveValidator::new();
    let violations = validator.validate_value("", "name", &[&NotBlank, &Length::new(2, 5)]);
    assert_eq!(violations.len(), 2);
    assert_eq!(violations.as_slice()[0].code, "NotBlank");
    assert_eq!(violations.as_slice()[1].code, "Length");
}

#[test]
fn valid_path_is_empty() {
    let validator = RecursiveValidator::new();
    let violations = validator.validate_value("ab", "name", &[&NotBlank, &Length::new(2, 5)]);
    assert!(violations.is_empty());
}

#[test]
fn range_rejects_out_of_bounds_and_non_integer() {
    let validator = RecursiveValidator::new();
    let bad = validator.validate_value("0", "qty", &[&Range::new(1, 10)]);
    assert_eq!(bad.len(), 1);
    let nan = validator.validate_value("x", "qty", &[&Range::new(1, 10)]);
    assert_eq!(
        nan.as_slice()[0].message,
        "This value should be a valid integer."
    );
}

#[test]
fn violation_helpers() {
    let mut list = ConstraintViolationList::new();
    list.add(Violation::new("code", "bad", "NotBlank"));
    assert!(!list.is_empty());
    assert_eq!(list.len(), 1);
}

#[test]
fn messenger_hook_rejects_invalid_command() {
    let mut hook = MessengerValidateHook::new(RecursiveValidator);
    hook.register::<EnqueueJob>();
    let mut bus = MessageBus::new();
    bus.add_middleware(ValidationMiddleware::new(hook));
    bus.register_command(EnqueueJobHandler).expect("register");

    let err = bus
        .dispatch_command(&EnqueueJob {
            code: String::new(),
            qty: "1".to_owned(),
        })
        .expect_err("rejected");
    assert!(matches!(err, MessengerError::Rejected { .. }));

    bus.dispatch_command(&EnqueueJob {
        code: "SKU-1".to_owned(),
        qty: "2".to_owned(),
    })
    .expect("valid");
}

#[test]
fn messenger_hook_ignores_unregistered_names() {
    let hook = MessengerValidateHook::new(RecursiveValidator);
    let mut bus = MessageBus::new();
    bus.add_middleware(ValidationMiddleware::new(hook));
    bus.register_command(EnqueueJobHandler).expect("register");
    bus.dispatch_command(&EnqueueJob {
        code: String::new(),
        qty: "1".to_owned(),
    })
    .expect("no registration means allow");
}

struct ObjectLevelFail;

impl Message for ObjectLevelFail {
    const NAME: &'static str = "order.object_fail";
}

impl Command for ObjectLevelFail {}

impl Validatable for ObjectLevelFail {
    fn validate(&self, _validator: &dyn Validator) -> ConstraintViolationList {
        let mut list = ConstraintViolationList::new();
        list.add(Violation::new("", "object invalid", "ObjectLevel"));
        list
    }
}

struct ObjectLevelFailHandler;

impl CommandHandler<ObjectLevelFail> for ObjectLevelFailHandler {
    fn handle(&self, _command: &ObjectLevelFail) -> Result<(), MessengerError> {
        Ok(())
    }
}

#[test]
fn messenger_hook_formats_empty_property_path() {
    let mut hook = MessengerValidateHook::new(RecursiveValidator);
    hook.register::<ObjectLevelFail>();
    let mut bus = MessageBus::new();
    bus.add_middleware(ValidationMiddleware::new(hook));
    bus.register_command(ObjectLevelFailHandler)
        .expect("register");
    let err = bus
        .dispatch_command(&ObjectLevelFail)
        .expect_err("rejected");
    match err {
        MessengerError::Rejected { message, .. } => {
            assert_eq!(message, "object invalid");
        }
        other => panic!("unexpected {other:?}"),
    }
}

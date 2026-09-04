use std::sync::Arc;

use super::{
    CompilePass, ContainerBuilder, DiError, ParameterBag, Reference, Scope, ServiceDefinition,
};

#[derive(Debug)]
struct Counter {
    value: u32,
}

struct AddParamPass;

impl CompilePass for AddParamPass {
    fn name(&self) -> &'static str {
        "add_param"
    }

    fn process(&self, builder: &mut ContainerBuilder) -> Result<(), DiError> {
        builder.parameters_mut().set("app.name", "serenade");
        Ok(())
    }
}

#[test]
fn version_is_non_empty() {
    assert_ne!(super::version(), "");
}

#[test]
fn singleton_is_shared_prototype_is_fresh() {
    let mut builder = ContainerBuilder::new();
    builder
        .register(ServiceDefinition::new("counter"), |_| {
            Ok(Box::new(Counter { value: 1 }))
        })
        .unwrap();
    builder
        .register(
            ServiceDefinition::new("ticket").with_scope(Scope::Prototype),
            |_| Ok(Box::new(Counter { value: 7 })),
        )
        .unwrap();
    let container = builder.compile().unwrap();

    let first = container.get_as::<Counter>("counter").unwrap();
    let second = container.get_as::<Counter>("counter").unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(first.value, 1);

    let ticket_a = container.get_as::<Counter>("ticket").unwrap();
    let ticket_b = container.get_as::<Counter>("ticket").unwrap();
    assert!(!Arc::ptr_eq(&ticket_a, &ticket_b));
    assert_eq!(ticket_a.value, 7);
}

#[test]
fn alias_and_parameters_resolve() {
    let mut builder = ContainerBuilder::new();
    builder.parameters_mut().set("db.url", "postgres://local");
    builder
        .register(ServiceDefinition::new("db"), |container| {
            let url = container.parameters().get("db.url")?.to_owned();
            Ok(Box::new(url))
        })
        .unwrap();
    builder.set_alias("database", "db").unwrap();
    let container = builder.compile().unwrap();
    let url = container.get_as::<String>("database").unwrap();
    assert_eq!(url.as_str(), "postgres://local");
}

#[test]
fn compile_pass_pipeline_runs() {
    let mut builder = ContainerBuilder::new();
    builder.add_compile_pass(AddParamPass);
    builder
        .register(ServiceDefinition::new("name"), |container| {
            Ok(Box::new(container.parameters().get("app.name")?.to_owned()))
        })
        .unwrap();
    let container = builder.compile().unwrap();
    assert_eq!(
        container.get_as::<String>("name").unwrap().as_str(),
        "serenade"
    );
}

#[test]
fn circular_dependency_is_detected_at_compile_time() {
    let mut builder = ContainerBuilder::new();
    builder
        .register(
            ServiceDefinition::new("a").with_dependencies(vec![Reference::new("b")]),
            |_| Ok(Box::new(1_u8)),
        )
        .unwrap();
    builder
        .register(
            ServiceDefinition::new("b").with_dependencies(vec![Reference::new("a")]),
            |_| Ok(Box::new(2_u8)),
        )
        .unwrap();
    let Err(error) = builder.compile() else {
        panic!("expected circular dependency");
    };
    assert!(matches!(error, DiError::CircularDependency(_)));
}

#[test]
fn runtime_circular_dependency_is_detected() {
    let mut builder = ContainerBuilder::new();
    builder
        .register(ServiceDefinition::new("a"), |container| {
            let _ = container.get("b")?;
            Ok(Box::new(1_u8))
        })
        .unwrap();
    builder
        .register(ServiceDefinition::new("b"), |container| {
            let _ = container.get("a")?;
            Ok(Box::new(2_u8))
        })
        .unwrap();
    let container = builder.compile().unwrap();
    let error = container.get("a").unwrap_err();
    assert!(matches!(error, DiError::CircularDependency(_)));
}

#[test]
fn parameter_bag_missing_key() {
    let bag = ParameterBag::new();
    assert!(matches!(
        bag.get("missing"),
        Err(DiError::ParameterNotFound(_))
    ));
}

#[test]
fn definition_and_definitions_list() {
    let mut builder = ContainerBuilder::new();
    builder
        .register(ServiceDefinition::new("svc"), |_| Ok(Box::new(9_u8)))
        .unwrap();
    builder.set_alias("alias", "svc").unwrap();
    let container = builder.compile().unwrap();
    assert!(container.definition("svc").is_some());
    assert!(container.definition("alias").is_some());
    assert!(container.definition("missing").is_none());
    assert_eq!(container.definitions().len(), 1);
}

#[test]
fn get_as_downcast_failure() {
    let mut builder = ContainerBuilder::new();
    builder
        .register(ServiceDefinition::new("n"), |_| Ok(Box::new(3_u8)))
        .unwrap();
    let container = builder.compile().unwrap();
    let err = container.get_as::<String>("n").unwrap_err();
    assert!(matches!(err, DiError::Factory { .. }));
}

#[test]
fn reference_from_str_and_missing_service() {
    let reference = Reference::from("catalog");
    assert_eq!(reference.id(), "catalog");
    let owned = Reference::from(String::from("orders"));
    assert_eq!(owned.id(), "orders");
    let mut builder = ContainerBuilder::new();
    builder
        .register(ServiceDefinition::new("only"), |_| Ok(Box::new(1_u8)))
        .unwrap();
    let container = builder.compile().unwrap();
    let err = container.get("missing").unwrap_err();
    assert!(matches!(err, DiError::NotFound(_)));
}

#[test]
fn duplicate_service_and_alias_ids_are_rejected() {
    let mut builder = ContainerBuilder::new();
    builder
        .register(ServiceDefinition::new("svc"), |_| Ok(Box::new(1_u8)))
        .unwrap();
    let err = builder
        .register(ServiceDefinition::new("svc"), |_| Ok(Box::new(2_u8)))
        .unwrap_err();
    assert!(matches!(err, DiError::DuplicateService(_)));
    builder.set_alias("alias", "svc").unwrap();
    let err = builder.set_alias("alias", "svc").unwrap_err();
    assert!(matches!(err, DiError::DuplicateService(_)));
    let err = builder.set_alias("svc", "other").unwrap_err();
    assert!(matches!(err, DiError::DuplicateService(_)));
}

#[test]
fn invalid_alias_and_missing_dependency_fail_compile() {
    let mut missing_target = ContainerBuilder::new();
    missing_target
        .register(ServiceDefinition::new("svc"), |_| Ok(Box::new(1_u8)))
        .unwrap();
    missing_target.set_alias("alias", "gone").unwrap();
    let Err(err) = missing_target.compile() else {
        panic!("expected invalid alias");
    };
    assert!(matches!(err, DiError::InvalidAlias { .. }));

    let mut missing_dep = ContainerBuilder::new();
    missing_dep
        .register(
            ServiceDefinition::new("consumer").with_dependencies(vec![Reference::new("missing")]),
            |_| Ok(Box::new(1_u8)),
        )
        .unwrap();
    let Err(err) = missing_dep.compile() else {
        panic!("expected missing dependency");
    };
    assert!(matches!(err, DiError::NotFound(_)));
}

#[test]
fn dependency_resolved_through_alias_and_circular_alias_chain() {
    let mut builder = ContainerBuilder::new();
    builder
        .register(ServiceDefinition::new("core"), |_| Ok(Box::new(9_u8)))
        .unwrap();
    builder.set_alias("core_alias", "core").unwrap();
    builder
        .register(
            ServiceDefinition::new("consumer")
                .with_dependencies(vec![Reference::new("core_alias")]),
            |container| {
                let value = container.get_as::<u8>("core_alias")?;
                Ok(Box::new(*value))
            },
        )
        .unwrap();
    let container = builder.compile().unwrap();
    assert_eq!(*container.get_as::<u8>("consumer").unwrap(), 9);

    let mut cyclic = ContainerBuilder::new();
    cyclic
        .register(ServiceDefinition::new("svc"), |_| Ok(Box::new(1_u8)))
        .unwrap();
    cyclic.set_alias("a", "b").unwrap();
    cyclic.set_alias("b", "a").unwrap();
    let Err(err) = cyclic.compile() else {
        panic!("expected circular alias");
    };
    assert!(matches!(err, DiError::CircularDependency(_)));
}

#[test]
fn factory_non_factory_errors_are_remapped() {
    let mut builder = ContainerBuilder::new();
    builder
        .register(ServiceDefinition::new("bad"), |_| {
            Err(DiError::ParameterNotFound("x".into()))
        })
        .unwrap();
    let container = builder.compile().unwrap();
    let err = container.get("bad").unwrap_err();
    assert!(matches!(
        err,
        DiError::Factory {
            service,
            ..
        } if service == "bad"
    ));
}

#[test]
fn definitions_lists_registered_services() {
    let mut builder = ContainerBuilder::new();
    builder
        .register(ServiceDefinition::new("one"), |_| Ok(Box::new(1_u8)))
        .unwrap();
    builder
        .register(ServiceDefinition::new("two"), |_| Ok(Box::new(2_u8)))
        .unwrap();
    let ids: Vec<_> = builder
        .definitions()
        .into_iter()
        .map(ServiceDefinition::id)
        .collect();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&"one"));
    assert!(ids.contains(&"two"));
}

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

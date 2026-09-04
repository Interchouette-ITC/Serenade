//! Mutable container builder used during compile.

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use crate::{CompilePass, Container, DiError, ParameterBag, ServiceDefinition};

/// Factory invoked when a service is resolved.
pub type ServiceFactory =
    Arc<dyn Fn(&Container) -> Result<Box<dyn Any + Send + Sync>, DiError> + Send + Sync>;

struct PendingService {
    definition: ServiceDefinition,
    factory: ServiceFactory,
}

/// Builds a [`Container`] from definitions, aliases, parameters, and compile passes.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use serenade_di::{ContainerBuilder, ServiceDefinition};
///
/// let mut builder = ContainerBuilder::new();
/// builder
///     .register(ServiceDefinition::new("greeting"), |_| {
///         Ok(Box::new(String::from("hello")))
///     })
///     .expect("register");
/// let container = builder.compile().expect("compile");
/// let greeting = container.get_as::<String>("greeting").expect("resolve");
/// assert_eq!(greeting.as_str(), "hello");
/// assert!(Arc::strong_count(&greeting) >= 1);
/// ```
#[derive(Default)]
pub struct ContainerBuilder {
    services: HashMap<String, PendingService>,
    aliases: HashMap<String, String>,
    parameters: ParameterBag,
    compile_passes: Vec<Box<dyn CompilePass>>,
}

impl ContainerBuilder {
    /// Creates an empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Shared parameter bag.
    #[must_use]
    pub const fn parameters(&self) -> &ParameterBag {
        &self.parameters
    }

    /// Mutable parameter bag.
    pub fn parameters_mut(&mut self) -> &mut ParameterBag {
        &mut self.parameters
    }

    /// Registers a service factory under `definition.id()`.
    ///
    /// # Errors
    ///
    /// Returns [`DiError::DuplicateService`] when the id already exists as a service or alias.
    pub fn register(
        &mut self,
        definition: ServiceDefinition,
        factory: impl Fn(&Container) -> Result<Box<dyn Any + Send + Sync>, DiError>
            + Send
            + Sync
            + 'static,
    ) -> Result<(), DiError> {
        let id = definition.id().to_owned();
        if self.services.contains_key(&id) || self.aliases.contains_key(&id) {
            return Err(DiError::DuplicateService(id));
        }
        self.services.insert(
            id,
            PendingService {
                definition,
                factory: Arc::new(factory),
            },
        );
        Ok(())
    }

    /// Adds an alias from `alias` to `target`.
    ///
    /// # Errors
    ///
    /// Returns [`DiError::DuplicateService`] when `alias` is already used.
    pub fn set_alias(
        &mut self,
        alias: impl Into<String>,
        target: impl Into<String>,
    ) -> Result<(), DiError> {
        let alias = alias.into();
        if self.services.contains_key(&alias) || self.aliases.contains_key(&alias) {
            return Err(DiError::DuplicateService(alias));
        }
        self.aliases.insert(alias, target.into());
        Ok(())
    }

    /// Appends a compile pass. Passes run in registration order during [`Self::compile`].
    pub fn add_compile_pass(&mut self, pass: impl CompilePass + 'static) {
        self.compile_passes.push(Box::new(pass));
    }

    /// Returns registered service definitions.
    #[must_use]
    pub fn definitions(&self) -> Vec<&ServiceDefinition> {
        self.services
            .values()
            .map(|pending| &pending.definition)
            .collect()
    }

    /// Runs compile passes, validates the graph, and freezes a [`Container`].
    ///
    /// # Errors
    ///
    /// Returns alias, missing-dependency, or circular-dependency errors.
    pub fn compile(mut self) -> Result<Container, DiError> {
        let passes = std::mem::take(&mut self.compile_passes);
        for pass in passes {
            pass.process(&mut self)?;
        }
        self.validate_aliases()?;
        self.validate_dependencies()?;
        detect_cycles(&self.services)?;

        let mut definitions = HashMap::new();
        let mut factories = HashMap::new();
        let mut scopes = HashMap::new();
        for (id, pending) in self.services {
            scopes.insert(id.clone(), pending.definition.scope());
            definitions.insert(id.clone(), pending.definition);
            factories.insert(id, pending.factory);
        }

        Ok(Container::from_parts(
            self.parameters,
            self.aliases,
            definitions,
            factories,
            scopes,
        ))
    }

    fn validate_aliases(&self) -> Result<(), DiError> {
        for (alias, target) in &self.aliases {
            let resolved = resolve_alias_chain(&self.aliases, target)?;
            if !self.services.contains_key(resolved) {
                return Err(DiError::InvalidAlias {
                    alias: alias.clone(),
                    target: target.clone(),
                });
            }
        }
        Ok(())
    }

    fn validate_dependencies(&self) -> Result<(), DiError> {
        for pending in self.services.values() {
            for dependency in pending.definition.dependencies() {
                if self.resolve_id(dependency.id()).is_err() {
                    return Err(DiError::NotFound(dependency.id().to_owned()));
                }
            }
        }
        Ok(())
    }

    fn resolve_id<'a>(&'a self, id: &'a str) -> Result<&'a str, DiError> {
        if self.services.contains_key(id) {
            return Ok(id);
        }
        if let Some(target) = self.aliases.get(id) {
            return resolve_alias_chain(&self.aliases, target);
        }
        Err(DiError::NotFound(id.to_owned()))
    }
}

fn resolve_alias_chain<'a>(
    aliases: &'a HashMap<String, String>,
    start: &'a str,
) -> Result<&'a str, DiError> {
    let mut current = start;
    let mut seen = Vec::new();
    while let Some(next) = aliases.get(current) {
        if seen.contains(&current) {
            seen.push(current);
            return Err(DiError::CircularDependency(seen.join(" -> ")));
        }
        seen.push(current);
        current = next.as_str();
    }
    Ok(current)
}

enum VisitMark {
    Visiting,
    Done,
}

fn detect_cycles(services: &HashMap<String, PendingService>) -> Result<(), DiError> {
    let mut marks = HashMap::<&str, VisitMark>::new();
    for id in services.keys() {
        visit(id, services, &mut marks, &mut Vec::new())?;
    }
    Ok(())
}

fn visit<'a>(
    id: &'a str,
    services: &'a HashMap<String, PendingService>,
    marks: &mut HashMap<&'a str, VisitMark>,
    stack: &mut Vec<&'a str>,
) -> Result<(), DiError> {
    match marks.get(id) {
        Some(VisitMark::Done) => return Ok(()),
        Some(VisitMark::Visiting) => {
            stack.push(id);
            return Err(DiError::CircularDependency(stack.join(" -> ")));
        }
        None => {}
    }

    marks.insert(id, VisitMark::Visiting);
    stack.push(id);
    // `detect_cycles` only visits ids present in `services`.
    let pending = services
        .get(id)
        .expect("visit is only called for registered service ids");
    for dependency in pending.definition.dependencies() {
        let dep_id = dependency.id();
        if services.contains_key(dep_id) {
            visit(dep_id, services, marks, stack)?;
        }
    }
    stack.pop();
    marks.insert(id, VisitMark::Done);
    Ok(())
}

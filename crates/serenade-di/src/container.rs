//! Compiled service container.

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::builder::ServiceFactory;
use crate::{DiError, ParameterBag, Scope, ServiceDefinition};

/// Frozen service container produced by [`crate::ContainerBuilder::compile`].
pub struct Container {
    parameters: ParameterBag,
    aliases: HashMap<String, String>,
    definitions: HashMap<String, ServiceDefinition>,
    factories: HashMap<String, ServiceFactory>,
    scopes: HashMap<String, Scope>,
    singletons: Mutex<HashMap<String, Arc<dyn Any + Send + Sync>>>,
    resolving: Mutex<Vec<String>>,
}

impl Container {
    pub(crate) fn from_parts(
        parameters: ParameterBag,
        aliases: HashMap<String, String>,
        definitions: HashMap<String, ServiceDefinition>,
        factories: HashMap<String, ServiceFactory>,
        scopes: HashMap<String, Scope>,
    ) -> Self {
        Self {
            parameters,
            aliases,
            definitions,
            factories,
            scopes,
            singletons: Mutex::new(HashMap::new()),
            resolving: Mutex::new(Vec::new()),
        }
    }

    /// Parameter bag frozen at compile time.
    #[must_use]
    pub const fn parameters(&self) -> &ParameterBag {
        &self.parameters
    }

    /// Returns a service definition by id or alias.
    #[must_use]
    pub fn definition(&self, id: &str) -> Option<&ServiceDefinition> {
        let resolved = self.resolve_alias(id).ok()?;
        self.definitions.get(resolved)
    }

    /// All registered service definitions (after alias resolution keys).
    #[must_use]
    pub fn definitions(&self) -> Vec<&ServiceDefinition> {
        self.definitions.values().collect()
    }

    /// Resolves a service by id or alias.
    ///
    /// # Errors
    ///
    /// Returns not-found, circular-dependency, or factory errors.
    pub fn get(&self, id: &str) -> Result<Arc<dyn Any + Send + Sync>, DiError> {
        let resolved = self.resolve_alias(id)?.to_owned();
        let scope = *self
            .scopes
            .get(&resolved)
            .ok_or_else(|| DiError::NotFound(resolved.clone()))?;

        if scope == Scope::Singleton {
            let singletons = self.singletons.lock().map_err(|_| DiError::Factory {
                service: resolved.clone(),
                message: "singleton lock poisoned".to_owned(),
            })?;
            if let Some(existing) = singletons.get(&resolved) {
                return Ok(Arc::clone(existing));
            }
            drop(singletons);
        }

        self.push_resolving(&resolved)?;
        let factory = self
            .factories
            .get(&resolved)
            .ok_or_else(|| DiError::NotFound(resolved.clone()))?
            .clone();
        let built = factory(self).map_err(|error| match error {
            DiError::Factory { .. } | DiError::CircularDependency(_) | DiError::NotFound(_) => {
                error
            }
            other => DiError::Factory {
                service: resolved.clone(),
                message: other.to_string(),
            },
        });
        self.pop_resolving(&resolved)?;
        let instance = Arc::from(built?);

        if scope == Scope::Singleton {
            let mut singletons = self.singletons.lock().map_err(|_| DiError::Factory {
                service: resolved.clone(),
                message: "singleton lock poisoned".to_owned(),
            })?;
            Ok(Arc::clone(
                singletons
                    .entry(resolved)
                    .or_insert_with(|| Arc::clone(&instance)),
            ))
        } else {
            Ok(instance)
        }
    }

    /// Resolves and downcasts a service.
    ///
    /// # Errors
    ///
    /// Returns [`Self::get`] errors, or a factory error when the type does not match.
    pub fn get_as<T>(&self, id: &str) -> Result<Arc<T>, DiError>
    where
        T: Send + Sync + 'static,
    {
        let service = self.get(id)?;
        Arc::downcast::<T>(service).map_err(|_| DiError::Factory {
            service: id.to_owned(),
            message: "downcast failed".to_owned(),
        })
    }

    fn resolve_alias<'a>(&'a self, id: &'a str) -> Result<&'a str, DiError> {
        let mut current = id;
        let mut seen = Vec::new();
        while let Some(next) = self.aliases.get(current) {
            if seen.contains(&current) {
                seen.push(current);
                return Err(DiError::CircularDependency(seen.join(" -> ")));
            }
            seen.push(current);
            current = next.as_str();
        }
        if self.definitions.contains_key(current) {
            Ok(current)
        } else {
            Err(DiError::NotFound(id.to_owned()))
        }
    }

    fn push_resolving(&self, id: &str) -> Result<(), DiError> {
        let mut stack = self.resolving.lock().map_err(|_| DiError::Factory {
            service: id.to_owned(),
            message: "resolving lock poisoned".to_owned(),
        })?;
        if stack.iter().any(|item| item == id) {
            stack.push(id.to_owned());
            return Err(DiError::CircularDependency(stack.join(" -> ")));
        }
        stack.push(id.to_owned());
        drop(stack);
        Ok(())
    }

    fn pop_resolving(&self, id: &str) -> Result<(), DiError> {
        let mut stack = self.resolving.lock().map_err(|_| DiError::Factory {
            service: id.to_owned(),
            message: "resolving lock poisoned".to_owned(),
        })?;
        if stack.last().map(String::as_str) == Some(id) {
            stack.pop();
        }
        drop(stack);
        Ok(())
    }

    #[cfg(test)]
    fn poison_singletons_for_test(&self) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = self.singletons.lock().expect("lock");
            panic!("poison singletons");
        }));
    }

    #[cfg(test)]
    fn poison_resolving_for_test(&self) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = self.resolving.lock().expect("lock");
            panic!("poison resolving");
        }));
    }
}

#[cfg(test)]
mod poison_and_alias_tests {
    use std::collections::HashMap;

    use crate::{DiError, ParameterBag, ServiceDefinition};

    use super::Container;

    #[test]
    fn resolve_alias_reports_runtime_cycle() {
        let aliases = HashMap::from([
            ("a".to_owned(), "b".to_owned()),
            ("b".to_owned(), "a".to_owned()),
        ]);
        let container = Container::from_parts(
            ParameterBag::new(),
            aliases,
            HashMap::new(),
            HashMap::new(),
            HashMap::new(),
        );
        let err = container.get("a").unwrap_err();
        assert!(matches!(err, DiError::CircularDependency(_)));
    }

    #[test]
    fn get_reports_poisoned_singleton_lock() {
        let mut builder = crate::ContainerBuilder::new();
        builder
            .register(ServiceDefinition::new("n"), |_| Ok(Box::new(1_u8)))
            .unwrap();
        let container = builder.compile().unwrap();
        container.poison_singletons_for_test();
        let err = container.get("n").unwrap_err();
        assert!(matches!(
            err,
            DiError::Factory { message, .. } if message == "singleton lock poisoned"
        ));
    }

    #[test]
    fn get_reports_poisoned_singleton_insert_lock() {
        let mut builder = crate::ContainerBuilder::new();
        builder
            .register(ServiceDefinition::new("n"), |container| {
                container.poison_singletons_for_test();
                Ok(Box::new(1_u8))
            })
            .unwrap();
        let container = builder.compile().unwrap();
        let err = container.get("n").unwrap_err();
        assert!(matches!(
            err,
            DiError::Factory { message, .. } if message == "singleton lock poisoned"
        ));
    }

    #[test]
    fn get_reports_poisoned_resolving_lock_on_push() {
        let mut builder = crate::ContainerBuilder::new();
        builder
            .register(ServiceDefinition::new("n"), |_| Ok(Box::new(1_u8)))
            .unwrap();
        let container = builder.compile().unwrap();
        container.poison_resolving_for_test();
        let err = container.get("n").unwrap_err();
        assert!(matches!(
            err,
            DiError::Factory { message, .. } if message == "resolving lock poisoned"
        ));
    }

    #[test]
    fn get_reports_poisoned_resolving_lock_on_pop() {
        let mut builder = crate::ContainerBuilder::new();
        builder
            .register(ServiceDefinition::new("n"), |container| {
                container.poison_resolving_for_test();
                Ok(Box::new(1_u8))
            })
            .unwrap();
        let container = builder.compile().unwrap();
        let err = container.get("n").unwrap_err();
        assert!(matches!(
            err,
            DiError::Factory { message, .. } if message == "resolving lock poisoned"
        ));
    }
}

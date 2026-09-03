//! Ordered collection of bundles with dependency sorting.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use crate::{BundleInterface, KernelError};

/// Holds bundles until the kernel compiles them.
///
/// Registration order is preserved when there are no dependencies; otherwise
/// [`Self::sorted`] returns a topologically sorted list (dependencies first).
#[derive(Default)]
pub struct BundleRegistry {
    bundles: Vec<Box<dyn BundleInterface>>,
}

impl BundleRegistry {
    /// Empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a bundle. Names must be unique.
    ///
    /// # Errors
    ///
    /// Returns [`KernelError::DuplicateBundle`] when `bundle.name()` is already present.
    pub fn register(&mut self, bundle: impl BundleInterface + 'static) -> Result<(), KernelError> {
        let name = bundle.name();
        if self.bundles.iter().any(|existing| existing.name() == name) {
            return Err(KernelError::DuplicateBundle(name));
        }
        self.bundles.push(Box::new(bundle));
        Ok(())
    }

    /// Registered names in registration order (before sorting).
    #[must_use]
    pub fn names(&self) -> Vec<&'static str> {
        self.bundles.iter().map(|bundle| bundle.name()).collect()
    }

    /// Number of registered bundles.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bundles.len()
    }

    /// Whether no bundles are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bundles.is_empty()
    }

    /// Returns bundles in dependency order (dependencies before dependents).
    ///
    /// Among ready nodes, lower registration index wins (stable).
    ///
    /// # Errors
    ///
    /// - [`KernelError::UnknownBundleDependency`] when a dependency was never registered.
    /// - [`KernelError::CyclicBundleDependency`] when the graph has a cycle.
    pub fn sorted(self) -> Result<Vec<Box<dyn BundleInterface>>, KernelError> {
        let order = topological_order(&self.bundles)?;
        let mut slots: Vec<Option<Box<dyn BundleInterface>>> =
            self.bundles.into_iter().map(Some).collect();
        let mut sorted = Vec::with_capacity(order.len());
        for index in order {
            let Some(bundle) = slots[index].take() else {
                return Err(KernelError::CyclicBundleDependency("internal"));
            };
            sorted.push(bundle);
        }
        Ok(sorted)
    }
}

fn topological_order(bundles: &[Box<dyn BundleInterface>]) -> Result<Vec<usize>, KernelError> {
    let n = bundles.len();
    let mut index_by_name = HashMap::with_capacity(n);
    for (index, bundle) in bundles.iter().enumerate() {
        index_by_name.insert(bundle.name(), index);
    }

    let mut indegree = vec![0_usize; n];
    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); n];

    for (index, bundle) in bundles.iter().enumerate() {
        for dependency in bundle.dependencies() {
            let Some(&dep_index) = index_by_name.get(dependency) else {
                return Err(KernelError::UnknownBundleDependency {
                    bundle: bundle.name(),
                    dependency,
                });
            };
            adjacency[dep_index].push(index);
            indegree[index] += 1;
        }
    }

    let mut ready: BinaryHeap<Reverse<usize>> = indegree
        .iter()
        .enumerate()
        .filter_map(|(index, degree)| (*degree == 0).then_some(Reverse(index)))
        .collect();

    let mut order = Vec::with_capacity(n);
    while let Some(Reverse(index)) = ready.pop() {
        order.push(index);
        for &dependent in &adjacency[index] {
            indegree[dependent] -= 1;
            if indegree[dependent] == 0 {
                ready.push(Reverse(dependent));
            }
        }
    }

    if order.len() != n {
        let cycle_member = bundles
            .iter()
            .enumerate()
            .find_map(|(index, bundle)| (indegree[index] > 0).then_some(bundle.name()))
            .unwrap_or("unknown");
        return Err(KernelError::CyclicBundleDependency(cycle_member));
    }
    Ok(order)
}

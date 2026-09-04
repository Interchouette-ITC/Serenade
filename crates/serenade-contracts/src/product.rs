//! Product read repository contract.

use crate::{EntityId, PageRequest, RepositoryError};
use std::future::Future;

/// Catalog product reads. Product writes live in the application.
///
/// Adapters in applications implement this trait with `SQLx`, `SeaORM`, or other stores.
/// Serenade never depends on a database crate.
///
/// # Examples
///
/// ```
/// use serenade_contracts::{PageRequest, PersistenceError, ProductRepository};
/// use std::future::Future;
///
/// struct MemoryProducts {
///     rows: Vec<(String, String)>,
/// }
///
/// impl ProductRepository for MemoryProducts {
///     type Error = PersistenceError;
///     type Id = String;
///     type Product = (String, String);
///
///     fn find_by_id(
///         &self,
///         id: &Self::Id,
///     ) -> impl Future<Output = Result<Option<Self::Product>, Self::Error>> + Send {
///         let found = self.rows.iter().find(|(row_id, _)| row_id == id).cloned();
///         async move { Ok(found) }
///     }
///
///     fn find_by_slug(
///         &self,
///         slug: &str,
///     ) -> impl Future<Output = Result<Option<Self::Product>, Self::Error>> + Send {
///         let found = self
///             .rows
///             .iter()
///             .find(|(_, row_slug)| row_slug == slug)
///             .cloned();
///         async move { Ok(found) }
///     }
///
///     fn list(
///         &self,
///         page: PageRequest,
///     ) -> impl Future<Output = Result<Vec<Self::Product>, Self::Error>> + Send {
///         let start = page.offset as usize;
///         let end = start.saturating_add(page.limit as usize);
///         let slice = self
///             .rows
///             .get(start..end.min(self.rows.len()))
///             .unwrap_or(&[])
///             .to_vec();
///         async move { Ok(slice) }
///     }
/// }
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let repo = MemoryProducts {
///     rows: vec![("1".into(), "hoodie".into())],
/// };
/// let page = ProductRepository::list(&repo, PageRequest::first(10))
///     .await
///     .expect("list");
/// assert_eq!(page.len(), 1);
/// # });
/// ```
pub trait ProductRepository: Send + Sync {
    /// Error type for this adapter.
    type Error: RepositoryError;
    /// Product identifier type (for example UUID string in applications).
    type Id: EntityId;
    /// Application-defined product aggregate or view model.
    type Product: Send + Sync;

    /// Load one product by primary key.
    fn find_by_id(
        &self,
        id: &Self::Id,
    ) -> impl Future<Output = Result<Option<Self::Product>, Self::Error>> + Send;

    /// Load one product by URL slug.
    fn find_by_slug(
        &self,
        slug: &str,
    ) -> impl Future<Output = Result<Option<Self::Product>, Self::Error>> + Send;

    /// List products for storefront browse.
    fn list(
        &self,
        page: PageRequest,
    ) -> impl Future<Output = Result<Vec<Self::Product>, Self::Error>> + Send;
}

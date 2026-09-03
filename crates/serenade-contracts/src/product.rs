//! Product read repository contract.

use crate::{EntityId, PageRequest, RepositoryError};
use std::future::Future;

/// Catalog product reads. Writes stay in product/admin slices until needed.
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

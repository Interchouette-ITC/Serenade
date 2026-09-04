//! Category read repository contract.

use crate::{EntityId, PageRequest, RepositoryError};
use std::future::Future;

/// Category tree reads for navigation and catalog filters.
///
/// # Examples
///
/// ```
/// use serenade_contracts::{CategoryRepository, PageRequest, PersistenceError};
/// use std::future::Future;
///
/// struct MemoryCategories;
///
/// impl CategoryRepository for MemoryCategories {
///     type Error = PersistenceError;
///     type Id = String;
///     type Category = String;
///
///     fn find_by_id(
///         &self,
///         _id: &Self::Id,
///     ) -> impl Future<Output = Result<Option<Self::Category>, Self::Error>> + Send {
///         async move { Ok(None) }
///     }
///
///     fn find_by_slug(
///         &self,
///         _slug: &str,
///         _parent_id: Option<&Self::Id>,
///     ) -> impl Future<Output = Result<Option<Self::Category>, Self::Error>> + Send {
///         async move { Ok(None) }
///     }
///
///     fn list_children(
///         &self,
///         _parent_id: Option<&Self::Id>,
///         _page: PageRequest,
///     ) -> impl Future<Output = Result<Vec<Self::Category>, Self::Error>> + Send {
///         async move { Ok(Vec::new()) }
///     }
/// }
/// ```
pub trait CategoryRepository: Send + Sync {
    /// Error type for this adapter.
    type Error: RepositoryError;
    /// Category identifier type.
    type Id: EntityId;
    /// Application-defined category model.
    type Category: Send + Sync;

    /// Load one category by primary key.
    fn find_by_id(
        &self,
        id: &Self::Id,
    ) -> impl Future<Output = Result<Option<Self::Category>, Self::Error>> + Send;

    /// Load one category by slug within an optional parent scope.
    fn find_by_slug(
        &self,
        slug: &str,
        parent_id: Option<&Self::Id>,
    ) -> impl Future<Output = Result<Option<Self::Category>, Self::Error>> + Send;

    /// List child categories for a parent (`None` = roots).
    fn list_children(
        &self,
        parent_id: Option<&Self::Id>,
        page: PageRequest,
    ) -> impl Future<Output = Result<Vec<Self::Category>, Self::Error>> + Send;
}

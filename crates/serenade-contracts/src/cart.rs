//! Cart write repository contract.

use crate::{EntityId, RepositoryError};
use std::future::Future;

/// Cart session persistence. Line mutations snapshot price in the application layer.
///
/// # Examples
///
/// ```
/// use serenade_contracts::{CartRepository, PersistenceError};
/// use std::future::Future;
///
/// struct MemoryCarts {
///     by_token: Vec<(String, String)>,
/// }
///
/// impl CartRepository for MemoryCarts {
///     type Error = PersistenceError;
///     type Id = String;
///     type Cart = String;
///
///     fn find_by_token(
///         &self,
///         token: &str,
///     ) -> impl Future<Output = Result<Option<Self::Cart>, Self::Error>> + Send {
///         let found = self
///             .by_token
///             .iter()
///             .find(|(row_token, _)| row_token == token)
///             .map(|(_, cart)| cart.clone());
///         async move { Ok(found) }
///     }
///
///     fn save(
///         &self,
///         _cart: &Self::Cart,
///     ) -> impl Future<Output = Result<(), Self::Error>> + Send {
///         async move { Ok(()) }
///     }
///
///     fn delete(
///         &self,
///         _id: &Self::Id,
///     ) -> impl Future<Output = Result<(), Self::Error>> + Send {
///         async move { Ok(()) }
///     }
/// }
/// ```
pub trait CartRepository: Send + Sync {
    /// Error type for this adapter.
    type Error: RepositoryError;
    /// Cart identifier type.
    type Id: EntityId;
    /// Application-defined cart aggregate.
    type Cart: Send + Sync;

    /// Resolve a cart by opaque session token.
    fn find_by_token(
        &self,
        token: &str,
    ) -> impl Future<Output = Result<Option<Self::Cart>, Self::Error>> + Send;

    /// Insert or update a cart aggregate.
    fn save(&self, cart: &Self::Cart) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Remove a cart when checkout completes or session expires.
    fn delete(&self, id: &Self::Id) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

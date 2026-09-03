//! Cart write repository contract.

use crate::{EntityId, RepositoryError};
use std::future::Future;

/// Cart session persistence. Line mutations snapshot price in the application layer.
pub trait CartRepository: Send + Sync {
    /// Error type for this adapter.
    type Error: RepositoryError;
    /// Cart identifier type.
    type Id: EntityId;
    /// Application-defined cart aggregate.
    type Cart: Send + Sync;

    /// Resolve a cart by opaque session token (Sylius-style `token_value` lesson).
    fn find_by_token(
        &self,
        token: &str,
    ) -> impl Future<Output = Result<Option<Self::Cart>, Self::Error>> + Send;

    /// Insert or update a cart aggregate.
    fn save(&self, cart: &Self::Cart) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Remove a cart when checkout completes or session expires.
    fn delete(&self, id: &Self::Id) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

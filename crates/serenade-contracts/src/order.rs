//! Order write repository contract.

use crate::{EntityId, RepositoryError};
use std::future::Future;

/// Order persistence for checkout and fulfillment hooks.
pub trait OrderRepository: Send + Sync {
    /// Error type for this adapter.
    type Error: RepositoryError;
    /// Order identifier type.
    type Id: EntityId;
    /// Application-defined order aggregate.
    type Order: Send + Sync;

    /// Load an order by human-readable number.
    fn find_by_number(
        &self,
        number: &str,
    ) -> impl Future<Output = Result<Option<Self::Order>, Self::Error>> + Send;

    /// Insert or update an order aggregate.
    fn save(&self, order: &Self::Order) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Idempotent checkout insert keyed by client idempotency token.
    fn save_idempotent(
        &self,
        order: &Self::Order,
        idempotency_key: &str,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

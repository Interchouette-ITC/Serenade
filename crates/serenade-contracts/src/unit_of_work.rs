//! Transaction boundary without naming `SQLx`, `SeaORM`, or Diesel.

use crate::RepositoryError;
use std::future::Future;

/// Unit of work spanning multiple repository writes in one transaction.
pub trait UnitOfWork: Send {
    /// Error type for this adapter.
    type Error: RepositoryError;

    /// Start a transaction or equivalent isolation scope.
    fn begin(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Commit the active transaction.
    fn commit(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Roll back the active transaction.
    fn rollback(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

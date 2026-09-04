//! Transaction boundary without naming `SQLx`, `SeaORM`, or Diesel.

use crate::RepositoryError;
use std::future::Future;

/// Unit of work spanning multiple repository writes in one transaction.
///
/// # Examples
///
/// ```
/// use serenade_contracts::{PersistenceError, UnitOfWork};
/// use std::future::Future;
///
/// struct MemoryUnitOfWork {
///     open: bool,
/// }
///
/// impl UnitOfWork for MemoryUnitOfWork {
///     type Error = PersistenceError;
///
///     fn begin(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send {
///         self.open = true;
///         async move { Ok(()) }
///     }
///
///     fn commit(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send {
///         self.open = false;
///         async move { Ok(()) }
///     }
///
///     fn rollback(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send {
///         self.open = false;
///         async move { Ok(()) }
///     }
/// }
/// ```
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

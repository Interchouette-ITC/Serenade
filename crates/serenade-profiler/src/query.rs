//! App-facing query collector hook.

use std::sync::Arc;

use crate::data::QueryEvent;
use crate::store::ProfileStore;

/// Records a database (or similar) statement against `token` in `store`.
///
/// Apps call this from repository adapters. Serenade does not embed an ORM.
pub fn record_query(store: &Arc<ProfileStore>, token: &str, event: QueryEvent) {
    store.push_query(token, event);
}

//! App-facing view / template collector.

use std::sync::Arc;
use std::time::Duration;

use crate::data::ViewEvent;
use crate::store::ProfileStore;

/// Records a rendered view (or template) against `token` in `store`.
///
/// Apps call this around HTML / template rendering. Serenade does not embed a
/// template engine; the name is whatever the app uses.
pub fn record_view(
    store: &Arc<ProfileStore>,
    token: &str,
    name: impl Into<String>,
    duration: Duration,
) {
    store.push_view(token, ViewEvent::new(name, duration));
}

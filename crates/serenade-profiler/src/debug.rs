//! App-facing debug dump collector (`DebugBundle` / `VarDumper` analogue).

use std::sync::Arc;

use crate::data::DebugDump;
use crate::store::ProfileStore;

/// Records a labeled debug value against `token` in `store`.
///
/// Apps call this from controllers when a dump belongs on the Debug panel.
/// Values are stored as plain text; there is no interactive dump UI.
pub fn record_debug(
    store: &Arc<ProfileStore>,
    token: &str,
    label: impl Into<String>,
    value: impl Into<String>,
) {
    store.push_debug(
        token,
        DebugDump {
            label: label.into(),
            value: value.into(),
        },
    );
}

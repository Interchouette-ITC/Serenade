//! Request-scoped log capture for the profiler.

use std::cell::RefCell;
use std::fmt::Write as _;
use std::sync::Arc;

use tracing::{Event, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

use crate::data::LogLine;
use crate::store::ProfileStore;

thread_local! {
    static SCOPE: RefCell<Option<(Arc<ProfileStore>, String)>> = const { RefCell::new(None) };
}

/// Runs `f` while log events are attributed to `token` in `store`.
pub fn with_profile_scope<R>(store: &Arc<ProfileStore>, token: String, f: impl FnOnce() -> R) -> R {
    SCOPE.with(|cell| {
        *cell.borrow_mut() = Some((Arc::clone(store), token));
    });
    let result = f();
    SCOPE.with(|cell| {
        *cell.borrow_mut() = None;
    });
    result
}

/// Installs the current profile scope (for async middleware around `await`).
pub fn install_log_scope(store: Arc<ProfileStore>, token: String) -> ProfileScopeGuard {
    SCOPE.with(|cell| {
        *cell.borrow_mut() = Some((store, token));
    });
    ProfileScopeGuard
}

/// Clears the thread-local profile scope when dropped.
#[derive(Debug)]
pub struct ProfileScopeGuard;

impl Drop for ProfileScopeGuard {
    fn drop(&mut self) {
        SCOPE.with(|cell| {
            *cell.borrow_mut() = None;
        });
    }
}

/// `tracing` layer that appends events to the active profile.
#[derive(Clone, Debug, Default)]
pub struct ProfilerLogLayer;

impl<S> Layer<S> for ProfilerLogLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        SCOPE.with(|cell| {
            let Some((store, token)) = cell
                .borrow()
                .as_ref()
                .map(|(s, t)| (Arc::clone(s), t.clone()))
            else {
                return;
            };
            let mut visitor = MessageVisitor::default();
            event.record(&mut visitor);
            let meta = event.metadata();
            store.push_log(
                &token,
                LogLine {
                    target: meta.target().to_owned(),
                    level: meta.level().to_string(),
                    message: visitor.message,
                },
            );
        });
    }
}

#[derive(Default)]
struct MessageVisitor {
    message: String,
}

fn apply_debug_field(message: &mut String, name: &str, formatted: String) {
    if name == "message" {
        *message = if formatted.starts_with('"') && formatted.ends_with('"') && formatted.len() >= 2
        {
            formatted[1..formatted.len() - 1].to_owned()
        } else {
            formatted
        };
    } else if message.is_empty() {
        let _ = write!(message, "{name}={formatted}");
    } else {
        let _ = write!(message, " {name}={formatted}");
    }
}

fn apply_str_field(message: &mut String, name: &str, value: &str) {
    if name == "message" {
        value.clone_into(message);
    } else if message.is_empty() {
        let _ = write!(message, "{name}={value}");
    } else {
        let _ = write!(message, " {name}={value}");
    }
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        apply_debug_field(&mut self.message, field.name(), format!("{value:?}"));
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        apply_str_field(&mut self.message, field.name(), value);
    }
}

#[cfg(test)]
mod logs_tests {
    use std::sync::Arc;

    use tracing_subscriber::prelude::*;

    use super::{
        ProfilerLogLayer, apply_debug_field, apply_str_field, install_log_scope, with_profile_scope,
    };
    use crate::ProfileStore;

    #[test]
    fn layer_ignores_events_without_scope() {
        let store = Arc::new(ProfileStore::new(2));
        store.ensure(crate::ProfileData::new(
            "tok".into(),
            "GET".into(),
            "/".into(),
        ));
        let _subscriber = tracing_subscriber::registry()
            .with(ProfilerLogLayer)
            .set_default();
        tracing::info!(target: "serenade::request", "outside scope");
        assert_eq!(store.get("tok").expect("tok").logs, []);
    }

    #[test]
    fn install_log_scope_clears_on_drop() {
        let store = Arc::new(ProfileStore::new(2));
        store.ensure(crate::ProfileData::new(
            "tok".into(),
            "GET".into(),
            "/".into(),
        ));
        let _subscriber = tracing_subscriber::registry()
            .with(ProfilerLogLayer)
            .set_default();
        {
            let _guard = install_log_scope(Arc::clone(&store), "tok".into());
            tracing::info!(target: "serenade::request", "inside");
        }
        tracing::info!(target: "serenade::request", "after drop");
        let logs = store.get("tok").expect("tok").logs;
        assert_eq!(logs.len(), 1);
        assert!(logs[0].message.contains("inside"));
    }

    #[test]
    fn apply_field_helpers_cover_branches() {
        let mut message = String::new();
        apply_debug_field(&mut message, "message", "\"quoted\"".into());
        assert_eq!(message, "quoted");
        apply_debug_field(&mut message, "message", "bare".into());
        assert_eq!(message, "bare");
        message.clear();
        apply_debug_field(&mut message, "k", "1".into());
        assert_eq!(message, "k=1");
        apply_debug_field(&mut message, "m", "2".into());
        assert_eq!(message, "k=1 m=2");

        message.clear();
        apply_str_field(&mut message, "message", "hi");
        assert_eq!(message, "hi");
        message.clear();
        apply_str_field(&mut message, "a", "x");
        assert_eq!(message, "a=x");
        apply_str_field(&mut message, "b", "y");
        assert_eq!(message, "a=x b=y");
    }

    #[test]
    fn with_scope_captures_structured_fields() {
        let store = Arc::new(ProfileStore::new(2));
        store.ensure(crate::ProfileData::new(
            "tok".into(),
            "GET".into(),
            "/".into(),
        ));
        let _subscriber = tracing_subscriber::registry()
            .with(ProfilerLogLayer)
            .set_default();
        with_profile_scope(&store, "tok".into(), || {
            tracing::info!(target: "t", key = "a", more = "b");
            tracing::info!(target: "t", message = "quoted");
        });
        let logs = store.get("tok").expect("tok").logs;
        assert!(logs.iter().any(|l| l.message.contains("key=a")));
        assert!(logs.iter().any(|l| l.message.contains("quoted")));
    }
}

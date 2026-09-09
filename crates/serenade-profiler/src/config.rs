//! Profiler enablement and capacity.

/// Runtime configuration for the web profiler.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfilerConfig {
    /// When `false`, middleware is a no-op and toolbar is not injected.
    pub enabled: bool,
    /// Maximum retained profiles in the in-memory store.
    pub capacity: usize,
    /// URL path prefix for the UI (`/_profiler` by default).
    pub path_prefix: String,
}

impl Default for ProfilerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            capacity: 50,
            path_prefix: crate::PROFILER_PATH_PREFIX.to_owned(),
        }
    }
}

impl ProfilerConfig {
    /// Disabled profiler (production default).
    #[must_use]
    pub fn disabled() -> Self {
        Self::default()
    }

    /// Enabled profiler with `capacity` retained profiles.
    #[must_use]
    pub fn enabled(capacity: usize) -> Self {
        Self {
            enabled: true,
            capacity: capacity.max(1),
            path_prefix: crate::PROFILER_PATH_PREFIX.to_owned(),
        }
    }

    /// Overrides the UI path prefix.
    #[must_use]
    pub fn with_path_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.path_prefix = prefix.into();
        self
    }
}

//! Wasmtime Engine / Store / Linker helpers and component load.

use std::path::Path;

use thiserror::Error;
use wasmtime::component::{Component, Linker};
use wasmtime::{Engine, Store};

/// Errors from Component Model host helpers.
#[derive(Debug, Error)]
pub enum ComponentHostError {
    /// Failed to load a component from path or bytes.
    #[error("load component from {path}: {source}")]
    LoadPath {
        /// Path that failed to load.
        path: String,
        /// Underlying wasmtime / IO error.
        #[source]
        source: anyhow::Error,
    },
    /// Failed to load a component from in-memory bytes.
    #[error("load component from bytes: {0}")]
    LoadBytes(#[source] anyhow::Error),
}

/// Builds a default wasmtime [`Engine`] with Component Model support.
#[must_use]
pub fn default_engine() -> Engine {
    Engine::default()
}

/// Loads a WebAssembly component from `path`.
///
/// # Errors
///
/// Returns an error when the file cannot be read or is not a valid component.
pub fn load_component(
    engine: &Engine,
    path: impl AsRef<Path>,
) -> Result<Component, ComponentHostError> {
    let path = path.as_ref();
    Component::from_file(engine, path).map_err(|source| ComponentHostError::LoadPath {
        path: path.display().to_string(),
        source,
    })
}

/// Loads a WebAssembly component from in-memory bytes (or WAT text).
///
/// # Errors
///
/// Returns an error when the bytes are not a valid component.
pub fn load_component_bytes(
    engine: &Engine,
    bytes: impl AsRef<[u8]>,
) -> Result<Component, ComponentHostError> {
    Component::new(engine, bytes.as_ref()).map_err(ComponentHostError::LoadBytes)
}

/// Creates a [`Linker`] with **no** host imports (deny-by-default).
///
/// Guests that import host functions fail at instantiate time unless the
/// product adds imports explicitly.
#[must_use]
pub fn empty_linker<T>(engine: &Engine) -> Linker<T> {
    Linker::new(engine)
}

/// Creates a [`Store`] with host state `data`.
#[must_use]
pub fn store_with_data<T>(engine: &Engine, data: T) -> Store<T> {
    Store::new(engine, data)
}

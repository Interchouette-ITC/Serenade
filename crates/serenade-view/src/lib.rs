//! HTML view helpers: reverse routing, assets, partials, and document composition.
//!
//! Escape stays in [`serenade_form`]; this crate re-exports it for one import surface.
//! Document composition is engine-neutral; template engines can integrate without
//! becoming mandatory dependencies of this crate. The optional `leptos-ssr` feature
//! renders complete documents under a request-scoped Leptos owner and injects managed
//! metadata. The `leptos-client` feature exposes the head root and lifecycle components
//! used by CSR/hydrated apps. Compatible AOT compilers can target these components
//! without becoming dependencies of this crate.

mod asset;
mod document;
mod error;
#[cfg(any(feature = "leptos-client", feature = "leptos-ssr"))]
mod head;
mod partial;
mod path;
#[cfg(not(target_arch = "wasm32"))]
mod static_output;
mod trunk_assets;

#[cfg(feature = "leptos-ssr")]
pub use any_spawner::Executor as LeptosExecutor;
pub use asset::{AssetConfig, asset};
#[cfg(feature = "leptos-ssr")]
pub use document::initialize_leptos_ssr;
pub use document::{Document, DocumentParts, DocumentTemplate, RenderedDocument};
pub use error::ViewError;
#[cfg(any(feature = "leptos-client", feature = "leptos-ssr"))]
pub use head::{JsonLd, ManagedHead, ManagedLink, ManagedMeta, ManagedTitle};
pub use partial::partial;
pub use path::path;
pub use serenade_form::{escape_attr, escape_html};
#[cfg(not(target_arch = "wasm32"))]
pub use static_output::{STATIC_OUTPUT_DIR_ENV, StaticDocumentWriter};
pub use trunk_assets::{
    TRUNK_ASSET_MANIFEST_ENV, TRUNK_ASSET_MANIFEST_FILE, TRUNK_DIST_DIRECTORY_ENV,
    TrunkAssetManifest, TrunkAssetReference, TrunkPreloadReference, TrunkWasmBundle,
};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;

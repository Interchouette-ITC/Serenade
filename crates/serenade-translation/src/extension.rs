//! `TranslationBundle` and DI extension.

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::Value;
use serenade_bundle::{BundleError, Extension};
use serenade_config::Config;
use serenade_di::{ContainerBuilder, ServiceDefinition};
use serenade_kernel::{BundleInterface, KernelError};

use crate::{Locale, TranslationError, Translator};

/// Canonical bundle / package name.
pub const TRANSLATION_BUNDLE: &str = "translation";

/// Container service id for [`Translator`] (`Arc<Translator>`).
pub const TRANSLATOR_SERVICE: &str = "translator";

/// Core translation bundle (lifecycle no-op; DI lives in [`TranslationExtension`]).
#[derive(Clone, Copy, Debug, Default)]
pub struct TranslationBundle;

impl BundleInterface for TranslationBundle {
    fn name(&self) -> &'static str {
        TRANSLATION_BUNDLE
    }

    fn build(&self) -> Result<(), KernelError> {
        Ok(())
    }
}

/// DI extension for the `translation` package key.
///
/// Example `config/packages/translation.toml`:
///
/// ```toml
/// default_locale = "en"
/// fallbacks = ["en"]
/// paths = ["translations"]
/// ```
///
/// Registers [`TRANSLATOR_SERVICE`] as an `Arc<Translator>`.
#[derive(Clone, Debug, Default)]
pub struct TranslationExtension {
    /// Extra catalogue directories merged after config `paths`.
    extra_paths: Vec<PathBuf>,
}

impl TranslationExtension {
    /// Extension with no extra paths.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds catalogue directories after those listed in config.
    #[must_use]
    pub fn with_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.extra_paths = paths;
        self
    }
}

impl Extension for TranslationExtension {
    fn alias(&self) -> &'static str {
        TRANSLATION_BUNDLE
    }

    fn load(&self, config: &Config, builder: &mut ContainerBuilder) -> Result<(), BundleError> {
        config.apply_to(builder.parameters_mut());

        let root = config.value();
        let default_locale = string_field(root, "default_locale").unwrap_or_else(|| "en".into());
        let locale = Locale::new(&default_locale).map_err(|err| map_err(&err))?;

        let fallback_locales = string_list(root, "fallbacks")
            .into_iter()
            .filter_map(|tag| Locale::new(tag).ok())
            .collect::<Vec<_>>();

        let mut paths = string_list(root, "paths")
            .into_iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        if paths.is_empty() {
            paths.push(PathBuf::from("translations"));
        }
        paths.extend(self.extra_paths.clone());

        let mut translator = Translator::new(locale).with_fallbacks(fallback_locales);
        for path in &paths {
            if path.is_dir() {
                translator.load_path(path).map_err(|err| map_err(&err))?;
            }
        }

        let translator = Arc::new(translator);
        builder.register(ServiceDefinition::new(TRANSLATOR_SERVICE), move |_| {
            Ok(Box::new(Arc::clone(&translator)))
        })?;
        Ok(())
    }
}

fn map_err(err: &TranslationError) -> BundleError {
    BundleError::Extension {
        alias: TRANSLATION_BUNDLE,
        message: err.to_string(),
    }
}

fn string_field(root: &Value, key: &str) -> Option<String> {
    root.get(key).and_then(Value::as_str).map(ToOwned::to_owned)
}

fn string_list(root: &Value, key: &str) -> Vec<String> {
    match root.get(key) {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| item.as_str().map(ToOwned::to_owned))
            .collect(),
        Some(Value::String(text)) => text
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

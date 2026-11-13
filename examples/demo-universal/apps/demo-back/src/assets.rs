use serenade_view::TrunkAssetManifest;

#[derive(Clone, Debug)]
pub struct CmsBootstrap {
    pub script_url: String,
    pub js_url: String,
    pub wasm_url: String,
    pub init_with_object: bool,
    pub import_bindings: bool,
    pub import_bindings_name: String,
    pub integrity: Option<String>,
    pub crossorigin: Option<String>,
}

impl CmsBootstrap {
    pub fn from_manifest(manifest: &TrunkAssetManifest) -> Result<Self, std::io::Error> {
        let mut main_bundles = manifest
            .wasm
            .iter()
            .filter(|bundle| bundle.app_type == "main");
        let bundle = main_bundles
            .next()
            .ok_or_else(|| invalid_manifest("Trunk manifest has no main WASM bundle"))?;
        if main_bundles.next().is_some() {
            return Err(invalid_manifest(
                "Trunk manifest has multiple main WASM bundles",
            ));
        }
        if bundle.compression_algorithm.is_some() || bundle.custom_script_pattern.is_some() {
            return Err(invalid_manifest(
                "Universal CMS bootstrap does not support compressed WASM or custom script patterns",
            ));
        }
        let mut bootstrap_assets = manifest.assets.iter().filter(|asset| {
            asset.role == "script"
                && asset
                    .attributes
                    .get("data-serenade-universal-bootstrap")
                    .map(String::as_str)
                    == Some("true")
        });
        let asset = bootstrap_assets
            .next()
            .ok_or_else(|| invalid_manifest("Trunk manifest has no Universal bootstrap script"))?;
        if bootstrap_assets.next().is_some() {
            return Err(invalid_manifest(
                "Trunk manifest has multiple Universal bootstrap scripts",
            ));
        }
        if asset.attributes.get("type").map(String::as_str) != Some("module") {
            return Err(invalid_manifest(
                "Universal bootstrap script must be emitted as an ES module",
            ));
        }

        Ok(Self {
            script_url: asset.url.clone(),
            js_url: bundle
                .initializer_url
                .as_ref()
                .unwrap_or(&bundle.js_url)
                .clone(),
            wasm_url: bundle.wasm_url.clone(),
            init_with_object: bundle.init_with_object,
            import_bindings: bundle.import_bindings,
            import_bindings_name: bundle
                .import_bindings_name
                .clone()
                .or_else(|| bundle.import_bindings.then(|| "wasmBindings".to_owned()))
                .unwrap_or_default(),
            integrity: asset.attributes.get("integrity").cloned(),
            crossorigin: asset.attributes.get("crossorigin").cloned(),
        })
    }
}

fn invalid_manifest(message: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serenade_view::{TrunkAssetReference, TrunkWasmBundle};

    use super::*;

    fn manifest() -> TrunkAssetManifest {
        TrunkAssetManifest {
            version: 1,
            public_url: "/".to_owned(),
            html: "index.html".to_owned(),
            assets: vec![TrunkAssetReference {
                order: 0,
                role: "script".to_owned(),
                url: "/universal-bootstrap-9e728797de5d0532.js".to_owned(),
                content_digest: None,
                attributes: BTreeMap::from([
                    (
                        "data-serenade-universal-bootstrap".to_owned(),
                        "true".to_owned(),
                    ),
                    ("type".to_owned(), "module".to_owned()),
                ]),
            }],
            wasm: vec![TrunkWasmBundle {
                app_type: "main".to_owned(),
                order: None,
                js_url: "/serenade-demo-front-207ccdec1d7c64a5.js".to_owned(),
                js_content_digest: None,
                wasm_url: "/serenade-demo-front-7be888fcafbfe0c3_bg.wasm".to_owned(),
                wasm_content_digest: None,
                wasm_size: 1,
                initializer_url: None,
                initializer_content_digest: None,
                init_with_object: true,
                import_bindings: true,
                import_bindings_name: None,
                compression_algorithm: None,
                cross_origin: "anonymous".to_owned(),
                inject_scripts: true,
                custom_script_pattern: None,
                custom_preload_pattern: None,
                pattern_parameters: BTreeMap::new(),
                preloads: Vec::new(),
            }],
        }
    }

    #[test]
    fn bootstrap_uses_exact_hashed_manifest_urls_and_trunk_default_binding_name() {
        let bootstrap = CmsBootstrap::from_manifest(&manifest()).expect("valid bootstrap");

        assert_eq!(
            bootstrap.script_url,
            "/universal-bootstrap-9e728797de5d0532.js"
        );
        assert_eq!(bootstrap.js_url, "/serenade-demo-front-207ccdec1d7c64a5.js");
        assert_eq!(
            bootstrap.wasm_url,
            "/serenade-demo-front-7be888fcafbfe0c3_bg.wasm"
        );
        assert!(bootstrap.init_with_object);
        assert!(bootstrap.import_bindings);
        assert_eq!(bootstrap.import_bindings_name, "wasmBindings");
    }

    #[test]
    fn bootstrap_preserves_an_explicit_trunk_bindings_name() {
        let mut manifest = manifest();
        manifest.wasm[0].import_bindings_name = Some("customBindings".to_owned());

        let bootstrap = CmsBootstrap::from_manifest(&manifest).expect("valid bootstrap");

        assert_eq!(bootstrap.import_bindings_name, "customBindings");
    }

    #[test]
    fn bootstrap_rejects_an_ambiguous_or_incomplete_manifest() {
        let mut missing_main = manifest();
        missing_main.wasm.clear();
        assert!(CmsBootstrap::from_manifest(&missing_main).is_err());

        let mut missing_script = manifest();
        missing_script.assets.clear();
        assert!(CmsBootstrap::from_manifest(&missing_script).is_err());

        let mut duplicate_script = manifest();
        duplicate_script
            .assets
            .push(duplicate_script.assets[0].clone());
        assert!(CmsBootstrap::from_manifest(&duplicate_script).is_err());
    }

    #[test]
    fn bootstrap_requires_the_exact_true_marker_and_module_type() {
        for value in [
            None,
            Some(""),
            Some("false"),
            Some("TRUE"),
            Some("unrelated"),
        ] {
            let mut manifest = manifest();
            let attributes = &mut manifest.assets[0].attributes;
            attributes.remove("data-serenade-universal-bootstrap");
            if let Some(value) = value {
                attributes.insert(
                    "data-serenade-universal-bootstrap".to_owned(),
                    value.to_owned(),
                );
            }
            assert!(CmsBootstrap::from_manifest(&manifest).is_err(), "{value:?}");
        }
        let mut manifest = manifest();
        manifest.assets[0].attributes.remove("type");
        assert!(CmsBootstrap::from_manifest(&manifest).is_err());
    }
}

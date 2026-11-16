use serenade_view::{DocumentAssets, TrunkAssetManifest};

/// Everything `/cms/hello` needs from the Trunk build: the public URL that frontend files
/// are served under and the exact markup Trunk generated for styles, preloads, and the
/// WASM loader.
#[derive(Clone, Debug)]
pub struct CmsAssets {
    pub public_url: String,
    pub document: DocumentAssets,
}

impl CmsAssets {
    pub fn from_manifest(manifest: &TrunkAssetManifest) -> Result<Self, std::io::Error> {
        let [bundle] = manifest.wasm.as_slice() else {
            return Err(invalid_manifest(
                "the Universal demo needs exactly one Trunk WASM bundle to hydrate",
            ));
        };
        if bundle.app_type != "main" || !bundle.inject_scripts {
            return Err(invalid_manifest(
                "the Universal demo requires one main Trunk WASM bundle with script injection for hydration",
            ));
        }
        Ok(Self {
            public_url: manifest.public_url.clone(),
            document: manifest
                .document_assets()
                .map_err(|error| invalid_manifest(&error.to_string()))?,
        })
    }
}

fn invalid_manifest(message: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serenade_view::{TrunkDocumentAsset, TrunkWasmBundle};

    use super::*;

    const STYLESHEET: &str = r#"<link rel="stylesheet" href="/style-1a.css"/>"#;
    const LOADER: &str = "<script type=\"module\">import init from '/serenade-demo-front-2b.js';await init({ module_or_path: '/serenade-demo-front-3c_bg.wasm' });</script>";

    fn manifest() -> TrunkAssetManifest {
        TrunkAssetManifest {
            public_url: "/".to_owned(),
            html: "index.html".to_owned(),
            assets: Vec::new(),
            wasm: vec![TrunkWasmBundle {
                app_type: "main".to_owned(),
                order: None,
                js_url: "/serenade-demo-front-2b.js".to_owned(),
                js_content_digest: None,
                wasm_url: "/serenade-demo-front-3c_bg.wasm".to_owned(),
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
            document_assets: vec![
                TrunkDocumentAsset {
                    order: Some(0),
                    location: "head".to_owned(),
                    html: STYLESHEET.to_owned(),
                },
                TrunkDocumentAsset {
                    order: None,
                    location: "head".to_owned(),
                    html: LOADER.to_owned(),
                },
            ],
        }
    }

    #[test]
    fn assets_carry_the_exact_trunk_markup_and_public_url() {
        let assets = CmsAssets::from_manifest(&manifest()).expect("valid assets");

        assert_eq!(assets.public_url, "/");
        assert_eq!(assets.document.head_html(), format!("{STYLESHEET}{LOADER}"));
        assert_eq!(assets.document.body_html(), "");
    }

    #[test]
    fn assets_reject_bundles_that_cannot_hydrate() {
        let mut no_bundle = manifest();
        no_bundle.wasm.clear();
        let mut two_bundles = manifest();
        two_bundles.wasm.push(two_bundles.wasm[0].clone());
        let mut no_injection = manifest();
        no_injection.wasm[0].inject_scripts = false;
        let mut worker = manifest();
        worker.wasm[0].app_type = "worker".to_owned();
        let mut misplaced = manifest();
        misplaced.document_assets[0].location = "unknown".to_owned();

        for manifest in [no_bundle, two_bundles, no_injection, worker, misplaced] {
            let error = CmsAssets::from_manifest(&manifest).expect_err("unusable manifest");
            assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        }
    }
}

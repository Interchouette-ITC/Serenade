use std::path::{Path, PathBuf};
use std::sync::Once;

use anyhow::{Context, bail};
use leptos::prelude::*;
use serenade_demo_ui::HelloUi;
use serenade_view::{
    Document, DocumentAssets, ManagedLink, ManagedMeta, ManagedTitle, RenderedDocument,
    STATIC_OUTPUT_DIR_ENV, StaticDocumentWriter, TrunkAssetManifest, initialize_leptos_ssr,
};

/// A prerendered route with its own head metadata.
struct Route {
    path: &'static str,
    page: &'static str,
    title: &'static str,
    description: &'static str,
}

const ROUTES: [Route; 2] = [
    Route {
        path: "/",
        page: "home",
        title: "Serenade SSG demo",
        description: "A shared hello UI component prerendered by Serenade's SSG mode.",
    },
    Route {
        path: "/about",
        page: "about",
        title: "About Serenade",
        description: "Serenade builds web apps with shared, framework-rendered UI.",
    },
];

/// The Trunk build's public URL and the exact markup Trunk generated for it.
struct Site {
    public_url: String,
    assets: DocumentAssets,
}

impl Site {
    fn from_manifest(manifest: &TrunkAssetManifest) -> anyhow::Result<Self> {
        let [bundle] = manifest.wasm.as_slice() else {
            bail!("the SSG demo needs exactly one Trunk WASM bundle to hydrate");
        };
        if !bundle.inject_scripts {
            bail!("the SSG demo requires Trunk WASM script injection for hydration");
        }
        Ok(Self {
            public_url: format!("{}/", manifest.public_url.trim_end_matches('/')),
            assets: manifest.document_assets()?,
        })
    }

    fn canonical(&self, route: &str) -> String {
        match route.trim_matches('/') {
            "" => self.public_url.clone(),
            path => format!("{}{path}/", self.public_url),
        }
    }
}

fn render_route(route: &'static Route, site: &Site) -> anyhow::Result<RenderedDocument> {
    let canonical = site.canonical(route.path);
    let favicon = format!("{}favicon.ico", site.public_url);
    let document = Document::new(
        move || {
            view! {
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <link rel="icon" type="image/x-icon" href=favicon />
                <ManagedTitle text=route.title />
                <ManagedMeta
                    head_key="meta:name:description"
                    name="description"
                    content=route.description
                />
                <ManagedLink head_key="link:canonical" rel="canonical" href=canonical />
            }
        },
        move || view! { <HelloUi page=route.page /> },
    )
    .with_assets(&site.assets);
    Ok(document.render()?)
}

fn initialize() -> anyhow::Result<()> {
    static INITIALIZE: Once = Once::new();
    let mut result = Ok(());
    INITIALIZE.call_once(|| result = initialize_leptos_ssr());
    Ok(result?)
}

fn main() -> anyhow::Result<()> {
    let output = std::env::var_os(STATIC_OUTPUT_DIR_ENV)
        .filter(|directory| !directory.is_empty())
        .map(PathBuf::from)
        .with_context(|| format!("{STATIC_OUTPUT_DIR_ENV} is not set; run through serenade-ssg"))?;
    let manifest = TrunkAssetManifest::read_from_env()?;
    prerender(&output, &manifest)?;
    println!("Prerendered / and /about/ to {}", output.display());
    Ok(())
}

fn prerender(output: &Path, manifest: &TrunkAssetManifest) -> anyhow::Result<()> {
    manifest.validate_output(output).with_context(|| {
        format!(
            "Trunk output {} does not match its manifest",
            output.display()
        )
    })?;
    let site = Site::from_manifest(manifest)?;
    initialize()?;
    let writer = StaticDocumentWriter::new(output);
    for route in &ROUTES {
        writer.write(route.path, render_route(route, &site)?)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serenade_view::TrunkDocumentAsset;

    use super::*;

    fn output_with_build(public_url: &str, name: &str) -> (PathBuf, TrunkAssetManifest) {
        let empty_digest = "sha256-47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=";
        let output = std::env::temp_dir().join(format!(
            "serenade-demo-ssg-prerender-{}-{name}",
            std::process::id()
        ));
        fs::create_dir_all(&output).expect("output directory");
        for file in ["index.html", "style-1a.css", "app-2b.js", "app-3c_bg.wasm"] {
            fs::write(output.join(file), "").expect("build file");
        }
        let manifest = TrunkAssetManifest::from_json(
            format!(
                r#"{{"public_url":"{public_url}","html":"index.html",
                "assets":[{{"order":0,"role":"stylesheet","url":"{public_url}style-1a.css","content_digest":"{empty_digest}","attributes":{{}}}}],
                "wasm":[{{"app_type":"main","order":1,"js_url":"{public_url}app-2b.js","js_content_digest":"{empty_digest}",
                "wasm_url":"{public_url}app-3c_bg.wasm","wasm_content_digest":"{empty_digest}",
                "wasm_size":0,"init_with_object":true,"import_bindings":true,"cross_origin":"anonymous","inject_scripts":true,"pattern_parameters":{{}},
                "preloads":[{{"rel":"modulepreload","url":"{public_url}app-2b.js","content_digest":"{empty_digest}","cross_origin":"anonymous"}}]}}],
                "document_assets":[]}}"#
            )
            .as_bytes(),
        )
        .unwrap_or_else(|error| panic!("valid manifest: {error}"));
        let mut manifest = manifest;
        manifest.document_assets = trunk_markup(public_url)
            .into_iter()
            .map(|(location, html)| TrunkDocumentAsset {
                order: None,
                location: location.to_owned(),
                html,
            })
            .collect();
        (output, manifest)
    }

    /// Markup in the shape Trunk records for the demo's `index.html`.
    fn trunk_markup(public_url: &str) -> [(&'static str, String); 4] {
        [
            (
                "head",
                format!(r#"<link rel="stylesheet" href="{public_url}style-1a.css"/>"#),
            ),
            (
                "head",
                format!(
                    r#"<link rel="modulepreload" href="{public_url}app-2b.js" crossorigin="anonymous">"#
                ),
            ),
            (
                "head",
                format!(
                    "<script type=\"module\">import init, * as bindings from '{public_url}app-2b.js';\
                     const wasm = await init({{ module_or_path: '{public_url}app-3c_bg.wasm' }});\
                     window.wasmBindings = bindings;\
                     dispatchEvent(new CustomEvent(\"TrunkApplicationStarted\", {{detail: {{wasm}}}}));</script>"
                ),
            ),
            ("body", "<svg data-trunk-inline></svg>".to_owned()),
        ]
    }

    fn head(page: &str) -> &str {
        page.split_once("<head>")
            .expect("document head")
            .1
            .split_once("</head>")
            .expect("document head end")
            .0
    }

    #[test]
    fn prerenders_each_route_with_its_own_head_and_manifest_assets() {
        let (output, manifest) = output_with_build("/app/", "routes");
        prerender(&output, &manifest).expect("prerender");
        let home = fs::read_to_string(output.join("index.html")).expect("home route");
        let about = fs::read_to_string(output.join("about/index.html")).expect("about route");

        for (page, page_name, title, description, canonical, heading) in [
            (
                &home,
                "home",
                "Serenade SSG demo",
                "prerendered by Serenade",
                "/app/",
                "Hello from Serenade",
            ),
            (
                &about,
                "about",
                "About Serenade",
                "framework-rendered UI",
                "/app/about/",
                "About Serenade",
            ),
        ] {
            assert!(page.to_ascii_lowercase().starts_with("<!doctype html>"));
            assert_eq!(page.matches("<html ").count(), 1, "{page}");
            assert_eq!(page.matches("<head>").count(), 1, "{page}");
            assert_eq!(page.matches("<body>").count(), 1, "{page}");
            let head = head(page);
            assert!(head.contains("<meta charset=\"utf-8\">"), "{head}");
            assert!(head.contains("name=\"viewport\""), "{head}");
            assert_eq!(head.matches("<title>").count(), 1, "{head}");
            assert!(head.contains(&format!("<title>{title}</title>")), "{head}");
            assert_eq!(head.matches("name=\"description\"").count(), 1, "{head}");
            assert!(head.contains(description), "{head}");
            assert!(
                head.contains(&format!("href=\"{canonical}\" rel=\"canonical\""))
                    || head.contains(&format!("rel=\"canonical\" href=\"{canonical}\"")),
                "{head}"
            );
            assert!(head.contains("href=\"/app/favicon.ico\""), "{head}");
            let markup = trunk_markup("/app/");
            let head_assets: String = markup
                .iter()
                .filter(|(location, _)| *location == "head")
                .map(|(_, html)| html.as_str())
                .collect();
            assert!(
                head.ends_with(&head_assets),
                "Trunk head markup must be inserted verbatim, in order, before </head>: {head}"
            );
            assert!(page.contains(&format!("<h1>{heading}</h1>")), "{page}");
            let body = page
                .split_once("<body>")
                .expect("document body")
                .1
                .split_once("</body>")
                .expect("document body end")
                .0;
            assert_eq!(
                body,
                format!(
                    "{}{}",
                    serenade_demo_ssg::render_page(page_name),
                    markup[3].1
                )
            );
            assert!(!page.contains("SERENADE_SSG_HEAD"), "{page}");
            assert!(!page.contains("SERENADE_SSG_CONTENT"), "{page}");
        }
        fs::remove_dir_all(output).expect("cleanup");
    }

    #[test]
    fn rejects_missing_output_and_bundles_that_cannot_hydrate() {
        let (output, manifest) = output_with_build("/", "rejects");
        assert!(prerender(&output.join("missing"), &manifest).is_err());

        let mut no_injection = manifest.clone();
        no_injection.wasm[0].inject_scripts = false;
        assert!(
            prerender(&output, &no_injection)
                .expect_err("hydration needs the loader")
                .to_string()
                .contains("requires Trunk WASM script injection")
        );

        let mut no_bundle = manifest.clone();
        no_bundle.wasm.clear();
        assert!(prerender(&output, &no_bundle).is_err());

        let mut misplaced = manifest;
        misplaced.document_assets[0].location = "unknown".to_owned();
        assert!(
            prerender(&output, &misplaced)
                .expect_err("markup outside head and body")
                .to_string()
                .contains("outside <head> and <body>")
        );
        fs::remove_dir_all(output).expect("cleanup");
    }
}

use std::path::{Path, PathBuf};
use std::sync::Once;

use anyhow::{Context, bail};
use leptos::prelude::*;
use serenade_demo_ui::HelloUi;
use serenade_view::{
    Document, ManagedLink, ManagedMeta, ManagedTitle, RenderedDocument, STATIC_OUTPUT_DIR_ENV,
    StaticDocumentWriter, TrunkAssetManifest, initialize_leptos_ssr,
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

const TRUNK_STARTED_EVENT: &str = "TrunkApplicationStarted";

#[derive(Clone)]
struct Tag {
    url: String,
    type_: Option<String>,
    integrity: Option<String>,
    crossorigin: Option<String>,
}

#[derive(Clone)]
struct Preload {
    rel: String,
    url: String,
    as_: Option<String>,
    type_: Option<String>,
    integrity: Option<String>,
    crossorigin: String,
}

/// Head tags for the frontend bundle, taken from Trunk's validated asset manifest
/// rather than from Trunk's generated HTML.
#[derive(Clone)]
struct PageAssets {
    public_url: String,
    stylesheets: Vec<Tag>,
    scripts: Vec<Tag>,
    icons: Vec<Tag>,
    preloads: Vec<Preload>,
    loader: Option<String>,
}

impl PageAssets {
    fn from_manifest(manifest: &TrunkAssetManifest) -> anyhow::Result<Self> {
        let mut assets = Self {
            public_url: format!("{}/", manifest.public_url.trim_end_matches('/')),
            stylesheets: Vec::new(),
            scripts: Vec::new(),
            icons: Vec::new(),
            preloads: Vec::new(),
            loader: None,
        };
        for asset in &manifest.assets {
            if let Some(name) = asset
                .attributes
                .keys()
                .find(|name| !matches!(name.as_str(), "type" | "integrity" | "crossorigin"))
            {
                bail!(
                    "unsupported attribute `{name}` on Trunk {} asset `{}`",
                    asset.role,
                    asset.url
                );
            }
            let tag = Tag {
                url: asset.url.clone(),
                type_: asset.attributes.get("type").cloned(),
                integrity: asset.attributes.get("integrity").cloned(),
                crossorigin: asset.attributes.get("crossorigin").cloned(),
            };
            match asset.role.as_str() {
                "stylesheet" => assets.stylesheets.push(tag),
                "script" => assets.scripts.push(tag),
                _ => assets.icons.push(tag),
            }
        }

        let [bundle] = manifest.wasm.as_slice() else {
            bail!("the SSG demo needs exactly one Trunk WASM bundle to hydrate");
        };
        if bundle.app_type != "main"
            || bundle.compression_algorithm.is_some()
            || bundle.custom_script_pattern.is_some()
            || bundle.custom_preload_pattern.is_some()
            || bundle.initializer_url.is_some()
        {
            bail!(
                "the SSG demo supports only an uncompressed main WASM bundle without custom patterns or initializers"
            );
        }
        assets.preloads = bundle
            .preloads
            .iter()
            .map(|preload| Preload {
                rel: preload.rel.clone(),
                url: preload.url.clone(),
                as_: preload.as_type.clone(),
                type_: preload.media_type.clone(),
                integrity: preload.integrity.clone(),
                crossorigin: preload.cross_origin.clone(),
            })
            .collect();
        if bundle.inject_scripts {
            let init = if bundle.init_with_object {
                format!(
                    "init({{ module_or_path: {} }})",
                    js_string(&bundle.wasm_url)
                )
            } else {
                format!("init({})", js_string(&bundle.wasm_url))
            };
            let bindings = if bundle.import_bindings {
                let name = bundle
                    .import_bindings_name
                    .as_deref()
                    .unwrap_or("wasmBindings");
                format!("window[{}] = bindings;\n", js_string(name))
            } else {
                String::new()
            };
            assets.loader = Some(format!(
                "import init, * as bindings from {};\nconst wasm = await {init};\n{bindings}dispatchEvent(new CustomEvent({}, {{ detail: {{ wasm }} }}));\n",
                js_string(&bundle.js_url),
                js_string(TRUNK_STARTED_EVENT),
            ));
        }
        Ok(assets)
    }

    fn canonical(&self, route: &str) -> String {
        match route.trim_matches('/') {
            "" => self.public_url.clone(),
            path => format!("{}{path}/", self.public_url),
        }
    }
}

/// Encodes `value` as a JavaScript string literal that is also safe inside an HTML
/// `<script>` element.
fn js_string(value: &str) -> String {
    let mut literal = String::from("\"");
    for unit in value.encode_utf16() {
        match char::from_u32(u32::from(unit)) {
            Some(character)
                if character.is_ascii_alphanumeric() || "/._-~:".contains(character) =>
            {
                literal.push(character);
            }
            _ => literal.push_str(&format!("\\u{unit:04x}")),
        }
    }
    literal.push('"');
    literal
}

fn asset_tags(assets: PageAssets) -> impl IntoView {
    let PageAssets {
        stylesheets,
        scripts,
        icons,
        preloads,
        loader,
        ..
    } = assets;
    view! {
        {icons
            .into_iter()
            .map(|tag| view! {
                <link rel="icon" href=tag.url type=tag.type_ integrity=tag.integrity crossorigin=tag.crossorigin />
            })
            .collect_view()}
        {stylesheets
            .into_iter()
            .map(|tag| view! {
                <link rel="stylesheet" href=tag.url type=tag.type_ integrity=tag.integrity crossorigin=tag.crossorigin />
            })
            .collect_view()}
        {scripts
            .into_iter()
            .map(|tag| view! {
                <script src=tag.url type=tag.type_ integrity=tag.integrity crossorigin=tag.crossorigin></script>
            })
            .collect_view()}
        {loader.map(|loader| view! { <script type="module">{loader}</script> })}
        {preloads
            .into_iter()
            .map(|preload| view! {
                <link
                    rel=preload.rel
                    href=preload.url
                    r#as=preload.as_
                    type=preload.type_
                    integrity=preload.integrity
                    crossorigin=preload.crossorigin
                />
            })
            .collect_view()}
    }
}

fn render_route(route: &'static Route, assets: &PageAssets) -> anyhow::Result<RenderedDocument> {
    let canonical = assets.canonical(route.path);
    let favicon = format!("{}favicon.ico", assets.public_url);
    let assets = assets.clone();
    let document = Document::new(
        move || {
            view! {
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
    );
    let rendered = document.render_with_leptos(move |head, body| {
        view! {
            <!DOCTYPE html>
            <html lang="en">
                <head>
                    <meta charset="utf-8" />
                    <meta name="viewport" content="width=device-width, initial-scale=1" />
                    <link rel="icon" type="image/x-icon" href=favicon />
                    {head()}
                    {asset_tags(assets)}
                </head>
                <body>{body()}</body>
            </html>
        }
    })?;
    Ok(rendered)
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
    let assets = PageAssets::from_manifest(manifest)?;
    initialize()?;
    let writer = StaticDocumentWriter::new(output);
    for route in &ROUTES {
        writer.write(route.path, render_route(route, &assets)?)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn output_with_build(public_url: &str, name: &str) -> (PathBuf, TrunkAssetManifest) {
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
                r#"{{"version":1,"public_url":"{public_url}","html":"index.html",
                "assets":[{{"order":0,"role":"stylesheet","url":"{public_url}style-1a.css","attributes":{{}}}}],
                "wasm":[{{"app_type":"main","order":1,"js_url":"{public_url}app-2b.js","wasm_url":"{public_url}app-3c_bg.wasm",
                "wasm_size":0,"init_with_object":true,"import_bindings":true,"cross_origin":"anonymous","inject_scripts":true,"pattern_parameters":{{}},
                "preloads":[{{"rel":"modulepreload","url":"{public_url}app-2b.js","cross_origin":"anonymous"}}]}}]}}"#
            )
            .as_bytes(),
        )
        .unwrap_or_else(|error| panic!("valid manifest: {error}"));
        (output, manifest)
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

        for (page, title, description, canonical, heading) in [
            (
                &home,
                "Serenade SSG demo",
                "prerendered by Serenade",
                "/app/",
                "Hello from Serenade",
            ),
            (
                &about,
                "About Serenade",
                "framework-rendered UI",
                "/app/about/",
                "About Serenade",
            ),
        ] {
            let head = head(page);
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
            assert!(
                head.contains("rel=\"stylesheet\" href=\"/app/style-1a.css\""),
                "{head}"
            );
            assert!(
                head.contains(r#"import init, * as bindings from "/app/app-2b.js";"#),
                "{head}"
            );
            assert!(
                head.contains(r#"await init({ module_or_path: "/app/app-3c_bg.wasm" })"#),
                "{head}"
            );
            assert!(
                head.contains(r#"window["wasmBindings"] = bindings;"#),
                "{head}"
            );
            assert!(
                head.contains("rel=\"modulepreload\" href=\"/app/app-2b.js\""),
                "{head}"
            );
            assert!(page.contains(&format!("<h1>{heading}</h1>")), "{page}");
            assert!(!page.contains("SERENADE_SSG_CONTENT"), "{page}");
        }
        fs::remove_dir_all(output).expect("cleanup");
    }

    #[test]
    fn rejects_missing_output_and_unsupported_bundles() {
        let (output, manifest) = output_with_build("/", "rejects");
        assert!(prerender(&output.join("missing"), &manifest).is_err());

        let mut compressed = manifest.clone();
        compressed.wasm[0].compression_algorithm = Some("brotli".to_owned());
        assert!(prerender(&output, &compressed).is_err());

        let mut no_bundle = manifest.clone();
        no_bundle.wasm.clear();
        assert!(prerender(&output, &no_bundle).is_err());

        let mut unknown_attribute = manifest;
        unknown_attribute.assets[0]
            .attributes
            .insert("data-x".to_owned(), "1".to_owned());
        assert!(prerender(&output, &unknown_attribute).is_err());
        fs::remove_dir_all(output).expect("cleanup");
    }

    #[test]
    fn js_strings_cannot_break_out_of_the_loader_script() {
        assert_eq!(js_string("/app/a-1.js"), r#""/app/a-1.js""#);
        assert_eq!(
            js_string("</script>\"'\u{2028}é"),
            r#""\u003c/script\u003e\u0022\u0027\u2028\u00e9""#
        );
    }
}

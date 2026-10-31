use std::fs;
use std::path::PathBuf;

use anyhow::{Context, bail};

fn main() -> anyhow::Result<()> {
    let app = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dist = app.join("dist");
    let shell_path = dist.join("index.html");
    let shell = fs::read_to_string(&shell_path)
        .with_context(|| format!("read Trunk output {}", shell_path.display()))?;
    let home = render_route(&shell, "home")?;
    let about = render_route(&shell, "about")?;

    fs::write(&shell_path, home)
        .with_context(|| format!("write prerendered route {}", shell_path.display()))?;

    let about_dir = dist.join("about");
    fs::create_dir_all(&about_dir)
        .with_context(|| format!("create route directory {}", about_dir.display()))?;
    let about_path = about_dir.join("index.html");
    fs::write(&about_path, about)
        .with_context(|| format!("write prerendered route {}", about_path.display()))?;

    println!("Prerendered / and /about/ to {}", dist.display());
    Ok(())
}

fn render_route(shell: &str, page: &'static str) -> anyhow::Result<String> {
    let marker = "<!--SERENADE_SSG_CONTENT-->";
    if shell.matches(marker).count() != 1 {
        bail!("Trunk output must contain exactly one {marker} marker");
    }

    let html = serenade_demo_ssg::render_page(page);
    Ok(shell.replacen(marker, &html, 1))
}

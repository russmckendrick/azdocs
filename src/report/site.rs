//! Multi-page HTML rendering of the markdown docs tree: same pages as
//! `markdown::write`, converted to styled, navigable HTML.

use std::path::Path;

use anyhow::Context;
use pulldown_cmark::{Options, Parser, html};

use super::ReportContext;
use super::markdown::render_pages;
use crate::diagram::assets::{DiagramAsset, DiagramAssetKind};

const STYLE: &str = r#"
:root { --bg:#fff; --fg:#1a1a2e; --muted:#666; --border:#ddd; --accent:#0078d4; }
@media (prefers-color-scheme: dark) {
  :root { --bg:#16161d; --fg:#e8e8ef; --muted:#9a9aa5; --border:#3a3a45; --accent:#4da3e8; }
}
body { font: 15px/1.55 -apple-system, "Segoe UI", Roboto, sans-serif;
       background: var(--bg); color: var(--fg); max-width: 1050px; margin: 0 auto; padding: 2rem 1rem; }
h1,h2,h3 { line-height: 1.2; } h2 { border-bottom: 1px solid var(--border); padding-bottom: .3rem; margin-top: 2.2rem; }
a { color: var(--accent); }
table { border-collapse: collapse; width: 100%; margin: 1rem 0; font-size: 14px; display: block; overflow-x: auto; }
th, td { text-align: left; padding: .35rem .6rem; border-bottom: 1px solid var(--border); white-space: nowrap; }
th { background: rgba(127,127,127,.08); }
code { background: rgba(127,127,127,.12); padding: .1em .3em; border-radius: 3px; font-size: 90%; }
nav { font-size: 13px; color: var(--muted); margin-bottom: 1.5rem; }
nav a { margin-right: .8rem; }
blockquote { margin: .6rem 0; padding: .5rem .9rem; border-left: 4px solid #d13438;
             background: rgba(209,52,56,.08); border-radius: 0 6px 6px 0; }
blockquote p { margin: 0; }
"#;

/// Write the docs tree as HTML pages under `out_dir` (index.html, ...),
/// plus a `diagrams/` directory of SVGs; the overview diagrams are embedded
/// on the index page.
pub fn write(
    report: &ReportContext,
    diagrams: &[DiagramAsset],
    out_dir: &Path,
) -> anyhow::Result<()> {
    if !diagrams.is_empty() {
        let diagrams_dir = out_dir.join("diagrams");
        std::fs::create_dir_all(&diagrams_dir)
            .with_context(|| format!("creating {}", diagrams_dir.display()))?;
        for asset in diagrams {
            let path = diagrams_dir.join(format!("{}.svg", asset.slug));
            std::fs::write(&path, &asset.svg)
                .with_context(|| format!("writing {}", path.display()))?;
        }
    }
    let pages = render_pages(report)?;
    let nav = navigation(&pages);
    for (relative, markdown) in &pages {
        let html_relative = relative.replace(".md", ".html");
        let depth = html_relative.matches('/').count();
        let prefix = "../".repeat(depth);
        let mut body = markdown_to_html(&rewrite_links(markdown));
        if relative == "index.md" {
            body.push_str(&diagram_section(diagrams));
        }
        let title = markdown
            .lines()
            .find_map(|l| l.strip_prefix("# "))
            .unwrap_or("azdocs");
        let nav_html: String = nav
            .iter()
            .map(|(href, label)| format!("<a href=\"{prefix}{href}\">{label}</a>"))
            .collect();
        let page = format!(
            "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\">\
             <title>{title}</title><style>{STYLE}</style></head>\n\
             <body><nav>{nav_html}</nav>\n{body}\n</body></html>\n"
        );
        let path = out_dir.join(&html_relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        std::fs::write(&path, page).with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(())
}

/// Overview diagrams (estate hierarchy + network topology) embedded on the
/// index page; the full set lives in `diagrams/` for linking.
fn diagram_section(diagrams: &[DiagramAsset]) -> String {
    let embedded: Vec<&DiagramAsset> = diagrams
        .iter()
        .filter(|asset| {
            matches!(
                asset.kind,
                DiagramAssetKind::Hierarchy | DiagramAssetKind::Network
            )
        })
        .collect();
    if embedded.is_empty() {
        return String::new();
    }
    let mut out = String::from("<h2>Diagrams</h2>\n");
    for asset in embedded {
        let title = html_escape(&asset.title);
        out.push_str(&format!(
            "<figure><img src=\"diagrams/{slug}.svg\" alt=\"{title}\" style=\"max-width:100%\">\
             <figcaption>{title}</figcaption></figure>\n",
            slug = asset.slug,
        ));
    }
    out
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn navigation(pages: &[(String, String)]) -> Vec<(String, String)> {
    pages
        .iter()
        .filter(|(relative, _)| {
            !relative.starts_with("subscriptions/") && !relative.starts_with("resources/")
        })
        .map(|(relative, _)| {
            let label = relative.trim_end_matches(".md").to_owned();
            (relative.replace(".md", ".html"), label)
        })
        .collect()
}

/// Point intra-docs links at the .html twins.
fn rewrite_links(markdown: &str) -> String {
    markdown.replace(".md)", ".html)")
}

fn markdown_to_html(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    let mut out = String::new();
    html::push_html(&mut out, Parser::new_ext(markdown, options));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrite_links_targets_html_twins() {
        assert_eq!(
            rewrite_links("see [findings](findings.md) here"),
            "see [findings](findings.html) here"
        );
    }

    #[test]
    fn markdown_tables_convert_to_html_tables() {
        let html = markdown_to_html("| a | b |\n|---|---|\n| 1 | 2 |\n");
        assert!(html.contains("<table>"), "html: {html}");
    }
}

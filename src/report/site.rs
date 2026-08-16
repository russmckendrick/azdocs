//! Multi-page HTML rendering of the markdown docs tree: same pages as
//! `markdown::write`, converted to styled, navigable HTML.

use std::path::Path;

use anyhow::Context;
use pulldown_cmark::{Options, Parser, html};

use super::ReportContext;
use super::markdown::render_pages;

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
"#;

/// Write the docs tree as HTML pages under `out_dir` (index.html, ...).
pub fn write(report: &ReportContext, out_dir: &Path) -> anyhow::Result<()> {
    let pages = render_pages(report)?;
    let nav = navigation(&pages);
    for (relative, markdown) in &pages {
        let html_relative = relative.replace(".md", ".html");
        let depth = html_relative.matches('/').count();
        let prefix = "../".repeat(depth);
        let body = markdown_to_html(&rewrite_links(markdown));
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

fn navigation(pages: &[(String, String)]) -> Vec<(String, String)> {
    pages
        .iter()
        .filter(|(relative, _)| !relative.starts_with("subscriptions/"))
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

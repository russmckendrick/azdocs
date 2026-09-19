//! Multi-page HTML rendering of the markdown docs tree: same pages as
//! `markdown::write`, converted to styled, navigable HTML.

use std::path::Path;

use anyhow::Context;
use pulldown_cmark::{Options, Parser, html};

use super::ReportContext;
use super::branding::BrandingContext;
use super::markdown::{DiagramEmbedding, render_pages, write_diagram_files};
use super::theme::TableStyle;
use crate::diagram::assets::DiagramAsset;

/// Placeholders are filled from the resolved theme in [`style`], so the docs
/// site is the same design system as the PDF and DOCX rather than a look-alike.
const STYLE: &str = r#"
:root { --bg:{surface}; --fg:{ink}; --muted:{muted}; --border:{rule};
        --primary:{primary}; --accent:{accent}; --tint:{tint}; --zebra:{zebra}; --on-accent:{on_primary};
        --high:{high}; --high-fill:{high_fill}; --radius:{radius}px; }
@media (prefers-color-scheme: dark) {
  :root { --bg:{dark_surface}; --fg:{dark_ink}; --muted:{dark_muted}; --border:{dark_rule};
          --tint:{dark_tint}; --zebra:{dark_zebra}; --accent:{dark_accent}; }
}
body { font: 15px/{line_height} "{sans}", -apple-system, "Segoe UI", Roboto, sans-serif;
       background: var(--bg); color: var(--fg); max-width: 1050px; margin: 0 auto; padding: 2rem 1rem; }
h1,h2,h3 { font-family: "{serif}", Georgia, serif; line-height: 1.2; }
h2 { border-bottom: 1px solid var(--border); padding-bottom: .3rem; margin-top: 2.2rem; }
a { color: var(--accent); }
table { border-collapse: collapse; width: 100%; margin: 1rem 0; font-size: 14px; display: block; overflow-x: auto; }
th, td { text-align: left; padding: .35rem .6rem; border-bottom: 1px solid var(--border); white-space: nowrap; }
th { background: var(--th-bg); color: var(--th-fg); }
tbody tr:nth-child(even) { background: var(--zebra); }
code { font-family: "{mono}", ui-monospace, SFMono-Regular, Menlo, monospace;
       background: rgba(127,127,127,.12); padding: .1em .3em; border-radius: 3px; font-size: 90%; }
nav { font-size: 13px; color: var(--muted); margin-bottom: 1.5rem; }
nav a { margin-right: .8rem; }
blockquote { margin: .6rem 0; padding: .5rem .9rem; border-left: 4px solid var(--high);
             background: var(--high-fill); border-radius: 0 var(--radius) var(--radius) 0; }
blockquote p { margin: 0; }
header.brand { display: flex; align-items: center; gap: .7rem; margin-bottom: .8rem;
               color: var(--muted); font-size: 14px; }
header.brand img { max-height: 40px; }
footer.brand { margin-top: 2.5rem; border-top: 1px solid var(--border); padding-top: .7rem;
               color: var(--muted); font-size: 13px; }
"#;

/// The docs-site stylesheet with the resolved theme substituted in.
fn style(branding: &BrandingContext) -> String {
    let tokens = &branding.tokens;
    let palette = &tokens.palette;
    // Table headers follow the theme's table strategy, the same choice the
    // PDF and DOCX make.
    let (th_bg, th_fg) = match tokens.layout.table {
        TableStyle::SolidHeader => (palette.primary.as_str(), palette.on_primary.as_str()),
        TableStyle::Banded => (palette.primary_tint.as_str(), palette.ink.as_str()),
        TableStyle::Hairline => ("transparent", palette.ink.as_str()),
    };
    let zebra = if tokens.layout.zebra_rows {
        palette.zebra.as_str()
    } else {
        "transparent"
    };

    STYLE
        .replace("{surface}", &palette.surface)
        .replace("{ink}", &palette.ink)
        .replace("{muted}", &palette.muted)
        .replace("{rule}", &palette.rule)
        .replace("{primary}", &palette.primary)
        .replace("{accent}", &palette.accent)
        .replace("{tint}", &palette.primary_tint)
        .replace("{zebra}", zebra)
        .replace("{on_primary}", &palette.on_primary)
        .replace("{high_fill}", &palette.severity.high.fill)
        .replace("{high}", &palette.severity.high.text)
        .replace("{radius}", &tokens.layout.radius_pt.to_string())
        .replace("{line_height}", &tokens.typography.line_height.to_string())
        .replace("{sans}", &tokens.typography.sans)
        .replace("{serif}", &tokens.typography.serif)
        .replace("{mono}", &tokens.typography.mono)
        .replace("{dark_surface}", &palette.dark.surface)
        .replace("{dark_ink}", &palette.dark.ink)
        .replace("{dark_muted}", &palette.dark.muted)
        .replace("{dark_rule}", &palette.dark.rule)
        .replace("{dark_tint}", &palette.dark.tint)
        .replace("{dark_zebra}", &palette.dark.zebra)
        .replace("{dark_accent}", &palette.dark.accent)
        + &format!(":root {{ --th-bg:{th_bg}; --th-fg:{th_fg}; }}\n")
}

/// Site-wide header shown above the nav: logo and/or company + subtitle.
/// Empty with default branding so unbranded output keeps today's look.
fn brand_header(branding: &BrandingContext) -> String {
    let mut parts = Vec::new();
    if let Some(logo) = &branding.logo {
        parts.push(format!(
            "<img src=\"{}\" alt=\"{}\">",
            logo.data_uri,
            html_escape(&branding.labels.common.cover.logo_alt)
        ));
    }
    if !branding.company.is_empty() {
        parts.push(format!("<b>{}</b>", html_escape(&branding.company)));
    }
    if !branding.subtitle.is_empty() {
        parts.push(html_escape(&branding.subtitle));
    }
    if parts.is_empty() {
        return String::new();
    }
    format!("<header class=\"brand\">{}</header>", parts.join(" "))
}

/// Write the docs tree as HTML pages under `out_dir` (index.html, ...),
/// plus a `diagrams/` directory of SVGs; the overview diagrams are embedded
/// on the index page.
pub fn write(
    report: &ReportContext,
    branding: &BrandingContext,
    diagrams: &[DiagramAsset],
    out_dir: &Path,
) -> anyhow::Result<()> {
    let style = style(branding);
    let header = brand_header(branding);
    let footer = format!(
        "<footer class=\"brand\">{}</footer>",
        html_escape(&branding.footer)
    );
    // Every page's image links point at `diagrams/<slug>.svg`; the Mermaid
    // source is left out of the site because a browser would show it as code.
    write_diagram_files(diagrams, out_dir)?;
    let pages = render_pages(
        report,
        &branding.labels,
        diagrams,
        DiagramEmbedding { mermaid: false },
    )?;
    report.websites.write_assets(out_dir)?;
    let nav = navigation(&pages, &branding.labels);
    let lang = html_escape(&branding.labels.meta.lang);
    for (relative, markdown) in &pages {
        let html_relative = relative.replace(".md", ".html");
        let depth = html_relative.matches('/').count();
        let prefix = "../".repeat(depth);
        let mut body = markdown_to_html(&rewrite_links(markdown));
        if relative == "index.md" {
            body.push_str(&report.websites.html(&branding.labels, None, Some(&prefix)));
        }
        if let Some(group) = report
            .details
            .iter()
            .find(|group| format!("{}.md", group.path) == *relative)
        {
            let ids = group
                .resources
                .iter()
                .map(|r| crate::model::normalize_arm_id(&r.arm_id))
                .collect();
            body.push_str(
                &report
                    .websites
                    .html(&branding.labels, Some(&ids), Some(&prefix)),
            );
        }
        let title = markdown
            .lines()
            .find_map(|l| l.strip_prefix("# "))
            .unwrap_or(&branding.title);
        let title = html_escape(title);
        let nav_html: String = nav
            .iter()
            .map(|(href, label)| format!("<a href=\"{prefix}{href}\">{label}</a>"))
            .collect();
        let page = format!(
            "<!doctype html>\n<html lang=\"{lang}\"><head><meta charset=\"utf-8\">\
             <title>{title}</title><style>{style}</style></head>\n\
             <body>{header}<nav>{nav_html}</nav>\n{body}\n{footer}</body></html>\n"
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

pub(crate) fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Top-level pages only, captioned from the labels pack rather than by their
/// file names: a `fr` pack reads "Aperçu", not "index".
fn navigation(pages: &[(String, String)], labels: &crate::labels::Labels) -> Vec<(String, String)> {
    let words = &labels.report.site;
    pages
        .iter()
        .filter(|(relative, _)| {
            !relative.starts_with("subscriptions/") && !relative.starts_with("resources/")
        })
        .map(|(relative, _)| {
            let stem = relative.trim_end_matches(".md");
            let label = match stem {
                "index" => words.index.clone(),
                "findings" => words.findings.clone(),
                "query-provenance" => words.provenance.clone(),
                category => crate::labels::fill(&words.category, &[("category", &category)]),
            };
            (relative.replace(".md", ".html"), html_escape(&label))
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

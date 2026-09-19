use std::path::Path;

use anyhow::Context;
use minijinja::value::{Kwargs, ViaDeserialize};
use minijinja::{Environment, context};
use serde_json::Value;

use super::{ReportContext, cell_to_string};
use crate::diagram::assets::{DiagramAsset, DiagramAssetKind};
use crate::labels::Labels;

/// What a page says about a diagram: the image link, and optionally the
/// Mermaid source for hosts that render it (GitHub, most wikis). The site
/// leaves the source out because a browser shows it as code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagramEmbedding {
    pub mermaid: bool,
}

#[derive(Debug, serde::Serialize)]
struct DiagramRef<'a> {
    slug: &'a str,
    title: &'a str,
    mermaid: Option<&'a str>,
}

fn diagram_refs<'a>(
    diagrams: &'a [DiagramAsset],
    embedding: DiagramEmbedding,
    keep: impl Fn(&DiagramAsset) -> bool,
) -> Vec<DiagramRef<'a>> {
    diagrams
        .iter()
        .filter(|asset| keep(asset))
        .map(|asset| DiagramRef {
            slug: &asset.slug,
            title: &asset.title,
            mermaid: embedding.mermaid.then_some(asset.mermaid.as_str()),
        })
        .collect()
}

/// Shared environment for markdown and HTML templates, with the custom
/// filters they rely on. The empty-table text is captured here because a
/// filter cannot reach the render context.
pub(crate) fn environment(labels: &Labels) -> Environment<'static> {
    let mut env = Environment::new();
    let no_rows = labels.report.markdown.no_rows.clone();
    env.add_filter("md_escape", md_escape);
    env.add_filter("md_text", md_text);
    env.add_filter(
        "md_table",
        move |rows: ViaDeserialize<Vec<Value>>, columns: ViaDeserialize<Vec<String>>| {
            md_table(rows, columns, &no_rows)
        },
    );
    env.add_filter("slug", slug);
    env.add_filter("fill", fill_filter);
    env.add_filter("md_cell", md_cell);
    env
}

/// A diff value (JSON or null) as one escaped table cell.
fn md_cell(value: ViaDeserialize<Value>) -> String {
    md_escape(&cell_to_string(Some(&value)))
}

/// `{{ label | fill(name=value, ...) }}`: the template-side twin of
/// `labels::fill`, so markdown and HTML share sentences with the print
/// document instead of restating them.
fn fill_filter(template: String, kwargs: Kwargs) -> Result<String, minijinja::Error> {
    let pairs: Vec<(&str, String)> = kwargs
        .args()
        .map(|name| {
            kwargs
                .get::<minijinja::Value>(name)
                .map(|value| (name, value.to_string()))
        })
        .collect::<Result<_, _>>()?;
    kwargs.assert_all_used()?;
    let args: Vec<(&str, &dyn std::fmt::Display)> = pairs
        .iter()
        .map(|(name, value)| (*name, value as &dyn std::fmt::Display))
        .collect();
    Ok(crate::labels::fill(&template, &args))
}

fn md_escape(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn md_text(value: &str) -> String {
    let mut value = value.replace('\\', "\\\\");
    for character in ['`', '*', '_', '[', ']'] {
        value = value.replace(character, &format!("\\{character}"));
    }
    md_escape(&value)
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Directory- and link-safe version of a display name.
/// Page slug for a Markdown filename and its anchor.
pub fn slug(value: &str) -> String {
    crate::model::slugify(value)
}

/// Render rows (JSON objects) as a markdown table with the given columns.
fn md_table(
    rows: ViaDeserialize<Vec<Value>>,
    columns: ViaDeserialize<Vec<String>>,
    no_rows: &str,
) -> String {
    if rows.is_empty() || columns.is_empty() {
        return no_rows.to_owned();
    }
    let mut out = String::new();
    out.push_str(&format!("| {} |\n", columns.join(" | ")));
    out.push_str(&format!("|{}\n", "---|".repeat(columns.len())));
    for row in rows.iter() {
        let cells: Vec<String> = columns
            .iter()
            .map(|c| md_escape(&cell_to_string(row.get(c))))
            .collect();
        out.push_str(&format!("| {} |\n", cells.join(" | ")));
    }
    out
}

/// Write the docs/ tree: index, findings, per-category pages, and one page per
/// subscription.
/// Render every docs page in memory as (relative path, markdown) pairs.
/// The markdown writer and the HTML site generator both consume this.
pub fn render_pages(
    report: &ReportContext,
    labels: &Labels,
    diagrams: &[DiagramAsset],
    embedding: DiagramEmbedding,
) -> anyhow::Result<Vec<(String, String)>> {
    let mut env = environment(labels);
    let overview_diagrams = diagram_refs(diagrams, embedding, |asset| {
        matches!(
            asset.kind,
            DiagramAssetKind::Hierarchy | DiagramAssetKind::Network
        )
    });
    let posture_tables = report.posture.tables(labels);
    let provenance_records = super::provenance::records(&report.analysis.query_runs, labels);
    let labels_ref = labels;
    let labels = minijinja::Value::from_serialize(labels);
    let tag_audit = super::governance::TAG_AUDIT;
    env.add_template(
        "index",
        include_str!("../../templates/markdown/index.md.j2"),
    )?;
    env.add_template(
        "category",
        include_str!("../../templates/markdown/category.md.j2"),
    )?;
    env.add_template(
        "findings",
        include_str!("../../templates/markdown/findings.md.j2"),
    )?;
    env.add_template(
        "subscription",
        include_str!("../../templates/markdown/subscription.md.j2"),
    )?;
    env.add_template(
        "resource_group",
        include_str!("../../templates/markdown/resource_group.md.j2"),
    )?;

    let mut pages = Vec::new();
    pages.push((
        "index.md".to_owned(),
        env.get_template("index")?.render(context! {
            labels,
            tag_audit,
            posture_tables,
            diagrams => overview_diagrams,
            has_provenance => !provenance_records.is_empty(),
            field_changes => report.changes.as_ref().map_or(0, |c| c.field_changes()),
            website_rows => report.websites.rows(labels_ref),
            ..minijinja::Value::from_serialize(report)
        })?,
    ));
    pages.push((
        "findings.md".to_owned(),
        env.get_template("findings")?.render(context! {
            labels,
            ..minijinja::Value::from_serialize(report)
        })?,
    ));
    if !provenance_records.is_empty() {
        env.add_template(
            "provenance",
            include_str!("../../templates/markdown/provenance.md.j2"),
        )?;
        pages.push((
            "query-provenance.md".to_owned(),
            env.get_template("provenance")?
                .render(context! { labels, records => provenance_records })?,
        ));
    }
    for category in &report.categories {
        pages.push((
            format!("{}.md", category.name),
            env.get_template("category")?
                .render(context! { labels, category })?,
        ));
    }
    for sub in &report.subscriptions {
        pages.push((
            format!("subscriptions/{}.md", slug(&sub.display_name)),
            env.get_template("subscription")?
                .render(context! { labels, sub })?,
        ));
    }
    for page in &report.details {
        let group_diagrams = diagram_refs(diagrams, embedding, |asset| {
            asset.group_key.as_deref() == Some(page.group_key.as_str())
        });
        pages.push((
            format!("{}.md", page.path),
            env.get_template("resource_group")?
                .render(context! { labels, page, diagrams => group_diagrams })?,
        ));
    }
    Ok(pages)
}

/// Write the docs tree plus a `diagrams/` directory holding each diagram as
/// SVG (for the image links) and `.mmd` (the Mermaid source on its own).
pub fn write(
    report: &ReportContext,
    labels: &Labels,
    diagrams: &[DiagramAsset],
    out_dir: &Path,
) -> anyhow::Result<()> {
    write_diagram_files(diagrams, out_dir)?;
    report.websites.write_assets(out_dir)?;
    for (relative, content) in
        render_pages(report, labels, diagrams, DiagramEmbedding { mermaid: true })?
    {
        let path = out_dir.join(&relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        std::fs::write(&path, content).with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(())
}

pub(crate) fn write_diagram_files(diagrams: &[DiagramAsset], out_dir: &Path) -> anyhow::Result<()> {
    if diagrams.is_empty() {
        return Ok(());
    }
    let dir = out_dir.join("diagrams");
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    for asset in diagrams {
        for (extension, content) in [("svg", &asset.svg), ("mmd", &asset.mermaid)] {
            let path = dir.join(format!("{}.{extension}", asset.slug));
            std::fs::write(&path, content)
                .with_context(|| format!("writing {}", path.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_flattens_spaces_and_case() {
        assert_eq!(slug("My Production (EU) Sub"), "my-production-eu-sub");
    }

    #[test]
    fn md_escape_neutralizes_pipes() {
        assert_eq!(md_escape("a|b"), "a\\|b");
    }
}

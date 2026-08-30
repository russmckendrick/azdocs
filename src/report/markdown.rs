use std::path::Path;

use anyhow::Context;
use minijinja::value::ViaDeserialize;
use minijinja::{Environment, context};
use serde_json::Value;

use super::{ReportContext, cell_to_string};

/// Shared environment for markdown and HTML templates, with the custom
/// filters they rely on.
pub(crate) fn environment() -> Environment<'static> {
    let mut env = Environment::new();
    env.add_filter("md_escape", md_escape);
    env.add_filter("md_table", md_table);
    env.add_filter("slug", slug);
    env
}

fn md_escape(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

/// Directory- and link-safe version of a display name.
/// Page slug for a Markdown filename and its anchor.
pub fn slug(value: &str) -> String {
    crate::model::slugify(value)
}

/// Render rows (JSON objects) as a markdown table with the given columns.
fn md_table(rows: ViaDeserialize<Vec<Value>>, columns: ViaDeserialize<Vec<String>>) -> String {
    if rows.is_empty() || columns.is_empty() {
        return "_No rows._".to_owned();
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
pub fn render_pages(report: &ReportContext) -> anyhow::Result<Vec<(String, String)>> {
    let mut env = environment();
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
        env.get_template("index")?
            .render(context! { ..minijinja::Value::from_serialize(report) })?,
    ));
    pages.push((
        "findings.md".to_owned(),
        env.get_template("findings")?
            .render(context! { ..minijinja::Value::from_serialize(report) })?,
    ));
    for category in &report.categories {
        pages.push((
            format!("{}.md", category.name),
            env.get_template("category")?
                .render(context! { category })?,
        ));
    }
    for sub in &report.subscriptions {
        pages.push((
            format!("subscriptions/{}.md", slug(&sub.display_name)),
            env.get_template("subscription")?.render(context! { sub })?,
        ));
    }
    for page in &report.details {
        pages.push((
            format!("{}.md", page.path),
            env.get_template("resource_group")?
                .render(context! { page })?,
        ));
    }
    Ok(pages)
}

pub fn write(report: &ReportContext, out_dir: &Path) -> anyhow::Result<()> {
    for (relative, content) in render_pages(report)? {
        let path = out_dir.join(&relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        std::fs::write(&path, content).with_context(|| format!("writing {}", path.display()))?;
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

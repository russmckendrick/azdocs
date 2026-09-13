use std::io::Read;
use std::path::Path;

use anyhow::{Context, bail};
use serde_json::Value;

use crate::arg::ArgClient;
use crate::cli::QueryOutputFormat;
use crate::config::Config;
use crate::labels::{Labels, fill};
use crate::model::AuthorizationScope;
use crate::model::rows::{cell_to_string, columns};
use crate::querypack::QueryPack;

/// Run one query live against ARG and print the rows.
pub async fn run(
    config: &Config,
    query: &str,
    format: QueryOutputFormat,
    subscriptions: &[String],
    labels: &Labels,
) -> anyhow::Result<()> {
    let (kql, authorization_scope) = resolve_query(query)?;
    let provider = super::token_provider(config)?;
    let scope = if subscriptions.is_empty() {
        &config.collect.subscriptions
    } else {
        subscriptions
    };
    let check = crate::auth::diagnostics::inspect(super::http_client(), &provider, scope).await?;
    super::check::print_permissions(&check, labels);
    let client = ArgClient::new(super::http_client(), provider);
    let outcome = client
        .query_all_with_scope(&kql, scope, authorization_scope)
        .await
        .context("query against Azure Resource Graph failed")?;

    print_rows(&outcome.rows, format, labels)?;
    eprintln!(
        "{}",
        fill(
            &labels.cli.query.rows_summary,
            &[("rows", &outcome.rows.len()), ("pages", &outcome.pages)]
        )
    );
    Ok(())
}

/// `-` reads stdin; an existing file path is read (a .toml query definition's
/// `kql` field, or raw KQL otherwise); anything else is a named query from the
/// query pack.
fn resolve_query(query: &str) -> anyhow::Result<(String, Option<AuthorizationScope>)> {
    if query == "-" {
        let mut kql = String::new();
        std::io::stdin()
            .read_to_string(&mut kql)
            .context("reading query from stdin")?;
        return Ok((kql, None));
    }
    let path = Path::new(query);
    if path.exists() {
        let raw =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        if path.extension().is_some_and(|ext| ext == "toml") {
            let parsed: toml::Table = toml::from_str(&raw).context("parsing query definition")?;
            let Some(kql) = parsed.get("kql").and_then(|v| v.as_str()) else {
                bail!("{} has no `kql` field", path.display());
            };
            let scope = parsed
                .get("authorization_scope")
                .cloned()
                .map(|value| value.try_into())
                .transpose()
                .context("parsing authorization_scope")?;
            return Ok((kql.to_owned(), scope));
        }
        return Ok((raw, None));
    }
    let pack = QueryPack::load()?;
    let Some(def) = pack.get(query) else {
        bail!("`{query}` is neither a file nor a known query name — see `azdocs query list`");
    };
    Ok((def.kql.clone(), def.authorization_scope))
}

pub fn list(category: Option<&str>, labels: &Labels) -> anyhow::Result<()> {
    let words = &labels.cli.query;
    let columns = &words.columns;
    let pack = QueryPack::load()?;
    let mut table = comfy_table::Table::new();
    table.load_style(comfy_table::presets::UTF8_BORDERS_ONLY);
    table.set_header([
        columns.name.as_str(),
        columns.category.as_str(),
        columns.kind.as_str(),
        columns.severity.as_str(),
        columns.description.as_str(),
    ]);
    for def in pack.all() {
        if category.is_some_and(|c| c != def.category) {
            continue;
        }
        table.add_row([
            def.name.clone(),
            def.category.clone(),
            format!("{:?}", def.kind).to_lowercase(),
            def.severity
                .map_or(String::new(), |s| s.as_str().to_owned()),
            def.description.clone(),
        ]);
    }
    println!("{table}");
    if let Some(dir) = crate::querypack::user_queries_dir() {
        println!(
            "{}",
            fill(&words.user_queries_dir, &[("path", &dir.display())])
        );
    }
    Ok(())
}

pub fn show(name: &str) -> anyhow::Result<()> {
    let pack = QueryPack::load()?;
    let Some(def) = pack.get(name) else {
        bail!("no query named `{name}` — see `azdocs query list`");
    };
    println!("# {} ({}, {:?})", def.name, def.category, def.kind);
    println!("# {}", def.description);
    println!("{}", def.kql.trim());
    Ok(())
}

fn print_rows(rows: &[Value], format: QueryOutputFormat, labels: &Labels) -> anyhow::Result<()> {
    match format {
        QueryOutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(rows)?);
        }
        QueryOutputFormat::Table => print_table(rows, &labels.cli.query.no_rows),
        QueryOutputFormat::Csv => print_csv(rows)?,
    }
    Ok(())
}

/// Column order follows the first row's key order (ARG preserves the
/// projection order in objectArray results).
fn print_table(rows: &[Value], no_rows: &str) {
    if rows.is_empty() {
        println!("{no_rows}");
        return;
    }
    let columns = columns(rows);
    let mut table = comfy_table::Table::new();
    table.load_style(comfy_table::presets::UTF8_BORDERS_ONLY);
    table.set_header(&columns);
    for row in rows {
        table.add_row(columns.iter().map(|c| cell_to_string(row.get(c))));
    }
    println!("{table}");
}

fn print_csv(rows: &[Value]) -> anyhow::Result<()> {
    let columns = columns(rows);
    let mut writer = csv::Writer::from_writer(std::io::stdout());
    writer.write_record(&columns)?;
    for row in rows {
        writer.write_record(columns.iter().map(|c| cell_to_string(row.get(c))))?;
    }
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_query_file_reads_a_toml_document_with_source_comments() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("query.toml");
        let definition = include_str!("../../queries/cost/advisor_cost_recommendations.toml")
            .replace("\r\n", "\n");
        for newline in ["\n", "\r\n"] {
            std::fs::write(&path, definition.replace('\n', newline)).unwrap();
            let (resolved, _) = resolve_query(path.to_str().unwrap()).unwrap();
            assert_eq!(resolved.lines().next(), Some("advisorresources"));
            assert_eq!(resolved.lines().last(), Some("| order by id asc"));
        }
    }
}

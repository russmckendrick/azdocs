use std::io::Read;
use std::path::Path;

use anyhow::{Context, bail};
use serde_json::Value;

use crate::arg::ArgClient;
use crate::cli::QueryOutputFormat;
use crate::config::Config;
use crate::querypack::QueryPack;

/// Run one query live against ARG and print the rows.
pub async fn run(
    config: &Config,
    query: &str,
    format: QueryOutputFormat,
    subscriptions: &[String],
) -> anyhow::Result<()> {
    let kql = resolve_kql(query)?;
    let provider = super::token_provider(config)?;
    let client = ArgClient::new(super::http_client(), provider);
    let scope = if subscriptions.is_empty() {
        &config.collect.subscriptions
    } else {
        subscriptions
    };
    let outcome = client
        .query_all(&kql, scope)
        .await
        .context("query against Azure Resource Graph failed")?;

    print_rows(&outcome.rows, format)?;
    eprintln!("{} rows ({} pages)", outcome.rows.len(), outcome.pages);
    Ok(())
}

/// `-` reads stdin; an existing file path is read (a .toml query definition's
/// `kql` field, or raw KQL otherwise); anything else is a named query from the
/// query pack.
fn resolve_kql(query: &str) -> anyhow::Result<String> {
    if query == "-" {
        let mut kql = String::new();
        std::io::stdin()
            .read_to_string(&mut kql)
            .context("reading query from stdin")?;
        return Ok(kql);
    }
    let path = Path::new(query);
    if path.exists() {
        let raw =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        if path.extension().is_some_and(|ext| ext == "toml") {
            let parsed: toml::Value = raw.parse().context("parsing query definition")?;
            let Some(kql) = parsed.get("kql").and_then(|v| v.as_str()) else {
                bail!("{} has no `kql` field", path.display());
            };
            return Ok(kql.to_owned());
        }
        return Ok(raw);
    }
    let pack = QueryPack::load()?;
    let Some(def) = pack.get(query) else {
        bail!("`{query}` is neither a file nor a known query name — see `azdocs query list`");
    };
    Ok(def.kql.clone())
}

pub fn list(category: Option<&str>) -> anyhow::Result<()> {
    let pack = QueryPack::load()?;
    let mut table = comfy_table::Table::new();
    table.load_style(comfy_table::presets::UTF8_BORDERS_ONLY);
    table.set_header(["name", "category", "kind", "severity", "description"]);
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
        println!("User queries dir: {}", dir.display());
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

fn print_rows(rows: &[Value], format: QueryOutputFormat) -> anyhow::Result<()> {
    match format {
        QueryOutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(rows)?);
        }
        QueryOutputFormat::Table => print_table(rows),
        QueryOutputFormat::Csv => print_csv(rows)?,
    }
    Ok(())
}

/// Column order follows the first row's key order (ARG preserves the
/// projection order in objectArray results).
fn columns(rows: &[Value]) -> Vec<String> {
    let Some(Value::Object(first)) = rows.first() else {
        return Vec::new();
    };
    first.keys().cloned().collect()
}

fn cell_text(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

fn print_table(rows: &[Value]) {
    if rows.is_empty() {
        println!("(no rows)");
        return;
    }
    let columns = columns(rows);
    let mut table = comfy_table::Table::new();
    table.load_style(comfy_table::presets::UTF8_BORDERS_ONLY);
    table.set_header(&columns);
    for row in rows {
        table.add_row(columns.iter().map(|c| cell_text(row.get(c))));
    }
    println!("{table}");
}

fn print_csv(rows: &[Value]) -> anyhow::Result<()> {
    let columns = columns(rows);
    let mut writer = csv::Writer::from_writer(std::io::stdout());
    writer.write_record(&columns)?;
    for row in rows {
        writer.write_record(columns.iter().map(|c| cell_text(row.get(c))))?;
    }
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn columns_follow_first_row_key_order() {
        let rows = vec![json!({"id": "a", "name": "x", "location": "uk"})];

        assert_eq!(columns(&rows), vec!["id", "name", "location"]);
    }

    #[test]
    fn cell_text_renders_nested_values_as_json() {
        let value = json!({"tags": {"env": "prod"}});

        assert_eq!(cell_text(value.get("tags")), r#"{"env":"prod"}"#);
    }
}

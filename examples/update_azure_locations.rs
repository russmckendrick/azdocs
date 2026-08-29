use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context, Result, ensure};

const SOURCE_URL: &str = "https://raw.githubusercontent.com/MicrosoftDocs/reliability-docs/main/articles/reliability/regions-list.md";

#[tokio::main]
async fn main() -> Result<()> {
    let check = std::env::args()
        .skip(1)
        .any(|argument| argument == "--check");
    let markdown = reqwest::get(SOURCE_URL)
        .await
        .context("download Microsoft Azure regions list")?
        .error_for_status()
        .context("Microsoft Azure regions list returned an error")?
        .text()
        .await
        .context("read Microsoft Azure regions list")?;
    let mut locations = parse_regions(&markdown)?;
    locations.insert("global".to_owned(), "Global".to_owned());
    ensure!(
        locations.len() >= 50,
        "refusing to replace metadata with only {} parsed regions",
        locations.len()
    );

    let rendered = render_locations(&locations)?;
    let output_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("azure_locations.toml");
    if check {
        let current = std::fs::read_to_string(&output_path)
            .with_context(|| format!("read {}", output_path.display()))?;
        ensure!(
            current == rendered,
            "{} is stale; run `cargo run --example update_azure_locations`",
            output_path.display()
        );
        println!("{} is current", output_path.display());
        return Ok(());
    }

    std::fs::write(&output_path, rendered)
        .with_context(|| format!("write {}", output_path.display()))?;
    println!(
        "wrote {} public-cloud locations to {}",
        locations.len(),
        output_path.display()
    );
    Ok(())
}

fn parse_regions(markdown: &str) -> Result<BTreeMap<String, String>> {
    let all_regions = markdown
        .split_once("#### [All]")
        .or_else(|| markdown.split_once("#### All"))
        .map(|(_, remainder)| remainder)
        .context("find the `All` regions section")?;
    let table = all_regions
        .split_once("#### [Americas]")
        .or_else(|| all_regions.split_once("#### Americas"))
        .map_or(all_regions, |(section, _)| section);
    let mut locations = BTreeMap::new();
    for line in table.lines() {
        let columns: Vec<&str> = line
            .trim()
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        if columns.len() != 6 {
            continue;
        }
        let programmatic_name = columns[5];
        if programmatic_name.is_empty()
            || programmatic_name == "Programmatic name"
            || programmatic_name.chars().all(|character| character == '-')
        {
            continue;
        }
        let display_name = strip_leading_directives(columns[0]);
        if display_name.is_empty() {
            continue;
        }
        locations.insert(
            programmatic_name.to_ascii_lowercase(),
            display_name.to_owned(),
        );
    }
    Ok(locations)
}

fn strip_leading_directives(mut value: &str) -> &str {
    value = value.trim();
    while let Some(directive) = value.strip_prefix(":::") {
        let Some(end) = directive.find(":::") else {
            break;
        };
        value = directive[end + 3..].trim_start();
    }
    value
}

fn render_locations(locations: &BTreeMap<String, String>) -> Result<String> {
    let mut output = String::from(
        "# Human display names for Azure public-cloud programmatic locations.\n\
         # Generated from Microsoft's Azure regions list; refresh with:\n\
         #   cargo run --example update_azure_locations\n\
         #\n\
         # User overrides: create <platform config dir>/azdocs/azure_locations.toml\n\
         # with the same flat `\"programmatic-name\" = \"Display Name\"` shape.\n\n",
    );
    for (programmatic_name, display_name) in locations {
        let key = serde_json::to_string(programmatic_name).context("encode location key")?;
        let value = serde_json::to_string(display_name).context("encode location name")?;
        writeln!(output, "{key} = {value}").context("render location metadata")?;
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_regions_reads_all_table_and_removes_leading_directives() {
        let markdown = "#### [All](#tab/all)\n\
            | Region | Availability zone support | Paired region | Physical location | Geography | Programmatic name |\n\
            | --- | --- | --- | --- | --- | --- |\n\
            | UK South | image | UK West | London | United Kingdom | uksouth |\n\
            | :::image source=restricted.svg::: UK West | | UK South | Cardiff | United Kingdom | ukwest |\n\
            #### [Americas](#tab/americas)\n\
            | East US | image | West US | Virginia | United States | eastus |\n";

        let locations = parse_regions(markdown).expect("fixture must parse");

        assert_eq!(
            locations,
            BTreeMap::from([
                ("uksouth".to_owned(), "UK South".to_owned()),
                ("ukwest".to_owned(), "UK West".to_owned()),
            ])
        );
    }
}

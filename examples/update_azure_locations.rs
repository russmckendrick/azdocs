//! Refreshes the two embedded region data files from Microsoft's published
//! sources: `data/azure_locations.toml` (programmatic name → display name, the
//! user-overridable map) and `data/azure_regions.toml` (the catalogue with
//! physical locations, geographies and coordinates that draws the desktop's
//! resource-locations map).
//!
//!     cargo run --example update_azure_locations            # rewrite both
//!     cargo run --example update_azure_locations -- --check # are they current?

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context, Result, ensure};

const SOURCE_URL: &str = "https://raw.githubusercontent.com/MicrosoftDocs/reliability-docs/main/articles/reliability/regions-list.md";
/// The data behind Microsoft's interactive datacenter map, which publishes a
/// latitude and longitude for every open and announced region.
const DATACENTER_URL: &str = "https://datacenters.microsoft.com/globe/data/geo/regions.json";

/// Approximate city-centre coordinates for the restricted-access regions the
/// datacenter map omits, keyed by programmatic name. The physical location
/// comes from the regions list; the coordinates are the city it names. At the
/// scale of an overview map a few kilometres are invisible, so these only
/// need refreshing if Microsoft starts publishing them.
const FALLBACK_COORDINATES: &[(&str, f64, f64)] = &[
    ("australiacentral2", -35.2809, 149.13),
    ("brazilsoutheast", -22.9068, -43.1729),
    ("francesouth", 43.2965, 5.3698),
    ("germanynorth", 52.52, 13.405),
    ("indiasouthcentral", 17.385, 78.4867),
    ("koreasouth", 35.1796, 129.0756),
    ("norwaywest", 58.97, 5.7331),
    ("southafricawest", -33.9249, 18.4241),
    ("switzerlandwest", 46.2044, 6.1432),
    ("uaecentral", 24.4539, 54.3773),
    ("westindia", 19.076, 72.8777),
];

#[derive(Debug, Clone, PartialEq)]
struct LearnRegion {
    display_name: String,
    physical_location: String,
    geography: String,
    availability_zones: bool,
    paired_region: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct CatalogueRegion {
    display_name: String,
    physical_location: String,
    geography: String,
    latitude: f64,
    longitude: f64,
    source: &'static str,
    availability_zones: bool,
    open: bool,
    paired_region: Option<String>,
    year_opened: Option<u64>,
    data_residency: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let check = std::env::args()
        .skip(1)
        .any(|argument| argument == "--check");
    // Microsoft's datacenter map sits behind a CDN that rejects anonymous
    // clients; a named user agent is enough for both sources.
    let client = reqwest::Client::builder()
        .user_agent(concat!(
            "azdocs/",
            env!("CARGO_PKG_VERSION"),
            " (update_azure_locations)"
        ))
        .build()
        .context("build HTTP client")?;
    let markdown = client
        .get(SOURCE_URL)
        .send()
        .await
        .context("download Microsoft Azure regions list")?
        .error_for_status()
        .context("Microsoft Azure regions list returned an error")?
        .text()
        .await
        .context("read Microsoft Azure regions list")?;
    let learn = parse_regions(&markdown)?;
    ensure!(
        learn.len() >= 50,
        "refusing to replace metadata with only {} parsed regions",
        learn.len()
    );
    let datacenters = client
        .get(DATACENTER_URL)
        .send()
        .await
        .context("download Microsoft datacenter map regions")?
        .error_for_status()
        .context("Microsoft datacenter map regions returned an error")?
        .json::<serde_json::Value>()
        .await
        .context("read Microsoft datacenter map regions")?;
    let catalogue = build_catalogue(&learn, &datacenters)?;
    ensure!(
        catalogue.len() >= 50,
        "refusing to replace the region catalogue with only {} regions",
        catalogue.len()
    );

    let mut locations: BTreeMap<String, String> = learn
        .iter()
        .map(|(name, region)| (name.clone(), region.display_name.clone()))
        .collect();
    for (name, region) in &catalogue {
        locations
            .entry(name.clone())
            .or_insert_with(|| region.display_name.clone());
    }
    locations.insert("global".to_owned(), "Global".to_owned());

    let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    let outputs = [
        (
            data_dir.join("azure_locations.toml"),
            render_locations(&locations)?,
        ),
        (
            data_dir.join("azure_regions.toml"),
            render_catalogue(&catalogue)?,
        ),
    ];
    if check {
        for (output_path, rendered) in &outputs {
            let current = std::fs::read_to_string(output_path)
                .with_context(|| format!("read {}", output_path.display()))?;
            ensure!(
                &current == rendered,
                "{} is stale; run `cargo run --example update_azure_locations`",
                output_path.display()
            );
            println!("{} is current", output_path.display());
        }
        return Ok(());
    }

    for (output_path, rendered) in &outputs {
        std::fs::write(output_path, rendered)
            .with_context(|| format!("write {}", output_path.display()))?;
    }
    println!(
        "wrote {} locations and {} catalogue regions to {}",
        locations.len(),
        catalogue.len(),
        data_dir.display()
    );
    Ok(())
}

/// Joins the regions list with the datacenter map: coordinates come from the
/// map, then from the fallback table; regions with neither are left to the
/// display-name file alone. Regions only the map knows (sovereign clouds,
/// announced regions) are included so a snapshot from those clouds still
/// plots.
fn build_catalogue(
    learn: &BTreeMap<String, LearnRegion>,
    datacenters: &serde_json::Value,
) -> Result<BTreeMap<String, CatalogueRegion>> {
    let entries = datacenters
        .as_array()
        .context("datacenter map regions must be a JSON array")?;
    let mut catalogue = BTreeMap::new();
    let mut mapped = BTreeMap::new();
    for entry in entries {
        let name = entry["id"]
            .as_str()
            .context("datacenter map region without an id")?
            .to_ascii_lowercase();
        let (Some(latitude), Some(longitude)) =
            (entry["latitude"].as_f64(), entry["longitude"].as_f64())
        else {
            continue;
        };
        let display_name = entry["displayName"].as_str().unwrap_or(&name).to_owned();
        let physical_location = entry["location"].as_str().unwrap_or("").to_owned();
        let geography = entry["geographyId"].as_str().unwrap_or("").to_owned();
        mapped.insert(
            name,
            CatalogueRegion {
                display_name,
                physical_location,
                geography,
                latitude,
                longitude,
                source: "datacenter-map",
                // "soon", "nearest" and "nearestSoon" all mean not here yet.
                availability_zones: entry["availabilityZonesId"].as_str() == Some("available"),
                open: entry["isOpen"].as_bool().unwrap_or(true),
                paired_region: None,
                year_opened: entry["yearOpen"].as_u64(),
                data_residency: entry["dataResidency"]
                    .as_str()
                    .filter(|text| !text.is_empty())
                    .map(str::to_owned),
            },
        );
    }
    for (name, region) in learn {
        let coordinates = mapped
            .get(name)
            .map(|found| (found.latitude, found.longitude, "datacenter-map"))
            .or_else(|| {
                FALLBACK_COORDINATES
                    .iter()
                    .find(|(fallback, _, _)| fallback == name)
                    .map(|(_, latitude, longitude)| (*latitude, *longitude, "physical-location"))
            });
        let Some((latitude, longitude, source)) = coordinates else {
            continue;
        };
        let from_map = mapped.get(name);
        catalogue.insert(
            name.clone(),
            CatalogueRegion {
                display_name: region.display_name.clone(),
                physical_location: region.physical_location.clone(),
                geography: region.geography.clone(),
                latitude,
                longitude,
                source,
                // The regions list is the documented word on zones and pairs;
                // the map adds what only it knows.
                availability_zones: region.availability_zones,
                open: from_map.is_none_or(|found| found.open),
                paired_region: region.paired_region.clone(),
                year_opened: from_map.and_then(|found| found.year_opened),
                data_residency: from_map.and_then(|found| found.data_residency.clone()),
            },
        );
    }
    for (name, region) in mapped {
        catalogue.entry(name).or_insert(region);
    }
    Ok(catalogue)
}

fn parse_regions(markdown: &str) -> Result<BTreeMap<String, LearnRegion>> {
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
        // The zone column holds a checkmark image directive when supported.
        let availability_zones = columns[1].contains("alt-text=\"Yes\"");
        let paired = strip_leading_directives(columns[2]);
        let paired_region = (!paired.is_empty() && paired != "N/A").then(|| paired.to_owned());
        locations.insert(
            programmatic_name.to_ascii_lowercase(),
            LearnRegion {
                display_name: display_name.to_owned(),
                physical_location: columns[3].to_owned(),
                geography: columns[4].to_owned(),
                availability_zones,
                paired_region,
            },
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

fn render_catalogue(catalogue: &BTreeMap<String, CatalogueRegion>) -> Result<String> {
    let mut output = String::from(
        "# Azure region catalogue: display name, physical location, geography and\n\
         # coordinates for every region azdocs can place on a map. Generated from\n\
         # Microsoft's Azure regions list and datacenter map; refresh with:\n\
         #   cargo run --example update_azure_locations\n\
         #\n\
         # `source` records where the coordinates came from: \"datacenter-map\" is\n\
         # Microsoft's published latitude/longitude; \"physical-location\" is the\n\
         # approximate centre of the city the regions list names, for the\n\
         # restricted-access regions the map omits. Regions without coordinates\n\
         # are not listed here; azure_locations.toml still names them.\n\
         # `availability_zones` and `paired_region` follow the regions list;\n\
         # `open`, `year_opened` and `data_residency` follow the datacenter map.\n",
    );
    for (name, region) in catalogue {
        let key = serde_json::to_string(name).context("encode region name")?;
        let display_name =
            serde_json::to_string(&region.display_name).context("encode region display name")?;
        let physical_location = serde_json::to_string(&region.physical_location)
            .context("encode region physical location")?;
        let geography =
            serde_json::to_string(&region.geography).context("encode region geography")?;
        write!(
            output,
            "\n[[region]]\nname = {key}\ndisplay_name = {display_name}\n\
             physical_location = {physical_location}\ngeography = {geography}\n\
             latitude = {:.4}\nlongitude = {:.4}\nsource = \"{}\"\n\
             availability_zones = {}\nopen = {}\n",
            region.latitude,
            region.longitude,
            region.source,
            region.availability_zones,
            region.open
        )
        .context("render region catalogue")?;
        if let Some(paired) = &region.paired_region {
            let paired = serde_json::to_string(paired).context("encode paired region")?;
            writeln!(output, "paired_region = {paired}").context("render region catalogue")?;
        }
        if let Some(year) = region.year_opened {
            writeln!(output, "year_opened = {year}").context("render region catalogue")?;
        }
        if let Some(residency) = &region.data_residency {
            let residency = serde_json::to_string(residency).context("encode data residency")?;
            writeln!(output, "data_residency = {residency}").context("render region catalogue")?;
        }
    }
    Ok(output)
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
            | UK South | :::image type=\"content\" source=\"media/icon-checkmark.svg\" alt-text=\"Yes\" border=\"false\"::: | UK West | London | United Kingdom | uksouth |\n\
            | :::image source=restricted.svg::: UK West | | N/A | Cardiff | United Kingdom | ukwest |\n\
            #### [Americas](#tab/americas)\n\
            | East US | image | West US | Virginia | United States | eastus |\n";

        let locations = parse_regions(markdown).expect("fixture must parse");

        assert_eq!(
            locations,
            BTreeMap::from([
                (
                    "uksouth".to_owned(),
                    LearnRegion {
                        display_name: "UK South".to_owned(),
                        physical_location: "London".to_owned(),
                        geography: "United Kingdom".to_owned(),
                        availability_zones: true,
                        paired_region: Some("UK West".to_owned()),
                    }
                ),
                (
                    "ukwest".to_owned(),
                    LearnRegion {
                        display_name: "UK West".to_owned(),
                        physical_location: "Cardiff".to_owned(),
                        geography: "United Kingdom".to_owned(),
                        availability_zones: false,
                        paired_region: None,
                    }
                ),
            ])
        );
    }

    #[test]
    fn build_catalogue_prefers_map_coordinates_and_falls_back_to_city_centres() {
        let learn = BTreeMap::from([
            (
                "uksouth".to_owned(),
                LearnRegion {
                    display_name: "UK South".to_owned(),
                    physical_location: "London".to_owned(),
                    geography: "United Kingdom".to_owned(),
                    availability_zones: true,
                    paired_region: Some("UK West".to_owned()),
                },
            ),
            (
                "koreasouth".to_owned(),
                LearnRegion {
                    display_name: "Korea South".to_owned(),
                    physical_location: "Busan".to_owned(),
                    geography: "Korea".to_owned(),
                    availability_zones: false,
                    paired_region: Some("Korea Central".to_owned()),
                },
            ),
            (
                "global".to_owned(),
                LearnRegion {
                    display_name: "Global".to_owned(),
                    physical_location: String::new(),
                    geography: String::new(),
                    availability_zones: false,
                    paired_region: None,
                },
            ),
        ]);
        let datacenters = serde_json::json!([
            { "id": "uksouth", "displayName": "UK South", "location": "London",
              "latitude": 50.941, "longitude": -0.799, "geographyId": "unitedkingdom",
              "availabilityZonesId": "available", "isOpen": true, "yearOpen": 2016,
              "dataResidency": "Stored at rest in the United Kingdom" },
            { "id": "chinanorth", "displayName": "China North", "location": "Beijing",
              "latitude": 39.9, "longitude": 116.4, "geographyId": "china",
              "availabilityZonesId": "soon", "isOpen": true, "yearOpen": 2014 }
        ]);

        let catalogue = build_catalogue(&learn, &datacenters).expect("fixture must join");

        assert_eq!(catalogue["uksouth"].latitude, 50.941);
        assert_eq!(catalogue["uksouth"].source, "datacenter-map");
        assert_eq!(catalogue["uksouth"].geography, "United Kingdom");
        assert_eq!(catalogue["uksouth"].year_opened, Some(2016));
        assert_eq!(
            catalogue["uksouth"].paired_region.as_deref(),
            Some("UK West")
        );
        assert!(catalogue["uksouth"].availability_zones);
        assert_eq!(catalogue["koreasouth"].source, "physical-location");
        assert_eq!(catalogue["koreasouth"].physical_location, "Busan");
        assert_eq!(catalogue["koreasouth"].year_opened, None);
        assert_eq!(catalogue["chinanorth"].physical_location, "Beijing");
        assert!(!catalogue["chinanorth"].availability_zones);
        assert!(catalogue["chinanorth"].data_residency.is_none());
        assert!(!catalogue.contains_key("global"));
    }
}

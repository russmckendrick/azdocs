use std::path::{Path, PathBuf};

use crate::cli::{ReportArgs, ReportFormat};
use crate::config::{BrandingConfig, Config};
use crate::error::ConfigError;
use crate::labels::fill;
use crate::report::branding::BrandingContext;
use crate::report::{self, ReportContext};
use crate::store::Store;

pub fn run(
    config: &Config,
    config_dir: Option<&Path>,
    store: &Store,
    args: &ReportArgs,
) -> anyhow::Result<()> {
    run_with_outputs(config, config_dir, store, args).map(|_| ())
}

/// Generate a report and return the primary path for every written artifact.
///
/// The CLI keeps printing its established progress messages while the desktop
/// boundary uses the returned paths to present a deterministic export receipt.
pub fn run_with_outputs(
    config: &Config,
    config_dir: Option<&Path>,
    store: &Store,
    args: &ReportArgs,
) -> anyhow::Result<Vec<PathBuf>> {
    let formats: &[ReportFormat] = match args.format {
        ReportFormat::All => &[
            ReportFormat::Md,
            ReportFormat::Html,
            ReportFormat::Csv,
            ReportFormat::Xlsx,
            ReportFormat::Pdf,
            ReportFormat::Docx,
        ],
        single => &[single],
    };
    run_selected_with_outputs(config, config_dir, store, args, formats)
}

/// Generate a caller-selected set of report formats in one composition pass.
/// Styled formats therefore share the expensive diagram build just as
/// `--format all` does, without forcing callers to request every format.
pub fn run_selected_with_outputs(
    config: &Config,
    config_dir: Option<&Path>,
    store: &Store,
    args: &ReportArgs,
    formats: &[ReportFormat],
) -> anyhow::Result<Vec<PathBuf>> {
    if formats.is_empty() {
        anyhow::bail!("select at least one report format");
    }
    if formats.contains(&ReportFormat::All) {
        anyhow::bail!("callers must expand the `all` report format");
    }
    let out_root = args.out.clone().unwrap_or_else(|| PathBuf::from("output"));
    std::fs::create_dir_all(&out_root)?;
    let snapshot_id = store.resolve_snapshot(&args.snapshot)?;
    let context = ReportContext::build(store, &snapshot_id)?;
    let branding = resolve_branding(&config.branding, config_dir, args.theme.as_deref())?;
    let resources = store.resources(&snapshot_id)?;
    let findings = store.findings(&snapshot_id)?;

    // HTML (docs site), PDF and DOCX all embed the prerendered diagram assets;
    // build them once.
    let mut diagrams = if formats.iter().any(|f| {
        matches!(
            f,
            ReportFormat::Html | ReportFormat::Pdf | ReportFormat::Docx
        )
    }) {
        crate::diagram::assets::build_overviews(
            store,
            &snapshot_id,
            &crate::diagram::DiagramScope::default(),
            &branding.labels.diagram,
        )?
    } else {
        Vec::new()
    };
    // Per-resource neighbourhood diagrams are only consumed by the print
    // formats, and cost a layout each, so the HTML-only path skips them.
    if formats
        .iter()
        .any(|f| matches!(f, ReportFormat::Pdf | ReportFormat::Docx))
    {
        diagrams.extend(crate::diagram::assets::build_resource_diagrams(
            store,
            &snapshot_id,
            &branding.labels.diagram,
        )?);
    }

    let words = &branding.labels.cli.report;
    let mut outputs = Vec::new();
    for format in formats {
        match format {
            ReportFormat::Md => {
                let out_dir = out_root.join("docs");
                report::markdown::write(&context, &branding.labels, &out_dir)?;
                let out = out_dir.join("index.md");
                println!(
                    "{}",
                    fill(&words.markdown_written, &[("path", &out.display())])
                );
                outputs.push(out);
            }
            ReportFormat::Html => {
                let out = out_root.join("report.html");
                report::html::write(&context, &branding, &out)?;
                println!("{}", fill(&words.html_written, &[("path", &out.display())]));
                outputs.push(out);
                let site_dir = out_root.join("docs-html");
                report::site::write(&context, &branding, &diagrams, &site_dir)?;
                let site_index = site_dir.join("index.html");
                println!(
                    "{}",
                    fill(&words.site_written, &[("path", &site_index.display())])
                );
                outputs.push(site_index);
            }
            ReportFormat::Csv => {
                let inventory = out_root.join("inventory.csv");
                let findings_path = out_root.join("findings.csv");
                report::csv::write_inventory(&resources, &branding.labels, &inventory)?;
                report::csv::write_findings(&findings, &branding.labels, &findings_path)?;
                println!(
                    "{}",
                    fill(
                        &words.csv_written,
                        &[
                            ("inventory", &inventory.display()),
                            ("findings", &findings_path.display()),
                        ]
                    )
                );
                outputs.push(inventory);
                outputs.push(findings_path);
            }
            ReportFormat::Xlsx => {
                let out = out_root.join("azdocs.xlsx");
                report::xlsx::write(&context, &branding, &resources, &out)?;
                println!("{}", fill(&words.xlsx_written, &[("path", &out.display())]));
                outputs.push(out);
            }
            ReportFormat::Pdf => {
                let out = out_root.join("report.pdf");
                report::pdf::write(&context, &branding, &diagrams, &out)?;
                println!("{}", fill(&words.pdf_written, &[("path", &out.display())]));
                outputs.push(out);
            }
            ReportFormat::Docx => {
                let out = out_root.join("report.docx");
                report::docx::write(&context, &branding, &diagrams, &out)?;
                println!("{}", fill(&words.docx_written, &[("path", &out.display())]));
                outputs.push(out);
            }
            ReportFormat::All => unreachable!("expanded above"),
        }
    }
    Ok(outputs)
}

fn resolve_branding(
    config: &BrandingConfig,
    config_dir: Option<&Path>,
    theme_override: Option<&str>,
) -> Result<BrandingContext, ConfigError> {
    let Some(theme) = theme_override else {
        return BrandingContext::resolve(config, config_dir);
    };

    let mut overridden = config.clone();
    overridden.theme = theme.to_owned();
    BrandingContext::resolve(&overridden, config_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_resolve_branding_prefers_cli_theme_when_present() {
        let config = BrandingConfig {
            theme: "missing-theme".to_owned(),
            ..BrandingConfig::default()
        };

        let branding = resolve_branding(&config, None, Some("field-report")).unwrap();

        assert_eq!(branding.tokens.name, "field-report");
    }
}

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
    run_selected_with_progress(config, config_dir, store, args, formats, |_| {})
}

/// Notify the caller before composing each artifact, including companions.
/// A PDF reference can take much longer than the assessment; the desktop must
/// identify that stage while both use this same offline export path.
pub fn run_selected_with_progress(
    config: &Config,
    config_dir: Option<&Path>,
    store: &Store,
    args: &ReportArgs,
    formats: &[ReportFormat],
    mut on_artifact: impl FnMut(&Path),
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
    let report_scope = args.scope();
    // Screenshot bytes are only read by the formats that draw them; the data
    // exports never touch the blobs.
    let include_images = formats.iter().any(|f| {
        matches!(
            f,
            ReportFormat::Md | ReportFormat::Html | ReportFormat::Pdf | ReportFormat::Docx
        )
    });
    let context = ReportContext::build_scoped(store, &snapshot_id, &report_scope, include_images)?;
    let branding = resolve_branding(&config.branding, config_dir, args.theme.as_deref())?;
    let resources = scoped_resources(store, &snapshot_id, &report_scope)?;
    let findings: Vec<_> = context
        .findings
        .iter()
        .map(|row| crate::model::Finding {
            query_name: row.query_name.clone(),
            category: row.category.clone(),
            severity: crate::model::Severity::parse(&row.severity)
                .unwrap_or(crate::model::Severity::Info),
            resource_id: row.resource_id.clone(),
            title: row.title.clone(),
            detail: None,
        })
        .collect();

    let print_selected = formats
        .iter()
        .any(|f| matches!(f, ReportFormat::Pdf | ReportFormat::Docx));
    let scope = crate::diagram::DiagramScope::from(&report_scope);
    let diagrams = if formats
        .iter()
        .any(|f| matches!(f, ReportFormat::Html | ReportFormat::Md))
    {
        crate::diagram::assets::build_overviews(
            store,
            &snapshot_id,
            &scope,
            &branding.labels.diagram,
        )?
    } else if print_selected && args.include_reference {
        crate::diagram::assets::build_group_summaries(
            store,
            &snapshot_id,
            &scope,
            &branding.labels.diagram,
        )?
    } else {
        Vec::new()
    };
    let assessment_diagrams = if print_selected {
        crate::diagram::assets::build_assessment(&context.analysis, &branding.labels)
    } else {
        Vec::new()
    };

    let words = &branding.labels.cli.report;
    let mut outputs = Vec::new();
    for format in formats {
        match format {
            ReportFormat::Md => {
                let out_dir = out_root.join("docs");
                let out = out_dir.join("index.md");
                on_artifact(&out);
                report::markdown::write(&context, &branding.labels, &diagrams, &out_dir)?;
                println!(
                    "{}",
                    fill(&words.markdown_written, &[("path", &out.display())])
                );
                outputs.push(out);
            }
            ReportFormat::Html => {
                let out = out_root.join("report.html");
                on_artifact(&out);
                report::html::write(&context, &branding, &diagrams, &out)?;
                println!("{}", fill(&words.html_written, &[("path", &out.display())]));
                outputs.push(out);
                let site_dir = out_root.join("docs-html");
                let site_index = site_dir.join("index.html");
                on_artifact(&site_index);
                report::site::write(&context, &branding, &diagrams, &site_dir)?;
                println!(
                    "{}",
                    fill(&words.site_written, &[("path", &site_index.display())])
                );
                outputs.push(site_index);
            }
            ReportFormat::Csv => {
                let inventory = out_root.join("inventory.csv");
                let findings_path = out_root.join("findings.csv");
                on_artifact(&inventory);
                report::csv::write_inventory(&resources, &branding.labels, &inventory)?;
                on_artifact(&findings_path);
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
                on_artifact(&out);
                report::xlsx::write(&context, &branding, &resources, &out)?;
                println!("{}", fill(&words.xlsx_written, &[("path", &out.display())]));
                outputs.push(out);
            }
            ReportFormat::Pdf => {
                let out = out_root.join("report.pdf");
                on_artifact(&out);
                report::pdf::write(&context, &branding, &assessment_diagrams, &out)?;
                println!("{}", fill(&words.pdf_written, &[("path", &out.display())]));
                outputs.push(out);
                if args.include_reference {
                    let out = out_root.join("technical-reference.pdf");
                    on_artifact(&out);
                    std::fs::write(
                        &out,
                        report::pdf::render_reference(&context, &branding, &diagrams)?,
                    )?;
                    println!("{}", fill(&words.pdf_written, &[("path", &out.display())]));
                    outputs.push(out);
                }
            }
            ReportFormat::Docx => {
                let out = out_root.join("report.docx");
                on_artifact(&out);
                report::docx::write(&context, &branding, &assessment_diagrams, &out)?;
                println!("{}", fill(&words.docx_written, &[("path", &out.display())]));
                outputs.push(out);
                if args.include_reference {
                    let out = out_root.join("technical-reference.docx");
                    on_artifact(&out);
                    std::fs::write(
                        &out,
                        report::docx::render_reference(&context, &branding, &diagrams)?,
                    )?;
                    println!("{}", fill(&words.docx_written, &[("path", &out.display())]));
                    outputs.push(out);
                }
            }
            ReportFormat::All => unreachable!("expanded above"),
        }
    }
    Ok(outputs)
}

/// The resources the scoped context describes, in store order, for the flat
/// exports that take the model rows rather than the report rows.
fn scoped_resources(
    store: &Store,
    snapshot_id: &str,
    scope: &report::ReportScope,
) -> anyhow::Result<Vec<crate::model::Resource>> {
    let mut resources = store.resources(snapshot_id)?;
    if !scope.is_unscoped() {
        resources.retain(|resource| {
            scope
                .subscription
                .as_ref()
                .is_none_or(|wanted| wanted.eq_ignore_ascii_case(&resource.subscription_id))
                && scope.resource_group.as_ref().is_none_or(|wanted| {
                    resource
                        .resource_group
                        .as_deref()
                        .is_some_and(|group| wanted.eq_ignore_ascii_case(group))
                })
        });
    }
    Ok(resources)
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

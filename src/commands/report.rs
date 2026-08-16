use std::path::{Path, PathBuf};

use crate::cli::{ReportArgs, ReportFormat};
use crate::config::Config;
use crate::report::branding::BrandingContext;
use crate::report::{self, ReportContext};
use crate::store::Store;

pub fn run(
    config: &Config,
    config_dir: Option<&Path>,
    store: &Store,
    args: &ReportArgs,
) -> anyhow::Result<()> {
    let out_root = args.out.clone().unwrap_or_else(|| PathBuf::from("output"));
    std::fs::create_dir_all(&out_root)?;
    let snapshot_id = store.resolve_snapshot(&args.snapshot)?;
    let context = ReportContext::build(store, &snapshot_id)?;
    let branding = BrandingContext::resolve(&config.branding, config_dir)?;
    let resources = store.resources(&snapshot_id)?;
    let findings = store.findings(&snapshot_id)?;

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
        )?);
    }

    for format in formats {
        match format {
            ReportFormat::Md => {
                let out_dir = out_root.join("docs");
                report::markdown::write(&context, &out_dir)?;
                println!("Markdown docs -> {}", out_dir.join("index.md").display());
            }
            ReportFormat::Html => {
                let out = out_root.join("report.html");
                report::html::write(&context, &branding, &out)?;
                println!("HTML report -> {}", out.display());
                let site_dir = out_root.join("docs-html");
                report::site::write(&context, &branding, &diagrams, &site_dir)?;
                println!("HTML docs -> {}", site_dir.join("index.html").display());
            }
            ReportFormat::Csv => {
                let inventory = out_root.join("inventory.csv");
                let findings_path = out_root.join("findings.csv");
                report::csv::write_inventory(&resources, &inventory)?;
                report::csv::write_findings(&findings, &findings_path)?;
                println!(
                    "CSV -> {} + {}",
                    inventory.display(),
                    findings_path.display()
                );
            }
            ReportFormat::Xlsx => {
                let out = out_root.join("azdocs.xlsx");
                report::xlsx::write(&context, &branding, &resources, &out)?;
                println!("XLSX workbook -> {}", out.display());
            }
            ReportFormat::Pdf => {
                let out = out_root.join("report.pdf");
                report::pdf::write(&context, &branding, &diagrams, &out)?;
                println!("PDF report -> {}", out.display());
            }
            ReportFormat::Docx => {
                let out = out_root.join("report.docx");
                report::docx::write(&context, &branding, &diagrams, &out)?;
                println!("DOCX report -> {}", out.display());
            }
            ReportFormat::All => unreachable!("expanded above"),
        }
    }
    Ok(())
}

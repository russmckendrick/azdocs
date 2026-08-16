use std::path::PathBuf;

use crate::cli::{ReportArgs, ReportFormat};
use crate::report::{self, ReportContext};
use crate::store::Store;

pub fn run(store: &Store, args: &ReportArgs) -> anyhow::Result<()> {
    let out_root = args.out.clone().unwrap_or_else(|| PathBuf::from("output"));
    std::fs::create_dir_all(&out_root)?;
    let snapshot_id = store.resolve_snapshot(&args.snapshot)?;
    let context = ReportContext::build(store, &snapshot_id)?;
    let resources = store.resources(&snapshot_id)?;
    let findings = store.findings(&snapshot_id)?;

    let formats: &[ReportFormat] = match args.format {
        ReportFormat::All => &[
            ReportFormat::Md,
            ReportFormat::Html,
            ReportFormat::Csv,
            ReportFormat::Xlsx,
        ],
        single => &[single],
    };

    for format in formats {
        match format {
            ReportFormat::Md => {
                let out_dir = out_root.join("docs");
                report::markdown::write(&context, &out_dir)?;
                println!("Markdown docs -> {}", out_dir.join("index.md").display());
            }
            ReportFormat::Html => {
                let out = out_root.join("report.html");
                report::html::write(&context, &out)?;
                println!("HTML report -> {}", out.display());
                let diagrams = crate::diagram::assets::build_all(
                    store,
                    &snapshot_id,
                    &crate::diagram::DiagramScope::default(),
                )?;
                let site_dir = out_root.join("docs-html");
                report::site::write(&context, &diagrams, &site_dir)?;
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
                report::xlsx::write(&context, &resources, &out)?;
                println!("XLSX workbook -> {}", out.display());
            }
            ReportFormat::All => unreachable!("expanded above"),
        }
    }
    Ok(())
}

use std::path::{Path, PathBuf};

use crate::cli::{ReportArgs, ReportFormat};
use crate::report::{self, ReportContext};
use crate::store::Store;

pub fn run(store: &Store, args: &ReportArgs) -> anyhow::Result<()> {
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
                let out_dir = args.out.clone().unwrap_or_else(|| PathBuf::from("docs"));
                report::markdown::write(&context, &out_dir)?;
                println!("Markdown report -> {}", out_dir.join("index.md").display());
            }
            ReportFormat::Html => {
                let out = single_file_path(args.out.as_deref(), "report.html");
                report::html::write(&context, &out)?;
                println!("HTML report -> {}", out.display());
            }
            ReportFormat::Csv => {
                let inventory = single_file_path(args.out.as_deref(), "inventory.csv");
                let findings_path = single_file_path(args.out.as_deref(), "findings.csv");
                report::csv::write_inventory(&resources, &inventory)?;
                report::csv::write_findings(&findings, &findings_path)?;
                println!(
                    "CSV -> {} + {}",
                    inventory.display(),
                    findings_path.display()
                );
            }
            ReportFormat::Xlsx => {
                let out = single_file_path(args.out.as_deref(), "azdocs.xlsx");
                report::xlsx::write(&context, &resources, &out)?;
                println!("XLSX workbook -> {}", out.display());
            }
            ReportFormat::All => unreachable!("expanded above"),
        }
    }
    Ok(())
}

/// Single-file outputs land in `--out` (treated as a directory) or the cwd.
fn single_file_path(out: Option<&Path>, file_name: &str) -> PathBuf {
    match out {
        Some(dir) => dir.join(file_name),
        None => PathBuf::from(file_name),
    }
}

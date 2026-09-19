//! The "changes since the previous snapshot" chapter renders in every format
//! once the tenant has two usable snapshots, and stays out otherwise.

mod common;

use std::io::Read;

use azdocs::model::{Finding, Severity};
use azdocs::report::branding::BrandingContext;
use azdocs::report::{ReportContext, csv, docx, html, markdown, xlsx};
use azdocs::store::Store;

fn archive_entry(bytes: &[u8], name: &str) -> String {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut entry = archive.by_name(name).unwrap();
    let mut content = String::new();
    entry.read_to_string(&mut content).unwrap();
    content
}

/// Two identical collections, then one extra finding on the newer so the
/// comparison has something to list.
fn two_snapshots() -> (Store, String) {
    let store = Store::open_in_memory().unwrap();
    let _older = common::seed_estate(&store);
    let newer = common::seed_estate(&store);
    store
        .insert_findings(
            &newer,
            &[Finding {
                query_name: "storage_shared_key_access".into(),
                category: "security".into(),
                severity: Severity::High,
                resource_id: Some(
                    "/subscriptions/sub-prod/resourcegroups/rg-app/providers/microsoft.storage/storageaccounts/stprodapp01"
                        .into(),
                ),
                title: "stprodapp01 allows shared key access".into(),
                detail: None,
            }],
        )
        .unwrap();
    (store, newer)
}

#[test]
fn unit_context_carries_changes_and_trend_when_a_previous_snapshot_exists() {
    let (store, newer) = two_snapshots();
    let report = ReportContext::build(&store, &newer).unwrap();
    let changes = report.changes.as_ref().expect("previous snapshot found");
    assert_eq!(changes.findings.added.len(), 1);
    assert!(changes.resources.added.is_empty());
    assert_eq!(report.trend.len(), 2);
}

#[test]
fn unit_context_has_no_changes_for_the_earliest_snapshot() {
    let store = Store::open_in_memory().unwrap();
    let only = common::seed_estate(&store);
    let report = ReportContext::build(&store, &only).unwrap();
    assert!(report.changes.is_none());
    assert_eq!(report.trend.len(), 1);
}

#[test]
fn unit_every_format_renders_the_changes_chapter() {
    let (store, newer) = two_snapshots();
    let report = ReportContext::build(&store, &newer).unwrap();
    let branding = BrandingContext::default();
    let words = &branding.labels.report.changes;
    let dir = tempfile::tempdir().unwrap();

    let pages = markdown::render_pages(
        &report,
        &branding.labels,
        &[],
        markdown::DiagramEmbedding { mermaid: false },
    )
    .unwrap();
    let index = &pages.iter().find(|(path, _)| path == "index.md").unwrap().1;
    assert!(index.contains(&format!("## {}", words.chapter)), "{index}");
    assert!(index.contains(&format!("### {}", words.new_findings)));
    assert!(index.contains(&format!("### {}", words.trend)));

    let page = html::render(&report, &branding, &[]).unwrap();
    assert!(page.contains(&format!("<h2>{}</h2>", words.chapter)));
    assert!(page.contains("stprodapp01 allows shared key access"));

    let out = dir.path().join("azdocs.xlsx");
    let resources = store.resources(&newer).unwrap();
    xlsx::write(&report, &branding, &resources, &out).unwrap();
    let workbook = archive_entry(&std::fs::read(&out).unwrap(), "xl/workbook.xml");
    for sheet in ["Changes", "Trend", "Locations", "Websites"] {
        assert!(
            workbook.contains(&format!("name=\"{sheet}\"")),
            "workbook is missing the {sheet} sheet"
        );
    }

    let bytes = docx::render(&report, &branding, &[]).unwrap();
    let document = archive_entry(&bytes, "word/document.xml");
    assert!(document.contains(words.chapter.as_str()));
    assert!(document.contains(words.new_findings.as_str()));
}

#[test]
fn unit_csv_tables_cover_every_query_and_the_governance_analysis() {
    let store = Store::open_in_memory().unwrap();
    let id = common::seed_estate(&store);
    let report = ReportContext::build(&store, &id).unwrap();
    let dir = tempfile::tempdir().unwrap();

    let written =
        csv::write_tables(&report, &azdocs::labels::Labels::default(), dir.path()).unwrap();

    let queries: usize = report.categories.iter().map(|c| c.queries.len()).sum();
    assert!(written >= queries + 3);
    assert!(
        dir.path()
            .join("queries/networking/virtual_networks.csv")
            .exists()
    );
    let groups = std::fs::read_to_string(dir.path().join("governance-groups.csv")).unwrap();
    assert!(groups.lines().count() > 1, "{groups}");
    assert!(dir.path().join("posture").is_dir());
}

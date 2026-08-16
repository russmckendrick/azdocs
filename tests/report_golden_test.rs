mod common;

use azdocs::report::{ReportContext, csv, html, markdown, xlsx};
use azdocs::store::Store;

fn insta_settings() -> insta::Settings {
    let mut settings = insta::Settings::clone_current();
    settings.add_filter(
        r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}",
        "[snapshot-id]",
    );
    settings.add_filter(
        r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(\+|-)\d{2}:\d{2}",
        "[timestamp]",
    );
    settings
}

fn seeded_context() -> (Store, String) {
    let store = Store::open_in_memory().unwrap();
    let id = common::seed_estate(&store);
    (store, id)
}

#[test]
fn markdown_pages_match_golden_files() {
    let (store, id) = seeded_context();
    let report = ReportContext::build(&store, &id).unwrap();
    let dir = tempfile::tempdir().unwrap();

    markdown::write(&report, dir.path()).unwrap();

    insta_settings().bind(|| {
        for page in [
            "index.md",
            "findings.md",
            "networking.md",
            "subscriptions/production.md",
            "subscriptions/development.md",
            "resources/production/rg-app.md",
        ] {
            let content = std::fs::read_to_string(dir.path().join(page))
                .unwrap_or_else(|_| panic!("missing page {page}"));
            insta::assert_snapshot!(page.replace('/', "_"), content);
        }
    });
}

#[test]
fn html_report_matches_golden_file() {
    let (store, id) = seeded_context();
    let report = ReportContext::build(&store, &id).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("report.html");

    html::write(&report, &out).unwrap();

    insta_settings().bind(|| {
        insta::assert_snapshot!("report_html", std::fs::read_to_string(&out).unwrap());
    });
}

#[test]
fn inventory_csv_has_row_per_resource() {
    let (store, id) = seeded_context();
    let resources = store.resources(&id).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("inventory.csv");

    csv::write_inventory(&resources, &out).unwrap();

    let content = std::fs::read_to_string(&out).unwrap();
    assert_eq!(content.lines().count(), resources.len() + 1);
}

#[test]
fn findings_csv_orders_high_severity_first() {
    let (store, id) = seeded_context();
    let findings = store.findings(&id).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("findings.csv");

    csv::write_findings(&findings, &out).unwrap();

    let content = std::fs::read_to_string(&out).unwrap();
    let second_line = content.lines().nth(1).unwrap();
    assert!(second_line.starts_with("high,"), "got: {second_line}");
}

#[test]
fn xlsx_workbook_writes_all_sheets() {
    let (store, id) = seeded_context();
    let report = ReportContext::build(&store, &id).unwrap();
    let resources = store.resources(&id).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("azdocs.xlsx");

    xlsx::write(&report, &resources, &out).unwrap();

    let size = std::fs::metadata(&out).unwrap().len();
    assert!(size > 4096, "workbook suspiciously small: {size} bytes");
}

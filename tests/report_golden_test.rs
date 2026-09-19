mod common;

use azdocs::config::BrandingConfig;
use azdocs::labels::Labels;
use azdocs::report::branding::BrandingContext;
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

/// The current fixture with its older sibling, so every golden shows the
/// "changes since the previous snapshot" section as well as the estate.
fn seeded_context() -> (Store, String) {
    let store = Store::open_in_memory().unwrap();
    let id = common::seed_history(&store);
    (store, id)
}

#[test]
fn markdown_pages_match_golden_files() {
    let (store, id) = seeded_context();
    let report = ReportContext::build(&store, &id).unwrap();

    let pages = markdown::render_pages(
        &report,
        &Labels::default(),
        &[],
        markdown::DiagramEmbedding { mermaid: true },
    )
    .unwrap();

    insta_settings().bind(|| {
        // Every page, so a new category or group cannot ship unreviewed.
        for (path, content) in &pages {
            insta::assert_snapshot!(path.replace('/', "_"), content);
        }
    });
}

#[test]
fn site_pages_match_golden_files() {
    let (store, id) = seeded_context();
    let report = ReportContext::build(&store, &id).unwrap();
    let dir = tempfile::tempdir().unwrap();

    azdocs::report::site::write(&report, &BrandingContext::default(), &[], dir.path()).unwrap();

    let mut settings = insta_settings();
    settings.add_filter(r"data:[a-z]+/[a-z+]+;base64,[A-Za-z0-9+/=]+", "data:[uri]");
    settings.bind(|| {
        for page in ["index.html", "resources/production/rg-app.html"] {
            let content = std::fs::read_to_string(dir.path().join(page))
                .unwrap_or_else(|_| panic!("missing page {page}"));
            insta::assert_snapshot!(format!("site_{}", page.replace('/', "_")), content);
        }
    });
}

#[test]
fn csv_exports_match_golden_files() {
    let (store, id) = seeded_context();
    let report = ReportContext::build(&store, &id).unwrap();
    let resources = store.resources(&id).unwrap();
    let findings = store.findings(&id).unwrap();
    let dir = tempfile::tempdir().unwrap();

    csv::write_inventory(
        &resources,
        &Labels::default(),
        &dir.path().join("inventory.csv"),
    )
    .unwrap();
    csv::write_findings(
        &findings,
        &Labels::default(),
        &dir.path().join("findings.csv"),
    )
    .unwrap();
    csv::write_tables(&report, &Labels::default(), dir.path()).unwrap();

    insta_settings().bind(|| {
        for file in ["inventory.csv", "findings.csv", "governance-groups.csv"] {
            insta::assert_snapshot!(
                format!("csv_{}", file.replace('.', "_")),
                std::fs::read_to_string(dir.path().join(file)).unwrap()
            );
        }
    });
}

/// The workbook's shape: every sheet in order with its row count. Cell
/// bytes are covered by the determinism test; the shape is what a reviewer
/// wants to see change.
#[test]
fn xlsx_structure_matches_golden_file() {
    let (store, id) = seeded_context();
    let report = ReportContext::build(&store, &id).unwrap();
    let resources = store.resources(&id).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("azdocs.xlsx");

    xlsx::write(&report, &BrandingContext::default(), &resources, &out).unwrap();

    let bytes = std::fs::read(&out).unwrap();
    let workbook = xlsx_part(&bytes, "xl/workbook.xml");
    let names: Vec<String> = workbook
        .split("<sheet ")
        .skip(1)
        .filter_map(|sheet| sheet.split("name=\"").nth(1))
        .filter_map(|rest| rest.split('"').next())
        .map(str::to_owned)
        .collect();
    let mut structure = String::new();
    for (index, name) in names.iter().enumerate() {
        let sheet = xlsx_part(&bytes, &format!("xl/worksheets/sheet{}.xml", index + 1));
        structure.push_str(&format!(
            "{name}: {} rows\n",
            sheet.matches("<row ").count()
        ));
    }
    insta::assert_snapshot!("xlsx_structure", structure);
}

#[test]
fn html_report_matches_golden_file() {
    let (store, id) = seeded_context();
    let report = ReportContext::build(&store, &id).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("report.html");

    html::write(&report, &BrandingContext::default(), &[], &out).unwrap();

    insta_settings().bind(|| {
        insta::assert_snapshot!("report_html", std::fs::read_to_string(&out).unwrap());
    });
}

#[test]
fn html_report_applies_custom_branding() {
    let (store, id) = seeded_context();
    let report = ReportContext::build(&store, &id).unwrap();
    let config = BrandingConfig {
        company: "Contoso Ltd".to_owned(),
        title: "Contoso Cloud Review".to_owned(),
        subtitle: "Quarterly estate audit".to_owned(),
        primary_color: "#112233".to_owned(),
        footer: "Contoso confidential".to_owned(),
        ..BrandingConfig::default()
    };
    let branding = BrandingContext::resolve(&config, None).unwrap();

    let rendered = html::render(&report, &branding, &[]).unwrap();

    assert!(rendered.contains("--accent:#112233"), "custom accent color");
    assert!(
        rendered.contains("<h1>Contoso Cloud Review</h1>"),
        "custom title"
    );
    assert!(rendered.contains("Quarterly estate audit"), "subtitle");
    assert!(rendered.contains("Contoso confidential"), "footer");
}

#[test]
fn inventory_csv_has_row_per_resource() {
    let (store, id) = seeded_context();
    let resources = store.resources(&id).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("inventory.csv");

    csv::write_inventory(&resources, &Labels::default(), &out).unwrap();

    let content = std::fs::read_to_string(&out).unwrap();
    assert_eq!(content.lines().count(), resources.len() + 1);
}

#[test]
fn findings_csv_orders_high_severity_first() {
    let (store, id) = seeded_context();
    let findings = store.findings(&id).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("findings.csv");

    csv::write_findings(&findings, &Labels::default(), &out).unwrap();

    let content = std::fs::read_to_string(&out).unwrap();
    let second_line = content.lines().nth(1).unwrap();
    assert!(second_line.starts_with("high,"), "got: {second_line}");
}

/// One entry of a written workbook, as text.
fn xlsx_part(bytes: &[u8], name: &str) -> String {
    use std::io::Read;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut entry = archive.by_name(name).unwrap();
    let mut content = String::new();
    entry.read_to_string(&mut content).unwrap();
    content
}

#[test]
fn xlsx_workbook_writes_all_sheets() {
    let (store, id) = seeded_context();
    let report = ReportContext::build(&store, &id).unwrap();
    let resources = store.resources(&id).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("azdocs.xlsx");

    xlsx::write(&report, &BrandingContext::default(), &resources, &out).unwrap();

    let bytes = std::fs::read(&out).unwrap();
    let workbook = xlsx_part(&bytes, "xl/workbook.xml");
    for sheet in [
        "Summary",
        "Inventory",
        "Findings",
        "Governance",
        "networking queries",
    ] {
        assert!(
            workbook.contains(&format!("name=\"{sheet}\"")),
            "workbook is missing the {sheet} sheet"
        );
    }
}

/// The workbook carries the same governance analysis as the printed report,
/// not a second one computed here.
#[test]
fn xlsx_governance_sheet_carries_the_worst_groups() {
    let (store, id) = seeded_context();
    let report = ReportContext::build(&store, &id).unwrap();
    let resources = store.resources(&id).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("azdocs.xlsx");

    xlsx::write(&report, &BrandingContext::default(), &resources, &out).unwrap();

    let strings = xlsx_part(&std::fs::read(&out).unwrap(), "xl/sharedStrings.xml");
    for expected in [
        "Distinct keys",
        "Share of tagged %",
        "below threshold",
        "Non-compliant",
        "Missed tags",
    ] {
        assert!(
            strings.contains(expected),
            "governance sheet is missing {expected:?}"
        );
    }
    let worst = &report.governance.worst_groups[0];
    assert!(worst.flagged || !worst.missed_tags.is_empty());
    assert!(
        strings.contains(worst.name.as_str()),
        "governance sheet is missing the worst group {}",
        worst.name
    );
}

#[test]
fn unit_microsoft_summary_escapes_service_text_in_markdown_and_html() {
    let (store, id) = seeded_context();
    let mut report = ReportContext::build(&store, &id).unwrap();
    let rows = std::collections::BTreeMap::from([(
        "advisor_cost_recommendations".into(),
        vec![serde_json::json!({
            "currency":"<script>alert(1)</script>", "savingsPeriod":"[click](javascript:alert(1))", "savingsAmount":12,
        })],
    )]);
    report.posture = azdocs::report::posture::PostureReport::build(
        &rows,
        &report.analysis.query_runs,
        "2026-09-13T12:00:00Z".parse().unwrap(),
        &[],
    );
    let pages = markdown::render_pages(
        &report,
        &Labels::default(),
        &[],
        markdown::DiagramEmbedding { mermaid: true },
    )
    .unwrap();
    let index = &pages.iter().find(|(path, _)| path == "index.md").unwrap().1;
    assert!(index.contains("&lt;script&gt;"));
    assert!(index.contains("\\[click\\]"));
    let html = html::render(&report, &BrandingContext::default(), &[]).unwrap();
    assert!(!html.contains("<script>alert(1)</script>"));
    assert!(html.contains("&lt;script&gt;"));
}

mod common;

use std::io::Read;

use azdocs::model::{Finding, QueryRun, Severity};
use azdocs::report::branding::BrandingContext;
use azdocs::report::{ReportContext, docx, pdf};
use azdocs::store::Store;
use quick_xml::events::Event;
use serde_json::json;

fn word_xml(bytes: &[u8]) -> String {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    xml
}

fn table_rows(xml: &str) -> Vec<Vec<String>> {
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut in_text = false;
    loop {
        match reader.read_event().unwrap() {
            Event::Start(e) => match e.name().as_ref() {
                "w:tr" => row.clear(),
                "w:tc" => cell.clear(),
                "w:t" => in_text = true,
                _ => {}
            },
            Event::Text(e) if in_text => cell.push_str(e.as_ref()),
            Event::End(e) => match e.name().as_ref() {
                "w:t" => in_text = false,
                "w:tc" => row.push(cell.replace('\u{200b}', "")),
                "w:tr" => rows.push(std::mem::take(&mut row)),
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }
    rows
}

#[test]
fn reference_groups_repeated_findings_without_losing_severity_or_scope() {
    let store = Store::open_in_memory().unwrap();
    let snapshot = common::seed_estate(&store);
    let resource_id = store.resources(&snapshot).unwrap()[0].id.clone();
    let mut findings = Vec::new();
    for (severity, id, count) in [
        (Severity::High, Some(resource_id.clone()), 124),
        (Severity::Low, Some(resource_id), 3),
        (Severity::Info, None, 2),
        (
            Severity::Medium,
            Some(
                "/subscriptions/missing/resourcegroups/missing/providers/example/type/absent"
                    .into(),
            ),
            4,
        ),
    ] {
        findings.extend((0..count).map(|_| Finding {
            query_name: "repeat_review".into(),
            category: "custom".into(),
            severity,
            resource_id: id.clone(),
            title: "original repetitive finding".into(),
            detail: Some(json!({"raw_finding_payload": "evidence".repeat(1000)})),
        }));
    }
    store.insert_findings(&snapshot, &findings).unwrap();
    let report = ReportContext::build(&store, &snapshot).unwrap();
    let xml = word_xml(&docx::render_reference(&report, &BrandingContext::default(), &[]).unwrap());
    let start = xml.rfind("repeat_review").unwrap();
    let start = start + xml[start..].find("<w:tbl>").unwrap();
    let end = start + xml[start..].find("</w:tbl>").unwrap() + "</w:tbl>".len();
    let rows = table_rows(&xml[start..end]);
    let rows: Vec<_> = rows
        .iter()
        .filter(|row| row.len() == 4 && row[3].parse::<usize>().is_ok())
        .collect();
    assert_eq!(rows.len(), 4, "one row per resource and severity");
    assert_eq!(
        rows.iter()
            .map(|row| row[3].parse::<usize>().unwrap())
            .sum::<usize>(),
        findings.len()
    );
    for (severity, count) in [
        ("High", "124"),
        ("Low", "3"),
        ("Info", "2"),
        ("Medium", "4"),
    ] {
        assert!(rows.iter().any(|row| row[2] == severity && row[3] == count));
    }
    assert!(rows.iter().any(|row| row[1] == "Scope unavailable"));
    assert!(
        rows.iter()
            .any(|row| row[1].contains("/subscriptions/missing"))
    );
    assert!(!xml.contains("raw_finding_payload"));
    assert!(!xml.contains("original repetitive finding"));
    assert!(store.findings(&snapshot).unwrap().iter().any(|finding| {
        finding
            .detail
            .as_ref()
            .is_some_and(|detail| detail.get("raw_finding_payload").is_some())
    }));
}

#[test]
fn reference_omits_large_evidence_catalogues_and_preserves_collection_uncertainty() {
    let store = Store::open_in_memory().unwrap();
    let snapshot = common::seed_estate(&store);
    let rows = vec![
        json!({
            "raw_catalogue_payload": "policy definition ".repeat(1000),
            "documentation": "https://example.com/policy",
        });
        2048
    ];
    store
        .insert_query_results(&snapshot, "legacy_catalogue", &rows)
        .unwrap();
    let mut report = ReportContext::build(&store, &snapshot).unwrap();
    report.analysis.query_runs = [
        ("completed", Some(5), None),
        ("empty", Some(0), None),
        (
            "failed",
            None,
            Some("Collection permission denied".to_owned()),
        ),
        ("unknown", None, None),
    ]
    .into_iter()
    .map(|(name, count, error)| QueryRun {
        provenance: None,
        query_name: name.into(),
        category: "custom".into(),
        row_count: count,
        duration_ms: None,
        error,
        rows_dropped: None,
    })
    .collect();
    let unknown = report.analysis.recorded_queries.len() + 1;
    let branding = BrandingContext::default();
    let xml = word_xml(&docx::render_reference(&report, &branding, &[]).unwrap());
    assert!(
        !xml.contains("raw_catalogue_payload"),
        "catalogues must not enter the print composition"
    );
    let bytes = pdf::render_reference(&report, &branding, &[]).unwrap();
    let document = lopdf::Document::load_mem(&bytes).unwrap();
    let pages: Vec<_> = document.get_pages().keys().copied().collect();
    let text = document.extract_text(&pages).unwrap();
    for content in [&xml, &text] {
        let content = content.replace('\u{200b}', "");
        let content = content.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(!content.contains("Evidence appendix"));
        assert!(!content.contains("raw_catalogue_payload"));
        assert!(
            content.contains("2 queries have successful run records, including 1 with zero rows")
        );
        assert!(content.contains(&format!("1 failed and {unknown} have no recorded outcome")));
        assert!(content.contains("Collection permission denied"));
        for resource in report.analysis.resources.values() {
            assert!(content.contains(&resource.name));
        }
    }
    assert!(
        pages.len() < 50,
        "a small estate must not expand with its provider catalogue"
    );
    assert_eq!(
        store.query_results(&snapshot, "legacy_catalogue").unwrap(),
        rows
    );
}

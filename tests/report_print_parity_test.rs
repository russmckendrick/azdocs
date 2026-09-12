mod common;

use std::io::Read;

use azdocs::diagram::assets::DiagramAsset;
use azdocs::report::branding::BrandingContext;
use azdocs::report::{ReportContext, docx, pdf};
use azdocs::store::Store;

fn seeded() -> (ReportContext, Vec<DiagramAsset>) {
    let store = Store::open_in_memory().unwrap();
    let id = common::seed_estate(&store);
    let report = ReportContext::build(&store, &id).unwrap();
    let diagrams = azdocs::diagram::assets::build_assessment(
        &report.analysis,
        &azdocs::labels::Labels::default(),
    );
    (report, diagrams)
}

fn pdf_text(bytes: &[u8]) -> String {
    let document = lopdf::Document::load_mem(bytes).unwrap();
    let pages: Vec<u32> = document.get_pages().keys().copied().collect();
    document.extract_text(&pages).unwrap()
}

fn document_xml(bytes: &[u8]) -> String {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut entry = archive.by_name("word/document.xml").unwrap();
    let mut content = String::new();
    entry.read_to_string(&mut content).unwrap();
    content
}

fn assert_ordered(text: &str, markers: &[&str], format: &str) {
    let mut cursor = 0;
    for marker in markers {
        let offset = text[cursor..]
            .find(marker)
            .unwrap_or_else(|| panic!("{format} marker {marker:?} missing after byte {cursor}"));
        cursor += offset + marker.len();
    }
}

#[test]
fn native_print_formats_share_main_and_reference_content() {
    let (report, diagrams) = seeded();
    let branding = BrandingContext::default();
    let main_pdf = pdf_text(&pdf::render(&report, &branding, &diagrams).unwrap());
    let main_word = document_xml(&docx::render(&report, &branding, &diagrams).unwrap());
    let w = &branding.labels.report.assessment;
    let markers = [
        &w.executive,
        &w.composition,
        &w.architecture,
        &w.profiles,
        &w.security,
        &w.governance,
        &w.actions,
        &w.coverage,
    ];
    for (text, format) in [(&main_pdf, "PDF"), (&main_word, "DOCX")] {
        assert_ordered(
            text,
            &markers.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            format,
        );
        assert!(!text.contains("Resources by type"));
        assert!(!text.contains("hostPoolType"));
    }
    let reference_pdf = pdf_text(&pdf::render_reference(&report, &branding, &[]).unwrap());
    let reference_word = document_xml(&docx::render_reference(&report, &branding, &[]).unwrap());
    for text in [&reference_pdf, &reference_word] {
        for marker in [
            "Resources by type",
            "hostPoolType",
            "stprodapp01 allows public blob access",
            "512 characters",
        ] {
            assert!(
                text.contains(marker),
                "missing reference evidence: {marker}"
            );
        }
        for resource in report.analysis.resources.values() {
            assert!(text.contains(&resource.name), "{} missing", resource.name);
        }
    }
}

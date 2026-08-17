mod common;

use std::io::Read;

use azdocs::diagram::DiagramScope;
use azdocs::diagram::assets::{self, DiagramAsset, DiagramAssetKind};
use azdocs::report::branding::BrandingContext;
use azdocs::report::{ReportContext, docx, pdf};
use azdocs::store::Store;

fn seeded() -> (ReportContext, Vec<DiagramAsset>) {
    let store = Store::open_in_memory().unwrap();
    let id = common::seed_estate(&store);
    let report = ReportContext::build(&store, &id).unwrap();
    let mut diagrams = assets::build_overviews(&store, &id, &DiagramScope::default()).unwrap();
    diagrams.extend(assets::build_resource_diagrams(&store, &id).unwrap());
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
fn native_print_formats_share_ordered_structure_and_overview_gallery_scope() {
    let (report, diagrams) = seeded();
    let branding = BrandingContext::default();

    let pdf = pdf::render(&report, &branding, &diagrams).unwrap();
    let docx = docx::render(&report, &branding, &diagrams).unwrap();
    let pdf_text = pdf_text(&pdf);
    let docx_xml = document_xml(&docx);

    let markers = [
        "Executive Summary",
        "Resources by type",
        "Findings",
        "Networking",
        "Resources by type",
        "Production",
        "rg-app",
        "Diagrams",
    ];
    assert_ordered(&pdf_text, &markers, "PDF");
    assert_ordered(&docx_xml, &markers, "DOCX");

    let pdf_gallery = &pdf_text[pdf_text.rfind("Diagrams").expect("PDF gallery")..];
    let docx_gallery = &docx_xml[docx_xml.rfind("Diagrams").expect("DOCX gallery")..];
    for asset in diagrams
        .iter()
        .filter(|asset| asset.kind == DiagramAssetKind::ResourceGroup)
    {
        assert!(
            !pdf_gallery.contains(&asset.title),
            "resource-group diagram leaked into PDF gallery: {}",
            asset.title
        );
        assert!(
            !docx_gallery.contains(&asset.title),
            "resource-group diagram leaked into DOCX gallery: {}",
            asset.title
        );
    }
}

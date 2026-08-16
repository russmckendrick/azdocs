mod common;

use azdocs::diagram::DiagramScope;
use azdocs::diagram::assets::{self, DiagramAsset};
use azdocs::report::branding::BrandingContext;
use azdocs::report::{ReportContext, pdf};
use azdocs::store::Store;

fn seeded() -> (ReportContext, Vec<DiagramAsset>) {
    let store = Store::open_in_memory().unwrap();
    let id = common::seed_estate(&store);
    let report = ReportContext::build(&store, &id).unwrap();
    let diagrams = assets::build_overviews(&store, &id, &DiagramScope::default()).unwrap();
    (report, diagrams)
}

fn extract_all_text(bytes: &[u8]) -> (usize, String) {
    let document = lopdf::Document::load_mem(bytes).unwrap();
    let pages: Vec<u32> = document.get_pages().keys().copied().collect();
    let text = document.extract_text(&pages).unwrap();
    (pages.len(), text)
}

#[test]
fn pdf_compiles_fixture_estate_with_zero_diagnostics() {
    let (report, diagrams) = seeded();
    let branding = BrandingContext::default();

    let (bytes, warnings) = pdf::render_with_warnings(&report, &branding, &diagrams).unwrap();

    assert!(warnings.is_empty(), "typst warnings: {warnings:?}");
    assert!(!bytes.is_empty());
}

#[test]
fn pdf_contains_title_findings_and_resource_group_text() {
    let (report, diagrams) = seeded();
    let branding = BrandingContext::default();

    let bytes = pdf::render(&report, &branding, &diagrams).unwrap();

    let (page_count, text) = extract_all_text(&bytes);
    assert!(page_count >= 3, "expected >= 3 pages, got {page_count}");
    assert!(text.contains("Azure Estate Report"), "title missing");
    assert!(text.contains("Findings"), "findings heading missing");
    assert!(text.contains("rg-app"), "resource group name missing");
}

#[test]
fn pdf_renders_byte_identical_across_runs() {
    let (report, diagrams) = seeded();
    let branding = BrandingContext::default();

    let first = pdf::render(&report, &branding, &diagrams).unwrap();
    let second = pdf::render(&report, &branding, &diagrams).unwrap();

    assert_eq!(first, second, "PDF output must be deterministic");
}

mod common;

use azdocs::config::BrandingConfig;
use azdocs::diagram::DiagramScope;
use azdocs::diagram::assets::{self, DiagramAsset};
use azdocs::report::branding::BrandingContext;
use azdocs::report::theme::ThemePack;
use azdocs::report::{ReportContext, pdf};
use azdocs::store::Store;

fn seeded() -> (ReportContext, Vec<DiagramAsset>) {
    let store = Store::open_in_memory().unwrap();
    let id = common::seed_estate(&store);
    let report = ReportContext::build(&store, &id).unwrap();
    let mut diagrams = assets::build_overviews(&store, &id, &DiagramScope::default()).unwrap();
    diagrams.extend(assets::build_resource_diagrams(&store, &id).unwrap());
    (report, diagrams)
}

fn themed(theme: &str) -> BrandingContext {
    BrandingContext::resolve(
        &BrandingConfig {
            theme: theme.to_owned(),
            ..BrandingConfig::default()
        },
        None,
    )
    .unwrap()
}

fn themes() -> Vec<String> {
    ThemePack::builtin()
        .unwrap()
        .names()
        .into_iter()
        .map(str::to_owned)
        .collect()
}

fn extract_all_text(bytes: &[u8]) -> (usize, String) {
    let document = lopdf::Document::load_mem(bytes).unwrap();
    let pages: Vec<u32> = document.get_pages().keys().copied().collect();
    let text = document.extract_text(&pages).unwrap();
    (pages.len(), text)
}

#[test]
fn pdf_compiles_every_theme_with_zero_diagnostics() {
    let (report, diagrams) = seeded();

    for theme in themes() {
        let (bytes, warnings) =
            pdf::render_with_warnings(&report, &themed(&theme), &diagrams).unwrap();

        assert!(
            warnings.is_empty(),
            "typst warnings in {theme}: {warnings:?}"
        );
        assert!(!bytes.is_empty(), "{theme} produced no bytes");
    }
}

#[test]
fn pdf_contains_title_findings_and_resource_group_text() {
    let (report, diagrams) = seeded();

    let bytes = pdf::render(&report, &BrandingContext::default(), &diagrams).unwrap();

    let (page_count, text) = extract_all_text(&bytes);
    assert!(page_count >= 3, "expected >= 3 pages, got {page_count}");
    assert!(text.contains("Azure Estate Report"), "title missing");
    assert!(text.contains("Findings"), "findings heading missing");
    assert!(text.contains("rg-app"), "resource group name missing");
}

/// The per-resource chapters are the point of the report: a type heading, then
/// each resource's own settings.
#[test]
fn pdf_documents_each_resource_with_its_settings() {
    let (report, diagrams) = seeded();

    let bytes = pdf::render(&report, &BrandingContext::default(), &diagrams).unwrap();

    let (_, text) = extract_all_text(&bytes);
    assert!(text.contains("VIRTUAL MACHINE"), "resource type chapter");
    assert!(text.contains("Settings"), "per-resource settings block");
    assert!(text.contains("Relationships"), "per-resource diagram block");
    // A flattened property, i.e. real configuration rather than a summary row.
    assert!(text.contains("Host Pool Type"), "flattened property");
}

#[test]
fn pdf_renders_every_theme_byte_identically_across_runs() {
    let (report, diagrams) = seeded();

    for theme in themes() {
        let branding = themed(&theme);
        let first = pdf::render(&report, &branding, &diagrams).unwrap();
        let second = pdf::render(&report, &branding, &diagrams).unwrap();

        assert_eq!(first, second, "{theme} PDF output must be deterministic");
    }
}

/// Themes must change layout, not just colour, or they are only a palette.
#[test]
fn pdf_themes_produce_different_documents() {
    let (report, diagrams) = seeded();

    let fluent = pdf::render(&report, &themed("fluent"), &diagrams).unwrap();
    let editorial = pdf::render(&report, &themed("editorial"), &diagrams).unwrap();

    // Editorial adds divider pages, so it is the longer document.
    let (fluent_pages, _) = extract_all_text(&fluent);
    let (editorial_pages, _) = extract_all_text(&editorial);
    assert!(
        editorial_pages > fluent_pages,
        "editorial divider pages should lengthen the document \
         ({editorial_pages} vs {fluent_pages})"
    );
}

/// A raw ARM id is longer than any table column, so it has to wrap rather than
/// run off the page. The zero-width joins that allow it must survive into the
/// text layer.
#[test]
fn pdf_wraps_long_arm_ids_instead_of_clipping_them() {
    let (report, diagrams) = seeded();

    let bytes = pdf::render(&report, &BrandingContext::default(), &diagrams).unwrap();

    let (_, text) = extract_all_text(&bytes);
    let stripped: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        stripped.contains("providers/Microsoft.DesktopVirtualization/hostPools/hp-prod"),
        "full ARM id should be present, wrapped rather than truncated"
    );
}

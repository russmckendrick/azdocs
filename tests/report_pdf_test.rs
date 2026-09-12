mod common;

use azdocs::config::BrandingConfig;
use azdocs::diagram::assets::DiagramAsset;
use azdocs::report::branding::BrandingContext;
use azdocs::report::theme::ThemePack;
use azdocs::report::{ReportContext, pdf};
use azdocs::store::Store;

/// Typst rendering shares process-global state, so two `pdf::render` calls on
/// different threads can interleave through it. Every real invocation is its
/// own process, so that is not a product concern — but it does mean the
/// byte-identity assertion below cannot race its sibling tests. Serialise
/// every render in this binary rather than weaken what is asserted.
static RENDER: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn render_locked(
    report: &ReportContext,
    branding: &BrandingContext,
    diagrams: &[DiagramAsset],
) -> Vec<u8> {
    let _guard = RENDER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    pdf::render(report, branding, diagrams).unwrap()
}

fn render_with_warnings_locked(
    report: &ReportContext,
    branding: &BrandingContext,
    diagrams: &[DiagramAsset],
) -> (Vec<u8>, Vec<String>) {
    let _guard = RENDER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    pdf::render_with_warnings(report, branding, diagrams).unwrap()
}

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
        let (bytes, warnings) = render_with_warnings_locked(&report, &themed(&theme), &diagrams);

        assert!(
            warnings.is_empty(),
            "typst warnings in {theme}: {warnings:?}"
        );
        assert!(!bytes.is_empty(), "{theme} produced no bytes");
    }
}

#[test]
fn pdf_missing_word_fonts_use_bundled_fallbacks_without_warnings() {
    let (report, diagrams) = seeded();
    let mut branding = BrandingContext::default();
    branding.system_fonts.clear();
    branding.tokens.typography.pdf_use_docx_fonts = true;
    branding.tokens.typography.docx_serif = "Unavailable test serif".into();
    branding.tokens.typography.docx_sans = "Unavailable test sans".into();
    branding.tokens.typography.docx_mono = "Unavailable test mono".into();
    let (bytes, warnings) = render_with_warnings_locked(&report, &branding, &diagrams);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(extract_all_text(&bytes).1.contains("Executive assessment"));
}

#[test]
fn pdf_contains_title_findings_and_resource_group_text() {
    let (report, diagrams) = seeded();

    let bytes = render_locked(&report, &BrandingContext::default(), &diagrams);

    let (page_count, text) = extract_all_text(&bytes);
    assert!(page_count >= 3, "expected >= 3 pages, got {page_count}");
    assert!(text.contains("Azure Estate Report"), "title missing");
    assert!(
        text.contains("Security and data protection"),
        "findings heading missing"
    );
    assert!(text.contains("rg-app"), "resource group name missing");
}

/// Tag coverage is judged against a named threshold rather than printed bare,
/// so the report and the desktop explorer agree about what "healthy" means.
#[test]
fn pdf_states_tag_coverage_against_the_healthy_threshold() {
    let (report, diagrams) = seeded();
    let bytes = render_locked(&report, &BrandingContext::default(), &diagrams);
    let (_, text) = extract_all_text(&bytes);
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(text.contains("60% as a tag-presence threshold only"));
    assert!(text.contains("does not demonstrate that required tags or useful values exist"));
}

#[test]
fn pdf_main_omits_resource_register_and_reference_retains_it() {
    let (report, diagrams) = seeded();
    let branding = BrandingContext::default();
    let main = render_locked(&report, &branding, &diagrams);
    let (_, main_text) = extract_all_text(&main);
    assert!(!main_text.contains("Resources by type"));
    let _guard = RENDER.lock().unwrap_or_else(|p| p.into_inner());
    let reference = pdf::render_reference(&report, &branding, &[]).unwrap();
    let (_, text) = extract_all_text(&reference);
    assert!(text.contains("Resources by type"));
    assert!(text.contains("Host Pool Type"));
    assert!(text.contains("Finding occurrence register"));
}

#[test]
fn pdf_reference_resource_links_resolve_to_document_pages() {
    let (report, _) = seeded();
    let _guard = RENDER.lock().unwrap_or_else(|p| p.into_inner());
    let bytes = pdf::render_reference(&report, &BrandingContext::default(), &[]).unwrap();
    let document = lopdf::Document::load_mem(&bytes).unwrap();
    let pages: std::collections::BTreeSet<_> = document.get_pages().into_values().collect();
    let mut internal_links = 0;
    for object in document.objects.values() {
        let Ok(dict) = object.as_dict() else { continue };
        if dict.get(b"Subtype").and_then(lopdf::Object::as_name).ok() != Some(b"Link") {
            continue;
        }
        let action = dict.get(b"A").and_then(lopdf::Object::as_dict).unwrap();
        if action.get(b"S").and_then(lopdf::Object::as_name).ok() == Some(b"GoTo") {
            let destination = action.get(b"D").and_then(lopdf::Object::as_array).unwrap();
            let target = destination[0].as_reference().unwrap();
            assert!(pages.contains(&target), "link points outside this PDF");
            internal_links += 1;
        }
    }
    assert!(
        internal_links >= report.analysis.resources.len(),
        "resource index must contain real PDF links"
    );
}

#[test]
fn pdf_renders_every_theme_byte_identically_across_runs() {
    let (report, diagrams) = seeded();

    for theme in themes() {
        let branding = themed(&theme);
        let first = render_locked(&report, &branding, &diagrams);
        let second = render_locked(&report, &branding, &diagrams);

        assert_eq!(first, second, "{theme} PDF output must be deterministic");
    }
}

#[test]
fn pdf_divider_page_strategy_changes_document_layout() {
    let (report, diagrams) = seeded();
    let field_report = BrandingContext::default();
    let mut divided = field_report.clone();
    divided.tokens.layout.divider_pages = true;

    let compact = render_locked(&report, &field_report, &diagrams);
    let with_dividers = render_locked(&report, &divided, &diagrams);

    let (compact_pages, _) = extract_all_text(&compact);
    let (divider_pages, _) = extract_all_text(&with_dividers);
    assert!(
        divider_pages > compact_pages,
        "divider pages should lengthen the document ({divider_pages} vs {compact_pages})"
    );
}

/// A raw ARM id is longer than any table column, so it has to wrap rather than
/// run off the page. The zero-width joins that allow it must survive into the
/// text layer.
#[test]
fn pdf_wraps_long_arm_ids_instead_of_clipping_them() {
    let (report, diagrams) = seeded();

    let _guard = RENDER.lock().unwrap_or_else(|p| p.into_inner());
    let bytes = pdf::render_reference(&report, &BrandingContext::default(), &diagrams).unwrap();

    let (_, text) = extract_all_text(&bytes);
    let stripped: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        stripped.contains("providers/Microsoft.DesktopVirtualization/hostPools/hp-prod"),
        "full ARM id should be present, wrapped rather than truncated"
    );
}

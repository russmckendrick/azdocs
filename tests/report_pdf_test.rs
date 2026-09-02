mod common;

use azdocs::labels::Labels;

use azdocs::config::BrandingConfig;
use azdocs::diagram::DiagramScope;
use azdocs::diagram::assets::{self, DiagramAsset};
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
    let mut diagrams = assets::build_overviews(
        &store,
        &id,
        &DiagramScope::default(),
        &Labels::default().diagram,
    )
    .unwrap();
    diagrams
        .extend(assets::build_resource_diagrams(&store, &id, &Labels::default().diagram).unwrap());
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
fn pdf_contains_title_findings_and_resource_group_text() {
    let (report, diagrams) = seeded();

    let bytes = render_locked(&report, &BrandingContext::default(), &diagrams);

    let (page_count, text) = extract_all_text(&bytes);
    assert!(page_count >= 3, "expected >= 3 pages, got {page_count}");
    assert!(text.contains("Azure Estate Report"), "title missing");
    assert!(text.contains("Findings"), "findings heading missing");
    assert!(text.contains("rg-app"), "resource group name missing");
}

/// Tag coverage is judged against a named threshold rather than printed bare,
/// so the report and the desktop explorer agree about what "healthy" means.
#[test]
fn pdf_states_tag_coverage_against_the_healthy_threshold() {
    let (report, diagrams) = seeded();
    // The canonical fixture sits at 52%, so this exercises the unhealthy
    // branch. If the fixture ever crosses 60% the expectation flips with it
    // rather than silently passing on the wrong sentence.
    let healthy = report.tag_coverage.is_healthy();
    let expected = if healthy {
        "at or above the 60%"
    } else {
        "below the 60%"
    };

    let bytes = render_locked(&report, &BrandingContext::default(), &diagrams);

    let (_, text) = extract_all_text(&bytes);
    let text: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        text.contains(expected),
        "the report should judge {}% coverage as `{expected}`",
        report.tag_coverage.percent
    );
    assert!(
        text.contains("this report treats as healthy"),
        "the threshold should be named, not applied silently"
    );
}

/// The per-resource detail is the point of the report: each resource's own
/// settings, reached through the estate structure.
#[test]
fn pdf_documents_each_resource_with_its_settings() {
    let (report, diagrams) = seeded();

    let bytes = render_locked(&report, &BrandingContext::default(), &diagrams);

    let (_, text) = extract_all_text(&bytes);
    assert!(text.contains("Settings"), "per-resource settings block");
    assert!(text.contains("Relationships"), "per-resource diagram block");
    // A flattened property, i.e. real configuration rather than a summary row.
    assert!(text.contains("Host Pool Type"), "flattened property");
}

/// The document is laid out the way Azure is — subscription, then resource
/// group, then resource — with a by-type index kept for compliance sweeps.
#[test]
fn pdf_lays_the_estate_out_by_subscription_then_group() {
    let (report, diagrams) = seeded();

    let bytes = render_locked(&report, &BrandingContext::default(), &diagrams);

    let (_, text) = extract_all_text(&bytes);
    let index = text.find("Resources by type").expect("type index missing");
    let subscription = text
        .find("Production")
        .expect("subscription chapter missing");
    let group = text.find("rg-app").expect("resource group heading missing");
    assert!(
        index < subscription,
        "the type index belongs before the estate body"
    );
    assert!(
        subscription < group,
        "resource groups belong under their subscription"
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

    let bytes = render_locked(&report, &BrandingContext::default(), &diagrams);

    let (_, text) = extract_all_text(&bytes);
    let stripped: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        stripped.contains("providers/Microsoft.DesktopVirtualization/hostPools/hp-prod"),
        "full ARM id should be present, wrapped rather than truncated"
    );
}

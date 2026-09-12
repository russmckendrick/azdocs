//! Developer affordance, not a check: renders the fixture estate in every
//! theme so the documents can be looked at without a real Azure tenant.
//!
//! ```sh
//! cargo test --test report_preview -- --ignored --nocapture
//! ```
//!
//! Output lands in `output/preview/<theme>/` (gitignored).

mod common;

use azdocs::labels::Labels;

use std::path::PathBuf;

use azdocs::config::BrandingConfig;
use azdocs::diagram::DiagramScope;
use azdocs::diagram::assets;
use azdocs::report::branding::BrandingContext;
use azdocs::report::theme::ThemePack;
use azdocs::report::{self, ReportContext};
use azdocs::store::Store;

#[test]
#[ignore = "writes PDF fixtures for typography review; run with --ignored"]
fn writes_print_typography_previews() {
    let store = Store::open_in_memory().unwrap();
    let snapshot = common::seed_estate(&store);
    common::seed_website_evidence(&store, &snapshot);
    store.save_website_capture(&snapshot, "https://web-dev.azurewebsites.net/", &azdocs::model::websites::CapturedWebsite {
        final_url: format!("https://login.example.com/00000000-0000-0000-0000-000000000000/oauth2/v2.0/authorize?client_id=fixture&redirect_uri=https%3A%2F%2Fweb-dev.azurewebsites.net%2F&nonce={}", "x".repeat(400)),
        captured_at: "2026-09-12T17:49:25.940770+00:00".into(),
        renderer: "fixture".into(),
        png: include_bytes!("../desktop/src/fixtures/website.png").to_vec(),
    }).unwrap();
    let mut context = ReportContext::build(&store, &snapshot).unwrap();
    let mut missing = context.websites.endpoints[0].clone();
    missing.url = Some("https://orange-stone-089140003.7.azurestaticapps.net/".into());
    missing.hostname = Some("orange-stone-089140003.7.azurestaticapps.net".into());
    context.websites.endpoints.push(missing);
    let branding = BrandingContext::default();
    let diagrams = assets::build_assessment(&context.analysis, &branding.labels);
    let out = PathBuf::from("output/pdf");
    std::fs::create_dir_all(&out).unwrap();
    report::pdf::write(&context, &branding, &diagrams, &out.join("report.pdf")).unwrap();
    std::fs::write(
        out.join("technical-reference.pdf"),
        report::pdf::render_reference(&context, &branding, &[]).unwrap(),
    )
    .unwrap();
    println!("Print typography previews: {}", out.display());
}

#[test]
#[ignore = "writes files for manual review; run with --ignored"]
fn writes_every_theme_in_every_format() {
    let store = Store::open_in_memory().unwrap();
    let snapshot = common::seed_estate(&store);
    let context = ReportContext::build(&store, &snapshot).unwrap();
    let diagrams = assets::build_overviews(
        &store,
        &snapshot,
        &DiagramScope::default(),
        &Labels::default().diagram,
    )
    .unwrap();
    let resources = store.resources(&snapshot).unwrap();
    let findings = store.findings(&snapshot).unwrap();

    let pack = ThemePack::builtin().unwrap();
    let themes: Vec<String> = pack.names().into_iter().map(str::to_owned).collect();

    for theme in &themes {
        let out = PathBuf::from("output/preview").join(theme);
        std::fs::create_dir_all(&out).unwrap();

        let branding = BrandingContext::resolve(
            &BrandingConfig {
                company: "Contoso Ltd".to_owned(),
                subtitle: "Quarterly estate review".to_owned(),
                theme: theme.clone(),
                ..BrandingConfig::default()
            },
            None,
        )
        .unwrap();

        let assessment = assets::build_assessment(&context.analysis, &branding.labels);
        report::pdf::write(&context, &branding, &assessment, &out.join("report.pdf")).unwrap();
        report::docx::write(&context, &branding, &assessment, &out.join("report.docx")).unwrap();
        std::fs::write(
            out.join("technical-reference.pdf"),
            report::pdf::render_reference(&context, &branding, &diagrams).unwrap(),
        )
        .unwrap();
        std::fs::write(
            out.join("technical-reference.docx"),
            report::docx::render_reference(&context, &branding, &diagrams).unwrap(),
        )
        .unwrap();
        report::html::write(&context, &branding, &out.join("report.html")).unwrap();
        report::site::write(&context, &branding, &diagrams, &out.join("docs-html")).unwrap();
        report::xlsx::write(&context, &branding, &resources, &out.join("azdocs.xlsx")).unwrap();
        report::csv::write_inventory(&resources, &branding.labels, &out.join("inventory.csv"))
            .unwrap();
        report::csv::write_findings(&findings, &branding.labels, &out.join("findings.csv"))
            .unwrap();

        println!("{theme:>10} -> {}", out.display());
    }
}

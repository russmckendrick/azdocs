//! Developer affordance, not a check: renders the fixture estate in every
//! theme so the documents can be looked at without a real Azure tenant.
//!
//! ```sh
//! cargo test --test report_preview -- --ignored --nocapture
//! ```
//!
//! Output lands in `output/preview/<theme>/` (gitignored).

mod common;

use std::path::PathBuf;

use azdocs::config::BrandingConfig;
use azdocs::diagram::DiagramScope;
use azdocs::diagram::assets;
use azdocs::report::branding::BrandingContext;
use azdocs::report::theme::ThemePack;
use azdocs::report::{self, ReportContext};
use azdocs::store::Store;

#[test]
#[ignore = "writes files for manual review; run with --ignored"]
fn writes_every_theme_in_every_format() {
    let store = Store::open_in_memory().unwrap();
    let snapshot = common::seed_estate(&store);
    let context = ReportContext::build(&store, &snapshot).unwrap();
    let mut diagrams =
        assets::build_overviews(&store, &snapshot, &DiagramScope::default()).unwrap();
    diagrams.extend(assets::build_resource_diagrams(&store, &snapshot).unwrap());
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

        report::pdf::write(&context, &branding, &diagrams, &out.join("report.pdf")).unwrap();
        report::docx::write(&context, &branding, &diagrams, &out.join("report.docx")).unwrap();
        report::html::write(&context, &branding, &out.join("report.html")).unwrap();
        report::site::write(&context, &branding, &diagrams, &out.join("docs-html")).unwrap();
        report::xlsx::write(&context, &branding, &resources, &out.join("azdocs.xlsx")).unwrap();
        report::csv::write_inventory(&resources, &out.join("inventory.csv")).unwrap();
        report::csv::write_findings(&findings, &out.join("findings.csv")).unwrap();

        println!("{theme:>10} -> {}", out.display());
    }
}

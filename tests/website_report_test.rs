mod common;

use azdocs::{
    model::websites::{CaptureStatus, CapturedWebsite},
    report::{ReportContext, branding::BrandingContext, docx, html, pdf, site},
    store::Store,
};
use std::io::{Cursor, Read};

fn seeded() -> (Store, String, Vec<u8>) {
    let store = Store::open_in_memory().unwrap();
    let id = common::seed_estate(&store);
    let png = common::seed_website_evidence(&store, &id);
    (store, id, png)
}

#[test]
fn html_and_site_embed_saved_evidence_and_retain_failed_refreshes_offline() {
    let (store, id, png) = seeded();
    let url = "https://web-dev.azurewebsites.net/";
    store
        .record_website_attempt(
            &id,
            url,
            CaptureStatus::Failed,
            Some("DNS failure <script>alert(1)</script>"),
        )
        .unwrap();
    let report = ReportContext::build(&store, &id).unwrap();
    let branding = BrandingContext::default();
    let html = html::render(&report, &branding).unwrap();
    assert!(html.contains("data:image/png;base64,"));
    assert!(html.contains("2026-09-12T10:00:00Z"));
    assert!(html.contains("DNS failure &lt;script&gt;"));
    assert!(!html.contains("DNS failure <script>"));
    assert_eq!(html, html::render(&report, &branding).unwrap());
    let dir = tempfile::tempdir().unwrap();
    site::write(&report, &branding, &[], dir.path()).unwrap();
    assert_eq!(
        std::fs::read(dir.path().join("websites/website-0.png")).unwrap(),
        png
    );
    let index = std::fs::read_to_string(dir.path().join("index.html")).unwrap();
    let resource =
        std::fs::read_to_string(dir.path().join("resources/development/rg-dev.html")).unwrap();
    assert!(index.contains("src=\"websites/website-0.png\""));
    assert!(resource.contains("src=\"../../websites/website-0.png\""));
    assert!(
        ReportContext::build_for_desktop(&store, &id)
            .unwrap()
            .websites
            .captures[url]
            .png
            .is_empty()
    );
}

#[test]
fn technical_references_embed_pngs_but_main_assessments_do_not() {
    let (store, id, png) = seeded();
    let final_url = format!(
        "https://login.example.com/oauth2/authorize?redirect_uri=https%3A%2F%2Fweb-dev.azurewebsites.net%2F&nonce={}",
        "capture-query-value".repeat(30)
    );
    store
        .save_website_capture(
            &id,
            "https://web-dev.azurewebsites.net/",
            &CapturedWebsite {
                final_url: final_url.clone(),
                captured_at: "2026-09-12T10:00:00Z".into(),
                renderer: "fixture".into(),
                png: png.clone(),
            },
        )
        .unwrap();
    let report = ReportContext::build(&store, &id).unwrap();
    let branding = BrandingContext::default();
    let reference = pdf::render_reference(&report, &branding, &[]).unwrap();
    assert_eq!(
        reference,
        pdf::render_reference(&report, &branding, &[]).unwrap()
    );
    let document = lopdf::Document::load_mem(&reference).unwrap();
    let pages: Vec<_> = document.get_pages().keys().copied().collect();
    let text = document.extract_text(&pages).unwrap();
    assert!(text.contains("Website screenshots"));
    assert!(text.contains("12 Sep 2026, 10:00:00 UTC"));
    let visible_text: String = text
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != '\u{200b}')
        .collect();
    assert!(visible_text.contains(&final_url));
    assert!(document.objects.values().any(|object| {
        object
            .as_dict()
            .ok()
            .and_then(|dict| dict.get(b"A").ok())
            .and_then(|action| action.as_dict().ok())
            .and_then(|action| action.get(b"URI").ok())
            .and_then(|uri| uri.as_str().ok())
            == Some(final_url.as_bytes())
    }));
    assert!(
        document
            .objects
            .values()
            .any(|value| value.as_stream().is_ok_and(|stream| stream
                .dict
                .get(b"Width")
                .and_then(lopdf::Object::as_i64)
                .ok()
                == Some(1440)))
    );
    let main = pdf::render(&report, &branding, &[]).unwrap();
    let main = lopdf::Document::load_mem(&main).unwrap();
    assert!(
        !main
            .extract_text(&main.get_pages().keys().copied().collect::<Vec<_>>())
            .unwrap()
            .contains("Website screenshots")
    );
    let docx = docx::render_reference(&report, &branding, &[]).unwrap();
    let mut archive = zip::ZipArchive::new(Cursor::new(docx)).unwrap();
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    assert!(xml.contains("Website screenshots"));
    assert!(xml.contains("12 Sep 2026, 10:00:00 UTC"));
    assert!(
        xml.replace('\u{200b}', "")
            .contains(&final_url.replace('&', "&amp;"))
    );
    assert!(xml.contains("w:hyperlink"));
    let mut relationships = String::new();
    archive
        .by_name("word/_rels/document.xml.rels")
        .unwrap()
        .read_to_string(&mut relationships)
        .unwrap();
    assert!(relationships.contains(&final_url.replace('&', "&amp;")));
    assert!((0..archive.len()).any(|index| {
        let mut file = archive.by_index(index).unwrap();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        bytes == png
    }));
    let main = docx::render(&report, &branding, &[]).unwrap();
    let mut archive = zip::ZipArchive::new(Cursor::new(main)).unwrap();
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    assert!(!xml.contains("Website screenshots"));
}

#[test]
fn captured_fixture_survives_a_cancelled_attempt_with_original_timestamp() {
    let (store, id, png) = seeded();
    let url = "https://web-dev.azurewebsites.net/";
    store
        .record_website_attempt(&id, url, CaptureStatus::Cancelled, None)
        .unwrap();
    let saved = store.website_captures(&id, true).unwrap().remove(0);
    assert_eq!(saved.png, png);
    assert_eq!(saved.captured_at.as_deref(), Some("2026-09-12T10:00:00Z"));
    assert_eq!(saved.status, CaptureStatus::Cancelled);
    store
        .save_website_capture(
            &id,
            url,
            &CapturedWebsite {
                final_url: url.into(),
                captured_at: "2026-09-13T10:00:00Z".into(),
                renderer: "fixture".into(),
                png: png.clone(),
            },
        )
        .unwrap();
    assert_eq!(store.website_png(&id, url).unwrap(), Some(png));
}

//! PDF report via an embedded Typst compiler: no external binaries, fonts
//! embedded from `data/fonts`, and byte-deterministic output (the document
//! date comes from the snapshot, not the wall clock).

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, anyhow};
use chrono::{Datelike, Timelike};
use include_dir::{Dir, include_dir};
use typst::Library;
use typst::diag::{FileError, FileResult, SourceDiagnostic, Warned};
use typst::foundations::{Bytes, Datetime, Dict, Smart, Value};
use typst::layout::PagedDocument;
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst_pdf::{PdfOptions, PdfStandards, Timestamp};

use super::ReportContext;
use super::branding::BrandingContext;
use super::document::PrintDocument;
use super::mark;
use crate::diagram::assets::DiagramAsset;

static TYPST_TEMPLATES: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/templates/typst");

/// The vendored report typeface. `typst-assets` ships no proportional sans, so
/// the document face has to come from the repo.
static VENDORED_FONTS: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/data/fonts");
static PLEX_SERIF_REGULAR: &[u8] =
    include_bytes!("../../desktop/src/assets/fonts/IBMPlexSerif-Regular.ttf");
static PLEX_SERIF_SEMIBOLD: &[u8] =
    include_bytes!("../../desktop/src/assets/fonts/IBMPlexSerif-SemiBold.ttf");

/// `typst-assets` families kept purely as a glyph fallback behind the vendored
/// faces, so a character IBM Plex lacks still renders instead of turning into
/// `.notdef`. Everything else there is skipped to keep the font book small.
const FALLBACK_FAMILIES: [&str; 2] = ["Libertinus Serif", "DejaVu Sans Mono"];

/// Render the PDF report and write it to `out_path`.
pub fn write(
    report: &ReportContext,
    branding: &BrandingContext,
    diagrams: &[DiagramAsset],
    out_path: &Path,
) -> anyhow::Result<()> {
    let bytes = render(report, branding, diagrams)?;
    std::fs::write(out_path, bytes).with_context(|| format!("writing {}", out_path.display()))?;
    Ok(())
}

/// Render the PDF report to bytes.
pub fn render(
    report: &ReportContext,
    branding: &BrandingContext,
    diagrams: &[DiagramAsset],
) -> anyhow::Result<Vec<u8>> {
    Ok(render_with_warnings(report, branding, diagrams)?.0)
}

/// Render the PDF and also return any compiler warnings (used by tests to
/// assert the template compiles cleanly).
pub fn render_with_warnings(
    report: &ReportContext,
    branding: &BrandingContext,
    diagrams: &[DiagramAsset],
) -> anyhow::Result<(Vec<u8>, Vec<String>)> {
    let document = PrintDocument::build(report, branding, diagrams);
    let world = ReportWorld::new(&document, branding, diagrams)?;
    let Warned { output, warnings } = typst::compile::<PagedDocument>(&world);
    let document = output.map_err(|diags| diagnostics_error("compiling PDF report", &diags))?;
    let options = PdfOptions {
        ident: Smart::Auto,
        timestamp: world.timestamp,
        page_ranges: None,
        standards: PdfStandards::default(),
    };
    let bytes = typst_pdf::pdf(&document, &options)
        .map_err(|diags| diagnostics_error("exporting PDF report", &diags))?;
    let warnings = warnings
        .iter()
        .map(|diag| diag.message.to_string())
        .collect();
    Ok((bytes, warnings))
}

fn diagnostics_error(action: &str, diagnostics: &[SourceDiagnostic]) -> anyhow::Error {
    let details: Vec<String> = diagnostics
        .iter()
        .map(|diag| diag.message.to_string())
        .collect();
    anyhow!("{action} failed: {}", details.join("; "))
}

/// Self-contained `typst::World`: templates and fonts are compiled into the
/// binary, report data arrives as JSON via `sys.inputs`, and diagrams (plus
/// the optional logo) are virtual files. Nothing touches the filesystem.
struct ReportWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    main: FileId,
    sources: BTreeMap<FileId, Source>,
    files: BTreeMap<FileId, Bytes>,
    today: Option<Datetime>,
    timestamp: Option<Timestamp>,
}

impl ReportWorld {
    fn new(
        document: &PrintDocument<'_>,
        branding: &BrandingContext,
        diagrams: &[DiagramAsset],
    ) -> anyhow::Result<Self> {
        let icon_types: std::collections::BTreeSet<&str> = document.icon_types().collect();
        let icons: BTreeMap<&str, String> = icon_types
            .iter()
            .map(|azure_type| (*azure_type, format!("/icons/{}.svg", icon_slug(azure_type))))
            .collect();

        let mut inputs = Dict::new();
        inputs.insert(
            "document".into(),
            Value::Str(serde_json::to_string(document)?.into()),
        );
        inputs.insert(
            "branding".into(),
            Value::Str(serde_json::to_string(branding)?.into()),
        );
        inputs.insert(
            "theme".into(),
            Value::Str(serde_json::to_string(&branding.tokens)?.into()),
        );
        inputs.insert(
            "icons".into(),
            Value::Str(serde_json::to_string(&icons)?.into()),
        );

        let mut files = BTreeMap::new();
        for asset in diagrams {
            let id = FileId::new(
                None,
                VirtualPath::new(format!("/diagrams/{}.svg", asset.slug)),
            );
            files.insert(id, Bytes::new(asset.svg.clone().into_bytes()));
        }
        for azure_type in icon_types {
            let path = format!("/icons/{}.svg", icon_slug(azure_type));
            let id = FileId::new(None, VirtualPath::new(path.as_str()));
            files.insert(id, Bytes::new(crate::diagram::icons::svg_bytes(azure_type)));
        }
        if let Some(logo) = &branding.logo {
            let path = format!("/logo.{}", logo.extension);
            let id = FileId::new(None, VirtualPath::new(path.as_str()));
            files.insert(id, Bytes::new(logo.bytes.clone()));
            inputs.insert("logo".into(), Value::Str(path.into()));
        }
        if document.cover.product_mark {
            for (path, bytes) in [
                (mark::PRIMARY_VIRTUAL_PATH, mark::PRIMARY_SVG),
                (mark::ON_DARK_VIRTUAL_PATH, mark::ON_DARK_SVG),
            ] {
                let id = FileId::new(None, VirtualPath::new(path));
                files.insert(id, Bytes::new(bytes.to_vec()));
            }
            inputs.insert(
                "product-mark-primary".into(),
                Value::Str(mark::PRIMARY_VIRTUAL_PATH.into()),
            );
            inputs.insert(
                "product-mark-on-dark".into(),
                Value::Str(mark::ON_DARK_VIRTUAL_PATH.into()),
            );
        }

        let mut sources = BTreeMap::new();
        let mut main = None;
        for file in TYPST_TEMPLATES.files() {
            let relative = file.path().to_string_lossy();
            let text = file
                .contents_utf8()
                .ok_or_else(|| anyhow!("typst template {relative} is not UTF-8"))?;
            let id = FileId::new(None, VirtualPath::new(format!("/{relative}")));
            if relative == "report.typ" {
                main = Some(id);
            }
            sources.insert(id, Source::new(id, text.to_owned()));
        }
        let main = main.ok_or_else(|| anyhow!("embedded templates/typst/report.typ missing"))?;

        // Order is load-bearing: Typst resolves a glyph through the book in
        // insertion order, so vendored faces must precede the fallbacks. The
        // vendored directory is walked in sorted order for determinism.
        let mut fonts = Vec::new();
        for data in [PLEX_SERIF_REGULAR, PLEX_SERIF_SEMIBOLD] {
            fonts.extend(Font::iter(Bytes::new(data.to_vec())));
        }
        let mut vendored: Vec<_> = VENDORED_FONTS.files().collect();
        vendored.sort_by_key(|file| file.path());
        for file in vendored {
            fonts.extend(Font::iter(Bytes::new(file.contents().to_vec())));
        }
        for data in &branding.extra_fonts {
            fonts.extend(Font::iter(Bytes::new(data.clone())));
        }
        for data in typst_assets::fonts() {
            for font in Font::iter(Bytes::new(data)) {
                if FALLBACK_FAMILIES.contains(&font.info().family.as_str()) {
                    fonts.push(font);
                }
            }
        }
        let book = FontBook::from_fonts(&fonts);

        // Both the `datetime.today()` value and the PDF creation timestamp
        // come from the snapshot so rendering is reproducible byte-for-byte.
        let (today, timestamp) = snapshot_datetime(&document.cover.collected);

        Ok(Self {
            library: LazyHash::new(Library::builder().with_inputs(inputs).build()),
            book: LazyHash::new(book),
            fonts,
            main,
            sources,
            files,
            today,
            timestamp,
        })
    }
}

/// Virtual-file stem for a resource type's icon. ARM types contain `/` and
/// `.`, neither of which belongs in a path segment.
fn icon_slug(azure_type: &str) -> String {
    crate::diagram::graph::slugify(azure_type)
}

fn snapshot_datetime(created_at: &str) -> (Option<Datetime>, Option<Timestamp>) {
    let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(created_at) else {
        return (None, None);
    };
    let utc = parsed.to_utc();
    let datetime = Datetime::from_ymd_hms(
        utc.year(),
        utc.month() as u8,
        utc.day() as u8,
        utc.hour() as u8,
        utc.minute() as u8,
        utc.second() as u8,
    );
    (datetime, datetime.map(Timestamp::new_utc))
}

impl typst::World for ReportWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        self.sources
            .get(&id)
            .cloned()
            .ok_or_else(|| FileError::NotFound(id.vpath().as_rootless_path().to_path_buf()))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files
            .get(&id)
            .cloned()
            .ok_or_else(|| FileError::NotFound(id.vpath().as_rootless_path().to_path_buf()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        self.today
    }
}

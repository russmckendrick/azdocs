//! SVG → PNG rasterisation via resvg, entirely in-process.

use std::sync::{Arc, OnceLock};

use crate::error::DiagramError;

/// Raster scale used everywhere a caller has no reason to differ: 2x keeps
/// text crisp when the PNG is embedded in reports.
pub const DEFAULT_SCALE: f32 = 2.0;

/// System font enumeration is slow; do it once per process.
///
/// The vendored IBM Plex faces are loaded first so the SVG's declared family
/// resolves the same on every machine and on a font-less container; system
/// fonts follow as glyph fallbacks.
fn fontdb() -> Arc<usvg::fontdb::Database> {
    static FONTDB: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    FONTDB.get_or_init(|| Arc::new(build_fontdb(true))).clone()
}

fn build_fontdb(system_fonts: bool) -> usvg::fontdb::Database {
    let mut db = usvg::fontdb::Database::new();
    for file in crate::report::fonts::VENDORED_FONTS.files() {
        db.load_font_data(file.contents().to_vec());
    }
    if system_fonts {
        db.load_system_fonts();
    }
    db
}

/// Rasterise an SVG document to PNG bytes on a white background.
pub fn from_svg(svg: &str, scale: f32) -> Result<Vec<u8>, DiagramError> {
    render(svg, scale, true)
}

/// Rasterise an SVG while preserving its transparent background. Product
/// marks need this path when Word places them on a coloured cover.
pub fn from_svg_transparent(svg: &str, scale: f32) -> Result<Vec<u8>, DiagramError> {
    render(svg, scale, false)
}

fn render(svg: &str, scale: f32, paper_background: bool) -> Result<Vec<u8>, DiagramError> {
    render_with(svg, scale, paper_background, fontdb())
}

fn render_with(
    svg: &str,
    scale: f32,
    paper_background: bool,
    fontdb: Arc<usvg::fontdb::Database>,
) -> Result<Vec<u8>, DiagramError> {
    let options = usvg::Options {
        fontdb,
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_str(svg, &options)?;

    let size = tree.size();
    let width = (size.width() * scale) as u32;
    let height = (size.height() * scale) as u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or(DiagramError::Pixmap { width, height })?;
    if paper_background {
        pixmap.fill(resvg::tiny_skia::Color::WHITE);
    }

    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    pixmap
        .encode_png()
        .map_err(|e| DiagramError::PngEncode(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_svg_renders_text_svg_into_nonempty_png() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="220" height="80" viewBox="0 0 220 80">
  <rect width="220" height="80" fill="white"/>
  <text x="20" y="48" font-family="Arial, Helvetica, sans-serif" font-size="24" fill="#1e293b">Diagram Label</text>
</svg>"##;

        let png = from_svg(svg, 1.0).unwrap();

        assert!(png.starts_with(&[0x89, b'P', b'N', b'G']), "PNG signature");
        assert!(!png.is_empty());
    }

    #[test]
    fn from_svg_rejects_invalid_svg() {
        assert!(from_svg("not svg at all", DEFAULT_SCALE).is_err());
    }

    #[test]
    fn from_svg_transparent_preserves_clear_pixels() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20">
  <circle cx="10" cy="10" r="4" fill="#0078d4"/>
</svg>"##;

        let png = from_svg_transparent(svg, 1.0).unwrap();
        let image = image::load_from_memory(&png).unwrap().to_rgba8();

        assert_eq!(image.get_pixel(0, 0).0[3], 0);
        assert_eq!(image.get_pixel(10, 10).0[3], 255);
    }

    #[test]
    fn unit_from_svg_renders_ibm_plex_without_system_fonts() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="40" viewBox="0 0 120 40"><text x="4" y="28" font-family="IBM Plex Sans" font-size="24" fill="#000">Plex</text></svg>"##;

        let png = render_with(svg, 1.0, true, Arc::new(build_fontdb(false))).unwrap();

        let decoded = image::load_from_memory(&png).unwrap().to_luma8();
        let dark = decoded.pixels().filter(|p| p.0[0] < 128).count();
        assert!(
            dark > 20,
            "text must render from the vendored face: {dark} dark pixels"
        );
    }
}

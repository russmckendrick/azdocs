//! SVG → PNG rasterisation via resvg, entirely in-process.

use std::sync::{Arc, OnceLock};

use crate::error::DiagramError;

/// Raster scale used everywhere a caller has no reason to differ: 2x keeps
/// text crisp when the PNG is embedded in reports.
pub const DEFAULT_SCALE: f32 = 2.0;

/// System font enumeration is slow; do it once per process.
fn fontdb() -> Arc<usvg::fontdb::Database> {
    static FONTDB: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    FONTDB
        .get_or_init(|| {
            let mut db = usvg::fontdb::Database::new();
            db.load_system_fonts();
            Arc::new(db)
        })
        .clone()
}

/// Rasterise an SVG document to PNG bytes on a white background.
pub fn from_svg(svg: &str, scale: f32) -> Result<Vec<u8>, DiagramError> {
    let options = usvg::Options {
        fontdb: fontdb(),
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_str(svg, &options)?;

    let size = tree.size();
    let width = (size.width() * scale) as u32;
    let height = (size.height() * scale) as u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or(DiagramError::Pixmap { width, height })?;
    pixmap.fill(resvg::tiny_skia::Color::WHITE);

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
}

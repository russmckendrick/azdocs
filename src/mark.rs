//! Embedded azdocs product-mark variants shared by the emitters.
//!
//! At the crate root rather than under `report/` because the diagram emitters
//! stamp the mark too, and `diagram` must not depend on `report`.

use std::sync::OnceLock;

const PRIMARY_RAW: &[u8] = include_bytes!("../docs/marks/assets/azdocs-mark-primary.svg");
const ON_DARK_RAW: &[u8] = include_bytes!("../docs/marks/assets/azdocs-mark-mono-paper.svg");

pub(crate) const PRIMARY_VIRTUAL_PATH: &str = "/azdocs-mark-primary.svg";
pub(crate) const ON_DARK_VIRTUAL_PATH: &str = "/azdocs-mark-on-dark.svg";

/// The mark, with LF line endings whatever the working copy holds.
pub(crate) fn primary_svg() -> &'static [u8] {
    static BYTES: OnceLock<Vec<u8>> = OnceLock::new();
    BYTES.get_or_init(|| lf(PRIMARY_RAW))
}

/// The reversed mark, likewise normalised.
pub(crate) fn on_dark_svg() -> &'static [u8] {
    static BYTES: OnceLock<Vec<u8>> = OnceLock::new();
    BYTES.get_or_init(|| lf(ON_DARK_RAW))
}

/// Drop carriage returns from an embedded text asset.
///
/// git rewrites text files to CRLF when it checks them out on Windows, and
/// `include_bytes!` embeds whatever is on disk. These bytes are base64'd
/// straight into diagram output, so the same snapshot encoded to a different
/// string there and the golden files failed on Windows and only on Windows.
///
/// `.gitattributes` marks the assets binary so git stops translating them, but
/// that only takes effect on checkout — a working copy cloned before it, or any
/// other route to a CRLF copy, would still produce output that differs by
/// platform. Normalising here is what actually makes the bytes deterministic,
/// which every golden file in the repo depends on.
fn lf(bytes: &[u8]) -> Vec<u8> {
    bytes.iter().copied().filter(|&b| b != b'\r').collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_embedded_marks_carry_no_carriage_returns() {
        for bytes in [primary_svg(), on_dark_svg()] {
            assert!(
                !bytes.contains(&b'\r'),
                "a CRLF checkout would change the base64 these encode to"
            );
        }
    }

    /// The normaliser has to leave an already-LF asset byte-identical, or it
    /// would be a second source of platform drift rather than the cure.
    #[test]
    fn unit_normalising_leaves_lf_bytes_untouched() {
        assert_eq!(lf(b"<svg>\n  <g/>\n</svg>\n"), b"<svg>\n  <g/>\n</svg>\n");
        assert_eq!(
            lf(b"<svg>\r\n  <g/>\r\n</svg>\r\n"),
            b"<svg>\n  <g/>\n</svg>\n"
        );
    }
}

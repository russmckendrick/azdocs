//! Embedded azdocs product-mark variants shared by the print emitters.

pub(crate) const PRIMARY_SVG: &[u8] =
    include_bytes!("../../docs/marks/assets/azdocs-mark-primary.svg");
pub(crate) const ON_DARK_SVG: &[u8] =
    include_bytes!("../../docs/marks/assets/azdocs-mark-mono-paper.svg");

pub(crate) const PRIMARY_VIRTUAL_PATH: &str = "/azdocs-mark-primary.svg";
pub(crate) const ON_DARK_VIRTUAL_PATH: &str = "/azdocs-mark-on-dark.svg";

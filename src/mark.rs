//! Embedded azdocs product-mark variants shared by the emitters.
//!
//! At the crate root rather than under `report/` because the diagram emitters
//! stamp the lockup too, and `diagram` must not depend on `report`.

pub(crate) const PRIMARY_SVG: &[u8] =
    include_bytes!("../docs/marks/assets/azdocs-mark-primary.svg");
pub(crate) const ON_DARK_SVG: &[u8] =
    include_bytes!("../docs/marks/assets/azdocs-mark-mono-paper.svg");

pub(crate) const PRIMARY_VIRTUAL_PATH: &str = "/azdocs-mark-primary.svg";
pub(crate) const ON_DARK_VIRTUAL_PATH: &str = "/azdocs-mark-on-dark.svg";

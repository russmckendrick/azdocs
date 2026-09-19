//! The bundled IBM Plex faces (OFL, `data/fonts`). The PDF font book loads
//! them ahead of everything else and the PNG rasteriser does the same, so a
//! diagram's labels and a report's body come out of one set of files.

use include_dir::{Dir, include_dir};

pub static VENDORED_FONTS: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/data/fonts");

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use azdocs::diagram::png;

#[test]
fn unit_rasterises_every_mark_asset_when_svg_is_valid() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/marks/assets");
    let mut assets = fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "svg"))
        .collect::<Vec<_>>();
    assets.sort();

    assert_eq!(assets.len(), 10, "all documented SVG variants are present");

    let preview_dir = env::var_os("AZDOCS_MARK_PREVIEW_DIR").map(PathBuf::from);
    if let Some(directory) = &preview_dir {
        fs::create_dir_all(directory).unwrap();
    }

    for asset in assets {
        let svg = fs::read_to_string(&asset).unwrap();
        let rendered = png::from_svg(&svg, 0.25)
            .unwrap_or_else(|error| panic!("{} did not rasterise: {error}", asset.display()));
        assert!(
            rendered.starts_with(&[0x89, b'P', b'N', b'G']),
            "{} did not produce PNG data",
            asset.display()
        );
        if let Some(directory) = &preview_dir {
            let filename = format!("{}.png", asset.file_stem().unwrap().to_string_lossy());
            fs::write(directory.join(filename), rendered).unwrap();
        }
    }
}

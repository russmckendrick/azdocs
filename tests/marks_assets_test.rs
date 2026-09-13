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
    let preview_scale = env::var("AZDOCS_MARK_PREVIEW_SCALE")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(0.25);

    for asset in assets {
        let svg = fs::read_to_string(&asset).unwrap();
        // App icons need transparent canvas margins around their rounded tile.
        let rasterise = if asset
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .starts_with("azdocs-app-icon-")
        {
            png::from_svg_transparent
        } else {
            png::from_svg
        };
        let rendered = rasterise(&svg, preview_scale)
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

#[test]
fn unit_production_mark_keeps_exact_simple_topology_geometry() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/marks/assets");
    for filename in [
        "azdocs-mark-primary.svg",
        "azdocs-mark-reversed.svg",
        "azdocs-app-icon-light.svg",
        "azdocs-app-icon-dark.svg",
    ] {
        let svg = fs::read_to_string(root.join(filename)).unwrap();
        assert!(
            svg.contains("M232 58 C242 38 270 38 280 58 L480 414"),
            "{filename} changed the rounded triangular silhouette"
        );
        assert!(
            svg.contains("M256 198 V330 L156 382 M256 330 L356 382"),
            "{filename} changed the three-way connector geometry"
        );
        assert!(
            svg.contains("<circle cx=\"256\" cy=\"198\" r=\"40\"")
                && svg.contains("<circle cx=\"156\" cy=\"382\" r=\"40\"")
                && svg.contains("<circle cx=\"356\" cy=\"382\" r=\"40\""),
            "{filename} changed the three topology nodes"
        );
    }
}

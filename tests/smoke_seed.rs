mod common;

// Not a golden test: writes a reusable on-disk fixture DB when AZDOCS_SEED_DB
// is set, so the CLI can be smoke-tested offline.
#[test]
fn seed_db_file_for_cli_smoke() {
    let Ok(path) = std::env::var("AZDOCS_SEED_DB") else {
        return;
    };
    let _ = std::fs::remove_file(&path);
    let store = azdocs::store::Store::open(std::path::Path::new(&path)).unwrap();
    common::seed_estate(&store);
}

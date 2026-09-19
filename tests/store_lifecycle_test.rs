//! Open modes, snapshot status semantics and strict decoding.

use azdocs::error::StoreError;
use azdocs::model::SnapshotStatus;
use azdocs::store::Store;

fn snapshot_with_status(store: &Store, status: SnapshotStatus) -> String {
    let snapshot = store.create_snapshot("tenant", None).unwrap();
    store.set_snapshot_status(&snapshot.id, status).unwrap();
    snapshot.id
}

#[test]
fn unit_open_read_only_reports_missing_database() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("absent.db");

    let error = Store::open_read_only(&path).err().unwrap();

    assert!(matches!(error, StoreError::DatabaseMissing(p) if p == path));
    assert!(
        !path.exists(),
        "a read-only open must never create the file"
    );
}

#[test]
fn unit_open_read_only_refuses_writes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("estate.db");
    Store::open(&path).unwrap();

    let store = Store::open_read_only(&path).unwrap();

    assert!(store.create_snapshot("tenant", None).is_err());
}

#[test]
fn unit_open_read_only_reports_migration_required_when_schema_is_old() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.db");
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO meta VALUES ('schema_version', '1');",
        )
        .unwrap();
    }

    let error = Store::open_read_only(&path).err().unwrap();

    assert!(matches!(
        error,
        StoreError::MigrationRequired { found: 1, .. }
    ));
}

#[test]
fn unit_open_rejects_schema_newer_than_supported() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("future.db");
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO meta VALUES ('schema_version', '999');",
        )
        .unwrap();
    }

    assert!(matches!(
        Store::open(&path).err().unwrap(),
        StoreError::SchemaTooNew { found: 999, .. }
    ));
    assert!(matches!(
        Store::open_read_only(&path).err().unwrap(),
        StoreError::SchemaTooNew { found: 999, .. }
    ));
}

#[test]
fn unit_open_reconciles_abandoned_running_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("estate.db");
    let id = {
        let store = Store::open(&path).unwrap();
        let id = store.create_snapshot("tenant", None).unwrap().id;
        // Backdate the heartbeat past the stale window.
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute(
                "UPDATE snapshots SET heartbeat_at = '2020-01-01T00:00:00+00:00' WHERE id = ?1",
                [&id],
            )
            .unwrap();
        id
    };

    let store = Store::open(&path).unwrap();

    let snapshot = store.get_snapshot(&id).unwrap();
    assert_eq!(snapshot.status, SnapshotStatus::Failed);
    assert!(snapshot.interrupted_at.is_some());
}

#[test]
fn unit_resolve_latest_skips_failed_and_cancelled_snapshots() {
    let store = Store::open_in_memory().unwrap();
    let good = snapshot_with_status(&store, SnapshotStatus::Partial);
    snapshot_with_status(&store, SnapshotStatus::Failed);
    snapshot_with_status(&store, SnapshotStatus::Cancelled);
    store.create_snapshot("tenant", None).unwrap();

    assert_eq!(store.resolve_snapshot("latest").unwrap(), good);
}

#[test]
fn unit_resolve_latest_reports_no_snapshots_when_only_failed_exist() {
    let store = Store::open_in_memory().unwrap();
    snapshot_with_status(&store, SnapshotStatus::Failed);

    assert!(matches!(
        store.resolve_snapshot("latest"),
        Err(StoreError::NoSnapshots)
    ));
}

#[test]
fn unit_previous_snapshot_skips_failed_and_running() {
    let store = Store::open_in_memory().unwrap();
    let baseline = snapshot_with_status(&store, SnapshotStatus::Complete);
    snapshot_with_status(&store, SnapshotStatus::Failed);
    store.create_snapshot("tenant", None).unwrap();
    let current = snapshot_with_status(&store, SnapshotStatus::Complete);

    let previous = store.previous_snapshot(&current).unwrap();

    assert_eq!(previous.map(|s| s.id), Some(baseline.clone()));
    assert!(store.previous_snapshot(&baseline).unwrap().is_none());
}

#[test]
fn unit_previous_snapshot_never_crosses_tenants() {
    let store = Store::open_in_memory().unwrap();
    let other = store.create_snapshot("other", None).unwrap();
    store
        .set_snapshot_status(&other.id, SnapshotStatus::Complete)
        .unwrap();
    let current = snapshot_with_status(&store, SnapshotStatus::Complete);

    assert!(store.previous_snapshot(&current).unwrap().is_none());
}

#[test]
fn unit_resolve_snapshot_rejects_wildcard_reference() {
    let store = Store::open_in_memory().unwrap();
    store.create_snapshot("tenant", None).unwrap();
    store.create_snapshot("tenant", None).unwrap();

    for reference in ["%", "_", "", "a%"] {
        assert!(
            matches!(
                store.resolve_snapshot(reference),
                Err(StoreError::SnapshotNotFound(_))
            ),
            "{reference:?} must not act as a pattern"
        );
    }
}

#[test]
fn unit_snapshot_row_with_bad_timestamp_is_an_error() {
    let store = Store::open_in_memory().unwrap();
    let id = store.create_snapshot("tenant", None).unwrap().id;
    // Reach the raw table the way corruption would: through a second
    // connection is not possible in memory, so use a writable open on disk.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("estate.db");
    let disk = Store::open(&path).unwrap();
    let disk_id = disk.create_snapshot("tenant", None).unwrap().id;
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute(
            "UPDATE snapshots SET created_at = 'yesterday' WHERE id = ?1",
            [&disk_id],
        )
        .unwrap();

    let error = disk.get_snapshot(&disk_id).unwrap_err().to_string();

    assert!(error.contains("snapshots.created_at"), "{error}");
    assert!(store.get_snapshot(&id).is_ok());
}

#[test]
fn unit_snapshot_row_with_unknown_status_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("estate.db");
    let store = Store::open(&path).unwrap();
    let id = store.create_snapshot("tenant", None).unwrap().id;
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute(
            "UPDATE snapshots SET status = 'paused' WHERE id = ?1",
            [&id],
        )
        .unwrap();

    let error = store.get_snapshot(&id).unwrap_err().to_string();

    assert!(error.contains("snapshots.status"), "{error}");
}

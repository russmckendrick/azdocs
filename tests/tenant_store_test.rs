use azdocs::{error::StoreError, model::SnapshotStatus, store::Store};
#[test]
fn unit_scopes_latest_history_and_deletion_to_the_selected_tenant() {
    let store = Store::open_in_memory().unwrap();
    let a = store.create_snapshot("tenant-a", None).unwrap();
    store
        .set_snapshot_status(&a.id, SnapshotStatus::Complete)
        .unwrap();
    let b = store.create_snapshot("tenant-b", None).unwrap();
    store
        .set_snapshot_status(&b.id, SnapshotStatus::Complete)
        .unwrap();
    assert!(matches!(
        store.resolve_snapshot("latest"),
        Err(StoreError::TenantSelectionRequired)
    ));
    let store = store.with_tenant(Some("TENANT-A"));
    assert_eq!(store.resolve_snapshot("latest").unwrap(), a.id);
    assert_eq!(store.list_snapshots().unwrap().len(), 1);
    assert!(store.resolve_snapshot(&b.id).is_err());
    assert!(store.get_snapshot(&b.id).is_err());
    assert!(store.delete_snapshot(&b.id).is_err());
}
#[test]
fn unit_rejects_cross_tenant_comparison_even_without_a_filter() {
    let store = Store::open_in_memory().unwrap();
    let a = store.create_snapshot("a", None).unwrap();
    let b = store.create_snapshot("b", None).unwrap();
    assert!(matches!(
        store.diff_snapshots(&a.id, &b.id),
        Err(StoreError::CrossTenantComparison)
    ));
}
#[test]
fn unit_opens_explicit_snapshots_in_an_unconfigured_multi_tenant_database() {
    let store = Store::open_in_memory().unwrap();
    let a = store.create_snapshot("a", None).unwrap();
    store.create_snapshot("b", None).unwrap();
    assert_eq!(store.resolve_snapshot(&a.id).unwrap(), a.id);
}

use serde_json::json;

use azdocs::collect::ingest;
use azdocs::model::SnapshotStatus;
use azdocs::querypack::QueryPack;
use azdocs::store::Store;

fn fixture_resources() -> Vec<serde_json::Value> {
    vec![
        json!({
            "id": "/subscriptions/s1/resourceGroups/RG-Net/providers/Microsoft.Network/virtualNetworks/vnet-hub",
            "name": "vnet-hub",
            "type": "microsoft.network/virtualnetworks",
            "location": "uksouth",
            "resourceGroup": "rg-net",
            "subscriptionId": "s1",
            "tags": {"env": "prod"},
            "properties": {"addressSpace": {"addressPrefixes": ["10.0.0.0/16"]}}
        }),
        json!({
            "id": "/subscriptions/s1/resourceGroups/RG-App/providers/Microsoft.Compute/virtualMachines/vm-1",
            "name": "vm-1",
            "type": "Microsoft.Compute/virtualMachines",
            "location": "uksouth",
            "resourceGroup": "RG-App",
            "subscriptionId": "s1",
            "properties": {"hardwareProfile": {"vmSize": "Standard_B2s"}}
        }),
    ]
}

#[test]
fn ingest_all_resources_populates_resources_table() {
    let store = Store::open_in_memory().unwrap();
    let snapshot = store.create_snapshot("tenant-1", None).unwrap();
    let pack = QueryPack::builtin().unwrap();
    let def = pack.get("all_resources").unwrap();

    ingest::ingest(&store, &snapshot.id, def, &fixture_resources()).unwrap();

    let resources = store.resources(&snapshot.id).unwrap();
    assert_eq!(resources.len(), 2);
}

#[test]
fn ingest_normalizes_mixed_case_ids_to_one_row_per_resource() {
    let store = Store::open_in_memory().unwrap();
    let snapshot = store.create_snapshot("tenant-1", None).unwrap();
    let pack = QueryPack::builtin().unwrap();
    let def = pack.get("all_resources").unwrap();
    // Same resource seen twice with different id casing (classic ARG quirk).
    let rows = vec![
        json!({
            "id": "/subscriptions/s1/resourceGroups/RG/providers/Microsoft.Compute/disks/d1",
            "name": "d1", "type": "microsoft.compute/disks", "subscriptionId": "s1"
        }),
        json!({
            "id": "/subscriptions/s1/resourcegroups/rg/providers/microsoft.compute/disks/d1",
            "name": "d1", "type": "microsoft.compute/disks", "subscriptionId": "s1"
        }),
    ];

    ingest::ingest(&store, &snapshot.id, def, &rows).unwrap();

    let resources = store.resources(&snapshot.id).unwrap();
    assert_eq!(resources.len(), 1, "mixed-case duplicates must collapse");
}

#[test]
fn ingest_subscriptions_and_resource_groups_populate_typed_tables() {
    let store = Store::open_in_memory().unwrap();
    let snapshot = store.create_snapshot("tenant-1", None).unwrap();
    let pack = QueryPack::builtin().unwrap();

    ingest::ingest(
        &store,
        &snapshot.id,
        pack.get("subscriptions").unwrap(),
        &[json!({"subscriptionId": "s1", "name": "Production", "state": "Enabled"})],
    )
    .unwrap();
    ingest::ingest(
        &store,
        &snapshot.id,
        pack.get("resource_groups").unwrap(),
        &[
            json!({"id": "/subscriptions/s1/resourceGroups/RG-App", "name": "RG-App",
                 "subscriptionId": "s1", "location": "uksouth"}),
        ],
    )
    .unwrap();

    let subs = store.subscriptions(&snapshot.id).unwrap();
    let groups = store.resource_groups(&snapshot.id).unwrap();
    assert_eq!(
        (subs.len(), groups.len(), groups[0].id.as_str()),
        (1, 1, "/subscriptions/s1/resourcegroups/rg-app")
    );
}

#[test]
fn ingest_other_inventory_queries_keep_raw_rows() {
    let store = Store::open_in_memory().unwrap();
    let snapshot = store.create_snapshot("tenant-1", None).unwrap();
    let pack = QueryPack::builtin().unwrap();
    let def = pack.get("resource_type_counts").unwrap();
    let rows = vec![json!({"type": "microsoft.compute/disks", "count": 4})];

    ingest::ingest(&store, &snapshot.id, def, &rows).unwrap();

    let stored = store
        .query_results(&snapshot.id, "resource_type_counts")
        .unwrap();
    assert_eq!(stored, rows);
}

#[test]
fn snapshot_diff_reports_added_removed_changed() {
    let store = Store::open_in_memory().unwrap();
    let pack = QueryPack::builtin().unwrap();
    let def = pack.get("all_resources").unwrap();
    let a = store.create_snapshot("tenant-1", None).unwrap();
    let b = store.create_snapshot("tenant-1", None).unwrap();
    let base = fixture_resources();
    let mut altered = vec![base[0].clone()];
    altered[0]["properties"]["addressSpace"]["addressPrefixes"][0] = json!("10.9.0.0/16");
    altered.push(json!({
        "id": "/subscriptions/s1/resourceGroups/rg-new/providers/Microsoft.Web/sites/app-1",
        "name": "app-1", "type": "microsoft.web/sites", "subscriptionId": "s1"
    }));

    ingest::ingest(&store, &a.id, def, &base).unwrap();
    ingest::ingest(&store, &b.id, def, &altered).unwrap();
    let diff = store.diff_snapshots(&a.id, &b.id).unwrap();

    assert_eq!(
        (diff.added.len(), diff.removed.len(), diff.changed.len()),
        (1, 1, 1),
        "diff: {diff:?}"
    );
}

#[test]
fn snapshot_delete_cascades_to_all_tables() {
    let store = Store::open_in_memory().unwrap();
    let snapshot = store.create_snapshot("tenant-1", None).unwrap();
    let pack = QueryPack::builtin().unwrap();
    ingest::ingest(
        &store,
        &snapshot.id,
        pack.get("all_resources").unwrap(),
        &fixture_resources(),
    )
    .unwrap();

    store.delete_snapshot(&snapshot.id).unwrap();

    let resources = store.resources(&snapshot.id).unwrap();
    assert!(resources.is_empty());
}

#[test]
fn resolve_snapshot_latest_skips_running_snapshots() {
    let store = Store::open_in_memory().unwrap();
    let done = store.create_snapshot("tenant-1", None).unwrap();
    store
        .set_snapshot_status(&done.id, SnapshotStatus::Complete)
        .unwrap();
    let _running = store.create_snapshot("tenant-1", None).unwrap();

    let resolved = store.resolve_snapshot("latest").unwrap();

    assert_eq!(resolved, done.id);
}

#[test]
fn resolve_snapshot_accepts_unambiguous_prefix() {
    let store = Store::open_in_memory().unwrap();
    let snapshot = store.create_snapshot("tenant-1", None).unwrap();

    let resolved = store.resolve_snapshot(&snapshot.id[..8]).unwrap();

    assert_eq!(resolved, snapshot.id);
}

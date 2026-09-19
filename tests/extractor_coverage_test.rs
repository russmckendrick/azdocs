//! The fixture must give every relationship kind a producer, so the goldens
//! and the desktop tests see each of them drawn at least once.

mod common;

use azdocs::model::EdgeKind;
use azdocs::store::Store;

const EVERY_KIND: [EdgeKind; 13] = [
    EdgeKind::SubnetOf,
    EdgeKind::InVnet,
    EdgeKind::PeeredWith,
    EdgeKind::NsgAttached,
    EdgeKind::NicInSubnet,
    EdgeKind::AttachedTo,
    EdgeKind::PrivateEndpointFor,
    EdgeKind::DnsLinked,
    EdgeKind::DependsOn,
    EdgeKind::RunsOn,
    EdgeKind::UsesIdentity,
    EdgeKind::LogsTo,
    EdgeKind::Monitors,
];

#[test]
fn fixture_exercises_every_edge_kind() {
    let store = Store::open_in_memory().unwrap();
    let id = common::seed_estate(&store);
    let edges = store.edges(&id).unwrap();
    let missing: Vec<_> = EVERY_KIND
        .iter()
        .filter(|kind| !edges.iter().any(|edge| edge.kind == **kind))
        .map(|kind| kind.as_str())
        .collect();
    assert!(missing.is_empty(), "no fixture edge of kind {missing:?}");
}

#[test]
fn fixture_history_records_a_real_change_set() {
    let store = Store::open_in_memory().unwrap();
    let current = common::seed_history(&store);
    let previous = store
        .previous_snapshot(&current)
        .unwrap()
        .expect("older snapshot");
    let changes = store.snapshot_changes(&previous.id, &current).unwrap();
    fn names(refs: &[azdocs::model::diff::ResourceRef]) -> Vec<&str> {
        refs.iter().map(|r| r.name.as_str()).collect()
    }
    assert_eq!(names(&changes.resources.removed), ["disk-old-test"]);
    assert!(names(&changes.resources.added).contains(&"pe-sql"));
    assert!(
        changes
            .resources
            .changed
            .iter()
            .any(|c| c.resource.name == "vm-app-01")
    );
    assert!(
        changes
            .findings
            .added
            .iter()
            .any(|f| f.query_name == "storage_public_blob_access"),
        "public blob access is new"
    );
    assert!(
        changes
            .findings
            .resolved
            .iter()
            .any(|f| f.query_name == "orphaned_resources"),
        "the orphaned disk's finding is gone with it"
    );
    assert!(
        changes
            .edges
            .added
            .iter()
            .any(|e| e.kind == "private_endpoint_for")
    );
}

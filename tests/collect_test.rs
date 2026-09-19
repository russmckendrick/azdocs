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

mod end_to_end {
    use std::sync::Arc;

    use azdocs::arg::ArgClient;
    use azdocs::auth::StaticTokenProvider;
    use azdocs::collect::{CollectRequest, run_with_progress};
    use serde_json::json;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    fn resource(name: &str, azure_type: &str) -> serde_json::Value {
        json!({
            "id": format!("/subscriptions/s1/resourceGroups/rg/providers/{azure_type}/{name}"),
            "name": name, "type": azure_type.to_lowercase(),
            "subscriptionId": "s1", "resourceGroup": "rg",
            "tags": {"env": "prod"}
        })
    }

    /// Pagination, a failing sibling query, ingest, extractors and the
    /// required-tag audit all in one run: the shape of a real collect.
    #[tokio::test]
    async fn unit_collect_follows_skip_token_pages_and_marks_partial_when_a_query_fails() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(|request: &Request| {
                let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
                let query = body["query"].as_str().unwrap_or_default();
                if query.contains("authorizationresources") {
                    return ResponseTemplate::new(400).set_body_json(json!({
                        "error": {"code": "BadRequest", "message": "no such table"}
                    }));
                }
                if body["options"]["$skipToken"].is_null() {
                    ResponseTemplate::new(200).set_body_json(json!({
                        "totalRecords": 3, "count": 2, "$skipToken": "page-2",
                        "data": [
                            resource("vnet-1", "Microsoft.Network/virtualNetworks"),
                            resource("vm-1", "Microsoft.Compute/virtualMachines"),
                        ]
                    }))
                } else {
                    ResponseTemplate::new(200).set_body_json(json!({
                        "totalRecords": 3, "count": 1,
                        "data": [{"id": "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Web/sites/app", "type": "microsoft.web/sites", "subscriptionId": "s1", "resourceGroup": "rg"}]
                    }))
                }
            })
            .mount(&server)
            .await;
        let store = azdocs::store::Store::open_in_memory().unwrap();
        let client = Arc::new(
            ArgClient::with_endpoint(
                reqwest::Client::new(),
                StaticTokenProvider("t".into()),
                &server.uri(),
            )
            .with_retry_policy(azdocs::arg::RetryPolicy {
                max_attempts: 1,
                ..azdocs::arg::RetryPolicy::default()
            }),
        );
        let pack = azdocs::querypack::QueryPack::builtin().unwrap();
        let queries = vec![
            pack.get("all_resources").unwrap().clone(),
            pack.get("role_assignments").unwrap().clone(),
        ];

        let summary = run_with_progress(
            &store,
            client,
            CollectRequest {
                tenant_id: "tenant-1".into(),
                queries,
                subscriptions: vec![],
                concurrency: 2,
                notes: None,
                required_tags: vec!["env".into()],
                quiet: true,
            },
            |_| {},
        )
        .await
        .unwrap();

        assert_eq!(summary.status, azdocs::model::SnapshotStatus::Partial);
        assert_eq!((summary.queries_run, summary.queries_failed), (2, 1));
        // Three rows came back over two pages; one had no name and was dropped.
        assert_eq!(store.resources(&summary.snapshot_id).unwrap().len(), 2);
        let runs = store.query_runs(&summary.snapshot_id).unwrap();
        let all = runs
            .iter()
            .find(|r| r.query_name == "all_resources")
            .unwrap();
        let failed = runs
            .iter()
            .find(|r| r.query_name == "role_assignments")
            .unwrap();
        assert_eq!((all.row_count, all.rows_dropped), (Some(3), Some(1)));
        assert!(
            failed
                .error
                .as_deref()
                .unwrap_or_default()
                .contains("BadRequest")
        );
        assert!(failed.provenance.is_some(), "failed runs keep provenance");
        assert_eq!(store.findings(&summary.snapshot_id).unwrap().len(), 0);
        assert_eq!(
            store.resolve_snapshot("latest").unwrap(),
            summary.snapshot_id
        );
    }

    #[tokio::test]
    async fn collect_run_stores_resources_edges_and_tag_findings() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "totalRecords": 2, "count": 2,
                "data": [
                    {
                        "id": "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Network/virtualNetworks/vnet-1",
                        "name": "vnet-1", "type": "microsoft.network/virtualnetworks",
                        "subscriptionId": "s1", "resourceGroup": "rg",
                        "tags": {"env": "prod"},
                        "properties": {"subnets": [{"id": "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Network/virtualNetworks/vnet-1/subnets/app"}]}
                    },
                    {
                        "id": "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Compute/virtualMachines/vm-1",
                        "name": "vm-1", "type": "microsoft.compute/virtualmachines",
                        "subscriptionId": "s1", "resourceGroup": "rg"
                    }
                ]
            })))
            .mount(&server)
            .await;
        let store = azdocs::store::Store::open_in_memory().unwrap();
        let client = Arc::new(ArgClient::with_endpoint(
            reqwest::Client::new(),
            StaticTokenProvider("t".into()),
            &server.uri(),
        ));
        let pack = azdocs::querypack::QueryPack::builtin().unwrap();
        let queries = vec![pack.get("all_resources").unwrap().clone()];

        let updates = std::sync::Mutex::new(Vec::new());
        let summary = run_with_progress(
            &store,
            client,
            CollectRequest {
                tenant_id: "tenant-1".into(),
                queries,
                subscriptions: vec![],
                concurrency: 2,
                notes: None,
                required_tags: vec!["env".into()],
                quiet: true,
            },
            |progress| updates.lock().unwrap().push(progress),
        )
        .await
        .unwrap();

        let updates = updates.into_inner().unwrap();
        assert_eq!(
            (updates[0].completed, updates[0].total, updates[0].rows),
            (0, 1, 0)
        );
        assert_eq!(
            (updates[1].completed, updates[1].rows, updates[1].failed),
            (1, summary.rows_ingested, summary.queries_failed)
        );
        assert!(updates[1].latest_query.is_some());
        let edges = store.edges(&summary.snapshot_id).unwrap();
        let findings = store.findings(&summary.snapshot_id).unwrap();
        assert_eq!(
            (
                summary.status,
                summary.rows_ingested,
                edges.len(),
                findings.len()
            ),
            (azdocs::model::SnapshotStatus::Complete, 2, 1, 1),
            "edges: {edges:?}, findings: {findings:?}"
        );
    }
}

#[test]
fn unit_policy_findings_link_to_affected_resources_without_losing_evidence_ids() {
    let pack = QueryPack::builtin().unwrap();
    let row = json!({"id":"/POLICY/STATE-1", "resourceId":"/Subscriptions/S1/ResourceGroups/RG/Providers/Microsoft.Compute/virtualMachines/VM", "summary":"Non-compliant assignment"});
    let finding = ingest::finding_from_row(pack.get("policy_non_compliant").unwrap(), &row);
    assert_eq!(
        finding.resource_id.as_deref(),
        Some("/subscriptions/s1/resourcegroups/rg/providers/microsoft.compute/virtualmachines/vm")
    );
    assert_eq!(finding.detail.unwrap()["id"], "/POLICY/STATE-1");
}

#[test]
fn unit_scope_findings_never_substitute_evidence_ids_for_missing_resources() {
    let pack = QueryPack::builtin().unwrap();
    let finding = ingest::finding_from_row(
        pack.get("policy_exemptions_expiring").unwrap(),
        &json!({"id":"/policy/exemption-1"}),
    );
    assert_eq!(finding.resource_id, None);
}

#[tokio::test]
async fn unit_failed_collection_preserves_the_attempted_query_and_scope() {
    use azdocs::arg::ArgClient;
    use azdocs::auth::StaticTokenProvider;
    use azdocs::collect::{CollectRequest, run};
    use azdocs::model::SnapshotStatus;
    use std::sync::Arc;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    let server = MockServer::start().await;
    Mock::given(wiremock::matchers::method("POST"))
        .respond_with(ResponseTemplate::new(400).set_body_json(
            serde_json::json!({"error":{"code":"InvalidQuery","message":"fixture failure"}}),
        ))
        .mount(&server)
        .await;
    let store = Store::open_in_memory().unwrap();
    let pack = QueryPack::builtin().unwrap();
    let def = pack.get("role_assignments").unwrap().clone();
    let expected = def.provenance(&["SUB-A".into()]);
    let summary = run(
        &store,
        Arc::new(ArgClient::with_endpoint(
            reqwest::Client::new(),
            StaticTokenProvider("t".into()),
            &server.uri(),
        )),
        CollectRequest {
            tenant_id: "tenant".into(),
            queries: vec![def],
            subscriptions: vec!["SUB-A".into()],
            concurrency: 1,
            notes: None,
            required_tags: vec![],
            quiet: true,
        },
    )
    .await
    .unwrap();
    assert_eq!(summary.status, SnapshotStatus::Failed);
    let runs = store.query_runs(&summary.snapshot_id).unwrap();
    assert_eq!(runs[0].provenance.as_ref(), Some(&expected));
    assert!(runs[0].error.is_some());
}

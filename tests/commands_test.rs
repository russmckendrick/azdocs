//! The command layer's pure and store-backed behaviour, without a terminal:
//! exit policies, machine output and the maintenance commands.

mod common;

use azdocs::auth::diagnostics::{
    AccessIssue, AccessIssueKind, ConnectionCheck, PermissionVerdict, VisibleSubscription,
};
use azdocs::cli::{FailOn, OutputFormat, SnapshotsCommand};
use azdocs::commands::{check, collect, init, query, snapshots};
use azdocs::config::ConfigDocument;
use azdocs::config::secrets::SecretStore;
use azdocs::error::ConfigError;
use azdocs::labels::Labels;
use azdocs::model::{QueryRun, SnapshotStatus};
use azdocs::querypack::QueryPack;
use azdocs::store::Store;

#[test]
fn unit_fail_on_policy_matrix() {
    use SnapshotStatus::*;
    for (status, policy, fails) in [
        (Complete, FailOn::Failed, false),
        (Warnings, FailOn::Failed, false),
        (Partial, FailOn::Failed, false),
        (Failed, FailOn::Failed, true),
        (Cancelled, FailOn::Failed, true),
        (Complete, FailOn::Partial, false),
        (Warnings, FailOn::Partial, true),
        (Partial, FailOn::Partial, true),
        (Failed, FailOn::Partial, true),
        (Cancelled, FailOn::Partial, true),
    ] {
        assert_eq!(
            collect::enforce_fail_on(status, policy).is_err(),
            fails,
            "{status:?} under {policy:?}"
        );
    }
}

fn connection_check(issues: Vec<AccessIssue>) -> ConnectionCheck {
    ConnectionCheck {
        checked_at: "2026-09-19T12:00:00Z".into(),
        subscriptions: vec![VisibleSubscription {
            id: "sub-prod".into(),
            name: "Production".into(),
        }],
        inaccessible_subscriptions: Vec::new(),
        verdict: PermissionVerdict::ReadOnly,
        grants: Vec::new(),
        issues,
    }
}

#[test]
fn unit_check_json_carries_a_blocking_verdict() {
    let clean = connection_check(Vec::new());
    let json: serde_json::Value =
        serde_json::from_str(&check::render_json("t", &clean).unwrap()).unwrap();
    assert_eq!(json["ok"], true);
    assert!(check::enforce_blocking(&clean).is_ok());

    let blocked = connection_check(vec![AccessIssue {
        kind: AccessIssueKind::NoSubscriptions,
        scope: "tenant".into(),
    }]);
    let json: serde_json::Value =
        serde_json::from_str(&check::render_json("t", &blocked).unwrap()).unwrap();
    assert_eq!(json["ok"], false);
    assert_eq!(json["check"]["issues"][0]["kind"], "no_subscriptions");
    assert!(check::enforce_blocking(&blocked).is_err());
}

#[test]
fn unit_show_json_includes_schema_version_and_dropped_rows() {
    let store = Store::open_in_memory().unwrap();
    let id = common::seed_estate(&store);
    store
        .record_query_run(
            &id,
            &QueryRun {
                provenance: None,
                query_name: "lossy_query".into(),
                category: "inventory".into(),
                row_count: Some(3),
                duration_ms: Some(5),
                error: None,
                rows_dropped: Some(2),
            },
        )
        .unwrap();
    let words = &Labels::default().cli.snapshots;

    let rendered = snapshots::render_show(&store, "latest", OutputFormat::Json, words).unwrap();

    let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();
    assert_eq!(json["schema_version"], store.schema_version().unwrap());
    assert_eq!(json["snapshot"]["id"], id);
    let lossy = json["query_runs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|run| run["query_name"] == "lossy_query")
        .expect("recorded run");
    assert_eq!(lossy["rows_dropped"], 2);
    let table = snapshots::render_show(&store, "latest", OutputFormat::Table, words).unwrap();
    assert!(table.contains("lossy_query"));
}

#[test]
fn unit_delete_refuses_a_running_snapshot_unless_forced() {
    let store = Store::open_in_memory().unwrap();
    let id = common::seed_estate(&store);
    store
        .set_snapshot_status(&id, SnapshotStatus::Running)
        .unwrap();
    let labels = Labels::default();
    let delete = |force: bool| SnapshotsCommand::Delete {
        snapshot: id.clone(),
        yes: true,
        force,
        vacuum: false,
    };

    let refused = snapshots::run(&store, &delete(false), &labels).unwrap_err();
    assert!(refused.to_string().contains("--force"), "{refused}");
    assert!(store.get_snapshot(&id).is_ok(), "nothing was deleted");

    snapshots::run(&store, &delete(true), &labels).unwrap();
    assert!(store.get_snapshot(&id).is_err());
}

#[test]
fn unit_delete_without_yes_explains_instead_of_deleting() {
    let store = Store::open_in_memory().unwrap();
    let id = common::seed_estate(&store);
    let command = SnapshotsCommand::Delete {
        snapshot: id.clone(),
        yes: false,
        force: false,
        vacuum: false,
    };

    let refused = snapshots::run(&store, &command, &Labels::default()).unwrap_err();

    assert!(refused.to_string().contains("--yes"), "{refused}");
    assert!(store.get_snapshot(&id).is_ok());
}

#[test]
fn unit_prune_never_selects_running_snapshots() {
    let store = Store::open_in_memory().unwrap();
    let older = common::seed_estate(&store);
    let running = common::seed_estate(&store);
    store
        .set_snapshot_status(&running, SnapshotStatus::Running)
        .unwrap();
    let newest = common::seed_estate(&store);
    let entries = store.list_snapshots().unwrap();

    let doomed = snapshots::select_prunable(&entries, Some(1), None, chrono::Utc::now());

    assert_eq!(
        doomed,
        vec![older.clone()],
        "keep 1 counts only usable rows"
    );
    assert!(!doomed.contains(&running));
    assert!(!doomed.contains(&newest));
    let stale = snapshots::select_prunable(
        &entries,
        None,
        Some(0),
        chrono::Utc::now() + chrono::Duration::days(2),
    );
    assert!(stale.contains(&older) && stale.contains(&newest) && !stale.contains(&running));
}

#[test]
fn unit_verify_passes_on_a_healthy_database() {
    let store = Store::open_in_memory().unwrap();
    common::seed_estate(&store);

    snapshots::run(
        &store,
        &SnapshotsCommand::Verify {
            format: OutputFormat::Json,
        },
        &Labels::default(),
    )
    .unwrap();
    assert!(store.integrity_check().unwrap().is_ok());
}

#[derive(Default)]
struct MemorySecrets(std::cell::RefCell<std::collections::BTreeMap<String, String>>);

impl SecretStore for MemorySecrets {
    fn get(&self, key: &str) -> Result<String, ConfigError> {
        self.0
            .borrow()
            .get(key)
            .cloned()
            .ok_or_else(|| ConfigError::Secret("missing".into()))
    }
    fn set(&self, key: &str, value: &str) -> Result<(), ConfigError> {
        self.0.borrow_mut().insert(key.into(), value.into());
        Ok(())
    }
    fn delete(&self, key: &str) -> Result<(), ConfigError> {
        self.0.borrow_mut().remove(key);
        Ok(())
    }
}

#[test]
fn unit_init_non_interactive_writes_a_secret_reference_not_a_secret() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("azdocs.toml");
    // Only this test touches these variables in this binary.
    unsafe {
        std::env::set_var(
            azdocs::config::ENV_TENANT_ID,
            "11111111-1111-4111-8111-111111111111",
        );
        std::env::set_var(
            azdocs::config::ENV_CLIENT_ID,
            "22222222-2222-4222-8222-222222222222",
        );
    }
    let secrets = MemorySecrets::default();
    let options = init::InitOptions {
        force: false,
        non_interactive: true,
        secret_env: Some("ACME_SECRET".into()),
    };

    init::run_with_store(Some(&path), &options, &Labels::default(), &secrets).unwrap();

    let document = ConfigDocument::load(Some(&path)).unwrap();
    let profile = document.values.tenants.values().next().expect("one tenant");
    assert_eq!(profile.secret_env.as_deref(), Some("ACME_SECRET"));
    assert!(profile.secret_ref.is_none());
    assert!(secrets.0.borrow().is_empty(), "nothing went to the store");
    assert!(
        !std::fs::read_to_string(&path)
            .unwrap()
            .contains("client_secret =")
    );
    let again = init::run_with_store(Some(&path), &options, &Labels::default(), &secrets);
    assert!(again.unwrap_err().to_string().contains("--force"));
}

#[test]
fn unit_query_list_json_names_every_builtin() {
    let pack = QueryPack::builtin().unwrap();
    let listings = query::listings(&pack, None);
    assert_eq!(listings.len(), pack.all().len());
    let json: serde_json::Value = serde_json::to_value(&listings).unwrap();
    let names: Vec<&str> = json
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"storage_accounts"));
    let findings = query::listings(&pack, Some("security"));
    assert!(findings.iter().all(|entry| entry.category == "security"));
    assert!(
        findings
            .iter()
            .any(|entry| entry.severity.as_deref() == Some("high"))
    );
}

mod common;

use std::collections::BTreeMap;

use azdocs::labels::Labels;
use azdocs::model::{AuthorizationScope, QueryRun};
use azdocs::querypack::{QueryDef, QueryPack};
use azdocs::report::{ReportContext, posture::PostureReport};
use azdocs::store::Store;
use serde_json::{Value, json};

fn run(name: &str, rows: usize) -> QueryRun {
    let pack = QueryPack::builtin().unwrap();
    let def = pack.get(name).unwrap();
    QueryRun {
        query_name: name.into(),
        category: def.category.clone(),
        row_count: Some(rows as u64),
        duration_ms: Some(1),
        error: None,
        provenance: Some(def.provenance(&[])),
    }
}

fn summaries(
    data: BTreeMap<String, Vec<Value>>,
    runs: &[QueryRun],
) -> Vec<azdocs::report::posture::EvidenceTable> {
    PostureReport::build(&data, runs, "2026-09-13T12:00:00Z".parse().unwrap(), &[])
        .tables(&Labels::default())
}

#[test]
fn unit_freshness_preserves_boundary_future_and_missing_timestamps() {
    let data = vec![
        json!({"assessedAt":"2026-09-10T12:00:00Z"}), // Exactly 72h is within threshold.
        json!({"assessedAt":"2026-09-10T11:59:59Z"}),
        json!({"assessedAt":"2026-09-13T12:00:01Z"}),
        json!({"assessedAt":"invalid"}),
        json!({"assessedAt":null}),
    ];
    let tables = summaries(
        BTreeMap::from([("patch_assessments".into(), data)]),
        &[run("patch_assessments", 5)],
    );
    let table = tables
        .iter()
        .find(|t| t.key == "evidence_freshness")
        .unwrap();
    let buckets: BTreeMap<_, _> = table
        .rows
        .iter()
        .map(|row| (row[2].as_str(), row[3].as_str()))
        .collect();
    assert_eq!(
        buckets,
        BTreeMap::from([
            ("Timestamp after collection", "1"),
            ("Older than review threshold", "1"),
            ("Within review threshold", "1"),
            ("Missing or invalid timestamp", "2"),
        ])
    );
}

#[test]
fn unit_freshness_uses_the_recorded_override_and_suppresses_failed_evidence() {
    let data = BTreeMap::from([(
        "patch_assessments".into(),
        vec![json!({"assessedAt":"2026-09-11T12:00:00Z"})],
    )]);
    let mut recorded = run("patch_assessments", 1);
    recorded
        .provenance
        .as_mut()
        .unwrap()
        .freshness
        .as_mut()
        .unwrap()
        .max_age_hours = 24;
    let tables = summaries(data.clone(), &[recorded.clone()]);
    let table = tables
        .iter()
        .find(|t| t.key == "evidence_freshness")
        .unwrap();
    assert_eq!(
        table.rows[0][1..],
        ["24", "Older than review threshold", "1"]
    );
    recorded.error = Some("collection failed".into());
    assert!(
        !summaries(data, &[recorded])
            .iter()
            .any(|t| t.key == "evidence_freshness")
    );
}

#[test]
fn unit_policy_coverage_joins_full_ids_and_keeps_unevaluated_assignments() {
    let data = BTreeMap::from([
        (
            "policy_assignments".into(),
            vec![
                json!({"id":"/SUBSCRIPTIONS/A/ASSIGNMENTS/BASELINE"}),
                json!({"id":"/subscriptions/b/assignments/baseline"}),
            ],
        ),
        (
            "policy_states".into(),
            vec![
                json!({"policyAssignmentId":"/subscriptions/a/assignments/baseline","complianceState":"Exempt"}),
            ],
        ),
    ]);
    let tables = summaries(
        data.clone(),
        &[run("policy_assignments", 2), run("policy_states", 1)],
    );
    let table = tables
        .iter()
        .find(|t| t.key == "policy_evaluation_coverage")
        .unwrap();
    assert_eq!(
        table
            .rows
            .iter()
            .map(|r| (&r[2], &r[3]))
            .collect::<Vec<_>>(),
        vec![
            (&"Evaluation observed".into(), &"1".into()),
            (&"No evaluation observed".into(), &"0".into())
        ]
    );
    // A failed state query cannot justify assigning zero evaluations.
    let mut states = run("policy_states", 1);
    states.error = Some("denied".into());
    let tables = summaries(data, &[run("policy_assignments", 2), states]);
    let table = tables
        .iter()
        .find(|t| t.key == "policy_evaluation_coverage")
        .unwrap();
    assert!(
        table
            .rows
            .iter()
            .all(|r| r[2] == "Evaluation evidence unavailable" && r[3] == "Unknown")
    );
}

#[test]
fn unit_patch_coverage_requires_verified_inventory_and_preserves_missing_assessments() {
    let store = Store::open_in_memory().unwrap();
    let id = common::seed_estate(&store);
    let resources = store.resources(&id).unwrap();
    let mut runs = vec![
        run("all_resources", resources.len()),
        run("patch_assessments", 0),
    ];
    let report = PostureReport::build(
        &BTreeMap::new(),
        &runs,
        "2026-09-13T12:00:00Z".parse().unwrap(),
        &resources,
    );
    let table = report
        .tables(&Labels::default())
        .into_iter()
        .find(|t| t.key == "patch_evaluation_coverage")
        .unwrap();
    assert_eq!(table.rows[0][1..], ["No evaluation observed", "1"]);
    runs[0].row_count = None;
    let report = PostureReport::build(
        &BTreeMap::new(),
        &runs,
        "2026-09-13T12:00:00Z".parse().unwrap(),
        &resources,
    );
    let table = report
        .tables(&Labels::default())
        .into_iter()
        .find(|t| t.key == "patch_evaluation_coverage")
        .unwrap();
    assert!(table.rows.is_empty());
    assert!(table.status.contains("incomplete"));
}

#[test]
fn unit_provenance_round_trips_without_using_current_definition() {
    let store = Store::open_in_memory().unwrap();
    let snapshot = store.create_snapshot("tenant", None).unwrap();
    let mut recorded = run("role_assignments", 0);
    recorded.provenance.as_mut().unwrap().description = "Description at collection".into();
    store.record_query_run(&snapshot.id, &recorded).unwrap();
    assert_eq!(store.query_runs(&snapshot.id).unwrap(), vec![recorded]);
}

#[test]
fn unit_reports_keep_removed_query_overrides_and_their_recorded_description() {
    let store = Store::open_in_memory().unwrap();
    let snapshot = store.create_snapshot("tenant", None).unwrap();
    let def=QueryDef::parse("name='removed_query'\ncategory='operations'\nkind='inventory'\ndescription='Original evidence'\nkql='resources | project id | order by id asc'", "fixture").unwrap();
    azdocs::collect::ingest::ingest(
        &store,
        &snapshot.id,
        &def,
        &[json!({"id":"/resource/a","state":"Observed"})],
    )
    .unwrap();
    store
        .record_query_run(
            &snapshot.id,
            &QueryRun {
                query_name: def.name.clone(),
                category: def.category.clone(),
                row_count: Some(1),
                duration_ms: Some(1),
                error: None,
                provenance: Some(def.provenance(&[])),
            },
        )
        .unwrap();
    let report = ReportContext::build(&store, &snapshot.id).unwrap();
    assert_eq!(
        report.categories[0].queries[0].description,
        "Original evidence"
    );
    assert_eq!(report.categories[0].queries[0].rows.len(), 1);
}

#[test]
fn unit_query_metadata_rejects_invalid_scope_threshold_and_unknown_keys() {
    let base = "name='test'\ncategory='operations'\nkind='inventory'\ndescription='test'\nkql='resources | order by id asc'\n";
    for suffix in [
        "authorization_scope='Everything'",
        "[freshness]\ntimestamp_field='at'\nmax_age_hours=0",
        "[freshness]\ntimestamp_field='at'\nmax_age_hours=24\nmax_age_hour=24",
    ] {
        assert!(QueryDef::parse(&format!("{base}{suffix}"), "fixture").is_err());
    }
    let def = QueryDef::parse(
        &format!("{base}authorization_scope='AtScopeAboveAndBelow'"),
        "fixture",
    )
    .unwrap();
    let p = def.provenance(&["B".into(), "a".into(), "A".into()]);
    assert_eq!(
        p.authorization_scope,
        AuthorizationScope::AtScopeAboveAndBelow
    );
    assert_eq!(p.subscriptions, ["a", "b"]);
    assert_eq!(p.kql_sha256.len(), 64);
}

#[test]
fn unit_operational_pack_has_guidance_and_labels_for_every_new_dataset() {
    let pack = QueryPack::builtin().unwrap();
    let labels = Labels::default();
    for def in pack.all().into_iter().filter(|d| d.source.is_some()) {
        if def.kind == azdocs::querypack::QueryKind::Finding {
            assert!(
                labels.report.assessment.checks.contains_key(&def.name),
                "{}",
                def.name
            );
        }
    }
    let runs: Vec<_> = pack.all().into_iter().map(|d| run(&d.name, 1)).collect();
    let data = pack
        .all()
        .into_iter()
        .map(|d| (d.name.clone(), vec![json!({})]))
        .collect();
    for table in summaries(data, &runs) {
        assert_ne!(table.title, "Unknown", "{}", table.key);
        assert!(
            !table.columns.iter().any(|s| s == "Unknown"),
            "{}",
            table.key
        );
    }
}

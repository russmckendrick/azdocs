mod common;

use azdocs::model::{Edge, EdgeKind, Finding, Resource, Severity};
use azdocs::report::ReportContext;
use azdocs::report::analysis::{ReportAnalysis, StudyReason};
use azdocs::store::Store;
use serde_json::json;

fn resource(name: &str, group: &str) -> Resource {
    Resource {
        id: format!(
            "/subscriptions/S/resourceGroups/{group}/providers/Microsoft.Compute/virtualMachines/{name}"
        ),
        display_id: name.into(),
        name: name.into(),
        azure_type: "Microsoft.Compute/virtualMachines".into(),
        subscription_id: "S".into(),
        resource_group: Some(group.into()),
        location: None,
        tags: None,
        sku: None,
        kind: None,
        identity: None,
        properties: None,
    }
}
fn finding(id: Option<&str>, severity: Severity) -> Finding {
    Finding {
        query_name: "custom_check".into(),
        category: "custom".into(),
        severity,
        resource_id: id.map(str::to_owned),
        title: format!("stored {severity:?} evidence"),
        detail: Some(json!({"original": true})),
    }
}
fn analyse(resources: &[Resource], findings: &[Finding], edges: &[Edge]) -> ReportAnalysis {
    ReportAnalysis::build(resources, &[], &[], findings, edges, vec![])
}
fn edge(a: &str, b: &str, kind: EdgeKind) -> Edge {
    Edge {
        source_id: a.into(),
        target_id: b.into(),
        kind,
        properties: None,
    }
}

#[test]
fn unit_grouping_preserves_occurrences_severity_and_unique_case_insensitive_ids() {
    let r = resource("vm", "GROUP");
    let a = analyse(
        std::slice::from_ref(&r),
        &[
            finding(Some(&r.id), Severity::High),
            finding(Some(&r.id.to_uppercase()), Severity::Medium),
            finding(None, Severity::Info),
            finding(Some("/missing"), Severity::Low),
        ],
        &[],
    );
    let issue = &a.issues[0];
    assert_eq!(issue.occurrences.len(), 4);
    assert_eq!(issue.resource_ids.len(), 2);
    assert_eq!(issue.severities, [1, 1, 1, 1]);
    assert_eq!(issue.estate_level, 1);
    assert_eq!(issue.unresolved, 1);
    assert_eq!(issue.subscriptions["s"].len(), 1);
    assert_eq!(issue.groups.values().next().unwrap().len(), 1);
    assert!(
        issue
            .occurrences
            .iter()
            .all(|f| f.detail == Some(json!({"original": true})))
    );
}

#[test]
fn unit_studies_resolve_overlaps_and_ties_deterministically() {
    let resources = vec![
        resource("a", "a"),
        resource("b", "b"),
        resource("c", "c"),
        resource("d", "d"),
        resource("d2", "d"),
    ];
    let findings = vec![
        finding(Some(&resources[0].id), Severity::High),
        finding(Some(&resources[1].id), Severity::High),
    ];
    let edges = vec![
        edge(&resources[0].id, &resources[1].id, EdgeKind::UsesIdentity),
        edge(&resources[0].id, &resources[2].id, EdgeKind::LogsTo),
    ];
    let first = analyse(&resources, &findings, &edges);
    let selected: Vec<_> = first
        .studies()
        .map(|g| (g.name.clone(), g.study_reason))
        .collect();
    assert_eq!(
        selected,
        vec![
            (Some("a".into()), Some(StudyReason::Findings)),
            (Some("b".into()), Some(StudyReason::Connections)),
            (Some("d".into()), Some(StudyReason::Population))
        ]
    );
    let mut reversed = resources.clone();
    reversed.reverse();
    let second = analyse(
        &reversed,
        &findings.iter().cloned().rev().collect::<Vec<_>>(),
        &edges.iter().cloned().rev().collect::<Vec<_>>(),
    );
    assert_eq!(
        selected,
        second
            .studies()
            .map(|g| (g.name.clone(), g.study_reason))
            .collect::<Vec<_>>()
    );
}

#[test]
fn unit_relationships_keep_missing_endpoints_and_resolve_subnets_without_false_resources() {
    let mut vnet = resource("net", "a");
    vnet.azure_type = "microsoft.network/virtualnetworks".into();
    let vm = resource("vm", "b");
    let subnet = format!("{}/subnets/private", vnet.id);
    let edges = vec![
        edge(&vm.id, &subnet.to_uppercase(), EdgeKind::NsgAttached),
        edge(&vm.id, "/missing", EdgeKind::LogsTo),
        edge(&vm.id, &vnet.id, EdgeKind::PeeredWith),
        edge(&vnet.id, &vm.id, EdgeKind::PeeredWith),
        edge(&subnet, &vnet.id, EdgeKind::SubnetOf),
    ];
    let a = analyse(&[vnet, vm], &[], &edges);
    assert_eq!(a.relationships.len(), 3);
    assert_eq!(a.unresolved_relationships, 1);
    assert_eq!(
        a.relationships.iter().filter(|r| r.crosses_group()).count(),
        2
    );
    assert_eq!(a.resources.len(), 2);
}

#[test]
fn unit_empty_analysis_does_not_invent_studies_or_successful_checks() {
    let a = analyse(&[], &[], &[]);
    assert_eq!(a.studies().count(), 0);
    assert!(a.issues.is_empty());
    assert!(a.query_runs.is_empty());
}

#[test]
fn unit_context_retains_removed_custom_query_evidence() {
    let store = Store::open_in_memory().unwrap();
    let id = common::seed_estate(&store);
    let evidence = json!({"id":"/custom", "custom_field":"historical value"});
    store
        .insert_query_results(&id, "removed_custom_check", std::slice::from_ref(&evidence))
        .unwrap();
    let report = ReportContext::build(&store, &id).unwrap();
    assert_eq!(
        report.analysis.recorded_queries["removed_custom_check"],
        vec![evidence]
    );
}

#[test]
fn unit_every_builtin_finding_has_typed_explanatory_guidance() {
    let labels = azdocs::labels::Labels::default();
    let pack = azdocs::querypack::QueryPack::builtin().unwrap();
    for check in pack
        .all()
        .into_iter()
        .filter(|q| q.kind == azdocs::querypack::QueryKind::Finding)
    {
        let guidance = labels
            .report
            .assessment
            .checks
            .get(&check.name)
            .unwrap_or_else(|| panic!("{} missing guidance", check.name));
        assert!(
            !guidance.observation.is_empty()
                && !guidance.significance.is_empty()
                && !guidance.verification.is_empty()
                && !guidance.policy.is_empty()
        );
    }
    assert!(labels.report.assessment.checks.contains_key("unknown"));
}

#[test]
fn unit_partial_report_distinguishes_failed_empty_and_unrecorded_audits() {
    use azdocs::model::{QueryRun, SnapshotStatus};
    use std::io::Read;
    let store = Store::open_in_memory().unwrap();
    let snapshot = store.create_snapshot("tenant", None).unwrap();
    store
        .set_snapshot_status(&snapshot.id, SnapshotStatus::Partial)
        .unwrap();
    for (name, count, error) in [
        ("empty_check", Some(0), None),
        ("failed_check", None, Some("Permission denied")),
        ("unknown_check", None, None),
    ] {
        store
            .record_query_run(
                &snapshot.id,
                &QueryRun {
                    query_name: name.into(),
                    category: "security".into(),
                    row_count: count,
                    duration_ms: None,
                    error: error.map(str::to_owned),
                },
            )
            .unwrap();
    }
    let report = ReportContext::build(&store, &snapshot.id).unwrap();
    let bytes = azdocs::report::docx::render(
        &report,
        &azdocs::report::branding::BrandingContext::default(),
        &[],
    )
    .unwrap();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    for phrase in [
        "Completed with zero rows",
        "Failed",
        "Outcome unavailable",
        "Required-tag audit scope is unavailable",
        "Not recorded",
    ] {
        assert!(xml.contains(phrase), "missing qualification: {phrase}");
    }
    assert!(!xml.contains("0 non-compliant"));
}

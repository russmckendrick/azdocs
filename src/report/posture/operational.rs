//! Summaries of observed operations. All joins and age checks run offline.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Duration, Utc};
use serde_json::Value;

use super::{Cell, Dataset, aggregate, evidence_status, text};
use crate::model::{QueryRun, Resource, normalize_arm_id};

fn rows<'a>(queries: &'a BTreeMap<String, Vec<Value>>, key: &str) -> &'a [Value] {
    queries.get(key).map_or(&[], Vec::as_slice)
}

pub(super) fn datasets(
    queries: &BTreeMap<String, Vec<Value>>,
    runs: &[QueryRun],
    resources: &[Resource],
    collected_at: DateTime<Utc>,
) -> Vec<Dataset> {
    let mut result = Vec::new();
    for (key, groups, metrics, columns) in [
        (
            "policy_assignments",
            vec!["enforcementMode"],
            vec![],
            vec!["enforcement", "assignments"],
        ),
        (
            "policy_definitions",
            vec!["policyType", "mode"],
            vec![],
            vec!["type", "mode", "definitions"],
        ),
        (
            "policy_initiatives",
            vec!["policyType"],
            vec![],
            vec!["type", "initiatives"],
        ),
        (
            "role_assignments",
            vec!["principalType", "roleDefinitionId"],
            vec![],
            vec!["principal_type", "role", "assignments"],
        ),
        (
            "role_definitions",
            vec!["roleType"],
            vec![],
            vec!["type", "definitions"],
        ),
        (
            "patch_assessments",
            vec!["osType", "status"],
            vec!["securityUpdates", "criticalUpdates"],
            vec![
                "os",
                "state",
                "assessments",
                "security_updates",
                "critical_updates",
            ],
        ),
        (
            "patch_installations",
            vec!["osType", "status"],
            vec!["failedPatches"],
            vec!["os", "state", "jobs", "failed_patches"],
        ),
        (
            "guest_configuration_assignments",
            vec!["complianceState"],
            vec![],
            vec!["state", "assignments"],
        ),
        (
            "backup_protected_items",
            vec!["dataSourceType", "protectionState"],
            vec![],
            vec!["type", "state", "items"],
        ),
        (
            "backup_policies",
            vec!["dataSourceType"],
            vec![],
            vec!["type", "policies"],
        ),
        (
            "backup_jobs",
            vec!["operation", "status"],
            vec![],
            vec!["operation", "state", "jobs"],
        ),
        (
            "defender_assessments",
            vec!["state", "severity"],
            vec![],
            vec!["state", "severity", "assessments"],
        ),
        (
            "defender_subassessments",
            vec!["state", "severity"],
            vec![],
            vec!["state", "severity", "occurrences"],
        ),
        (
            "defender_active_alerts",
            vec!["severity"],
            vec![],
            vec!["severity", "alerts"],
        ),
        (
            "defender_secure_score_controls",
            vec!["subscriptionId"],
            vec![
                "healthyResources",
                "unhealthyResources",
                "notApplicableResources",
            ],
            vec![
                "subscription",
                "controls",
                "healthy",
                "unhealthy",
                "not_applicable",
            ],
        ),
        (
            "resource_health",
            vec!["state"],
            vec![],
            vec!["state", "resources"],
        ),
        (
            "service_health_events",
            vec!["eventType", "state"],
            vec![],
            vec!["type", "state", "events"],
        ),
        (
            "resource_changes",
            vec!["changeType", "changedByType"],
            vec![],
            vec!["type", "actor_type", "events"],
        ),
    ] {
        let data = rows(queries, key);
        // New datasets should not add pages of unavailable sections to older snapshots.
        // Once attempted, even a failed/empty query remains visible in its own right.
        if data.is_empty() && !runs.iter().any(|r| r.query_name == key) {
            continue;
        }
        let status = evidence_status(runs, key, data.len());
        result.push(Dataset {
            key,
            status,
            columns,
            rows: if status == "recorded" {
                aggregate(data, &groups, &metrics)
            } else {
                Vec::new()
            },
        });
    }
    if let Some(coverage) = policy_coverage(queries, runs) {
        result.push(coverage);
    }
    if let Some(coverage) = patch_coverage(queries, runs, resources) {
        result.push(coverage);
    }
    let mut freshness = Vec::new();
    for run in runs {
        let Some(rule) = run.provenance.as_ref().and_then(|p| p.freshness.as_ref()) else {
            continue;
        };
        let data = rows(queries, &run.query_name);
        if evidence_status(runs, &run.query_name, data.len()) != "recorded" {
            continue;
        }
        let mut buckets: BTreeMap<&str, usize> = BTreeMap::new();
        for row in data {
            // Freeze age at collection. Re-exporting a snapshot must not turn it stale.
            let state = match DateTime::parse_from_rfc3339(&text(row, &rule.timestamp_field)) {
                Ok(at) if at > collected_at => "future_timestamp",
                Ok(at)
                    if collected_at.signed_duration_since(at)
                        > Duration::hours(i64::from(rule.max_age_hours)) =>
                {
                    "stale"
                }
                Ok(_) => "within_threshold",
                Err(_) => "unknown_timestamp",
            };
            *buckets.entry(state).or_default() += 1;
        }
        for (state, count) in buckets {
            freshness.push(vec![
                Cell::Text(run.query_name.clone()),
                Cell::Count(rule.max_age_hours as usize),
                Cell::Label(state.to_owned()),
                Cell::Count(count),
            ]);
        }
    }
    if !freshness.is_empty() {
        result.push(Dataset {
            key: "evidence_freshness",
            status: "recorded",
            columns: vec!["query", "threshold_hours", "state", "items"],
            rows: freshness,
        });
    }
    result
}

fn policy_coverage(queries: &BTreeMap<String, Vec<Value>>, runs: &[QueryRun]) -> Option<Dataset> {
    let assignments = rows(queries, "policy_assignments");
    if evidence_status(runs, "policy_assignments", assignments.len()) != "recorded" {
        return None;
    }
    let states = rows(queries, "policy_states");
    let states_available = matches!(
        evidence_status(runs, "policy_states", states.len()),
        "recorded" | "empty"
    );
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for state in states {
        let id = normalize_arm_id(&text(state, "policyAssignmentId"));
        if !id.is_empty() {
            *counts.entry(id).or_default() += 1;
        }
    }
    let mut coverage = assignments
        .iter()
        .map(|assignment| {
            let id = normalize_arm_id(&text(assignment, "id"));
            let count = counts.get(&id).copied().unwrap_or(0);
            // Microsoft samples can label zero-resource initiatives compliant. Our
            // evidence register deliberately says only whether evaluations were observed.
            // https://learn.microsoft.com/azure/governance/policy/samples/resource-graph-samples
            let state = if !states_available {
                "evaluation_unknown"
            } else if count == 0 {
                "no_observed_evaluation"
            } else {
                "evaluation_observed"
            };
            (
                id.clone(),
                vec![
                    Cell::Text(id),
                    Cell::Text(text(assignment, "displayName")),
                    Cell::Label(state.to_owned()),
                    if states_available {
                        Cell::Count(count)
                    } else {
                        Cell::Unknown
                    },
                ],
            )
        })
        .collect::<Vec<_>>();
    coverage.sort_by(|a, b| a.0.cmp(&b.0));
    Some(Dataset {
        key: "policy_evaluation_coverage",
        status: "recorded",
        columns: vec!["assignment", "name", "state", "evaluations"],
        rows: coverage.into_iter().map(|(_, row)| row).collect(),
    })
}

fn patch_coverage(
    queries: &BTreeMap<String, Vec<Value>>,
    runs: &[QueryRun],
    resources: &[Resource],
) -> Option<Dataset> {
    if !runs.iter().any(|r| r.query_name == "patch_assessments") {
        return None;
    }
    let assessments = rows(queries, "patch_assessments");
    let available = matches!(
        evidence_status(runs, "patch_assessments", assessments.len()),
        "recorded" | "empty"
    );
    let observed: BTreeSet<_> = assessments
        .iter()
        .map(|r| normalize_arm_id(&text(r, "resourceId")))
        .filter(|id| !id.is_empty())
        .collect();
    let mut counts: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    for resource in resources.iter().filter(|r| {
        matches!(
            r.azure_type.as_str(),
            "microsoft.compute/virtualmachines" | "microsoft.hybridcompute/machines"
        )
    }) {
        let state = if !available {
            "evaluation_unknown"
        } else if observed.contains(&normalize_arm_id(&resource.id)) {
            "evaluation_observed"
        } else {
            "no_observed_evaluation"
        };
        *counts.entry((&resource.azure_type, state)).or_default() += 1;
    }
    // Inventory failure invalidates a denominator even when assessment collection succeeds.
    // Partial inventory may still identify observed machines, but cannot establish coverage.
    let inventory = runs.iter().find(|r| r.query_name == "all_resources");
    let complete =
        inventory.is_some_and(|r| r.error.is_none() && r.row_count == Some(resources.len() as u64));
    Some(Dataset {
        key: "patch_evaluation_coverage",
        status: if complete { "recorded" } else { "incomplete" },
        columns: vec!["type", "state", "resources"],
        rows: if complete {
            counts
                .into_iter()
                .map(|((kind, state), count)| {
                    vec![
                        Cell::Text(kind.to_owned()),
                        Cell::Label(state.to_owned()),
                        Cell::Count(count),
                    ]
                })
                .collect()
        } else {
            Vec::new()
        },
    })
}

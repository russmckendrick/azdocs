//! Interpret collected Microsoft evidence once, without querying Azure or
//! re-evaluating expiry against the export machine's clock.

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use serde_json::Value;

use crate::labels::{Labels, fill};
use crate::model::{QueryRun, normalize_arm_id};

#[derive(Debug, Default)]
pub struct PostureReport {
    datasets: Vec<Dataset>,
}

#[derive(Debug)]
struct Dataset {
    key: &'static str,
    status: &'static str,
    columns: Vec<&'static str>,
    rows: Vec<Vec<Cell>>,
}

#[derive(Debug)]
enum Cell {
    Text(String),
    Label(String),
    Count(usize),
    Number(f64),
    Amount(f64),
    Unknown,
}

#[derive(Debug, Serialize)]
pub struct EvidenceTable {
    pub title: String,
    pub note: String,
    pub status: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

fn text(row: &Value, field: &str) -> String {
    row.get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn number(row: &Value, field: &str) -> Option<f64> {
    row.get(field)
        .and_then(|value| value.as_f64().or_else(|| value.as_str()?.parse().ok()))
        .filter(|value| value.is_finite() && *value >= 0.0)
}

fn evidence_status(runs: &[QueryRun], key: &str, row_count: usize) -> &'static str {
    match runs.iter().find(|run| run.query_name == key) {
        None if row_count == 0 => "not_collected",
        Some(run) if run.error.is_some() => "failed",
        Some(run) if run.row_count == Some(row_count as u64) => {
            if row_count == 0 {
                "empty"
            } else {
                "recorded"
            }
        }
        _ => "incomplete",
    }
}

// Missing metrics poison their aggregate: absence must never become a zero.
fn aggregate(rows: &[Value], groups: &[&str], metrics: &[&str]) -> Vec<Vec<Cell>> {
    type Aggregate = (usize, Vec<Option<f64>>);
    let mut totals: BTreeMap<Vec<String>, Aggregate> = BTreeMap::new();
    for row in rows {
        let key = groups
            .iter()
            .map(|field| {
                let value = text(row, field);
                if field.ends_with("Id") {
                    normalize_arm_id(&value)
                } else {
                    value
                }
            })
            .collect();
        let (count, sums) = totals
            .entry(key)
            .or_insert_with(|| (0, vec![Some(0.0); metrics.len()]));
        *count += 1;
        for (sum, field) in sums.iter_mut().zip(metrics) {
            *sum = sum
                .zip(number(row, field))
                .map(|(a, b)| a + b)
                .filter(|n| n.is_finite());
        }
    }
    totals
        .into_iter()
        .map(|(key, (count, sums))| {
            let mut cells: Vec<_> = key
                .into_iter()
                .zip(groups)
                .map(|(value, field)| {
                    if value.is_empty() {
                        Cell::Unknown
                    } else if *field == "expiryState" {
                        Cell::Label(value)
                    } else {
                        Cell::Text(value)
                    }
                })
                .collect();
            cells.push(Cell::Count(count));
            cells.extend(sums.into_iter().zip(metrics).map(|(sum, field)| {
                sum.map_or(Cell::Unknown, |value| {
                    if field.to_ascii_lowercase().contains("savings") {
                        Cell::Amount(value)
                    } else {
                        Cell::Number(value)
                    }
                })
            }));
            cells
        })
        .collect()
}

impl PostureReport {
    pub fn build(
        queries: &BTreeMap<String, Vec<Value>>,
        runs: &[QueryRun],
        collected_at: DateTime<Utc>,
    ) -> Self {
        let mut datasets = Vec::new();
        for (key, groups, metrics, columns) in [
            (
                "advisor_cost_recommendations",
                vec!["currency", "savingsPeriod"],
                vec!["savingsAmount", "annualSavingsAmount"],
                vec![
                    "currency",
                    "period",
                    "recommendations",
                    "savings",
                    "annual_savings",
                ],
            ),
            (
                "policy_states",
                vec![
                    "subscriptionId",
                    "policyAssignmentId",
                    "policySetDefinitionId",
                    "complianceState",
                ],
                vec![],
                vec![
                    "subscription",
                    "assignment",
                    "initiative",
                    "state",
                    "evaluations",
                ],
            ),
            (
                "policy_exemptions",
                vec!["exemptionCategory", "expiryState"],
                vec![],
                vec!["category", "expiry", "exemptions"],
            ),
            (
                "defender_compliance_standards",
                vec!["complianceStandard", "state"],
                vec![],
                vec!["standard", "state", "standards"],
            ),
            (
                "defender_compliance_controls",
                vec!["complianceStandard", "state"],
                vec![],
                vec!["standard", "state", "controls"],
            ),
            (
                "defender_compliance_assessments",
                vec!["complianceStandard", "state"],
                vec!["passedResources", "failedResources", "skippedResources"],
                vec![
                    "standard",
                    "state",
                    "assessments",
                    "passed",
                    "failed_resources",
                    "skipped",
                ],
            ),
        ] {
            let rows = queries.get(key).map_or(&[][..], Vec::as_slice);
            let status = evidence_status(runs, key, rows.len());
            let mut exemption_rows;
            let rows = if key == "policy_exemptions" {
                exemption_rows = rows.to_vec();
                for row in &mut exemption_rows {
                    let expiry = text(row, "expiresOn");
                    let state = if expiry.is_empty() {
                        "no_expiry"
                    } else {
                        match DateTime::parse_from_rfc3339(&expiry) {
                            Err(_) => "unknown",
                            Ok(date) if date <= collected_at => "expired",
                            Ok(date) if date <= collected_at + Duration::days(90) => "due_soon",
                            Ok(_) => "later",
                        }
                    };
                    if let Some(object) = row.as_object_mut() {
                        object.insert("expiryState".into(), Value::String(state.to_owned()));
                    }
                }
                exemption_rows.as_slice()
            } else {
                rows
            };
            datasets.push(Dataset {
                key,
                status,
                columns,
                rows: if status == "recorded" {
                    aggregate(rows, &groups, &metrics)
                } else {
                    Vec::new()
                },
            });
        }
        Self { datasets }
    }

    pub fn tables(&self, labels: &Labels) -> Vec<EvidenceTable> {
        let words = &labels.report.posture;
        let label = |key: &str| words.values.get(key).unwrap_or(&words.unknown).clone();
        self.datasets
            .iter()
            .map(|dataset| {
                let guidance = words.datasets.get(dataset.key);
                EvidenceTable {
                    title: guidance.map_or_else(|| words.unknown.clone(), |g| g.title.clone()),
                    note: guidance.map_or_else(|| words.unknown.clone(), |g| g.note.clone()),
                    status: fill(&words.status, &[("state", &label(dataset.status))]),
                    columns: dataset.columns.iter().map(|key| label(key)).collect(),
                    rows: dataset
                        .rows
                        .iter()
                        .map(|row| {
                            row.iter()
                                .map(|cell| match cell {
                                    Cell::Text(value) => value.clone(),
                                    Cell::Label(key) => label(key),
                                    Cell::Count(value) => value.to_string(),
                                    Cell::Number(value) => format!("{value:.0}"),
                                    Cell::Amount(value) => format!("{value:.2}"),
                                    Cell::Unknown => words.unknown.clone(),
                                })
                                .collect()
                        })
                        .collect(),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn report(
        key: &str,
        rows: Vec<Value>,
        count: Option<u64>,
        error: Option<&str>,
    ) -> PostureReport {
        PostureReport::build(
            &BTreeMap::from([(key.into(), rows)]),
            &[QueryRun {
                query_name: key.into(),
                category: "test".into(),
                row_count: count,
                duration_ms: None,
                error: error.map(str::to_owned),
            }],
            "2026-09-13T12:00:00Z".parse().unwrap(),
        )
    }

    #[test]
    fn unit_savings_keep_currencies_periods_and_missing_values_separate() {
        let p = report(
            "advisor_cost_recommendations",
            vec![
                json!({"currency":"GBP","savingsPeriod":"Month","savingsAmount":10}),
                json!({"currency":"GBP","savingsPeriod":"Month","savingsAmount":null}),
                json!({"currency":"USD","savingsPeriod":"Month","savingsAmount":20}),
                json!({"currency":"USD","savingsAmount":30}),
            ],
            Some(4),
            None,
        );
        let tables = p.tables(&Labels::default());
        assert_eq!(
            tables[0].rows,
            vec![
                vec!["GBP", "Month", "2", "Unknown", "Unknown"],
                vec!["USD", "Unknown", "1", "30.00", "Unknown"],
                vec!["USD", "Month", "1", "20.00", "Unknown"],
            ]
        );
    }

    #[test]
    fn unit_policy_keeps_assignment_scope_and_non_pass_states_distinct() {
        let rows = vec![
            json!({"policyAssignmentId":"/A/Rule", "complianceState":"Exempt"}),
            json!({"policyAssignmentId":"/a/rule", "complianceState":"Exempt"}),
            json!({"policyAssignmentId":"/B/Rule", "complianceState":"Exempt"}),
            json!({"policyAssignmentId":"/B/Rule", "complianceState":"Unknown"}),
        ];
        let p = report("policy_states", rows, Some(4), None);
        let tables = p.tables(&Labels::default());
        assert_eq!(
            tables[1].rows,
            vec![
                vec!["Unknown", "/a/rule", "Unknown", "Exempt", "2"],
                vec!["Unknown", "/b/rule", "Unknown", "Exempt", "1"],
                vec!["Unknown", "/b/rule", "Unknown", "Unknown", "1"],
            ]
        );
    }

    #[test]
    fn unit_exemption_expiry_uses_the_snapshot_instant_and_preserves_invalid_dates() {
        let p = report(
            "policy_exemptions",
            vec![
                json!({"expiresOn":"2026-09-13T11:00:00Z"}),
                json!({"expiresOn":"2026-12-12T12:00:00Z"}),
                json!({"expiresOn":"2026-12-12T12:00:01Z"}),
                json!({"expiresOn":"invalid"}),
                json!({"expiresOn":null}),
            ],
            Some(5),
            None,
        );
        let tables = p.tables(&Labels::default());
        let buckets: Vec<_> = tables[2].rows.iter().map(|r| r[1].as_str()).collect();
        assert_eq!(
            buckets,
            [
                "Due within 90 days",
                "Expired",
                "Due after 90 days",
                "No expiry specified",
                "Unknown"
            ]
        );
    }

    #[test]
    fn unit_coverage_distinguishes_failed_empty_uncollected_and_incomplete_evidence() {
        for (rows, count, error, expected) in [
            (vec![], Some(0), None, "empty"),
            (vec![], None, Some("denied"), "failed"),
            (vec![json!({})], Some(2), None, "incomplete"),
        ] {
            let p = report("policy_states", rows, count, error);
            assert_eq!(p.datasets[1].status, expected);
            assert!(p.datasets[1].rows.is_empty());
            assert_eq!(p.datasets[0].status, "not_collected");
        }
    }

    #[test]
    fn unit_summary_labels_cover_every_dataset_column_and_status() {
        let labels = Labels::default();
        let p = PostureReport::build(&BTreeMap::new(), &[], Utc::now());
        for dataset in p.datasets {
            assert!(labels.report.posture.datasets.contains_key(dataset.key));
            for column in dataset.columns {
                assert!(
                    labels.report.posture.values.contains_key(column),
                    "missing {column}"
                );
            }
        }
    }
}

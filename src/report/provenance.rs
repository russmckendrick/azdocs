//! Render metadata recorded at collection, never reconstruct it from today's pack.

use serde::Serialize;

use crate::labels::Labels;
use crate::model::QueryRun;

#[derive(Debug, Serialize)]
pub struct ProvenanceRecord {
    pub name: String,
    pub fields: Vec<Vec<String>>,
    pub kql: String,
    /// Indentation avoids a query containing backticks breaking a Markdown fence.
    pub markdown_kql: String,
}

pub fn records(runs: &[QueryRun], labels: &Labels) -> Vec<ProvenanceRecord> {
    let words = &labels.report.posture;
    let label = |key: &str| words.values.get(key).unwrap_or(&words.unknown).clone();
    let mut records = runs
        .iter()
        .filter_map(|run| {
            let p = run.provenance.as_ref()?;
            let mut fields = vec![
                vec![label("hash"), p.kql_sha256.clone()],
                vec![label("scope"), p.authorization_scope.as_str().to_owned()],
                vec![
                    label("subscriptions"),
                    if p.subscriptions.is_empty() {
                        label("all_visible")
                    } else {
                        p.subscriptions.join(", ")
                    },
                ],
            ];
            if let Some(source) = &p.source {
                fields.push(vec![label("sources"), source.urls.join("\n")]);
                fields.push(vec![label("reviewed_on"), source.reviewed_on.clone()]);
                if let Some(revision) = &source.revision {
                    fields.push(vec![label("revision"), revision.clone()]);
                }
            }
            Some(ProvenanceRecord {
                name: run.query_name.clone(),
                fields,
                kql: p.kql.clone(),
                markdown_kql: p.kql.lines().map(|line| format!("    {line}\n")).collect(),
            })
        })
        .collect::<Vec<_>>();
    records.sort_by(|a, b| a.name.cmp(&b.name));
    records
}

use rusqlite::params;

use super::{Store, decode_error, json_text, parse_json};
use crate::error::StoreError;
use crate::model::{Finding, Severity};

impl Store {
    pub fn insert_findings(
        &self,
        snapshot_id: &str,
        findings: &[Finding],
    ) -> Result<(), StoreError> {
        let tx = self.conn().unchecked_transaction()?;
        {
            let mut statement = tx.prepare_cached(
                "INSERT INTO findings
                 (snapshot_id, query_name, category, severity, resource_id, title, detail)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )?;
            for finding in findings {
                statement.execute(params![
                    snapshot_id,
                    finding.query_name,
                    finding.category,
                    finding.severity.as_str(),
                    finding.resource_id,
                    finding.title,
                    json_text(&finding.detail),
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Findings ordered by severity (high first), then category and title.
    pub fn findings(&self, snapshot_id: &str) -> Result<Vec<Finding>, StoreError> {
        let mut statement = self.conn().prepare(
            "SELECT query_name, category, severity, resource_id, title, detail
             FROM findings WHERE snapshot_id = ?1
             ORDER BY CASE severity
                 WHEN 'high' THEN 0 WHEN 'medium' THEN 1 WHEN 'low' THEN 2 ELSE 3
             END, category, title",
        )?;
        let rows = statement.query_map([snapshot_id], |row| {
            let severity: String = row.get(2)?;
            Ok(Finding {
                query_name: row.get(0)?,
                category: row.get(1)?,
                severity: Severity::parse(&severity)
                    .ok_or_else(|| decode_error("findings.severity", severity))?,
                resource_id: row.get(3)?,
                title: row.get(4)?,
                detail: parse_json("findings.detail", row.get(5)?)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_unknown_severity_is_an_error() {
        let store = Store::open_in_memory().unwrap();
        let id = store.create_snapshot("tenant", None).unwrap().id;
        store
            .conn()
            .execute(
                "INSERT INTO findings (snapshot_id, query_name, category, severity, title)
                 VALUES (?1, 'q', 'c', 'catastrophic', 't')",
                [&id],
            )
            .unwrap();

        let error = store.findings(&id).unwrap_err().to_string();

        assert!(error.contains("findings.severity"), "{error}");
    }
}

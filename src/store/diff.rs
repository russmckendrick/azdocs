//! Loading both sides of a comparison and the per-snapshot trend. The
//! comparison itself is `model::diff`, so it can be tested without a store.

use rusqlite::params;

use super::Store;
use crate::error::StoreError;
use crate::model::diff::{IgnoreRules, SnapshotChanges, SnapshotSide, compare};

/// One row of the history trend: what a usable snapshot held.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct TrendPoint {
    pub snapshot_id: String,
    pub created_at: String,
    pub status: String,
    pub subscriptions: u64,
    pub resources: u64,
    pub tagged: u64,
    pub findings: u64,
    pub high: u64,
    pub medium: u64,
    pub low: u64,
    pub info: u64,
    pub edges: u64,
}

impl Store {
    fn side(&self, snapshot_id: &str) -> Result<SnapshotSide, StoreError> {
        Ok(SnapshotSide {
            snapshot: self.get_snapshot(snapshot_id)?,
            subscriptions: self.subscriptions(snapshot_id)?,
            resource_groups: self.resource_groups(snapshot_id)?,
            resources: self.resources(snapshot_id)?,
            findings: self.findings(snapshot_id)?,
            edges: self.edges(snapshot_id)?,
        })
    }

    /// Field-level changes from `base` to `target`, both in the selected
    /// tenant. Comparing across tenants is refused even when no tenant is
    /// selected.
    pub fn snapshot_changes(
        &self,
        base: &str,
        target: &str,
    ) -> Result<SnapshotChanges, StoreError> {
        self.snapshot_changes_with(base, target, IgnoreRules::builtin())
    }

    pub fn snapshot_changes_with(
        &self,
        base: &str,
        target: &str,
        ignore: &IgnoreRules,
    ) -> Result<SnapshotChanges, StoreError> {
        let left = self.get_snapshot(base)?;
        let right = self.get_snapshot(target)?;
        if !left.tenant_id.eq_ignore_ascii_case(&right.tenant_id) {
            return Err(StoreError::CrossTenantComparison);
        }
        Ok(compare(&self.side(base)?, &self.side(target)?, ignore))
    }

    /// The last `limit` usable snapshots of the selected tenant, oldest
    /// first, with the totals a trend table needs. One query; no snapshot is
    /// loaded in full.
    pub fn snapshot_trend(&self, limit: usize) -> Result<Vec<TrendPoint>, StoreError> {
        self.require_tenant()?;
        self.trend_query(self.tenant_id.as_deref(), limit)
    }

    /// The same trend for an explicit tenant, for callers that already hold a
    /// snapshot and must not depend on the store's selection.
    pub fn snapshot_trend_for(
        &self,
        tenant_id: &str,
        limit: usize,
    ) -> Result<Vec<TrendPoint>, StoreError> {
        self.trend_query(Some(&tenant_id.to_ascii_lowercase()), limit)
    }

    fn trend_query(
        &self,
        tenant_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<TrendPoint>, StoreError> {
        let mut statement = self.conn().prepare(
            "SELECT s.id, s.created_at, s.status,
                    (SELECT COUNT(*) FROM subscriptions WHERE snapshot_id = s.id),
                    (SELECT COUNT(*) FROM resources WHERE snapshot_id = s.id),
                    (SELECT COUNT(*) FROM resources WHERE snapshot_id = s.id
                        AND tags IS NOT NULL AND tags != '{}' AND tags != 'null'),
                    (SELECT COUNT(*) FROM findings WHERE snapshot_id = s.id),
                    (SELECT COUNT(*) FROM findings WHERE snapshot_id = s.id AND severity = 'high'),
                    (SELECT COUNT(*) FROM findings WHERE snapshot_id = s.id AND severity = 'medium'),
                    (SELECT COUNT(*) FROM findings WHERE snapshot_id = s.id AND severity = 'low'),
                    (SELECT COUNT(*) FROM findings WHERE snapshot_id = s.id AND severity = 'info'),
                    (SELECT COUNT(*) FROM edges WHERE snapshot_id = s.id)
             FROM snapshots s
             WHERE s.status IN ('complete','partial') AND (?1 IS NULL OR lower(s.tenant_id) = ?1)
             ORDER BY s.created_at DESC, s.id DESC LIMIT ?2",
        )?;
        let count =
            |row: &rusqlite::Row<'_>, index: usize| row.get::<_, i64>(index).map(|v| v as u64);
        let mut points: Vec<TrendPoint> = statement
            .query_map(params![tenant_id, limit as i64], |row| {
                Ok(TrendPoint {
                    snapshot_id: row.get(0)?,
                    created_at: row.get(1)?,
                    status: row.get(2)?,
                    subscriptions: count(row, 3)?,
                    resources: count(row, 4)?,
                    tagged: count(row, 5)?,
                    findings: count(row, 6)?,
                    high: count(row, 7)?,
                    medium: count(row, 8)?,
                    low: count(row, 9)?,
                    info: count(row, 10)?,
                    edges: count(row, 11)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        points.reverse();
        Ok(points)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Finding, Severity, SnapshotStatus};

    #[test]
    fn unit_trend_orders_oldest_first_and_counts_severities() {
        let store = Store::open_in_memory().unwrap();
        let older = store.create_snapshot("tenant", None).unwrap();
        store
            .set_snapshot_status(&older.id, SnapshotStatus::Complete)
            .unwrap();
        let newer = store.create_snapshot("tenant", None).unwrap();
        store
            .set_snapshot_status(&newer.id, SnapshotStatus::Partial)
            .unwrap();
        store
            .conn()
            .execute(
                "UPDATE snapshots SET created_at = '2026-02-01T00:00:00+00:00' WHERE id = ?1",
                [&newer.id],
            )
            .unwrap();
        store
            .conn()
            .execute(
                "UPDATE snapshots SET created_at = '2026-01-01T00:00:00+00:00' WHERE id = ?1",
                [&older.id],
            )
            .unwrap();
        store
            .insert_findings(
                &newer.id,
                &[
                    Finding {
                        query_name: "q".into(),
                        category: "c".into(),
                        severity: Severity::High,
                        resource_id: None,
                        title: "a".into(),
                        detail: None,
                    },
                    Finding {
                        query_name: "q".into(),
                        category: "c".into(),
                        severity: Severity::Low,
                        resource_id: None,
                        title: "b".into(),
                        detail: None,
                    },
                ],
            )
            .unwrap();

        let trend = store.snapshot_trend(12).unwrap();

        assert_eq!(trend.len(), 2);
        assert_eq!(trend[0].snapshot_id, older.id);
        assert_eq!((trend[1].findings, trend[1].high, trend[1].low), (2, 1, 1));
        assert_eq!(trend[1].status, "partial");
    }

    #[test]
    fn unit_trend_excludes_failed_and_running_snapshots() {
        let store = Store::open_in_memory().unwrap();
        let good = store.create_snapshot("tenant", None).unwrap();
        store
            .set_snapshot_status(&good.id, SnapshotStatus::Complete)
            .unwrap();
        let failed = store.create_snapshot("tenant", None).unwrap();
        store
            .set_snapshot_status(&failed.id, SnapshotStatus::Failed)
            .unwrap();
        store.create_snapshot("tenant", None).unwrap();

        let trend = store.snapshot_trend(12).unwrap();

        assert_eq!(trend.len(), 1);
        assert_eq!(trend[0].snapshot_id, good.id);
    }
}

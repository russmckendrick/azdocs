use chrono::{DateTime, Utc};
use rusqlite::params;

use super::Store;
use crate::error::StoreError;
use crate::model::{Snapshot, SnapshotStatus};

/// Row for `snapshots list`: snapshot plus per-snapshot counts.
#[derive(Debug)]
pub struct SnapshotCounts {
    pub snapshot: Snapshot,
    pub subscriptions: u64,
    pub resources: u64,
    pub findings: u64,
}

/// Result of comparing two snapshots by resource id.
#[derive(Debug, Default)]
pub struct SnapshotDiff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<String>,
}

impl Store {
    pub fn tenant_ids(&self) -> Result<Vec<String>, StoreError> {
        Ok(self
            .conn()
            .prepare("SELECT DISTINCT lower(tenant_id) FROM snapshots ORDER BY lower(tenant_id)")?
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?)
    }

    pub fn create_snapshot(
        &self,
        tenant_id: &str,
        notes: Option<&str>,
    ) -> Result<Snapshot, StoreError> {
        let snapshot = Snapshot {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            tenant_id: tenant_id.to_owned(),
            tool_version: env!("CARGO_PKG_VERSION").to_owned(),
            status: SnapshotStatus::Running,
            notes: notes.map(str::to_owned),
        };
        self.conn().execute(
            "INSERT INTO snapshots (id, created_at, tenant_id, tool_version, status, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                snapshot.id,
                snapshot.created_at.to_rfc3339(),
                snapshot.tenant_id,
                snapshot.tool_version,
                snapshot.status.as_str(),
                snapshot.notes,
            ],
        )?;
        Ok(snapshot)
    }

    pub fn set_snapshot_status(
        &self,
        snapshot_id: &str,
        status: SnapshotStatus,
    ) -> Result<(), StoreError> {
        self.conn().execute(
            "UPDATE snapshots SET status = ?2 WHERE id = ?1",
            params![snapshot_id, status.as_str()],
        )?;
        Ok(())
    }

    pub fn get_snapshot(&self, snapshot_id: &str) -> Result<Snapshot, StoreError> {
        self.conn()
            .query_row(
                "SELECT id, created_at, tenant_id, tool_version, status, notes
                 FROM snapshots WHERE id = ?1 AND (?2 IS NULL OR lower(tenant_id) = ?2)",
                params![snapshot_id, self.tenant_id],
                snapshot_from_row,
            )
            .map_err(|err| match err {
                rusqlite::Error::QueryReturnedNoRows => {
                    StoreError::SnapshotNotFound(snapshot_id.to_owned())
                }
                other => other.into(),
            })
    }

    pub fn list_snapshots(&self) -> Result<Vec<SnapshotCounts>, StoreError> {
        let mut statement = self.conn().prepare(
            "SELECT s.id, s.created_at, s.tenant_id, s.tool_version, s.status, s.notes,
                    (SELECT COUNT(*) FROM subscriptions WHERE snapshot_id = s.id),
                    (SELECT COUNT(*) FROM resources WHERE snapshot_id = s.id),
                    (SELECT COUNT(*) FROM findings WHERE snapshot_id = s.id)
             FROM snapshots s WHERE (?1 IS NULL OR lower(s.tenant_id) = ?1) ORDER BY s.created_at DESC, s.id DESC",
        )?;
        let rows = statement.query_map([self.tenant_id.as_deref()], |row| {
            Ok(SnapshotCounts {
                snapshot: snapshot_from_row(row)?,
                subscriptions: row.get::<_, i64>(6)? as u64,
                resources: row.get::<_, i64>(7)? as u64,
                findings: row.get::<_, i64>(8)? as u64,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn delete_snapshot(&self, snapshot_id: &str) -> Result<(), StoreError> {
        self.get_snapshot(snapshot_id)?;
        self.conn()
            .execute("DELETE FROM snapshots WHERE id = ?1", [snapshot_id])?;
        Ok(())
    }

    /// Compare resources between two snapshots. `changed` compares the full
    /// properties JSON text.
    pub fn diff_snapshots(&self, a: &str, b: &str) -> Result<SnapshotDiff, StoreError> {
        let left = self.get_snapshot(a)?;
        let right = self.get_snapshot(b)?;
        if !left.tenant_id.eq_ignore_ascii_case(&right.tenant_id) {
            return Err(StoreError::CrossTenantComparison);
        }
        let mut diff = SnapshotDiff::default();
        let mut statement = self.conn().prepare(
            "SELECT COALESCE(ra.id, rb.id),
                    ra.id IS NULL,
                    rb.id IS NULL,
                    COALESCE(ra.properties, '') != COALESCE(rb.properties, '')
             FROM (SELECT * FROM resources WHERE snapshot_id = ?1) ra
             FULL OUTER JOIN (SELECT * FROM resources WHERE snapshot_id = ?2) rb
               ON ra.id = rb.id
             ORDER BY 1",
        )?;
        let rows = statement.query_map([a, b], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, bool>(1)?,
                row.get::<_, bool>(2)?,
                row.get::<_, bool>(3)?,
            ))
        })?;
        for row in rows {
            let (id, missing_in_a, missing_in_b, properties_differ) = row?;
            if missing_in_a {
                diff.added.push(id);
            } else if missing_in_b {
                diff.removed.push(id);
            } else if properties_differ {
                diff.changed.push(id);
            }
        }
        Ok(diff)
    }
}

fn snapshot_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Snapshot> {
    let created_at: String = row.get(1)?;
    let status: String = row.get(4)?;
    Ok(Snapshot {
        id: row.get(0)?,
        created_at: created_at
            .parse::<DateTime<Utc>>()
            .unwrap_or_else(|_| Utc::now()),
        tenant_id: row.get(2)?,
        tool_version: row.get(3)?,
        status: SnapshotStatus::parse(&status).unwrap_or(SnapshotStatus::Failed),
        notes: row.get(5)?,
    })
}

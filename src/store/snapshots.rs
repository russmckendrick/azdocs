use chrono::Utc;
use rusqlite::{OptionalExtension, params};

use super::{Store, decode_error, parse_timestamp};
use crate::error::StoreError;
use crate::model::{Snapshot, SnapshotStatus};

/// The columns every snapshot reader selects, in `snapshot_from_row` order.
pub(crate) const SNAPSHOT_COLUMNS: &str =
    "id, created_at, tenant_id, tool_version, status, notes, heartbeat_at, interrupted_at";

/// Row for `snapshots list`: snapshot plus per-snapshot counts.
#[derive(Debug, serde::Serialize)]
pub struct SnapshotCounts {
    pub snapshot: Snapshot,
    pub subscriptions: u64,
    pub resources: u64,
    pub findings: u64,
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
            heartbeat_at: None,
            interrupted_at: None,
        };
        self.conn().execute(
            "INSERT INTO snapshots (id, created_at, tenant_id, tool_version, status, notes, heartbeat_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?2)",
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

    /// The most recent usable snapshot before `snapshot_id` in the same
    /// tenant: the baseline "changes since" compares against. Running,
    /// failed and cancelled rows are skipped so a broken collect never makes
    /// the whole estate look newly added.
    pub fn previous_snapshot(&self, snapshot_id: &str) -> Result<Option<Snapshot>, StoreError> {
        let current = self.get_snapshot(snapshot_id)?;
        Ok(self
            .conn()
            .query_row(
                &format!(
                    "SELECT {SNAPSHOT_COLUMNS} FROM snapshots
                     WHERE lower(tenant_id) = lower(?1) AND id != ?2
                       AND status IN ('complete','warnings','partial')
                       AND (created_at < ?3 OR (created_at = ?3 AND id < ?2))
                     ORDER BY created_at DESC, id DESC LIMIT 1"
                ),
                params![
                    current.tenant_id,
                    current.id,
                    current.created_at.to_rfc3339()
                ],
                snapshot_from_row,
            )
            .optional()?)
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
                &format!(
                    "SELECT {SNAPSHOT_COLUMNS}
                     FROM snapshots WHERE id = ?1 AND (?2 IS NULL OR lower(tenant_id) = ?2)"
                ),
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
                    s.heartbeat_at, s.interrupted_at,
                    (SELECT COUNT(*) FROM subscriptions WHERE snapshot_id = s.id),
                    (SELECT COUNT(*) FROM resources WHERE snapshot_id = s.id),
                    (SELECT COUNT(*) FROM findings WHERE snapshot_id = s.id)
             FROM snapshots s WHERE (?1 IS NULL OR lower(s.tenant_id) = ?1) ORDER BY s.created_at DESC, s.id DESC",
        )?;
        let rows = statement.query_map([self.tenant_id.as_deref()], |row| {
            Ok(SnapshotCounts {
                snapshot: snapshot_from_row(row)?,
                subscriptions: row.get::<_, i64>(8)? as u64,
                resources: row.get::<_, i64>(9)? as u64,
                findings: row.get::<_, i64>(10)? as u64,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn delete_snapshot(&self, snapshot_id: &str) -> Result<(), StoreError> {
        self.delete_snapshots(std::slice::from_ref(&snapshot_id.to_owned()))
            .map(|_| ())
    }
}

/// Strict on purpose: a timestamp or status this build cannot read is
/// corruption or a newer format, and pretending it is "now" or "failed"
/// would make exports non-reproducible while hiding the cause.
pub(crate) fn snapshot_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Snapshot> {
    let created_at: String = row.get(1)?;
    let status: String = row.get(4)?;
    Ok(Snapshot {
        id: row.get(0)?,
        created_at: parse_timestamp("snapshots.created_at", created_at)?,
        tenant_id: row.get(2)?,
        tool_version: row.get(3)?,
        status: SnapshotStatus::parse(&status)
            .ok_or_else(|| decode_error("snapshots.status", status))?,
        notes: row.get(5)?,
        heartbeat_at: row
            .get::<_, Option<String>>(6)?
            .map(|t| parse_timestamp("snapshots.heartbeat_at", t))
            .transpose()?,
        interrupted_at: row
            .get::<_, Option<String>>(7)?
            .map(|t| parse_timestamp("snapshots.interrupted_at", t))
            .transpose()?,
    })
}

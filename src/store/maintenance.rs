//! Lifecycle operations that change a database outside a collect: batch
//! deletion, compaction, integrity checks and the reconciliation of collects
//! that died mid-run.

use chrono::{DateTime, Duration, Utc};
use rusqlite::params;

use super::Store;
use crate::error::StoreError;

/// A `running` snapshot whose collector has not written a heartbeat for this
/// long is treated as abandoned. Heartbeats arrive every 30 s while a collect
/// is alive, so ten minutes survives a laptop sleep without stranding a real
/// run, yet still clears a crash before the next report.
pub const STALE_RUNNING_AFTER: Duration = Duration::minutes(10);

/// What `PRAGMA integrity_check` and `foreign_key_check` reported.
#[derive(Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct IntegrityReport {
    /// Empty when the file is sound.
    pub problems: Vec<String>,
}

impl IntegrityReport {
    pub fn is_ok(&self) -> bool {
        self.problems.is_empty()
    }
}

impl Store {
    /// Prove a collect is still alive. Called on an interval by the runner.
    pub fn touch_snapshot(&self, snapshot_id: &str, at: DateTime<Utc>) -> Result<(), StoreError> {
        self.conn().execute(
            "UPDATE snapshots SET heartbeat_at = ?2 WHERE id = ?1 AND status = 'running'",
            params![snapshot_id, at.to_rfc3339()],
        )?;
        Ok(())
    }

    /// Mark `running` snapshots with no recent heartbeat as `failed`,
    /// recording when that happened. Not tenant-scoped: a stale row is stale
    /// for everyone. Returns the ids it changed.
    pub fn reconcile_stale_running(
        &self,
        now: DateTime<Utc>,
        stale_after: Duration,
    ) -> Result<Vec<String>, StoreError> {
        let cutoff = (now - stale_after).to_rfc3339();
        let mut statement = self.conn().prepare(
            "UPDATE snapshots SET status = 'failed', interrupted_at = ?1
             WHERE status = 'running' AND COALESCE(heartbeat_at, created_at) < ?2
             RETURNING id",
        )?;
        let ids = statement
            .query_map(params![now.to_rfc3339(), cutoff], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        Ok(ids)
    }

    /// Delete several snapshots atomically. Every id must exist in the
    /// selected tenant or nothing is deleted; cascades reach every table.
    pub fn delete_snapshots(&self, snapshot_ids: &[String]) -> Result<usize, StoreError> {
        let tx = self.conn().unchecked_transaction()?;
        for id in snapshot_ids {
            self.get_snapshot(id)?;
            tx.execute("DELETE FROM snapshots WHERE id = ?1", [id])?;
        }
        tx.commit()?;
        Ok(snapshot_ids.len())
    }

    /// Reclaim space after deletions. Must run outside a transaction, which
    /// is why it is separate from `delete_snapshots`.
    pub fn vacuum(&self) -> Result<(), StoreError> {
        self.conn().execute_batch("VACUUM;")?;
        Ok(())
    }

    /// SQLite's own consistency checks, plus foreign keys (which cascade
    /// deletes depend on).
    pub fn integrity_check(&self) -> Result<IntegrityReport, StoreError> {
        let mut problems: Vec<String> = self
            .conn()
            .prepare("PRAGMA integrity_check")?
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<_, _>>()?;
        problems.retain(|line| line != "ok");
        let mut fk = self.conn().prepare("PRAGMA foreign_key_check")?;
        let violations = fk
            .query_map([], |row| {
                Ok(format!(
                    "foreign key violation in {} row {}",
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        problems.extend(violations);
        Ok(IntegrityReport { problems })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SnapshotStatus;

    fn running_since(store: &Store, created: &str) -> String {
        let snapshot = store.create_snapshot("tenant", None).unwrap();
        store
            .conn()
            .execute(
                "UPDATE snapshots SET created_at = ?2, heartbeat_at = ?2 WHERE id = ?1",
                params![snapshot.id, created],
            )
            .unwrap();
        snapshot.id
    }

    #[test]
    fn unit_reconcile_marks_running_without_heartbeat_as_failed() {
        let store = Store::open_in_memory().unwrap();
        let id = running_since(&store, "2026-01-01T00:00:00+00:00");
        let now = "2026-01-01T01:00:00Z".parse().unwrap();

        let reconciled = store
            .reconcile_stale_running(now, STALE_RUNNING_AFTER)
            .unwrap();

        let snapshot = store.get_snapshot(&id).unwrap();
        assert_eq!(reconciled, vec![id]);
        assert_eq!(snapshot.status, SnapshotStatus::Failed);
        assert_eq!(snapshot.interrupted_at, Some(now));
    }

    #[test]
    fn unit_reconcile_keeps_recent_heartbeat_running() {
        let store = Store::open_in_memory().unwrap();
        let id = running_since(&store, "2026-01-01T00:00:00+00:00");
        store
            .touch_snapshot(&id, "2026-01-01T00:55:00Z".parse().unwrap())
            .unwrap();
        let now = "2026-01-01T01:00:00Z".parse().unwrap();

        let reconciled = store
            .reconcile_stale_running(now, STALE_RUNNING_AFTER)
            .unwrap();

        assert!(reconciled.is_empty());
        assert_eq!(
            store.get_snapshot(&id).unwrap().status,
            SnapshotStatus::Running
        );
    }

    #[test]
    fn unit_delete_snapshots_is_atomic_when_one_id_is_foreign_tenant() {
        let store = Store::open_in_memory().unwrap();
        let mine = store.create_snapshot("tenant-a", None).unwrap();
        let theirs = store.create_snapshot("tenant-b", None).unwrap();
        let store = store.with_tenant(Some("tenant-a"));

        let result = store.delete_snapshots(&[mine.id.clone(), theirs.id.clone()]);

        assert!(result.is_err());
        assert!(store.get_snapshot(&mine.id).is_ok(), "rolled back");
    }

    #[test]
    fn unit_integrity_check_reports_ok_for_fresh_database() {
        let store = Store::open_in_memory().unwrap();
        assert!(store.integrity_check().unwrap().is_ok());
        store.vacuum().unwrap();
    }
}

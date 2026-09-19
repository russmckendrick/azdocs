mod edges;
mod findings;
mod maintenance;
mod resources;
mod schema;
mod snapshots;
mod websites;

pub use maintenance::{IntegrityReport, STALE_RUNNING_AFTER};
pub use snapshots::{SnapshotCounts, SnapshotDiff};

use std::path::Path;

use rusqlite::{Connection, OpenFlags};

use crate::error::{StoreDecodeError, StoreError};

/// Snapshot-scoped SQLite store. All writers and readers go through here;
/// nothing outside this module touches SQL.
pub struct Store {
    conn: Connection,
    tenant_id: Option<String>,
}

impl Store {
    /// Open for writing: creates the file, runs pending migrations, switches
    /// to WAL and reconciles any collect that died without finishing. This
    /// is the only open that changes a database, so collect, prune, delete
    /// and verify use it and everything that merely reads does not.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            // Default locations live under the platform data dir, which may
            // not exist yet on first run.
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(path)?;
        schema::migrate(&conn)?;
        let store = Self {
            conn,
            tenant_id: None,
        };
        let reconciled = store.reconcile_stale_running(chrono::Utc::now(), STALE_RUNNING_AFTER)?;
        if !reconciled.is_empty() {
            tracing::warn!(
                snapshots = ?reconciled,
                "marked abandoned running snapshots as failed"
            );
        }
        Ok(store)
    }

    /// Open for reading only. Never migrates, never touches the journal
    /// mode, never creates a file: an archived baseline stays byte-for-byte
    /// the artefact that was archived, and a stray write in a report,
    /// diagram or TUI path fails loudly instead of silently mutating history.
    pub fn open_read_only(path: &Path) -> Result<Self, StoreError> {
        if !path.is_file() {
            return Err(StoreError::DatabaseMissing(path.to_path_buf()));
        }
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        schema::configure(&conn)?;
        let found = schema::current_version(&conn)?;
        if found > schema::SUPPORTED_VERSION {
            return Err(StoreError::SchemaTooNew {
                found,
                supported: schema::SUPPORTED_VERSION,
            });
        }
        if found < schema::SUPPORTED_VERSION {
            return Err(StoreError::MigrationRequired {
                found,
                required: schema::SUPPORTED_VERSION,
            });
        }
        Ok(Self {
            conn,
            tenant_id: None,
        })
    }

    pub fn open_in_memory() -> Result<Self, StoreError> {
        let conn = Connection::open_in_memory()?;
        schema::migrate(&conn)?;
        Ok(Self {
            conn,
            tenant_id: None,
        })
    }

    /// The schema version stored in `meta`, for `snapshots show`/`verify`.
    pub fn schema_version(&self) -> Result<usize, StoreError> {
        schema::current_version(&self.conn)
    }

    pub(crate) fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn with_tenant(mut self, tenant_id: Option<&str>) -> Self {
        self.tenant_id = tenant_id.map(str::to_ascii_lowercase);
        self
    }

    pub fn require_tenant(&self) -> Result<(), StoreError> {
        if self.tenant_id.is_none() && self.tenant_ids()?.len() > 1 {
            return Err(StoreError::TenantSelectionRequired);
        }
        Ok(())
    }

    /// Resolve references within the selected tenant. Implicit latest is never
    /// cross-tenant and never a failed, cancelled or running snapshot: a
    /// pipeline that reports on `latest` after a broken collect must see the
    /// last good evidence, not an empty estate.
    pub fn resolve_snapshot(&self, reference: &str) -> Result<String, StoreError> {
        if reference == "latest" {
            self.require_tenant()?;
            return self.conn.query_row(
                "SELECT id FROM snapshots WHERE status IN ('complete','partial') AND (?1 IS NULL OR lower(tenant_id) = ?1) ORDER BY created_at DESC, id DESC LIMIT 1",
                [self.tenant_id.as_deref()], |row| row.get(0),
            ).map_err(|err| match err { rusqlite::Error::QueryReturnedNoRows => StoreError::NoSnapshots, other => other.into() });
        }
        // Ids are UUIDs, so anything outside hex and dashes cannot match;
        // rejecting it up front also keeps `%` and `_` from acting as LIKE
        // wildcards in the prefix match below.
        if reference.is_empty() || !reference.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
            return Err(StoreError::SnapshotNotFound(reference.into()));
        }
        let matched: Vec<String> = self.conn.prepare(
            "SELECT id FROM snapshots WHERE id LIKE ?1 || '%' AND (?2 IS NULL OR lower(tenant_id) = ?2) ORDER BY id"
        )?.query_map(rusqlite::params![reference,self.tenant_id], |row| row.get(0))?.collect::<Result<_,_>>()?;
        match matched.as_slice() {
            [id] => Ok(id.clone()),
            [] => Err(StoreError::SnapshotNotFound(reference.into())),
            _ => Err(StoreError::SnapshotNotFound(format!(
                "{reference} (ambiguous, matches {} snapshots)",
                matched.len()
            ))),
        }
    }
}

pub(crate) fn json_text(value: &Option<serde_json::Value>) -> Option<String> {
    value.as_ref().map(std::string::ToString::to_string)
}

/// Decode a stored JSON column. Malformed text is corruption, not an
/// absent value, so it surfaces as a conversion error naming the column.
pub(crate) fn parse_json(
    column: &'static str,
    text: Option<String>,
) -> rusqlite::Result<Option<serde_json::Value>> {
    text.map(|t| {
        serde_json::from_str(&t).map_err(|_| decode_error(column, t.chars().take(80).collect()))
    })
    .transpose()
}

pub(crate) fn decode_error(column: &'static str, value: String) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(StoreDecodeError { column, value }),
    )
}

pub(crate) fn parse_timestamp(
    column: &'static str,
    text: String,
) -> rusqlite::Result<chrono::DateTime<chrono::Utc>> {
    text.parse::<chrono::DateTime<chrono::Utc>>()
        .map_err(|_| decode_error(column, text))
}

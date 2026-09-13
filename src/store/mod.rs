mod edges;
mod findings;
mod resources;
mod schema;
mod snapshots;
mod websites;

pub use snapshots::{SnapshotCounts, SnapshotDiff};

use std::path::Path;

use rusqlite::Connection;

use crate::error::StoreError;

/// Snapshot-scoped SQLite store. All writers and readers go through here;
/// nothing outside this module touches SQL.
pub struct Store {
    conn: Connection,
    tenant_id: Option<String>,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            // Default locations live under the platform data dir, which may
            // not exist yet on first run.
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(path)?;
        schema::migrate(&conn)?;
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

    /// Resolve references within the selected tenant. Implicit latest is never cross-tenant.
    pub fn resolve_snapshot(&self, reference: &str) -> Result<String, StoreError> {
        if reference == "latest" {
            self.require_tenant()?;
            return self.conn.query_row(
                "SELECT id FROM snapshots WHERE status != 'running' AND (?1 IS NULL OR lower(tenant_id) = ?1) ORDER BY created_at DESC, id DESC LIMIT 1",
                [self.tenant_id.as_deref()], |row| row.get(0),
            ).map_err(|err| match err { rusqlite::Error::QueryReturnedNoRows => StoreError::NoSnapshots, other => other.into() });
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

pub(crate) fn parse_json(text: Option<String>) -> Option<serde_json::Value> {
    text.and_then(|t| serde_json::from_str(&t).ok())
}

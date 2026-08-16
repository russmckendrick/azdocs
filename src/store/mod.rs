mod edges;
mod findings;
mod resources;
mod schema;
mod snapshots;

pub use snapshots::{SnapshotCounts, SnapshotDiff};

use std::path::Path;

use rusqlite::Connection;

use crate::error::StoreError;

/// Snapshot-scoped SQLite store. All writers and readers go through here;
/// nothing outside this module touches SQL.
pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        let conn = Connection::open(path)?;
        schema::migrate(&conn)?;
        Ok(Self { conn })
    }

    pub fn open_in_memory() -> Result<Self, StoreError> {
        let conn = Connection::open_in_memory()?;
        schema::migrate(&conn)?;
        Ok(Self { conn })
    }

    pub(crate) fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Resolve "latest" or a (possibly abbreviated) snapshot id to a full id.
    pub fn resolve_snapshot(&self, reference: &str) -> Result<String, StoreError> {
        if reference == "latest" {
            return self
                .conn
                .query_row(
                    "SELECT id FROM snapshots WHERE status != 'running'
                     ORDER BY created_at DESC LIMIT 1",
                    [],
                    |row| row.get(0),
                )
                .map_err(|err| match err {
                    rusqlite::Error::QueryReturnedNoRows => StoreError::NoSnapshots,
                    other => other.into(),
                });
        }
        let matched: Vec<String> = self
            .conn
            .prepare("SELECT id FROM snapshots WHERE id LIKE ?1 || '%'")?
            .query_map([reference], |row| row.get(0))?
            .collect::<Result<_, _>>()?;
        match matched.as_slice() {
            [id] => Ok(id.clone()),
            [] => Err(StoreError::SnapshotNotFound(reference.to_owned())),
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

use rusqlite::Connection;

use crate::error::StoreError;

/// Ordered migrations; `meta.schema_version` records how many have run.
/// Append new migrations — never edit existing ones.
const MIGRATIONS: &[&str] = &[
    // 1: initial schema
    "
    CREATE TABLE meta (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );

    CREATE TABLE snapshots (
        id           TEXT PRIMARY KEY,
        created_at   TEXT NOT NULL,
        tenant_id    TEXT NOT NULL,
        tool_version TEXT NOT NULL,
        status       TEXT NOT NULL,
        notes        TEXT
    );

    CREATE TABLE subscriptions (
        snapshot_id     TEXT NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
        subscription_id TEXT NOT NULL,
        display_name    TEXT NOT NULL,
        state           TEXT,
        tags            TEXT,
        PRIMARY KEY (snapshot_id, subscription_id)
    );

    CREATE TABLE resource_groups (
        snapshot_id     TEXT NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
        id              TEXT NOT NULL,
        name            TEXT NOT NULL,
        subscription_id TEXT NOT NULL,
        location        TEXT,
        tags            TEXT,
        PRIMARY KEY (snapshot_id, id)
    );

    CREATE TABLE resources (
        snapshot_id     TEXT NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
        id              TEXT NOT NULL,
        display_id      TEXT NOT NULL,
        name            TEXT NOT NULL,
        type            TEXT NOT NULL,
        kind            TEXT,
        location        TEXT,
        resource_group  TEXT,
        subscription_id TEXT NOT NULL,
        tags            TEXT,
        sku             TEXT,
        identity        TEXT,
        properties      TEXT,
        PRIMARY KEY (snapshot_id, id)
    );
    CREATE INDEX idx_resources_type ON resources(snapshot_id, type);
    CREATE INDEX idx_resources_sub  ON resources(snapshot_id, subscription_id, resource_group);

    CREATE TABLE edges (
        snapshot_id TEXT NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
        source_id   TEXT NOT NULL,
        target_id   TEXT NOT NULL,
        edge_type   TEXT NOT NULL,
        properties  TEXT,
        PRIMARY KEY (snapshot_id, source_id, target_id, edge_type)
    );
    CREATE INDEX idx_edges_target ON edges(snapshot_id, target_id);

    CREATE TABLE findings (
        id          INTEGER PRIMARY KEY,
        snapshot_id TEXT NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
        query_name  TEXT NOT NULL,
        category    TEXT NOT NULL,
        severity    TEXT NOT NULL,
        resource_id TEXT,
        title       TEXT NOT NULL,
        detail      TEXT
    );
    CREATE INDEX idx_findings_snapshot ON findings(snapshot_id, severity);

    CREATE TABLE query_runs (
        snapshot_id TEXT NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
        query_name  TEXT NOT NULL,
        category    TEXT NOT NULL,
        row_count   INTEGER,
        duration_ms INTEGER,
        error       TEXT,
        PRIMARY KEY (snapshot_id, query_name)
    );

    -- Raw rows from shaped inventory queries (mv-expanded subnets, peerings,
    -- ...) kept verbatim for report tables; core queries load typed tables.
    CREATE TABLE query_results (
        snapshot_id TEXT NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
        query_name  TEXT NOT NULL,
        row_index   INTEGER NOT NULL,
        row         TEXT NOT NULL,
        PRIMARY KEY (snapshot_id, query_name, row_index)
    );
    ",
    // 2: drop indexes whose only readers are gone.
    //
    // `idx_resources_type` served `Store::resources_of_type` and
    // `idx_edges_target` served `Store::edges_for_resource`. Both methods were
    // removed: every caller loads the full set for a snapshot and filters in
    // memory, so the indexes only cost insert time and file size on collect.
    // IF EXISTS keeps this idempotent for databases created before migration 1
    // was the whole schema.
    "
    DROP INDEX IF EXISTS idx_resources_type;
    DROP INDEX IF EXISTS idx_edges_target;
    ",
];

pub fn migrate(conn: &Connection) -> Result<(), StoreError> {
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    let current = current_version(conn)?;
    for (index, migration) in MIGRATIONS.iter().enumerate().skip(current) {
        let version = index + 1;
        conn.execute_batch(&format!(
            "BEGIN;\n{migration}\nINSERT INTO meta(key, value) VALUES ('schema_version', '{version}')\n\
             ON CONFLICT(key) DO UPDATE SET value = excluded.value;\nCOMMIT;"
        ))?;
    }
    Ok(())
}

fn current_version(conn: &Connection) -> Result<usize, StoreError> {
    let meta_exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='meta')",
        [],
        |row| row.get(0),
    )?;
    if !meta_exists {
        return Ok(0);
    }
    let version: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .map(Some)
        .or_else(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })?;
    Ok(version.and_then(|v| v.parse().ok()).unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrate_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();

        migrate(&conn).unwrap();
        migrate(&conn).unwrap();

        assert_eq!(current_version(&conn).unwrap(), MIGRATIONS.len());
    }
}

use rusqlite::params;

use super::{Store, json_text, parse_json};
use crate::error::StoreError;
use crate::model::{Edge, EdgeKind};

impl Store {
    pub fn insert_edges(&self, snapshot_id: &str, edges: &[Edge]) -> Result<(), StoreError> {
        let tx = self.conn().unchecked_transaction()?;
        {
            let mut statement = tx.prepare_cached(
                "INSERT OR REPLACE INTO edges
                 (snapshot_id, source_id, target_id, edge_type, properties)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            for edge in edges {
                statement.execute(params![
                    snapshot_id,
                    edge.source_id,
                    edge.target_id,
                    edge.kind.as_str(),
                    json_text(&edge.properties),
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Edges whose kind this build knows. Unlike every other decoder in the
    /// store, an unknown `edge_type` is skipped rather than an error: the
    /// schema version does not guard the kind vocabulary, so a newer azdocs
    /// may legitimately have stored a kind this one cannot name, and one
    /// warning beats refusing to open the snapshot.
    pub fn edges(&self, snapshot_id: &str) -> Result<Vec<Edge>, StoreError> {
        let mut statement = self.conn().prepare(
            "SELECT source_id, target_id, edge_type, properties
             FROM edges WHERE snapshot_id = ?1 ORDER BY source_id, target_id, edge_type",
        )?;
        let rows = statement.query_map([snapshot_id], edge_from_row)?;
        let mut edges = Vec::new();
        let mut unknown = std::collections::BTreeMap::<String, usize>::new();
        for row in rows {
            match row? {
                Ok(edge) => edges.push(edge),
                Err(kind) => *unknown.entry(kind).or_default() += 1,
            }
        }
        for (kind, count) in unknown {
            tracing::warn!(
                kind,
                count,
                "skipped edges of a kind this build does not know"
            );
        }
        Ok(edges)
    }
}

/// `Err(kind)` when the stored kind is not one this build knows.
fn edge_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Result<Edge, String>> {
    let kind: String = row.get(2)?;
    let Some(kind) = EdgeKind::parse(&kind) else {
        return Ok(Err(kind));
    };
    Ok(Ok(Edge {
        source_id: row.get(0)?,
        target_id: row.get(1)?,
        kind,
        properties: parse_json("edges.properties", row.get(3)?)?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_unknown_edge_kind_is_skipped_with_warning() {
        let store = Store::open_in_memory().unwrap();
        let id = store.create_snapshot("tenant", None).unwrap().id;
        store
            .conn()
            .execute(
                "INSERT INTO edges VALUES (?1, '/a', '/b', 'from_the_future', NULL),
                                         (?1, '/a', '/c', 'attached_to', NULL)",
                [&id],
            )
            .unwrap();

        let edges = store.edges(&id).unwrap();

        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].kind, EdgeKind::AttachedTo);
    }
}

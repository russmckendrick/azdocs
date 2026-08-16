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

    pub fn edges(&self, snapshot_id: &str) -> Result<Vec<Edge>, StoreError> {
        let mut statement = self.conn().prepare(
            "SELECT source_id, target_id, edge_type, properties
             FROM edges WHERE snapshot_id = ?1 ORDER BY source_id, target_id, edge_type",
        )?;
        let rows = statement.query_map([snapshot_id], edge_from_row)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Edges that touch `resource_id` in either direction.
    pub fn edges_for_resource(
        &self,
        snapshot_id: &str,
        resource_id: &str,
    ) -> Result<Vec<Edge>, StoreError> {
        let mut statement = self.conn().prepare(
            "SELECT source_id, target_id, edge_type, properties
             FROM edges WHERE snapshot_id = ?1 AND (source_id = ?2 OR target_id = ?2)
             ORDER BY source_id, target_id, edge_type",
        )?;
        let rows = statement.query_map([snapshot_id, resource_id], edge_from_row)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}

fn edge_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Edge> {
    let kind: String = row.get(2)?;
    Ok(Edge {
        source_id: row.get(0)?,
        target_id: row.get(1)?,
        kind: EdgeKind::parse(&kind).unwrap_or(EdgeKind::DependsOn),
        properties: parse_json(row.get(3)?),
    })
}

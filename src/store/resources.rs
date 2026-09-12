use rusqlite::params;
use serde_json::Value;

use super::{Store, json_text, parse_json};
use crate::error::StoreError;
use crate::model::{QueryRun, Resource, ResourceGroup, Subscription};

impl Store {
    pub fn insert_subscriptions(
        &self,
        snapshot_id: &str,
        subscriptions: &[Subscription],
    ) -> Result<(), StoreError> {
        let tx = self.conn().unchecked_transaction()?;
        {
            let mut statement = tx.prepare_cached(
                "INSERT OR REPLACE INTO subscriptions
                 (snapshot_id, subscription_id, display_name, state, tags)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            for sub in subscriptions {
                statement.execute(params![
                    snapshot_id,
                    sub.subscription_id,
                    sub.display_name,
                    sub.state,
                    json_text(&sub.tags),
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn insert_resource_groups(
        &self,
        snapshot_id: &str,
        groups: &[ResourceGroup],
    ) -> Result<(), StoreError> {
        let tx = self.conn().unchecked_transaction()?;
        {
            let mut statement = tx.prepare_cached(
                "INSERT OR REPLACE INTO resource_groups
                 (snapshot_id, id, name, subscription_id, location, tags)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )?;
            for group in groups {
                statement.execute(params![
                    snapshot_id,
                    group.id,
                    group.name,
                    group.subscription_id,
                    group.location,
                    json_text(&group.tags),
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn insert_resources(
        &self,
        snapshot_id: &str,
        resources: &[Resource],
    ) -> Result<(), StoreError> {
        let tx = self.conn().unchecked_transaction()?;
        {
            let mut statement = tx.prepare_cached(
                "INSERT OR REPLACE INTO resources
                 (snapshot_id, id, display_id, name, type, kind, location,
                  resource_group, subscription_id, tags, sku, identity, properties)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            )?;
            for resource in resources {
                statement.execute(params![
                    snapshot_id,
                    resource.id,
                    resource.display_id,
                    resource.name,
                    resource.azure_type,
                    resource.kind,
                    resource.location,
                    resource.resource_group,
                    resource.subscription_id,
                    json_text(&resource.tags),
                    json_text(&resource.sku),
                    json_text(&resource.identity),
                    json_text(&resource.properties),
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn insert_query_results(
        &self,
        snapshot_id: &str,
        query_name: &str,
        rows: &[Value],
    ) -> Result<(), StoreError> {
        let tx = self.conn().unchecked_transaction()?;
        {
            let mut statement = tx.prepare_cached(
                "INSERT OR REPLACE INTO query_results (snapshot_id, query_name, row_index, row)
                 VALUES (?1, ?2, ?3, ?4)",
            )?;
            for (index, row) in rows.iter().enumerate() {
                statement.execute(params![
                    snapshot_id,
                    query_name,
                    index as i64,
                    row.to_string()
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn record_query_run(&self, snapshot_id: &str, run: &QueryRun) -> Result<(), StoreError> {
        self.conn().execute(
            "INSERT OR REPLACE INTO query_runs
             (snapshot_id, query_name, category, row_count, duration_ms, error)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                snapshot_id,
                run.query_name,
                run.category,
                run.row_count.map(|v| v as i64),
                run.duration_ms.map(|v| v as i64),
                run.error,
            ],
        )?;
        Ok(())
    }

    pub fn subscriptions(&self, snapshot_id: &str) -> Result<Vec<Subscription>, StoreError> {
        let mut statement = self.conn().prepare(
            "SELECT subscription_id, display_name, state, tags
             FROM subscriptions WHERE snapshot_id = ?1 ORDER BY display_name",
        )?;
        let rows = statement.query_map([snapshot_id], |row| {
            Ok(Subscription {
                subscription_id: row.get(0)?,
                display_name: row.get(1)?,
                state: row.get(2)?,
                tags: parse_json(row.get(3)?),
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn resource_groups(&self, snapshot_id: &str) -> Result<Vec<ResourceGroup>, StoreError> {
        let mut statement = self.conn().prepare(
            "SELECT id, name, subscription_id, location, tags
             FROM resource_groups WHERE snapshot_id = ?1 ORDER BY subscription_id, name",
        )?;
        let rows = statement.query_map([snapshot_id], |row| {
            Ok(ResourceGroup {
                id: row.get(0)?,
                name: row.get(1)?,
                subscription_id: row.get(2)?,
                location: row.get(3)?,
                tags: parse_json(row.get(4)?),
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn resources(&self, snapshot_id: &str) -> Result<Vec<Resource>, StoreError> {
        let mut statement = self.conn().prepare(
            "SELECT id, display_id, name, type, kind, location, resource_group,
                    subscription_id, tags, sku, identity, properties
             FROM resources WHERE snapshot_id = ?1 ORDER BY id",
        )?;
        let rows = statement.query_map([snapshot_id], resource_from_row)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Stored queries may outlive or replace their query-pack definition.
    pub fn query_result_names(&self, snapshot_id: &str) -> Result<Vec<String>, StoreError> {
        let mut statement = self.conn().prepare("SELECT DISTINCT query_name FROM query_results WHERE snapshot_id = ?1 ORDER BY query_name")?;
        Ok(statement
            .query_map([snapshot_id], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn query_results(
        &self,
        snapshot_id: &str,
        query_name: &str,
    ) -> Result<Vec<Value>, StoreError> {
        let mut statement = self.conn().prepare(
            "SELECT row FROM query_results
             WHERE snapshot_id = ?1 AND query_name = ?2 ORDER BY row_index",
        )?;
        let rows = statement.query_map([snapshot_id, query_name], |row| {
            let text: String = row.get(0)?;
            Ok(serde_json::from_str(&text).unwrap_or(Value::Null))
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn query_runs(&self, snapshot_id: &str) -> Result<Vec<QueryRun>, StoreError> {
        let mut statement = self.conn().prepare(
            "SELECT query_name, category, row_count, duration_ms, error
             FROM query_runs WHERE snapshot_id = ?1 ORDER BY category, query_name",
        )?;
        let rows = statement.query_map([snapshot_id], |row| {
            Ok(QueryRun {
                query_name: row.get(0)?,
                category: row.get(1)?,
                row_count: row.get::<_, Option<i64>>(2)?.map(|v| v as u64),
                duration_ms: row.get::<_, Option<i64>>(3)?.map(|v| v as u64),
                error: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}

fn resource_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Resource> {
    Ok(Resource {
        id: row.get(0)?,
        display_id: row.get(1)?,
        name: row.get(2)?,
        azure_type: row.get(3)?,
        kind: row.get(4)?,
        location: row.get(5)?,
        resource_group: row.get(6)?,
        subscription_id: row.get(7)?,
        tags: parse_json(row.get(8)?),
        sku: parse_json(row.get(9)?),
        identity: parse_json(row.get(10)?),
        properties: parse_json(row.get(11)?),
    })
}

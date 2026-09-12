use rusqlite::{OptionalExtension, params};

use super::Store;
use crate::error::StoreError;
use crate::model::websites::{
    CaptureStatus, CapturedWebsite, WebsiteCapture, WebsiteEndpoint, WebsiteEvidence,
};

fn json<T: serde::Serialize>(value: &T) -> Result<String, StoreError> {
    serde_json::to_string(value)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)).into())
}

fn decode<T: serde::de::DeserializeOwned>(text: String) -> rusqlite::Result<T> {
    serde_json::from_str(&text).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

impl Store {
    pub fn save_website_inventory(
        &self,
        snapshot: &str,
        endpoints: &[WebsiteEndpoint],
        evidence: &[WebsiteEvidence],
    ) -> Result<(), StoreError> {
        let tx = self.conn().unchecked_transaction()?;
        tx.execute(
            "DELETE FROM website_endpoints WHERE snapshot_id=?1",
            [snapshot],
        )?;
        for (index, endpoint) in endpoints.iter().enumerate() {
            tx.execute(
                "INSERT INTO website_endpoints VALUES (?1, ?2, ?3)",
                params![snapshot, index as i64, json(endpoint)?],
            )?;
        }
        for item in evidence {
            tx.execute(
                "INSERT OR REPLACE INTO website_evidence VALUES (?1, ?2, ?3, ?4)",
                params![snapshot, item.resource_id, item.kind, json(item)?],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn website_evidence(&self, snapshot: &str) -> Result<Vec<WebsiteEvidence>, StoreError> {
        let mut stmt = self.conn().prepare(
            "SELECT evidence FROM website_evidence WHERE snapshot_id=?1 ORDER BY resource_id, kind",
        )?;
        Ok(stmt
            .query_map([snapshot], |row| decode(row.get(0)?))?
            .collect::<Result<_, _>>()?)
    }

    /// Older CLI snapshots can discover their known hostnames entirely offline.
    pub fn website_endpoints(&self, snapshot: &str) -> Result<Vec<WebsiteEndpoint>, StoreError> {
        let mut stmt = self.conn().prepare(
            "SELECT endpoint FROM website_endpoints WHERE snapshot_id=?1 ORDER BY ordinal",
        )?;
        let endpoints: Vec<_> = stmt
            .query_map([snapshot], |row| decode(row.get(0)?))?
            .collect::<Result<_, _>>()?;
        if endpoints.is_empty() {
            return Ok(crate::collect::websites::discover(
                &self.resources(snapshot)?,
                &self.website_evidence(snapshot)?,
            ));
        }
        Ok(endpoints)
    }

    pub fn website_captures(
        &self,
        snapshot: &str,
        include_png: bool,
    ) -> Result<Vec<WebsiteCapture>, StoreError> {
        let mut stmt = self.conn().prepare("SELECT url, final_url, captured_at, attempted_at, status, error, renderer, width, height, CASE WHEN ?2 THEN png ELSE NULL END FROM website_captures WHERE snapshot_id=?1 ORDER BY url")?;
        Ok(stmt
            .query_map(params![snapshot, include_png], |row| {
                Ok(WebsiteCapture {
                    url: row.get(0)?,
                    final_url: row.get(1)?,
                    captured_at: row.get(2)?,
                    attempted_at: row.get(3)?,
                    status: decode(row.get(4)?)?,
                    error: row.get(5)?,
                    renderer: row.get(6)?,
                    width: row.get(7)?,
                    height: row.get(8)?,
                    png: row.get::<_, Option<Vec<u8>>>(9)?.unwrap_or_default(),
                })
            })?
            .collect::<Result<_, _>>()?)
    }

    pub fn website_png(&self, snapshot: &str, url: &str) -> Result<Option<Vec<u8>>, StoreError> {
        Ok(self
            .conn()
            .query_row(
                "SELECT png FROM website_captures WHERE snapshot_id=?1 AND url=?2",
                params![snapshot, url],
                |row| row.get(0),
            )
            .optional()?
            .flatten())
    }

    pub fn record_website_attempt(
        &self,
        snapshot: &str,
        url: &str,
        status: CaptureStatus,
        error: Option<&str>,
    ) -> Result<(), StoreError> {
        self.conn().execute("INSERT INTO website_captures(snapshot_id,url,attempted_at,status,error) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(snapshot_id,url) DO UPDATE SET attempted_at=excluded.attempted_at, status=excluded.status, error=excluded.error", params![snapshot,url,chrono::Utc::now().to_rfc3339(),json(&status)?,error])?;
        Ok(())
    }

    pub fn save_website_capture(
        &self,
        snapshot: &str,
        url: &str,
        capture: &CapturedWebsite,
    ) -> Result<(), StoreError> {
        self.conn().execute("INSERT INTO website_captures(snapshot_id,url,final_url,captured_at,attempted_at,status,renderer,width,height,png) VALUES(?1,?2,?3,?4,?4,?5,?6,1440,900,?7) ON CONFLICT(snapshot_id,url) DO UPDATE SET final_url=excluded.final_url,captured_at=excluded.captured_at,attempted_at=excluded.attempted_at,status=excluded.status,error=NULL,renderer=excluded.renderer,width=excluded.width,height=excluded.height,png=excluded.png", params![snapshot,url,capture.final_url,capture.captured_at,json(&CaptureStatus::Captured)?,capture.renderer,capture.png])?;
        Ok(())
    }

    /// Called once when a desktop process opens a database, before any batch.
    pub fn recover_website_captures(&self) -> Result<(), StoreError> {
        self.conn().execute(
            "UPDATE website_captures SET status=?1 WHERE status=?2",
            params![
                json(&CaptureStatus::Interrupted)?,
                json(&CaptureStatus::Running)?
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_refresh_preserves_last_success_and_delete_cascades() {
        let store = Store::open_in_memory().unwrap();
        let snapshot = store.create_snapshot("tenant", None).unwrap();
        let capture = CapturedWebsite {
            final_url: "https://example.test/".into(),
            captured_at: "2026-09-12T10:00:00Z".into(),
            renderer: "test".into(),
            png: vec![1, 2, 3],
        };
        store
            .save_website_capture(&snapshot.id, &capture.final_url, &capture)
            .unwrap();
        store
            .record_website_attempt(
                &snapshot.id,
                &capture.final_url,
                CaptureStatus::Failed,
                Some("offline"),
            )
            .unwrap();
        let saved = store
            .website_captures(&snapshot.id, true)
            .unwrap()
            .remove(0);
        assert_eq!(
            (saved.png, saved.captured_at, saved.status),
            (
                capture.png,
                Some(capture.captured_at),
                CaptureStatus::Failed
            )
        );
        store.delete_snapshot(&snapshot.id).unwrap();
        assert!(
            store
                .website_captures(&snapshot.id, true)
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn shared_url_keeps_associations_and_evidence_without_duplicate_images() {
        let store = Store::open_in_memory().unwrap();
        let snapshot = store.create_snapshot("tenant", None).unwrap();
        let url = "https://shared.example/";
        let endpoints: Vec<_> = ["/resources/a", "/resources/b"]
            .into_iter()
            .map(|resource| WebsiteEndpoint {
                resource_id: resource.into(),
                resource_name: resource.into(),
                source: "default".into(),
                hostname: Some("shared.example".into()),
                url: Some(url.into()),
                status: crate::model::websites::EndpointStatus::Ready,
            })
            .collect();
        let evidence = WebsiteEvidence {
            resource_id: "/resources/a".into(),
            kind: "domains".into(),
            collected_at: "2026-01-01".into(),
            rows: vec![],
            error: Some("403".into()),
        };
        store
            .save_website_inventory(&snapshot.id, &endpoints, &[evidence])
            .unwrap();
        store
            .save_website_capture(
                &snapshot.id,
                url,
                &CapturedWebsite {
                    final_url: url.into(),
                    captured_at: "2026-01-01".into(),
                    renderer: "fixture".into(),
                    png: vec![1, 2],
                },
            )
            .unwrap();
        assert_eq!(store.website_endpoints(&snapshot.id).unwrap(), endpoints);
        assert_eq!(
            store.website_captures(&snapshot.id, false).unwrap().len(),
            1
        );
        assert!(
            store.website_captures(&snapshot.id, false).unwrap()[0]
                .png
                .is_empty()
        );
        store.delete_snapshot(&snapshot.id).unwrap();
        assert!(store.website_endpoints(&snapshot.id).unwrap().is_empty());
        assert!(store.website_evidence(&snapshot.id).unwrap().is_empty());
        assert!(store.website_png(&snapshot.id, url).unwrap().is_none());
    }

    #[test]
    fn recovery_marks_running_capture_interrupted() {
        let store = Store::open_in_memory().unwrap();
        let id = store.create_snapshot("tenant", None).unwrap().id;
        store
            .record_website_attempt(&id, "https://example.test/", CaptureStatus::Running, None)
            .unwrap();
        store.recover_website_captures().unwrap();
        assert_eq!(
            store.website_captures(&id, false).unwrap()[0].status,
            CaptureStatus::Interrupted
        );
    }
}

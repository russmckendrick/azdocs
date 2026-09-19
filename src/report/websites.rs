//! Saved raster evidence shared by offline report emitters.
use super::{
    document::{Block, TableColumn, TableKind, TableLink},
    site::html_escape,
};
use crate::{
    error::StoreError,
    labels::{Labels, fill},
    model::websites::{WebsiteCapture, WebsiteEndpoint},
    store::Store,
};
use base64::Engine as _;
use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(Debug, Default)]
pub struct WebsiteReport {
    pub endpoints: Vec<WebsiteEndpoint>,
    pub captures: BTreeMap<String, WebsiteCapture>,
}

/// A website endpoint as a table row, already worded.
#[derive(Debug, Clone, serde::Serialize)]
pub struct WebsiteRow {
    pub resource_id: String,
    pub resource_name: String,
    /// The URL, else the hostname, else the source the endpoint came from.
    pub target: String,
    pub status: String,
    pub captured_at: Option<String>,
    pub final_url: Option<String>,
    /// Slug of the saved PNG under `websites/`, when one exists.
    pub image: Option<String>,
    pub error: Option<String>,
}

impl WebsiteReport {
    pub fn build(store: &Store, snapshot: &str, include_images: bool) -> Result<Self, StoreError> {
        Ok(Self {
            endpoints: store.website_endpoints(snapshot)?,
            captures: store
                .website_captures(snapshot, include_images)?
                .into_iter()
                .map(|c| (c.url.clone(), c))
                .collect(),
        })
    }

    /// Keep only the endpoints (and the captures they reference) of the given
    /// resources, for a scoped report.
    pub fn retain_resources(&mut self, resource_ids: &std::collections::HashSet<&str>) {
        self.endpoints
            .retain(|endpoint| resource_ids.contains(endpoint.resource_id.as_str()));
        let referenced: BTreeSet<&str> = self
            .endpoints
            .iter()
            .filter_map(|endpoint| endpoint.url.as_deref())
            .collect();
        self.captures
            .retain(|url, _| referenced.contains(url.as_str()));
    }

    pub fn image_slug(&self, url: &str) -> String {
        // BTreeMap order makes asset names stable for fixed stored evidence.
        format!(
            "website-{}",
            self.captures.keys().position(|key| key == url).unwrap_or(0)
        )
    }

    pub fn write_assets(&self, root: &Path) -> std::io::Result<()> {
        for (url, capture) in &self.captures {
            if !capture.png.is_empty() {
                std::fs::create_dir_all(root.join("websites"))?;
                std::fs::write(
                    root.join("websites")
                        .join(format!("{}.png", self.image_slug(url))),
                    &capture.png,
                )?;
            }
        }
        Ok(())
    }

    fn entries<'a>(&'a self, resource_ids: Option<&BTreeSet<String>>) -> Vec<&'a WebsiteEndpoint> {
        let mut seen = BTreeSet::new();
        self.endpoints
            .iter()
            .filter(|e| resource_ids.is_none_or(|ids| ids.contains(&e.resource_id)))
            .filter(|e| seen.insert((&e.resource_id, &e.url, &e.hostname, e.status.as_str())))
            .collect()
    }

    /// One row per website endpoint, worded for a table: the assessment, the
    /// Markdown index and the workbook all draw this instead of reading the
    /// endpoint and capture records themselves.
    pub fn rows(&self, labels: &Labels) -> Vec<WebsiteRow> {
        let words = &labels.common.websites;
        self.entries(None)
            .into_iter()
            .map(|endpoint| {
                let capture = endpoint.url.as_ref().and_then(|url| self.captures.get(url));
                let state = capture.map(|c| c.status.as_str()).unwrap_or(
                    if endpoint.status.as_str() == "ready" {
                        "pending"
                    } else {
                        endpoint.status.as_str()
                    },
                );
                let captured = capture.filter(|c| !c.png.is_empty() && c.captured_at.is_some());
                WebsiteRow {
                    resource_id: endpoint.resource_id.clone(),
                    resource_name: endpoint.resource_name.clone(),
                    target: endpoint
                        .url
                        .clone()
                        .or_else(|| endpoint.hostname.clone())
                        .unwrap_or_else(|| endpoint.source.clone()),
                    status: words.states.get(state).unwrap_or(&words.no_image).clone(),
                    captured_at: captured
                        .and_then(|c| c.captured_at.as_deref())
                        .map(print_time),
                    final_url: capture
                        .and_then(|c| c.final_url.clone())
                        .filter(|url| capture.is_some_and(|c| *url != c.url)),
                    image: captured.map(|c| self.image_slug(&c.url)),
                    error: capture.and_then(|c| c.error.clone()),
                }
            })
            .collect()
    }

    pub fn html(
        &self,
        labels: &Labels,
        resource_ids: Option<&BTreeSet<String>>,
        asset_prefix: Option<&str>,
    ) -> String {
        let entries = self.entries(resource_ids);
        if entries.is_empty() {
            return String::new();
        }
        let words = &labels.common.websites;
        let mut html = format!(
            "<section class=\"website-evidence\"><h2>{}</h2><p>{}</p>",
            html_escape(&words.title),
            html_escape(&words.detail)
        );
        for endpoint in entries {
            let capture = endpoint.url.as_ref().and_then(|url| self.captures.get(url));
            let state = capture.map(|c| c.status.as_str()).unwrap_or(
                if endpoint.status.as_str() == "ready" {
                    "pending"
                } else {
                    endpoint.status.as_str()
                },
            );
            let status = words
                .states
                .get(state)
                .map(String::as_str)
                .unwrap_or(&words.no_image);
            html.push_str(&format!(
                "<article><h3>{}</h3><p><code>{}</code> · {}</p>",
                html_escape(&endpoint.resource_name),
                html_escape(
                    endpoint
                        .url
                        .as_deref()
                        .or(endpoint.hostname.as_deref())
                        .unwrap_or(&endpoint.source)
                ),
                html_escape(status)
            ));
            if let Some(capture) = capture {
                if let Some(time) = &capture.captured_at {
                    let caption = fill(&words.caption, &[("url", &capture.url), ("time", time)]);
                    if !capture.png.is_empty() {
                        let source = match asset_prefix {
                            Some(prefix) => {
                                format!("{prefix}websites/{}.png", self.image_slug(&capture.url))
                            }
                            None => format!(
                                "data:image/png;base64,{}",
                                base64::engine::general_purpose::STANDARD.encode(&capture.png)
                            ),
                        };
                        html.push_str(&format!("<figure><img src=\"{}\" alt=\"{}\" width=\"1440\" height=\"900\" style=\"display:block;max-width:100%;height:auto\" loading=\"lazy\"><figcaption>{}</figcaption></figure>", html_escape(&source),html_escape(&caption),html_escape(&caption)));
                    }
                }
                if let Some(url) = &capture.final_url {
                    html.push_str(&format!(
                        "<p>{}: <code>{}</code></p>",
                        html_escape(&words.final_url),
                        html_escape(url)
                    ));
                }
                html.push_str(&format!(
                    "<p>{}</p>",
                    html_escape(&fill(
                        &words.attempted_at,
                        &[("time", &capture.attempted_at)]
                    ))
                ));
                if capture.png.is_empty() {
                    html.push_str(&format!("<p>{}</p>", html_escape(&words.no_image)));
                } else if capture.status != crate::model::websites::CaptureStatus::Captured {
                    html.push_str(&format!("<p>{}</p>", html_escape(&words.previous_image)));
                }
                if let Some(error) = &capture.error {
                    html.push_str(&format!("<p>{}</p>", html_escape(error)));
                }
            } else {
                html.push_str(&format!(
                    "<p>{}</p>",
                    html_escape(if endpoint.status.as_str() == "missing_evidence" {
                        &words.missing_evidence
                    } else {
                        &words.no_image
                    })
                ));
            }
            html.push_str("</article>");
        }
        html.push_str("</section>");
        html
    }

    pub(crate) fn print_blocks<'a>(
        &'a self,
        resource_id: &str,
        labels: &'a Labels,
        blocks: &mut Vec<Block<'a>>,
    ) {
        let ids = BTreeSet::from([resource_id.to_owned()]);
        let entries = self.entries(Some(&ids));
        if entries.is_empty() {
            return;
        }
        let words = &labels.common.websites;
        blocks.push(Block::SubLabel {
            title: Cow::Borrowed(&words.title),
        });
        // Keep the gallery together; every URL and attempt belongs to one
        // section afterwards, with a table for each website, including those
        // without a saved image.
        for endpoint in &entries {
            if let Some(capture) = endpoint.url.as_ref().and_then(|url| self.captures.get(url))
                && capture.captured_at.is_some()
                && !capture.png.is_empty()
            {
                blocks.push(Block::RasterImage {
                    slug: self.image_slug(&capture.url),
                    png: &capture.png,
                    caption: Cow::Borrowed(&capture.url),
                });
            }
        }
        blocks.push(Block::SubLabel {
            title: Cow::Borrowed(&words.links_title),
        });
        for endpoint in entries {
            let mut rows = Vec::new();
            let mut links = Vec::new();
            let capture = endpoint.url.as_ref().and_then(|url| self.captures.get(url));
            let state = capture.map(|c| c.status.as_str()).unwrap_or(
                if endpoint.status.as_str() == "ready" {
                    "pending"
                } else {
                    endpoint.status.as_str()
                },
            );
            let status = words.states.get(state).unwrap_or(&words.no_image);
            let target = endpoint
                .url
                .as_deref()
                .or(endpoint.hostname.as_deref())
                .unwrap_or(&endpoint.source);
            if let Some(url) = &endpoint.url {
                links.push(TableLink {
                    row: rows.len(),
                    column: 1,
                    target: url.clone(),
                    external: true,
                });
            }
            rows.push(vec![
                Cow::Borrowed(words.website.as_str()),
                Cow::Borrowed(target),
            ]);
            rows.push(vec![
                Cow::Borrowed(labels.common.columns.status.as_str()),
                Cow::Borrowed(status.as_str()),
            ]);
            if let Some(capture) = capture {
                if let Some(time) = &capture.captured_at
                    && !capture.png.is_empty()
                {
                    rows.push(vec![
                        Cow::Borrowed(words.capture_time.as_str()),
                        Cow::Owned(print_time(time)),
                    ]);
                }
                if let Some(url) = &capture.final_url
                    && url != &capture.url
                {
                    links.push(TableLink {
                        row: rows.len(),
                        column: 1,
                        target: url.clone(),
                        external: true,
                    });
                    rows.push(vec![
                        Cow::Borrowed(words.final_url.as_str()),
                        Cow::Borrowed(url),
                    ]);
                }
                if capture.status != crate::model::websites::CaptureStatus::Captured {
                    rows.push(vec![
                        Cow::Borrowed(words.attempt_time.as_str()),
                        Cow::Owned(print_time(&capture.attempted_at)),
                    ]);
                    if !capture.png.is_empty() {
                        rows.push(vec![
                            Cow::Borrowed(words.capture_note.as_str()),
                            Cow::Borrowed(words.previous_image.as_str()),
                        ]);
                    }
                }
                if let Some(error) = &capture.error {
                    rows.push(vec![
                        Cow::Borrowed(words.failure_detail.as_str()),
                        Cow::Borrowed(error.as_str()),
                    ]);
                }
            } else if endpoint.status.as_str() == "missing_evidence" {
                rows.push(vec![
                    Cow::Borrowed(labels.common.columns.coverage.as_str()),
                    Cow::Borrowed(words.missing_evidence.as_str()),
                ]);
            }
            blocks.push(Block::Table {
                style: TableKind::Data,
                // Compact records stay together; long redirects can paginate
                // without dropping any part of the stored URL.
                keep_together: rows
                    .iter()
                    .flatten()
                    .map(|value| value.chars().count())
                    .sum::<usize>()
                    <= 600,
                columns: vec![
                    TableColumn {
                        label: Cow::Borrowed(&labels.common.columns.setting),
                        mono: false,
                        weight: 1,
                    },
                    TableColumn {
                        label: Cow::Borrowed(&labels.common.columns.value),
                        mono: false,
                        weight: 2,
                    },
                ],
                rows,
                links,
            });
        }
    }
}

fn print_time(value: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(value).map_or_else(
        |_| value.to_owned(),
        |time| {
            time.with_timezone(&chrono::Utc)
                .format("%d %b %Y, %H:%M:%S UTC")
                .to_string()
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::websites::{CaptureStatus, EndpointStatus};

    #[test]
    fn print_blocks_puts_all_images_before_separate_website_tables_including_uncaptured_urls() {
        let mut report = WebsiteReport::default();
        for host in [
            "first.example.com",
            "second.example.com",
            "missing.example.com",
        ] {
            let url = format!("https://{host}/");
            report.endpoints.push(WebsiteEndpoint {
                resource_id: "/resource".into(),
                resource_name: "fixture".into(),
                source: "fixture".into(),
                hostname: Some(host.into()),
                url: Some(url.clone()),
                status: EndpointStatus::Ready,
            });
            if !host.starts_with("missing") {
                report.captures.insert(
                    url.clone(),
                    WebsiteCapture {
                        url,
                        final_url: None,
                        captured_at: Some("2026-09-12T10:00:00Z".into()),
                        attempted_at: "2026-09-13T10:00:00Z".into(),
                        status: CaptureStatus::Failed,
                        error: Some("DNS failure".into()),
                        renderer: Some("fixture".into()),
                        width: Some(1440),
                        height: Some(900),
                        png: vec![1],
                    },
                );
            }
        }
        let labels = Labels::default();
        let mut blocks = Vec::new();
        report.print_blocks("/resource", &labels, &mut blocks);
        let images: Vec<_> = blocks
            .iter()
            .enumerate()
            .filter_map(|(i, block)| matches!(block, Block::RasterImage { .. }).then_some(i))
            .collect();
        let tables: Vec<_> = blocks
            .iter()
            .enumerate()
            .filter(|(_, block)| matches!(block, Block::Table { .. }))
            .collect();
        assert_eq!(images.len(), 2);
        assert_eq!(tables.len(), 3);
        assert!(images.iter().all(|index| *index < tables[0].0));
        let mut values = Vec::new();
        for (_, block) in tables {
            let Block::Table { rows, links, .. } = block else {
                unreachable!()
            };
            assert_eq!(links.len(), 1);
            assert!(links[0].external);
            values.extend(rows.iter().flatten().map(|value| value.as_ref()));
        }
        assert!(values.contains(&"Not captured"));
        assert!(values.contains(&"DNS failure"));
        assert!(values.contains(&"Showing the previous successful capture."));
        assert!(values.contains(&"12 Sep 2026, 10:00:00 UTC"));
        assert!(values.contains(&"13 Sep 2026, 10:00:00 UTC"));
    }
}

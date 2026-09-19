//! Deterministic endpoint discovery and read-only Front Door enrichment.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use reqwest::Url;
use serde_json::Value;

use crate::auth::TokenProvider;
use crate::model::websites::{EndpointStatus, WebsiteEndpoint, WebsiteEvidence};
use crate::model::{Resource, normalize_arm_id};

fn array(value: &Value, key: &str) -> Vec<Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
}

fn properties(value: &Value) -> &Value {
    value.get("properties").unwrap_or(value)
}

/// Only a hostname is accepted here: paths, credentials and arbitrary schemes
/// in stored evidence must never become navigations.
pub fn endpoint_url(host: &str, scheme: &str, port: Option<u16>) -> Option<String> {
    let host = host.trim().trim_end_matches('.');
    if host.is_empty()
        || host.contains(['/', '\\', '@', '?', '#', '*', ':'])
        || host.chars().any(char::is_whitespace)
        || !matches!(scheme, "http" | "https")
    {
        return None;
    }
    let mut url = Url::parse(&format!("{scheme}://{host}/")).ok()?;
    url.set_port(port).ok()?;
    Some(url.into())
}

fn add(
    out: &mut Vec<WebsiteEndpoint>,
    resource: &Resource,
    source: &str,
    host: Option<&str>,
    scheme: &str,
    port: Option<u16>,
    disabled: bool,
) {
    let host = host.map(str::trim).filter(|h| !h.is_empty());
    if matches!(
        resource.azure_type.to_ascii_lowercase().as_str(),
        "microsoft.web/sites" | "microsoft.web/sites/slots"
    ) && host.is_some_and(|h| h.to_ascii_lowercase().contains(".scm."))
    {
        return;
    }
    let url = host.and_then(|h| endpoint_url(h, scheme, port));
    let status = if disabled {
        EndpointStatus::Disabled
    } else if host.is_none() {
        EndpointStatus::MissingHostname
    } else if host.is_some_and(|h| h.contains('*')) {
        EndpointStatus::Wildcard
    } else if url.is_none() {
        EndpointStatus::InvalidHostname
    } else {
        EndpointStatus::Ready
    };
    out.push(WebsiteEndpoint {
        resource_id: normalize_arm_id(&resource.id),
        resource_name: resource.name.clone(),
        source: source.to_owned(),
        hostname: host.map(str::to_owned),
        url,
        status,
    });
}

pub fn discover(resources: &[Resource], evidence: &[WebsiteEvidence]) -> Vec<WebsiteEndpoint> {
    let mut out = Vec::new();
    for resource in resources {
        let p = resource.properties.as_ref().unwrap_or(&Value::Null);
        match resource.azure_type.to_ascii_lowercase().as_str() {
            "microsoft.web/staticsites" => {
                add(
                    &mut out,
                    resource,
                    "default",
                    string(p, "defaultHostname"),
                    "https",
                    None,
                    false,
                );
                for domain in array(p, "customDomains") {
                    add(
                        &mut out,
                        resource,
                        "custom",
                        domain
                            .as_str()
                            .or_else(|| string(&domain, "name"))
                            .or_else(|| string(&domain, "hostName")),
                        "https",
                        None,
                        false,
                    );
                }
            }
            "microsoft.web/sites" | "microsoft.web/sites/slots" => {
                let disabled = string(p, "state")
                    .is_some_and(|s| s.eq_ignore_ascii_case("stopped"))
                    || p.get("enabled") == Some(&Value::Bool(false));
                add(
                    &mut out,
                    resource,
                    "default",
                    string(p, "defaultHostName"),
                    "https",
                    None,
                    disabled,
                );
                for host in array(p, "hostNames")
                    .into_iter()
                    .chain(array(p, "enabledHostNames"))
                {
                    add(
                        &mut out,
                        resource,
                        "binding",
                        host.as_str(),
                        "https",
                        None,
                        disabled,
                    );
                }
            }
            "microsoft.network/applicationgateways" => {
                let ports: BTreeMap<_, _> = array(p, "frontendPorts")
                    .into_iter()
                    .filter_map(|v| {
                        Some((
                            normalize_arm_id(string(&v, "id")?),
                            properties(&v).get("port")?.as_u64()?,
                        ))
                    })
                    .collect();
                let listeners = array(p, "httpListeners");
                if listeners.is_empty() {
                    add(&mut out, resource, "listener", None, "https", None, false);
                }
                for listener in listeners {
                    let lp = properties(&listener);
                    let scheme = string(lp, "protocol")
                        .unwrap_or("Https")
                        .to_ascii_lowercase();
                    let port = lp
                        .pointer("/frontendPort/id")
                        .and_then(Value::as_str)
                        .and_then(|id| ports.get(&normalize_arm_id(id)))
                        .and_then(|n| u16::try_from(*n).ok());
                    let source = string(&listener, "name").unwrap_or("listener");
                    let mut hosts = array(lp, "hostNames");
                    if let Some(host) = string(lp, "hostName") {
                        hosts.push(Value::String(host.to_owned()));
                    }
                    if hosts.is_empty() {
                        hosts.push(Value::Null);
                    }
                    for host in hosts {
                        add(
                            &mut out,
                            resource,
                            source,
                            host.as_str(),
                            &scheme,
                            port,
                            string(p, "operationalState")
                                .is_some_and(|s| s.eq_ignore_ascii_case("stopped")),
                        );
                        if port.is_none()
                            && let Some(endpoint) = out.last_mut()
                            && endpoint.status == EndpointStatus::Ready
                        {
                            endpoint.status = EndpointStatus::MissingEvidence;
                            endpoint.url = None;
                        }
                    }
                }
            }
            "microsoft.network/frontdoors" => {
                let frontends = array(p, "frontendEndpoints");
                if frontends.is_empty() {
                    add(&mut out, resource, "frontend", None, "https", None, false);
                }
                for frontend in frontends {
                    let id = string(&frontend, "id").map(normalize_arm_id);
                    let routes: Vec<_> = array(p, "routingRules")
                        .into_iter()
                        .filter(|rule| {
                            array(properties(rule), "frontendEndpoints")
                                .iter()
                                .any(|r| string(r, "id").map(normalize_arm_id) == id)
                        })
                        .collect();
                    let enabled: Vec<_> = routes
                        .iter()
                        .filter(|r| string(properties(r), "enabledState") != Some("Disabled"))
                        .collect();
                    let http_only = !enabled.is_empty()
                        && enabled.iter().all(|r| {
                            !array(properties(r), "acceptedProtocols")
                                .iter()
                                .any(|v| v.as_str() == Some("Https"))
                        });
                    add(
                        &mut out,
                        resource,
                        "frontend",
                        string(properties(&frontend), "hostName"),
                        if http_only { "http" } else { "https" },
                        None,
                        !routes.is_empty() && enabled.is_empty(),
                    );
                }
            }
            "microsoft.cdn/profiles/afdendpoints" => {
                let routes = evidence.iter().find(|e| {
                    normalize_arm_id(&e.resource_id) == normalize_arm_id(&resource.id)
                        && e.kind == "routes"
                        && e.error.is_none()
                });
                let enabled: Vec<_> = routes
                    .into_iter()
                    .flat_map(|e| &e.rows)
                    .filter(|r| string(properties(r), "enabledState") != Some("Disabled"))
                    .collect();
                let default_routes: Vec<_> = enabled
                    .iter()
                    .filter(|r| string(properties(r), "linkToDefaultDomain") != Some("Disabled"))
                    .collect();
                let disabled = string(p, "enabledState") == Some("Disabled")
                    || (routes.is_some() && default_routes.is_empty());
                let http_only = !default_routes.is_empty()
                    && default_routes.iter().all(|r| {
                        !array(properties(r), "supportedProtocols")
                            .iter()
                            .any(|v| v.as_str() == Some("Https"))
                    });
                add(
                    &mut out,
                    resource,
                    "default",
                    string(p, "hostName"),
                    if http_only { "http" } else { "https" },
                    None,
                    disabled,
                );
            }
            "microsoft.cdn/profiles" if is_front_door(resource) => {
                let domains = evidence.iter().find(|e| {
                    normalize_arm_id(&e.resource_id) == normalize_arm_id(&resource.id)
                        && e.kind == "domains"
                        && e.error.is_none()
                });
                if let Some(domains) = domains {
                    for domain in &domains.rows {
                        let domain_id = string(domain, "id").map(normalize_arm_id);
                        let profile_prefix =
                            format!("{}/afdendpoints/", normalize_arm_id(&resource.id));
                        let route_evidence: Vec<_> = evidence
                            .iter()
                            .filter(|e| {
                                e.kind == "routes"
                                    && normalize_arm_id(&e.resource_id).starts_with(&profile_prefix)
                            })
                            .collect();
                        let routes: Vec<_> = route_evidence
                            .iter()
                            .flat_map(|e| &e.rows)
                            .filter(|r| {
                                array(properties(r), "customDomains")
                                    .iter()
                                    .any(|d| string(d, "id").map(normalize_arm_id) == domain_id)
                            })
                            .collect();
                        let active: Vec<_> = routes
                            .iter()
                            .filter(|r| string(properties(r), "enabledState") != Some("Disabled"))
                            .collect();
                        let disabled = !route_evidence.is_empty()
                            && route_evidence.iter().all(|e| e.error.is_none())
                            && active.is_empty();
                        let http_only = !active.is_empty()
                            && active.iter().all(|r| {
                                !array(properties(r), "supportedProtocols")
                                    .iter()
                                    .any(|v| v.as_str() == Some("Https"))
                            });
                        add(
                            &mut out,
                            resource,
                            "custom",
                            string(properties(domain), "hostName"),
                            if http_only { "http" } else { "https" },
                            None,
                            disabled,
                        );
                        // The profile owns the domain; each routed endpoint also
                        // owns an association to the same deduplicated capture.
                        for route_set in &route_evidence {
                            if route_set.rows.iter().any(|route| {
                                array(properties(route), "customDomains")
                                    .iter()
                                    .any(|d| string(d, "id").map(normalize_arm_id) == domain_id)
                            }) && let Some(endpoint) = resources.iter().find(|r| {
                                normalize_arm_id(&r.id) == normalize_arm_id(&route_set.resource_id)
                            }) {
                                add(
                                    &mut out,
                                    endpoint,
                                    "custom",
                                    string(properties(domain), "hostName"),
                                    if http_only { "http" } else { "https" },
                                    None,
                                    disabled
                                        || string(
                                            endpoint.properties.as_ref().unwrap_or(&Value::Null),
                                            "enabledState",
                                        ) == Some("Disabled"),
                                );
                            }
                        }
                    }
                } else {
                    out.push(WebsiteEndpoint {
                        resource_id: normalize_arm_id(&resource.id),
                        resource_name: resource.name.clone(),
                        source: "domains".into(),
                        hostname: None,
                        url: None,
                        status: EndpointStatus::MissingEvidence,
                    });
                }
            }
            _ => {}
        }
    }
    for e in evidence.iter().filter(|e| e.error.is_some()) {
        if let Some(r) = resources
            .iter()
            .find(|r| normalize_arm_id(&r.id) == normalize_arm_id(&e.resource_id))
        {
            out.push(WebsiteEndpoint {
                resource_id: normalize_arm_id(&r.id),
                resource_name: r.name.clone(),
                source: e.kind.clone(),
                hostname: None,
                url: None,
                status: EndpointStatus::MissingEvidence,
            });
        }
    }
    out.sort_by(|a, b| {
        (&a.resource_id, &a.url, &a.source, &a.hostname).cmp(&(
            &b.resource_id,
            &b.url,
            &b.source,
            &b.hostname,
        ))
    });
    out.dedup();
    out
}

fn is_front_door(resource: &Resource) -> bool {
    resource
        .sku
        .as_ref()
        .and_then(|s| string(s, "name"))
        .is_some_and(|s| s.to_ascii_lowercase().contains("azurefrontdoor"))
}

#[derive(Debug, thiserror::Error)]
pub enum EnrichmentError {
    #[error("management authentication failed: {0}")]
    Auth(#[from] crate::error::AuthError),
    #[error("management request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("management response was invalid: {0}")]
    Invalid(String),
}

/// Redirects are disabled and every pagination URL is checked before a bearer
/// token is attached. Website requests never use this client or token provider.
pub struct WebsiteManagement<P> {
    client: reqwest::Client,
    provider: P,
    base: Url,
}

impl<P: TokenProvider> WebsiteManagement<P> {
    pub fn new(provider: P, cloud: crate::cloud::Cloud) -> Result<Self, EnrichmentError> {
        Self::with_base(
            provider,
            Url::parse(&format!("{}/", cloud.endpoints().arm))
                .expect("static management URLs are valid"),
        )
    }

    fn with_base(provider: P, base: Url) -> Result<Self, EnrichmentError> {
        Ok(Self {
            provider,
            base,
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(30))
                .build()?,
        })
    }

    async fn list(&self, resource_id: &str, kind: &str) -> Result<Vec<Value>, EnrichmentError> {
        let suffix = if kind == "domains" {
            "customDomains"
        } else {
            "routes"
        };
        let path = format!("{resource_id}/{suffix}?api-version=2025-04-15");
        let mut next = Some(
            self.base
                .join(&path)
                .map_err(|e| EnrichmentError::Invalid(e.to_string()))?,
        );
        let mut seen = BTreeSet::new();
        let mut rows = Vec::new();
        while let Some(url) = next.take() {
            if url.origin() != self.base.origin()
                || !url.username().is_empty()
                || url.password().is_some()
                || !url
                    .path()
                    .to_ascii_lowercase()
                    .starts_with(&format!("{resource_id}/").to_ascii_lowercase())
                || !seen.insert(url.to_string())
            {
                return Err(EnrichmentError::Invalid(
                    "unsafe or repeated pagination URL".into(),
                ));
            }
            let mut attempt = 0;
            let response = loop {
                let response = self
                    .client
                    .get(url.clone())
                    .bearer_auth(self.provider.token().await?)
                    .send()
                    .await?;
                if (response.status().as_u16() == 429 || response.status().is_server_error())
                    && attempt < 3
                {
                    let delay = response
                        .headers()
                        .get("retry-after")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.parse::<u64>().ok())
                        .unwrap_or(1 << attempt)
                        .min(30);
                    tokio::time::sleep(Duration::from_secs(delay)).await;
                    attempt += 1;
                } else {
                    break response.error_for_status()?;
                }
            };
            let body: Value = response.json().await?;
            rows.extend(
                body.get("value")
                    .and_then(Value::as_array)
                    .ok_or_else(|| EnrichmentError::Invalid("missing value array".into()))?
                    .iter()
                    .cloned(),
            );
            next = string(&body, "nextLink")
                .map(|s| {
                    self.base
                        .join(s)
                        .map_err(|e| EnrichmentError::Invalid(e.to_string()))
                })
                .transpose()?;
        }
        rows.sort_by(|a, b| string(a, "id").cmp(&string(b, "id")));
        Ok(rows)
    }

    pub async fn enrich(&self, resources: &[Resource]) -> Vec<WebsiteEvidence> {
        let mut evidence = Vec::new();
        for r in resources {
            let kind = match r.azure_type.as_str() {
                "microsoft.cdn/profiles" if is_front_door(r) => "domains",
                "microsoft.cdn/profiles/afdendpoints" => "routes",
                _ => continue,
            };
            let result = self.list(&r.id, kind).await;
            let (rows, error) = match result {
                Ok(rows) => (rows, None),
                Err(e) => (vec![], Some(e.to_string())),
            };
            evidence.push(WebsiteEvidence {
                resource_id: normalize_arm_id(&r.id),
                kind: kind.into(),
                collected_at: chrono::Utc::now().to_rfc3339(),
                rows,
                error,
            });
        }
        evidence
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::StaticTokenProvider;
    use serde_json::json;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{header, method, path},
    };

    fn resource(kind: &str, props: Value) -> Resource {
        crate::collect::ingest::resource_from_row(&json!({"id":"/subscriptions/S/resourceGroups/R/providers/X/thing","name":"website","type":kind,"subscriptionId":"s","properties":props})).unwrap()
    }
    #[test]
    fn static_sites_discover_default_and_custom_domains_with_normalized_urls() {
        let r = resource(
            "Microsoft.Web/staticSites",
            json!({"defaultHostname":"APP.AzureStaticApps.NET", "customDomains":["WWW.Example.COM",{"name":"other.example.com"}]}),
        );
        let entries = discover(&[r], &[]);
        assert_eq!(
            entries
                .iter()
                .filter_map(|e| e.url.as_deref())
                .collect::<Vec<_>>(),
            vec![
                "https://app.azurestaticapps.net/",
                "https://other.example.com/",
                "https://www.example.com/"
            ]
        );
        assert!(
            entries
                .iter()
                .all(|e| e.resource_id == e.resource_id.to_lowercase())
        );
    }
    #[test]
    fn app_services_exclude_scm_but_preserve_shared_resource_associations() {
        let r = resource(
            "microsoft.web/sites",
            json!({"defaultHostName":"shared.test","hostNames":["shared.test","app.scm.azurewebsites.net"],"enabledHostNames":["shared.test"]}),
        );
        let mut other = r.clone();
        other.id = "/other".into();
        let entries = discover(&[r, other], &[]);
        assert!(
            entries
                .iter()
                .all(|e| e.url.as_deref() == Some("https://shared.test/"))
        );
        assert_eq!(
            entries
                .iter()
                .map(|e| &e.resource_id)
                .collect::<BTreeSet<_>>()
                .len(),
            2
        );
    }
    #[test]
    fn stopped_sites_remain_visible_but_are_not_capture_candidates() {
        let r = resource(
            "microsoft.web/sites",
            json!({"defaultHostName":"stopped.test","state":"Stopped"}),
        );
        assert_eq!(discover(&[r], &[])[0].status, EndpointStatus::Disabled);
    }
    #[test]
    fn gateway_listeners_use_real_ports_and_record_unnamed_and_wildcard_hosts() {
        let r = resource(
            "microsoft.network/applicationgateways",
            json!({"frontendPorts":[{"id":"/PORT","properties":{"port":8080}}],"httpListeners":[{"name":"a","properties":{"hostNames":["internal.test","*.test"],"protocol":"Http","frontendPort":{"id":"/port"}}},{"name":"basic","properties":{}}]}),
        );
        let entries = discover(&[r], &[]);
        assert!(
            entries
                .iter()
                .any(|e| e.url.as_deref() == Some("http://internal.test:8080/"))
        );
        assert!(entries.iter().any(|e| e.status == EndpointStatus::Wildcard));
        assert!(
            entries
                .iter()
                .any(|e| e.status == EndpointStatus::MissingHostname)
        );
    }
    #[test]
    fn unit_gateway_records_missing_evidence_when_listener_port_is_unknown() {
        let r = resource(
            "microsoft.network/applicationgateways",
            json!({"httpListeners":[{"properties":{"hostName":"internal.test","protocol":"Https","frontendPort":{"id":"/missing"}}}]}),
        );
        let entries = discover(&[r], &[]);
        assert_eq!(entries[0].status, EndpointStatus::MissingEvidence);
        assert_eq!(entries[0].url, None);
    }

    #[test]
    fn unit_gateway_disables_listeners_when_operational_state_is_stopped() {
        let r = resource(
            "microsoft.network/applicationgateways",
            json!({"operationalState":"Stopped","frontendPorts":[{"id":"/port","properties":{"port":443}}],"httpListeners":[{"properties":{"hostName":"internal.test","frontendPort":{"id":"/port"}}}]}),
        );
        assert_eq!(discover(&[r], &[])[0].status, EndpointStatus::Disabled);
    }

    #[test]
    fn unit_static_sites_keep_scm_named_domains_when_they_are_not_kudu_endpoints() {
        let r = resource(
            "microsoft.web/staticsites",
            json!({"defaultHostname":"site.scm.example.com"}),
        );
        let entries = discover(&[r], &[]);
        assert_eq!(entries[0].status, EndpointStatus::Ready);
        assert_eq!(
            entries[0].url.as_deref(),
            Some("https://site.scm.example.com/")
        );
    }

    #[test]
    fn front_door_classic_uses_frontend_protocols() {
        let r = resource(
            "microsoft.network/frontdoors",
            json!({"frontendEndpoints":[{"id":"/FRONT","properties":{"hostName":"classic.test"}}],"routingRules":[{"properties":{"frontendEndpoints":[{"id":"/front"}],"acceptedProtocols":["Http"],"enabledState":"Enabled"}}]}),
        );
        assert_eq!(
            discover(&[r], &[])[0].url.as_deref(),
            Some("http://classic.test/")
        );
    }
    #[test]
    fn front_door_custom_domains_use_stored_route_evidence() {
        let mut profile = resource("microsoft.cdn/profiles", json!({}));
        profile.id = "/profile".into();
        profile.sku = Some(json!({"name":"Premium_AzureFrontDoor"}));
        let mut endpoint = resource(
            "microsoft.cdn/profiles/afdendpoints",
            json!({"hostName":"default.azurefd.net"}),
        );
        endpoint.id = "/profile/afdendpoints/one".into();
        let evidence = vec![
            WebsiteEvidence {
                resource_id: profile.id.clone(),
                kind: "domains".into(),
                collected_at: "now".into(),
                rows: vec![json!({"id":"/domain","properties":{"hostName":"custom.test"}})],
                error: None,
            },
            WebsiteEvidence {
                resource_id: endpoint.id.clone(),
                kind: "routes".into(),
                collected_at: "now".into(),
                rows: vec![
                    json!({"properties":{"customDomains":[{"id":"/DOMAIN"}],"supportedProtocols":["Http"],"enabledState":"Enabled","linkToDefaultDomain":"Disabled"}}),
                ],
                error: None,
            },
        ];
        let entries = discover(&[profile, endpoint], &evidence);
        assert!(
            entries
                .iter()
                .any(|e| e.url.as_deref() == Some("http://custom.test/")
                    && e.status == EndpointStatus::Ready)
        );
        assert!(
            entries
                .iter()
                .any(|e| e.url.as_deref() == Some("https://default.azurefd.net/")
                    && e.status == EndpointStatus::Disabled)
        );
    }
    #[test]
    fn missing_front_door_enrichment_is_not_an_empty_domain_list() {
        let mut profile = resource("microsoft.cdn/profiles", json!({}));
        profile.sku = Some(json!({"name":"Standard_AzureFrontDoor"}));
        assert_eq!(
            discover(&[profile], &[])[0].status,
            EndpointStatus::MissingEvidence
        );
    }
    #[test]
    fn malformed_hostnames_cannot_inject_navigation_paths_or_credentials() {
        for host in [
            "example.test/path",
            "user@example.test",
            "example.test?token=x",
            "example.test:443",
            "*.test",
            "",
            "https://example.test",
            "example.test\\path",
        ] {
            assert!(endpoint_url(host, "https", None).is_none(), "{host}");
        }
    }
    #[tokio::test]
    async fn management_follows_pagination_with_bearer_only_on_same_origin() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/profile/customDomains"))
            .and(header("authorization", "Bearer test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                json!({"value":[{"id":"/b"}],"nextLink":format!("{}/profile/page2",server.uri())}),
            ))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(path("/profile/page2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value":[{"id":"/a"}]})))
            .expect(1)
            .mount(&server)
            .await;
        let client = WebsiteManagement::with_base(
            StaticTokenProvider("test-token".into()),
            Url::parse(&format!("{}/", server.uri())).unwrap(),
        )
        .unwrap();
        assert_eq!(
            client.list("/profile", "domains").await.unwrap(),
            vec![json!({"id":"/a"}), json!({"id":"/b"})]
        );
    }
    #[tokio::test]
    async fn management_never_sends_credentials_to_a_pagination_target_on_another_origin() {
        let server = MockServer::start().await;
        let other = MockServer::start().await;
        Mock::given(path("/profile/customDomains"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                json!({"value":[],"nextLink":format!("{}/profile/stolen",other.uri())}),
            ))
            .mount(&server)
            .await;
        let client = WebsiteManagement::with_base(
            StaticTokenProvider("test-token".into()),
            Url::parse(&server.uri()).unwrap(),
        )
        .unwrap();
        assert!(client.list("/profile", "domains").await.is_err());
        assert!(other.received_requests().await.unwrap().is_empty());
    }
    #[tokio::test]
    async fn management_bounds_throttling_and_records_permission_failures() {
        let server = MockServer::start().await;
        Mock::given(path("/profile/customDomains"))
            .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "0"))
            .expect(4)
            .mount(&server)
            .await;
        let client = WebsiteManagement::with_base(
            StaticTokenProvider("test-token".into()),
            Url::parse(&server.uri()).unwrap(),
        )
        .unwrap();
        assert!(client.list("/profile", "domains").await.is_err());
        server.reset().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&server)
            .await;
        let mut profile = resource("microsoft.cdn/profiles", json!({}));
        profile.sku = Some(json!({"name":"Standard_AzureFrontDoor"}));
        let evidence = client.enrich(&[profile]).await;
        assert!(evidence[0].error.as_deref().unwrap().contains("403"));
    }
}

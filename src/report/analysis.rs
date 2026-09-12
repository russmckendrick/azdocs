//! Snapshot interpretation for print reports. No renderer infers relationships,
//! priorities or audit success, and no current configuration rewrites history.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::{
    Edge, EdgeKind, Finding, QueryRun, Resource, ResourceGroup, Severity, Subscription,
    normalize_arm_id,
};

use super::details::group_key;

#[derive(Debug, Default)]
pub struct ReportAnalysis {
    pub resources: BTreeMap<String, Resource>,
    pub subscriptions: BTreeMap<String, String>,
    pub groups: Vec<GroupProfile>,
    pub issues: Vec<Issue>,
    pub relationships: Vec<Relationship>,
    pub edges: Vec<Edge>,
    pub query_runs: Vec<QueryRun>,
    pub recorded_queries: BTreeMap<String, Vec<serde_json::Value>>,
    pub unresolved_relationships: usize,
}

#[derive(Debug)]
pub struct GroupProfile {
    pub key: String,
    pub subscription_id: String,
    pub name: Option<String>,
    pub resource_ids: BTreeSet<String>,
    pub severities: [usize; 4],
    pub cross_group_connections: usize,
    pub study_reason: Option<StudyReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudyReason {
    Findings,
    Connections,
    Population,
}

#[derive(Debug)]
pub struct Issue {
    pub query_name: String,
    pub occurrences: Vec<Finding>,
    pub resource_ids: BTreeSet<String>,
    pub severities: [usize; 4],
    pub subscriptions: BTreeMap<String, BTreeSet<String>>,
    pub groups: BTreeMap<String, BTreeSet<String>>,
    pub unresolved: usize,
    pub estate_level: usize,
}

impl Issue {
    pub fn severity(&self) -> &'static str {
        ["high", "medium", "low", "info"]
            .into_iter()
            .zip(self.severities)
            .find(|(_, count)| *count > 0)
            .map_or("info", |(key, _)| key)
    }
}

#[derive(Debug, Clone)]
pub struct Relationship {
    pub source: String,
    pub target: String,
    pub kind: EdgeKind,
    pub source_group: Option<String>,
    pub target_group: Option<String>,
}

impl Relationship {
    pub fn crosses_group(&self) -> bool {
        matches!((&self.source_group, &self.target_group), (Some(a), Some(b)) if a != b)
    }
}

/// Group-less resources have a stable machine key; their display label is
/// supplied later by the caller's labels, just like other synthetic names.
pub fn resource_group_key(resource: &Resource) -> String {
    group_key(
        &resource.subscription_id,
        resource.resource_group.as_deref().unwrap_or(""),
    )
}

fn severity_index(severity: Severity) -> usize {
    match severity {
        Severity::High => 0,
        Severity::Medium => 1,
        Severity::Low => 2,
        Severity::Info => 3,
    }
}

fn endpoint_resource<'a>(
    id: &str,
    resources: &'a BTreeMap<String, Resource>,
) -> Option<&'a Resource> {
    resources.get(id).or_else(|| {
        // Subnets are embedded properties, not resource rows. Resolve their
        // owning VNet without treating an arbitrary missing child as present.
        let (parent, _) = id.split_once("/subnets/")?;
        resources
            .get(parent)
            .filter(|r| r.azure_type == "microsoft.network/virtualnetworks")
    })
}

impl ReportAnalysis {
    pub fn build(
        resources: &[Resource],
        subscriptions: &[Subscription],
        groups: &[ResourceGroup],
        findings: &[Finding],
        edges: &[Edge],
        query_runs: Vec<QueryRun>,
    ) -> Self {
        let resources: BTreeMap<String, Resource> = resources
            .iter()
            .cloned()
            .map(|mut resource| {
                resource.id = normalize_arm_id(&resource.id);
                resource.subscription_id = normalize_arm_id(&resource.subscription_id);
                resource.azure_type = resource.azure_type.to_lowercase();
                resource.resource_group = resource.resource_group.map(|g| g.to_lowercase());
                (resource.id.clone(), resource)
            })
            .collect();
        let mut subscription_names: BTreeMap<String, String> = subscriptions
            .iter()
            .map(|s| (normalize_arm_id(&s.subscription_id), s.display_name.clone()))
            .collect();
        let mut profiles: BTreeMap<String, GroupProfile> = groups
            .iter()
            .map(|g| {
                let key = group_key(&g.subscription_id, &g.name);
                (
                    key.clone(),
                    GroupProfile {
                        key,
                        subscription_id: normalize_arm_id(&g.subscription_id),
                        name: Some(g.name.clone()),
                        resource_ids: BTreeSet::new(),
                        severities: [0; 4],
                        cross_group_connections: 0,
                        study_reason: None,
                    },
                )
            })
            .collect();
        for resource in resources.values() {
            subscription_names
                .entry(resource.subscription_id.clone())
                .or_insert_with(|| resource.subscription_id.clone());
            let key = resource_group_key(resource);
            profiles
                .entry(key.clone())
                .or_insert_with(|| GroupProfile {
                    key,
                    subscription_id: resource.subscription_id.clone(),
                    name: resource.resource_group.clone(),
                    resource_ids: BTreeSet::new(),
                    severities: [0; 4],
                    cross_group_connections: 0,
                    study_reason: None,
                })
                .resource_ids
                .insert(resource.id.clone());
        }

        let mut issues: BTreeMap<String, Issue> = BTreeMap::new();
        for finding in findings {
            let mut finding = finding.clone();
            finding.resource_id = finding.resource_id.map(|id| normalize_arm_id(&id));
            let issue = issues
                .entry(finding.query_name.clone())
                .or_insert_with(|| Issue {
                    query_name: finding.query_name.clone(),
                    occurrences: Vec::new(),
                    resource_ids: BTreeSet::new(),
                    severities: [0; 4],
                    subscriptions: BTreeMap::new(),
                    groups: BTreeMap::new(),
                    unresolved: 0,
                    estate_level: 0,
                });
            issue.severities[severity_index(finding.severity)] += 1;
            if let Some(id) = &finding.resource_id {
                issue.resource_ids.insert(id.clone());
                if let Some(resource) = resources.get(id) {
                    let key = resource_group_key(resource);
                    issue
                        .subscriptions
                        .entry(resource.subscription_id.clone())
                        .or_default()
                        .insert(id.clone());
                    issue
                        .groups
                        .entry(key.clone())
                        .or_default()
                        .insert(id.clone());
                    if let Some(profile) = profiles.get_mut(&key) {
                        profile.severities[severity_index(finding.severity)] += 1;
                    }
                } else {
                    issue.unresolved += 1;
                }
            } else {
                issue.estate_level += 1;
            }
            issue.occurrences.push(finding);
        }
        let mut issues: Vec<_> = issues.into_values().collect();
        for issue in &mut issues {
            issue.occurrences.sort_by(|a, b| {
                (a.severity, &a.resource_id, &a.title)
                    .cmp(&(b.severity, &b.resource_id, &b.title))
                    .then_with(|| {
                        a.detail
                            .as_ref()
                            .map(serde_json::Value::to_string)
                            .cmp(&b.detail.as_ref().map(serde_json::Value::to_string))
                    })
            });
        }
        issues.sort_by(|a, b| {
            a.severities
                .iter()
                .position(|n| *n > 0)
                .cmp(&b.severities.iter().position(|n| *n > 0))
                .then(b.resource_ids.len().cmp(&a.resource_ids.len()))
                .then(a.query_name.cmp(&b.query_name))
        });

        let mut seen = BTreeSet::new();
        let mut relationships = Vec::new();
        let mut unresolved_relationships = 0;
        for edge in edges {
            if matches!(
                edge.kind,
                EdgeKind::SubnetOf | EdgeKind::InVnet | EdgeKind::NicInSubnet
            ) {
                continue;
            }
            let mut source = normalize_arm_id(&edge.source_id);
            let mut target = normalize_arm_id(&edge.target_id);
            // Peering is commonly collected in both directions; it is one
            // connection. Directed service dependencies retain direction.
            if edge.kind == EdgeKind::PeeredWith && source > target {
                std::mem::swap(&mut source, &mut target);
            }
            if !seen.insert((source.clone(), target.clone(), edge.kind)) {
                continue;
            }
            let source_resource = endpoint_resource(&source, &resources);
            let target_resource = endpoint_resource(&target, &resources);
            if source_resource.is_none() || target_resource.is_none() {
                unresolved_relationships += 1;
            }
            let source_group = source_resource.map(resource_group_key);
            let target_group = target_resource.map(resource_group_key);
            let relation = Relationship {
                source,
                target,
                kind: edge.kind,
                source_group,
                target_group,
            };
            if relation.crosses_group() {
                for key in [&relation.source_group, &relation.target_group]
                    .into_iter()
                    .flatten()
                {
                    if let Some(profile) = profiles.get_mut(key) {
                        profile.cross_group_connections += 1;
                    }
                }
            }
            relationships.push(relation);
        }
        relationships
            .sort_by(|a, b| (a.kind, &a.source, &a.target).cmp(&(b.kind, &b.source, &b.target)));
        let mut groups: Vec<_> = profiles.into_values().collect();
        select_studies(&mut groups, subscription_names.keys());
        let mut original_edges = edges.to_vec();
        for edge in &mut original_edges {
            edge.source_id = normalize_arm_id(&edge.source_id);
            edge.target_id = normalize_arm_id(&edge.target_id);
        }
        original_edges.sort_by(|a, b| {
            (&a.source_id, &a.target_id, a.kind).cmp(&(&b.source_id, &b.target_id, b.kind))
        });
        Self {
            edges: original_edges,
            resources,
            subscriptions: subscription_names,
            groups,
            issues,
            relationships,
            query_runs,
            recorded_queries: BTreeMap::new(),
            unresolved_relationships,
        }
    }

    pub fn group_resources<'a>(
        &'a self,
        group: &'a GroupProfile,
    ) -> impl Iterator<Item = &'a Resource> {
        group
            .resource_ids
            .iter()
            .filter_map(|id| self.resources.get(id))
    }

    pub fn display_name<'a>(&'a self, id: &'a str) -> &'a str {
        self.resources
            .get(id)
            .map_or_else(|| crate::model::short_name(id), |r| r.name.as_str())
    }

    pub fn studies(&self) -> impl Iterator<Item = &GroupProfile> {
        self.groups.iter().filter(|g| g.study_reason.is_some())
    }
}

fn select_studies<'a>(
    groups: &mut [GroupProfile],
    subscriptions: impl Iterator<Item = &'a String>,
) {
    for subscription in subscriptions {
        for reason in [
            StudyReason::Findings,
            StudyReason::Connections,
            StudyReason::Population,
        ] {
            let selected = groups
                .iter()
                .enumerate()
                .filter(|(_, g)| {
                    &g.subscription_id == subscription
                        && !g.resource_ids.is_empty()
                        && g.study_reason.is_none()
                })
                .filter(|(_, g)| match reason {
                    StudyReason::Findings => g.severities.iter().sum::<usize>() > 0,
                    StudyReason::Connections => g.cross_group_connections > 0,
                    StudyReason::Population => true,
                })
                .min_by(|(_, a), (_, b)| {
                    let order = match reason {
                        StudyReason::Findings => b.severities.cmp(&a.severities),
                        StudyReason::Connections => {
                            b.cross_group_connections.cmp(&a.cross_group_connections)
                        }
                        StudyReason::Population => b.resource_ids.len().cmp(&a.resource_ids.len()),
                    };
                    order.then(a.name.cmp(&b.name)).then(a.key.cmp(&b.key))
                })
                .map(|(index, _)| index);
            if let Some(index) = selected {
                groups[index].study_reason = Some(reason);
            }
        }
    }
}

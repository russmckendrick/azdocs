//! View-ready topology graphs for the desktop explorer, built in Rust so the
//! semantics (folding, aggregation, honest counts) are tested once and the
//! frontend only renders. Everything is a pure function over model types —
//! the Tauri command fetches from the store and calls in here.
//!
//! The contract with the renderer: every resource in scope is *represented* —
//! drawn as its own node, folded into a host node (NICs/disks into their VM,
//! child resources into their parent), or aggregated into a ×N tile — and the
//! response carries the arithmetic (`counts`) so the UI can say so honestly.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use azdocs::model::{Edge, EdgeKind, Resource, ResourceGroup, Subscription, azure_types, network};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// How many resource-group cards the estate view draws before whole
/// subscriptions collapse into expandable lane bars.
const ESTATE_CARD_BUDGET: usize = 24;
/// Same-type neighbours beyond this fold into one ×N node in a neighbourhood.
const NEIGHBOUR_FANOUT_LIMIT: usize = 6;

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(optional_fields = nullable)]
pub struct TopologyRequest {
    pub snapshot_id: Option<String>,
    pub mode: TopologyMode,
    #[serde(default)]
    pub scope: TopologyScope,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
#[ts(optional_fields = nullable)]
pub enum TopologyMode {
    #[serde(rename_all = "camelCase")]
    Estate {
        /// Subscription ids the user has expanded; empty means "auto".
        #[serde(default)]
        expanded_subscriptions: Vec<String>,
    },
    #[serde(rename_all = "camelCase")]
    Group { group_id: String },
    #[serde(rename_all = "camelCase")]
    Neighbourhood {
        resource_id: String,
        #[serde(default = "default_depth")]
        depth: u32,
        /// Edge-kind classes to include; empty means all.
        #[serde(default)]
        kind_classes: Vec<String>,
    },
}

fn default_depth() -> u32 {
    1
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(optional_fields = nullable)]
pub struct TopologyScope {
    /// Subscription ids to include; empty means all.
    #[serde(default)]
    pub subscriptions: Vec<String>,
    /// Azure types to include; empty means all.
    #[serde(default)]
    pub azure_types: Vec<String>,
    #[serde(default = "default_true")]
    pub show_unconnected: bool,
}

impl Default for TopologyScope {
    fn default() -> Self {
        Self {
            subscriptions: Vec::new(),
            azure_types: Vec::new(),
            show_unconnected: true,
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "TopologyGraph", optional_fields = nullable)]
pub struct TopologyGraphDto {
    #[ts(type = "TopologyLevel")]
    pub level: String,
    pub lanes: Vec<LaneDto>,
    pub nodes: Vec<TopologyNodeDto>,
    pub links: Vec<TopologyLinkDto>,
    pub kind_classes: Vec<KindClassCountDto>,
    pub counts: TopologyCountsDto,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "TopologyLane", optional_fields = nullable)]
pub struct LaneDto {
    pub subscription_id: String,
    pub name: String,
    pub expanded: bool,
    pub group_count: usize,
    pub resource_count: usize,
    pub finding_count: usize,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "TopologyNode", optional_fields = nullable)]
pub struct TopologyNodeDto {
    pub id: String,
    /// resource | resource-group | subscription | vnet | subnet | aggregate
    #[ts(type = "TopologyNodeKind")]
    pub kind: String,
    pub name: String,
    pub subtitle: String,
    pub azure_type: Option<String>,
    /// Subscription id this node belongs to (estate mode lanes).
    pub lane: Option<String>,
    /// Containment: a subnet's vnet, a placed resource's subnet.
    pub parent_id: Option<String>,
    /// core | unconnected — which shelf the group view lays the node in.
    #[ts(optional = nullable, type = "TopologyZone")]
    pub zone: Option<String>,
    /// BFS distance from the subject in neighbourhood mode.
    pub hop: Option<u32>,
    /// Members folded or aggregated into this node (resource ids).
    pub member_ids: Vec<String>,
    pub count: usize,
    pub finding_count: usize,
    pub resource_id: Option<String>,
    pub group_id: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "TopologyLink", optional_fields = nullable)]
pub struct TopologyLinkDto {
    pub source_id: String,
    pub target_id: String,
    pub label: String,
    #[ts(type = "KindClass")]
    pub kind_class: String,
    pub count: usize,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "KindClassCount", optional_fields = nullable)]
pub struct KindClassCountDto {
    pub class: String,
    pub count: usize,
}

#[derive(Debug, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "TopologyCounts", optional_fields = nullable)]
pub struct TopologyCountsDto {
    /// Resources (or groups, at estate level) in scope.
    pub total: usize,
    /// Drawn as their own node.
    pub drawn: usize,
    /// Folded into a host node (NIC into VM, child into parent).
    pub folded: usize,
    /// Rolled into ×N aggregate tiles or collapsed lanes.
    pub aggregated: usize,
    /// Neighbours in other groups drawn as ghost stubs (group view only).
    pub external: usize,
    /// Excluded by the request's filters (never silently — always counted).
    pub hidden_by_filter: usize,
    pub total_links: usize,
    pub drawn_links: usize,
}

pub struct TopologyInput<'a> {
    pub subscriptions: &'a [Subscription],
    pub resource_groups: &'a [ResourceGroup],
    pub resources: &'a [Resource],
    pub edges: &'a [Edge],
    pub finding_counts: &'a BTreeMap<String, usize>,
}

/// The coarse family an edge kind belongs to — what the UI filters by.
pub fn kind_class(kind: EdgeKind) -> &'static str {
    match kind {
        EdgeKind::PeeredWith
        | EdgeKind::NsgAttached
        | EdgeKind::NicInSubnet
        | EdgeKind::SubnetOf
        | EdgeKind::InVnet
        | EdgeKind::DnsLinked => "network",
        EdgeKind::AttachedTo | EdgeKind::RunsOn => "structure",
        EdgeKind::PrivateEndpointFor | EdgeKind::DependsOn => "data",
        EdgeKind::UsesIdentity => "identity",
        EdgeKind::LogsTo | EdgeKind::Monitors => "monitoring",
    }
}

fn kind_label(kind: EdgeKind) -> String {
    kind.as_str().replace('_', " ")
}

pub fn build(request: &TopologyRequest, input: &TopologyInput) -> TopologyGraphDto {
    let scoped = scope_resources(input, &request.scope);
    match &request.mode {
        TopologyMode::Estate {
            expanded_subscriptions,
        } => estate_graph(input, &scoped, &request.scope, expanded_subscriptions),
        TopologyMode::Group { group_id } => group_graph(input, &scoped, &request.scope, group_id),
        TopologyMode::Neighbourhood {
            resource_id,
            depth,
            kind_classes,
        } => neighbourhood_graph(input, &scoped, resource_id, *depth, kind_classes),
    }
}

struct ScopedResources<'a> {
    resources: Vec<&'a Resource>,
    hidden_by_filter: usize,
}

fn scope_resources<'a>(input: &TopologyInput<'a>, scope: &TopologyScope) -> ScopedResources<'a> {
    let subscriptions: HashSet<&str> = scope.subscriptions.iter().map(|s| s.as_str()).collect();
    let types: HashSet<&str> = scope.azure_types.iter().map(|s| s.as_str()).collect();
    let mut resources = Vec::new();
    let mut hidden = 0usize;
    for resource in input.resources {
        let in_subscription =
            subscriptions.is_empty() || subscriptions.contains(resource.subscription_id.as_str());
        let in_types = types.is_empty() || types.contains(resource.azure_type.as_str());
        if in_subscription && in_types {
            resources.push(resource);
        } else {
            hidden += 1;
        }
    }
    ScopedResources {
        resources,
        hidden_by_filter: hidden,
    }
}

fn group_node_id(group_id: &str) -> String {
    format!("resource-group:{group_id}")
}

fn lane_node_id(subscription_id: &str) -> String {
    format!("subscription:{subscription_id}")
}

fn finding_count(input: &TopologyInput, resource_id: &str) -> usize {
    input
        .finding_counts
        .get(resource_id)
        .copied()
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Estate: subscription lanes of resource-group cards, links bundled.
// ---------------------------------------------------------------------------

struct GroupSummary<'a> {
    id: String,
    name: String,
    subscription_id: String,
    resources: Vec<&'a Resource>,
    finding_count: usize,
    external_links: usize,
}

fn group_key(subscription_id: &str, group_name: &str) -> String {
    format!(
        "{}\0{}",
        subscription_id.to_lowercase(),
        group_name.to_lowercase()
    )
}

fn collect_groups<'a>(
    input: &TopologyInput<'a>,
    scoped: &ScopedResources<'a>,
    scope: &TopologyScope,
) -> (Vec<GroupSummary<'a>>, HashMap<String, usize>) {
    let scope_subscriptions: HashSet<&str> =
        scope.subscriptions.iter().map(|s| s.as_str()).collect();
    let mut order: Vec<GroupSummary> = Vec::new();
    let mut index_by_key: HashMap<String, usize> = HashMap::new();
    for group in input.resource_groups {
        if !scope_subscriptions.is_empty()
            && !scope_subscriptions.contains(group.subscription_id.as_str())
        {
            continue;
        }
        let key = group_key(&group.subscription_id, &group.name);
        if index_by_key.contains_key(&key) {
            continue;
        }
        index_by_key.insert(key, order.len());
        order.push(GroupSummary {
            id: group.id.clone(),
            name: group.name.clone(),
            subscription_id: group.subscription_id.clone(),
            resources: Vec::new(),
            finding_count: 0,
            external_links: 0,
        });
    }
    let mut group_by_resource: HashMap<String, usize> = HashMap::new();
    for resource in &scoped.resources {
        let name = resource
            .resource_group
            .clone()
            .unwrap_or_else(|| "subscription scope".to_owned());
        let key = group_key(&resource.subscription_id, &name);
        let index = *index_by_key.entry(key).or_insert_with(|| {
            order.push(GroupSummary {
                id: format!(
                    "/subscriptions/{}/resourcegroups/{}",
                    resource.subscription_id.to_lowercase(),
                    name.to_lowercase()
                ),
                name,
                subscription_id: resource.subscription_id.clone(),
                resources: Vec::new(),
                finding_count: 0,
                external_links: 0,
            });
            order.len() - 1
        });
        group_by_resource.insert(resource.id.clone(), index);
        let summary = &mut order[index];
        summary.finding_count += finding_count(input, &resource.id);
        summary.resources.push(resource);
    }
    (order, group_by_resource)
}

fn estate_graph(
    input: &TopologyInput,
    scoped: &ScopedResources,
    scope: &TopologyScope,
    expanded_subscriptions: &[String],
) -> TopologyGraphDto {
    let (mut groups, group_by_resource) = collect_groups(input, scoped, scope);

    // Bundle cross-group links: (source group, target group, class) → count.
    // Intra-group edges are geometry inside a card, so the link arithmetic the
    // status chip reports covers cross-group edges only.
    let mut bundles: BTreeMap<(usize, usize, &'static str), usize> = BTreeMap::new();
    let mut total_links = 0usize;
    for edge in input.edges {
        let (Some(&source), Some(&target)) = (
            group_by_resource.get(&edge.source_id),
            group_by_resource.get(&edge.target_id),
        ) else {
            continue;
        };
        if source == target {
            continue;
        }
        total_links += 1;
        groups[source].external_links += 1;
        groups[target].external_links += 1;
        *bundles
            .entry((source, target, kind_class(edge.kind)))
            .or_default() += 1;
    }

    // Lanes, sorted by subscription display name for a stable reading order.
    let subscription_names: HashMap<&str, &str> = input
        .subscriptions
        .iter()
        .map(|s| (s.subscription_id.as_str(), s.display_name.as_str()))
        .collect();
    let mut lane_ids: Vec<&str> = groups
        .iter()
        .map(|g| g.subscription_id.as_str())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    lane_ids.sort_by_key(|id| {
        (
            subscription_names.get(id).copied().unwrap_or(id).to_owned(),
            (*id).to_owned(),
        )
    });

    // Expansion: honour the explicit request, otherwise expand the busiest
    // lanes (most cross-group links, then most resources) within the budget.
    let lane_group_count = |lane: &str| groups.iter().filter(|g| g.subscription_id == lane).count();
    let expanded: HashSet<&str> = if expanded_subscriptions.is_empty() {
        let mut ranked = lane_ids.clone();
        ranked.sort_by_key(|lane| {
            let links: usize = groups
                .iter()
                .filter(|g| g.subscription_id == *lane)
                .map(|g| g.external_links)
                .sum();
            let resources: usize = groups
                .iter()
                .filter(|g| g.subscription_id == *lane)
                .map(|g| g.resources.len())
                .sum();
            (std::cmp::Reverse(links), std::cmp::Reverse(resources))
        });
        let mut budget = ESTATE_CARD_BUDGET;
        let mut chosen = HashSet::new();
        for lane in ranked {
            let cards = lane_group_count(lane);
            if chosen.is_empty() || cards <= budget {
                chosen.insert(lane);
                budget = budget.saturating_sub(cards);
            }
        }
        chosen
    } else {
        expanded_subscriptions.iter().map(|s| s.as_str()).collect()
    };

    let lanes: Vec<LaneDto> = lane_ids
        .iter()
        .map(|lane| LaneDto {
            subscription_id: (*lane).to_owned(),
            name: subscription_names
                .get(lane)
                .copied()
                .unwrap_or(lane)
                .to_owned(),
            expanded: expanded.contains(lane),
            group_count: lane_group_count(lane),
            resource_count: groups
                .iter()
                .filter(|g| g.subscription_id == *lane)
                .map(|g| g.resources.len())
                .sum(),
            finding_count: groups
                .iter()
                .filter(|g| g.subscription_id == *lane)
                .map(|g| g.finding_count)
                .sum(),
        })
        .collect();

    // Nodes: a card per group in an expanded lane; one bar per collapsed lane.
    let mut nodes = Vec::new();
    let mut drawn = 0usize;
    let mut aggregated = 0usize;
    let mut sorted_group_indices: Vec<usize> = (0..groups.len()).collect();
    sorted_group_indices.sort_by(|&a, &b| {
        groups[a]
            .name
            .cmp(&groups[b].name)
            .then(groups[a].id.cmp(&groups[b].id))
    });
    for &index in &sorted_group_indices {
        let group = &groups[index];
        if !expanded.contains(group.subscription_id.as_str()) {
            continue;
        }
        drawn += 1;
        let mut type_counts: BTreeMap<&str, usize> = BTreeMap::new();
        for resource in &group.resources {
            *type_counts.entry(resource.azure_type.as_str()).or_default() += 1;
        }
        nodes.push(TopologyNodeDto {
            id: group_node_id(&group.id),
            kind: "resource-group".to_owned(),
            name: group.name.clone(),
            subtitle: format!("{} resources", group.resources.len()),
            azure_type: None,
            lane: Some(group.subscription_id.clone()),
            parent_id: None,
            zone: None,
            hop: None,
            member_ids: group.resources.iter().map(|r| r.id.clone()).collect(),
            count: group.resources.len(),
            finding_count: group.finding_count,
            resource_id: None,
            group_id: Some(group.id.clone()),
        });
    }
    for lane in lanes.iter().filter(|lane| !lane.expanded) {
        aggregated += lane.group_count;
        nodes.push(TopologyNodeDto {
            id: lane_node_id(&lane.subscription_id),
            kind: "subscription".to_owned(),
            name: lane.name.clone(),
            subtitle: format!(
                "{} groups · {} resources",
                lane.group_count, lane.resource_count
            ),
            azure_type: None,
            lane: Some(lane.subscription_id.clone()),
            parent_id: None,
            zone: None,
            hop: None,
            member_ids: Vec::new(),
            count: lane.group_count,
            finding_count: lane.finding_count,
            resource_id: None,
            group_id: None,
        });
    }

    // Links between whatever the group resolved to (its card or its lane bar).
    let node_for_group = |index: usize| {
        let group = &groups[index];
        if expanded.contains(group.subscription_id.as_str()) {
            group_node_id(&group.id)
        } else {
            lane_node_id(&group.subscription_id)
        }
    };
    let mut merged: BTreeMap<(String, String, &'static str), usize> = BTreeMap::new();
    for ((source, target, class), count) in &bundles {
        let source_node = node_for_group(*source);
        let target_node = node_for_group(*target);
        if source_node == target_node {
            continue;
        }
        *merged.entry((source_node, target_node, class)).or_default() += count;
    }
    let links: Vec<TopologyLinkDto> = merged
        .into_iter()
        .map(|((source_id, target_id, class), count)| TopologyLinkDto {
            source_id,
            target_id,
            label: format!("{count} link{}", if count == 1 { "" } else { "s" }),
            kind_class: class.to_owned(),
            count,
        })
        .collect();

    let drawn_links = links.len();
    let kind_classes = class_counts(links.iter().map(|l| (l.kind_class.as_str(), l.count)));
    TopologyGraphDto {
        level: "estate".to_owned(),
        lanes,
        nodes,
        links,
        kind_classes,
        counts: TopologyCountsDto {
            total: groups.len(),
            drawn,
            folded: 0,
            aggregated,
            external: 0,
            hidden_by_filter: scoped.hidden_by_filter,
            total_links,
            drawn_links,
        },
    }
}

// ---------------------------------------------------------------------------
// Group: containment + folding + an aggregated shelf for the unconnected.
// ---------------------------------------------------------------------------

fn group_graph(
    input: &TopologyInput,
    scoped: &ScopedResources,
    scope: &TopologyScope,
    group_id: &str,
) -> TopologyGraphDto {
    let (groups, group_by_resource) = collect_groups(input, scoped, scope);
    let Some(group_index) = groups.iter().position(|g| g.id == group_id) else {
        return empty_graph("group", scoped.hidden_by_filter);
    };
    let group = &groups[group_index];
    let member_ids: HashSet<&str> = group.resources.iter().map(|r| r.id.as_str()).collect();

    // Fold NICs, disks and child resources into their host: any AttachedTo /
    // RunsOn-style structural edge whose target is a VM or ARM parent within
    // the group. The host inherits the folded resources' findings.
    let resource_by_id: HashMap<&str, &&Resource> =
        group.resources.iter().map(|r| (r.id.as_str(), r)).collect();
    let mut folded_into: HashMap<&str, &str> = HashMap::new();
    for edge in input.edges {
        if edge.kind != EdgeKind::AttachedTo {
            continue;
        }
        let (Some(source), Some(target)) = (
            resource_by_id.get(edge.source_id.as_str()),
            resource_by_id.get(edge.target_id.as_str()),
        ) else {
            continue;
        };
        let folds = matches!(
            source.azure_type.as_str(),
            "microsoft.network/networkinterfaces" | "microsoft.compute/disks"
        ) && target.azure_type == "microsoft.compute/virtualmachines"
            || is_child_of(source, target);
        if folds {
            folded_into.insert(source.id.as_str(), target.id.as_str());
        }
    }
    // Resolve chains (disk → vm even when the edge went via a folded NIC).
    let resolve = |id: &str| -> String {
        let mut current = id;
        for _ in 0..4 {
            match folded_into.get(current) {
                Some(next) => current = next,
                None => break,
            }
        }
        current.to_owned()
    };

    // Network containment from the group's VNets. A VNet is drawn as a
    // container, so it counts as drawn even though it is not a tile.
    let mut nodes = Vec::new();
    let mut drawn = 0usize;
    let mut subnet_parent: HashMap<String, String> = HashMap::new(); // subnet id → subnet node id
    for vnet in group
        .resources
        .iter()
        .filter(|r| r.azure_type == "microsoft.network/virtualnetworks")
    {
        drawn += 1;
        let vnet_node = format!("vnet:{}", vnet.id);
        let prefixes = network::vnet_address_prefixes(vnet);
        let prefix = prefixes.first().cloned().unwrap_or_default();
        nodes.push(TopologyNodeDto {
            id: vnet_node.clone(),
            kind: "vnet".to_owned(),
            name: vnet.name.clone(),
            subtitle: prefix,
            azure_type: Some(vnet.azure_type.clone()),
            lane: None,
            parent_id: None,
            zone: Some("core".to_owned()),
            hop: None,
            member_ids: vec![vnet.id.clone()],
            count: 1,
            finding_count: finding_count(input, &vnet.id),
            resource_id: Some(vnet.id.clone()),
            group_id: None,
        });
        for subnet in network::vnet_subnets(vnet) {
            let node_id = format!("subnet:{}", subnet.id);
            subnet_parent.insert(subnet.id, node_id.clone());
            nodes.push(TopologyNodeDto {
                id: node_id,
                kind: "subnet".to_owned(),
                name: subnet.name,
                subtitle: subnet.address_prefix.unwrap_or_default(),
                azure_type: None,
                lane: None,
                parent_id: Some(vnet_node.clone()),
                zone: Some("core".to_owned()),
                hop: None,
                member_ids: Vec::new(),
                count: 0,
                finding_count: 0,
                resource_id: None,
                group_id: None,
            });
        }
    }

    // Where does each drawn resource sit? A resource lands in a subnet when
    // it (or a resource folded into it) carries a placement edge to one.
    let mut placement: HashMap<String, String> = HashMap::new(); // host id → subnet node id
    for edge in input.edges {
        if !matches!(edge.kind, EdgeKind::NicInSubnet | EdgeKind::InVnet) {
            continue;
        }
        if let Some(subnet_node) = subnet_parent.get(&edge.target_id) {
            let host = resolve(&edge.source_id);
            if member_ids.contains(host.as_str()) {
                placement.entry(host).or_insert_with(|| subnet_node.clone());
            }
        }
    }

    // Which resources are connected to anything drawable? Cross-group edges
    // count too: their other endpoint becomes an external stub below, so the
    // in-group resource must stay in the connected core rather than sending a
    // real connector down to the "unconnected" shelf.
    let mut link_touch: HashSet<&str> = HashSet::new();
    for edge in input.edges {
        if member_ids.contains(edge.source_id.as_str()) {
            link_touch.insert(edge.source_id.as_str());
        }
        if member_ids.contains(edge.target_id.as_str()) {
            link_touch.insert(edge.target_id.as_str());
        }
    }

    let mut folded_counts: HashMap<&str, Vec<String>> = HashMap::new();
    for (child, host) in &folded_into {
        folded_counts
            .entry(host)
            .or_default()
            .push((*child).to_owned());
    }

    let mut aggregated_members: BTreeMap<&str, Vec<&Resource>> = BTreeMap::new();
    let mut sorted_members: Vec<&&Resource> = group.resources.iter().collect();
    sorted_members.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
    for resource in sorted_members {
        if folded_into.contains_key(resource.id.as_str())
            || resource.azure_type == "microsoft.network/virtualnetworks"
        {
            continue; // represented by its host / drawn as a container
        }
        let connected = link_touch.contains(resource.id.as_str())
            || placement.contains_key(resource.id.as_str())
            || folded_counts.contains_key(resource.id.as_str());
        if !connected {
            aggregated_members
                .entry(resource.azure_type.as_str())
                .or_default()
                .push(resource);
            continue;
        }
        drawn += 1;
        let folded_here = folded_counts
            .get(resource.id.as_str())
            .cloned()
            .unwrap_or_default();
        let subtitle = if folded_here.is_empty() {
            String::new()
        } else {
            format!("+{} attached", folded_here.len())
        };
        let findings = finding_count(input, &resource.id)
            + folded_here
                .iter()
                .map(|id| finding_count(input, id))
                .sum::<usize>();
        nodes.push(TopologyNodeDto {
            id: resource.id.clone(),
            kind: "resource".to_owned(),
            name: resource.name.clone(),
            subtitle,
            azure_type: Some(resource.azure_type.clone()),
            lane: None,
            parent_id: placement.get(resource.id.as_str()).cloned(),
            zone: Some("core".to_owned()),
            hop: None,
            member_ids: folded_here,
            count: 1,
            finding_count: findings,
            resource_id: Some(resource.id.clone()),
            group_id: None,
        });
    }

    // The unconnected shelf: singletons keep their name, plurals become ×N.
    // Hiding the shelf is a filter, so it reports what it hid.
    let mut aggregated = 0usize;
    let mut hidden_unconnected = 0usize;
    if !scope.show_unconnected {
        hidden_unconnected = aggregated_members.values().map(Vec::len).sum();
        aggregated_members.clear();
    }
    for (azure_type, members) in &aggregated_members {
        if members.len() == 1 {
            let resource = members[0];
            drawn += 1;
            nodes.push(TopologyNodeDto {
                id: resource.id.clone(),
                kind: "resource".to_owned(),
                name: resource.name.clone(),
                subtitle: String::new(),
                azure_type: Some(resource.azure_type.clone()),
                lane: None,
                parent_id: None,
                zone: Some("unconnected".to_owned()),
                hop: None,
                member_ids: Vec::new(),
                count: 1,
                finding_count: finding_count(input, &resource.id),
                resource_id: Some(resource.id.clone()),
                group_id: None,
            });
            continue;
        }
        aggregated += members.len();
        nodes.push(TopologyNodeDto {
            id: format!("aggregate:{group_id}:{azure_type}"),
            kind: "aggregate".to_owned(),
            name: azure_types::display_name(azure_type).to_owned(),
            subtitle: format!("×{}", members.len()),
            azure_type: Some((*azure_type).to_owned()),
            lane: None,
            parent_id: None,
            zone: Some("unconnected".to_owned()),
            hop: None,
            member_ids: members.iter().map(|r| r.id.clone()).collect(),
            count: members.len(),
            finding_count: members.iter().map(|r| finding_count(input, &r.id)).sum(),
            resource_id: None,
            group_id: None,
        });
    }

    // Neighbours in other groups: a group that mostly talks outwards (VNet
    // peerings, shared workspaces) must not look dead, so the resources its
    // edges reach are drawn as ghost stubs — aggregated by type past a
    // handful. VNets drawn as containers are addressed by their frame id.
    let vnet_node_ids: HashSet<&str> = group
        .resources
        .iter()
        .filter(|r| r.azure_type == "microsoft.network/virtualnetworks")
        .map(|r| r.id.as_str())
        .collect();
    let internal_rep = |id: &str| -> String {
        if vnet_node_ids.contains(id) {
            format!("vnet:{id}")
        } else {
            resolve(id)
        }
    };
    let estate_index: HashMap<&str, &Resource> = scoped
        .resources
        .iter()
        .map(|r| (r.id.as_str(), *r))
        .collect();
    let mut externals: BTreeSet<&str> = BTreeSet::new();
    for edge in input.edges {
        let source_in = member_ids.contains(edge.source_id.as_str());
        let target_in = member_ids.contains(edge.target_id.as_str());
        if source_in == target_in {
            continue;
        }
        let other = if source_in {
            edge.target_id.as_str()
        } else {
            edge.source_id.as_str()
        };
        if estate_index.contains_key(other) {
            externals.insert(estate_index[other].id.as_str());
        }
    }
    let mut externals_by_type: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for id in &externals {
        externals_by_type
            .entry(estate_index[id].azure_type.as_str())
            .or_default()
            .push(id);
    }
    let mut external_rep: HashMap<&str, String> = HashMap::new();
    for (azure_type, members) in &externals_by_type {
        if members.len() > 4 {
            let stub_id = format!("external:type:{azure_type}");
            for member in members {
                external_rep.insert(member, stub_id.clone());
            }
            nodes.push(TopologyNodeDto {
                id: stub_id,
                kind: "external".to_owned(),
                name: azure_types::display_name(azure_type).to_owned(),
                subtitle: format!("×{} in other groups", members.len()),
                azure_type: Some((*azure_type).to_owned()),
                lane: None,
                parent_id: None,
                zone: Some("external".to_owned()),
                hop: None,
                member_ids: members.iter().map(|id| (*id).to_owned()).collect(),
                count: members.len(),
                finding_count: 0,
                resource_id: None,
                group_id: None,
            });
        } else {
            for member in members {
                let resource = estate_index[member];
                let stub_id = format!("external:{member}");
                external_rep.insert(member, stub_id.clone());
                nodes.push(TopologyNodeDto {
                    id: stub_id,
                    kind: "external".to_owned(),
                    name: resource.name.clone(),
                    subtitle: resource
                        .resource_group
                        .as_deref()
                        .map(|group| format!("in {group}"))
                        .unwrap_or_default(),
                    azure_type: Some(resource.azure_type.clone()),
                    lane: None,
                    parent_id: None,
                    zone: Some("external".to_owned()),
                    hop: None,
                    member_ids: Vec::new(),
                    count: 1,
                    finding_count: 0,
                    resource_id: Some(resource.id.clone()),
                    group_id: None,
                });
            }
        }
    }

    // Links between drawn representations. Placement and containment edges
    // are already shown as geometry, so they are not drawn as lines again.
    let node_ids: HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();
    let mut total_links = 0usize;
    let mut merged: BTreeMap<(String, String, EdgeKind), usize> = BTreeMap::new();
    for edge in input.edges {
        let source_in = group_by_resource.get(&edge.source_id) == Some(&group_index);
        let target_in = group_by_resource.get(&edge.target_id) == Some(&group_index);
        if !source_in && !target_in {
            continue;
        }
        total_links += 1;
        if matches!(
            edge.kind,
            EdgeKind::NicInSubnet | EdgeKind::InVnet | EdgeKind::SubnetOf
        ) {
            continue;
        }
        let rep = |id: &str, inside: bool| -> Option<String> {
            if inside {
                Some(internal_rep(id))
            } else {
                external_rep.get(id).cloned()
            }
        };
        let (Some(source), Some(target)) = (
            rep(&edge.source_id, source_in),
            rep(&edge.target_id, target_in),
        ) else {
            continue;
        };
        if source == target {
            continue;
        }
        if node_ids.contains(source.as_str()) && node_ids.contains(target.as_str()) {
            *merged.entry((source, target, edge.kind)).or_default() += 1;
        }
    }
    let links: Vec<TopologyLinkDto> = merged
        .into_iter()
        .map(|((source_id, target_id, kind), count)| TopologyLinkDto {
            source_id,
            target_id,
            label: kind_label(kind),
            kind_class: kind_class(kind).to_owned(),
            count,
        })
        .collect();

    let folded = folded_into.len();
    let drawn_links = links.len();
    let kind_classes = class_counts(links.iter().map(|l| (l.kind_class.as_str(), l.count)));
    TopologyGraphDto {
        level: "group".to_owned(),
        lanes: Vec::new(),
        nodes,
        links,
        kind_classes,
        counts: TopologyCountsDto {
            total: group.resources.len(),
            drawn,
            folded,
            aggregated,
            external: externals.len(),
            hidden_by_filter: scoped.hidden_by_filter + hidden_unconnected,
            total_links,
            drawn_links,
        },
    }
}

fn is_child_of(child: &Resource, parent: &Resource) -> bool {
    azure_types::is_child_type(&child.azure_type) && child.id.starts_with(parent.id.as_str())
}

// ---------------------------------------------------------------------------
// Neighbourhood: BFS with a depth dial, class filters, and fan-out folding.
// ---------------------------------------------------------------------------

fn neighbourhood_graph(
    input: &TopologyInput,
    scoped: &ScopedResources,
    resource_id: &str,
    depth: u32,
    kind_classes: &[String],
) -> TopologyGraphDto {
    let depth = depth.clamp(1, 2);
    let resource_by_id: HashMap<&str, &Resource> = scoped
        .resources
        .iter()
        .map(|r| (r.id.as_str(), *r))
        .collect();
    if !resource_by_id.contains_key(resource_id) {
        return empty_graph("neighbourhood", scoped.hidden_by_filter);
    }
    let classes: HashSet<&str> = kind_classes.iter().map(|s| s.as_str()).collect();
    let class_allowed = |kind: EdgeKind| classes.is_empty() || classes.contains(kind_class(kind));

    // Adjacency over edges whose both ends are known resources.
    let mut adjacency: BTreeMap<&str, Vec<(&str, EdgeKind, bool)>> = BTreeMap::new();
    for edge in input.edges {
        let (Some(_), Some(_)) = (
            resource_by_id.get(edge.source_id.as_str()),
            resource_by_id.get(edge.target_id.as_str()),
        ) else {
            continue;
        };
        adjacency.entry(edge.source_id.as_str()).or_default().push((
            edge.target_id.as_str(),
            edge.kind,
            true,
        ));
        adjacency.entry(edge.target_id.as_str()).or_default().push((
            edge.source_id.as_str(),
            edge.kind,
            false,
        ));
    }

    let mut hops: BTreeMap<&str, u32> = BTreeMap::new();
    hops.insert(resource_id, 0);
    let mut frontier = vec![resource_id];
    let mut hidden_neighbours: BTreeSet<&str> = BTreeSet::new();
    for hop in 1..=depth {
        let mut next = Vec::new();
        for &node in &frontier {
            for (neighbour, kind, _) in adjacency.get(node).into_iter().flatten() {
                if hops.contains_key(neighbour) {
                    continue;
                }
                if !class_allowed(*kind) {
                    hidden_neighbours.insert(neighbour);
                    continue;
                }
                hops.insert(neighbour, hop);
                next.push(*neighbour);
            }
        }
        frontier = next;
    }
    let hidden_by_filter = hidden_neighbours
        .iter()
        .filter(|id| !hops.contains_key(**id))
        .count();

    // Fold same-type fan-outs at each hop into a ×N node.
    let mut by_hop_type: BTreeMap<(u32, &str), Vec<&str>> = BTreeMap::new();
    for (&id, &hop) in &hops {
        if hop == 0 {
            continue;
        }
        let resource = resource_by_id[id];
        by_hop_type
            .entry((hop, resource.azure_type.as_str()))
            .or_default()
            .push(id);
    }
    let mut aggregate_of: HashMap<&str, String> = HashMap::new();
    let mut nodes = Vec::new();
    let mut drawn = 0usize;
    let mut aggregated = 0usize;
    let subject = resource_by_id[resource_id];
    nodes.push(resource_node(input, subject, 0));
    drawn += 1;
    for ((hop, azure_type), members) in &by_hop_type {
        if members.len() > NEIGHBOUR_FANOUT_LIMIT {
            let id = format!("aggregate:hop{hop}:{azure_type}");
            aggregated += members.len();
            for member in members {
                aggregate_of.insert(member, id.clone());
            }
            nodes.push(TopologyNodeDto {
                id,
                kind: "aggregate".to_owned(),
                name: azure_types::display_name(azure_type).to_owned(),
                subtitle: format!("×{}", members.len()),
                azure_type: Some((*azure_type).to_owned()),
                lane: None,
                parent_id: None,
                zone: None,
                hop: Some(*hop),
                member_ids: members.iter().map(|id| (*id).to_owned()).collect(),
                count: members.len(),
                finding_count: members.iter().map(|id| finding_count(input, id)).sum(),
                resource_id: None,
                group_id: None,
            });
        } else {
            for member in members {
                drawn += 1;
                nodes.push(resource_node(input, resource_by_id[member], *hop));
            }
        }
    }

    // Links between drawn representations, deduped and counted.
    let representative = |id: &str| -> Option<String> {
        if !hops.contains_key(id) {
            return None;
        }
        Some(
            aggregate_of
                .get(id)
                .cloned()
                .unwrap_or_else(|| id.to_owned()),
        )
    };
    let mut total_links = 0usize;
    let mut merged: BTreeMap<(String, String, EdgeKind), usize> = BTreeMap::new();
    for edge in input.edges {
        let touches = hops.contains_key(edge.source_id.as_str())
            || hops.contains_key(edge.target_id.as_str());
        if !touches {
            continue;
        }
        total_links += 1;
        let (Some(source), Some(target)) = (
            representative(&edge.source_id),
            representative(&edge.target_id),
        ) else {
            continue;
        };
        if source == target || !class_allowed(edge.kind) {
            continue;
        }
        *merged.entry((source, target, edge.kind)).or_default() += 1;
    }
    let links: Vec<TopologyLinkDto> = merged
        .into_iter()
        .map(|((source_id, target_id, kind), count)| TopologyLinkDto {
            source_id,
            target_id,
            label: if count == 1 {
                kind_label(kind)
            } else {
                format!("{} ×{count}", kind_label(kind))
            },
            kind_class: kind_class(kind).to_owned(),
            count,
        })
        .collect();

    let drawn_links = links.len();
    let kind_classes = class_counts(links.iter().map(|l| (l.kind_class.as_str(), l.count)));
    TopologyGraphDto {
        level: "neighbourhood".to_owned(),
        lanes: Vec::new(),
        nodes,
        links,
        kind_classes,
        counts: TopologyCountsDto {
            total: hops.len(),
            drawn,
            folded: 0,
            aggregated,
            external: 0,
            hidden_by_filter: scoped.hidden_by_filter + hidden_by_filter,
            total_links,
            drawn_links,
        },
    }
}

fn resource_node(input: &TopologyInput, resource: &Resource, hop: u32) -> TopologyNodeDto {
    TopologyNodeDto {
        id: resource.id.clone(),
        kind: "resource".to_owned(),
        name: resource.name.clone(),
        subtitle: String::new(),
        azure_type: Some(resource.azure_type.clone()),
        lane: None,
        parent_id: None,
        zone: None,
        hop: Some(hop),
        member_ids: Vec::new(),
        count: 1,
        finding_count: finding_count(input, &resource.id),
        resource_id: Some(resource.id.clone()),
        group_id: None,
    }
}

fn class_counts<'a>(links: impl Iterator<Item = (&'a str, usize)>) -> Vec<KindClassCountDto> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for (class, count) in links {
        *counts.entry(class).or_default() += count;
    }
    counts
        .into_iter()
        .map(|(class, count)| KindClassCountDto {
            class: class.to_owned(),
            count,
        })
        .collect()
}

fn empty_graph(level: &str, hidden_by_filter: usize) -> TopologyGraphDto {
    TopologyGraphDto {
        level: level.to_owned(),
        lanes: Vec::new(),
        nodes: Vec::new(),
        links: Vec::new(),
        kind_classes: Vec::new(),
        counts: TopologyCountsDto {
            hidden_by_filter,
            ..TopologyCountsDto::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subscription(id: &str, name: &str) -> Subscription {
        Subscription {
            subscription_id: id.to_owned(),
            display_name: name.to_owned(),
            state: Some("Enabled".to_owned()),
            tags: None,
        }
    }

    fn group(sub: &str, name: &str) -> ResourceGroup {
        ResourceGroup {
            id: format!("/subscriptions/{sub}/resourcegroups/{name}"),
            name: name.to_owned(),
            subscription_id: sub.to_owned(),
            location: Some("uksouth".to_owned()),
            tags: None,
        }
    }

    fn resource(sub: &str, rg: &str, name: &str, azure_type: &str) -> Resource {
        let id = format!("/subscriptions/{sub}/resourcegroups/{rg}/providers/{azure_type}/{name}");
        Resource {
            id: id.to_lowercase(),
            display_id: id,
            name: name.to_owned(),
            azure_type: azure_type.to_owned(),
            kind: None,
            location: Some("uksouth".to_owned()),
            resource_group: Some(rg.to_owned()),
            subscription_id: sub.to_owned(),
            tags: None,
            sku: None,
            identity: None,
            properties: None,
        }
    }

    fn edge(source: &Resource, target: &Resource, kind: EdgeKind) -> Edge {
        Edge {
            source_id: source.id.clone(),
            target_id: target.id.clone(),
            kind,
            properties: None,
        }
    }

    /// A synthetic 1,000-resource estate: 10 subscriptions × 10 groups × 10
    /// resources, VM+NIC pairs, per-group SQL chains, and cross-group links.
    fn large_estate() -> (
        Vec<Subscription>,
        Vec<ResourceGroup>,
        Vec<Resource>,
        Vec<Edge>,
    ) {
        let mut subscriptions = Vec::new();
        let mut groups = Vec::new();
        let mut resources = Vec::new();
        let mut edges = Vec::new();
        for s in 0..10 {
            let sub = format!("sub-{s:02}");
            subscriptions.push(subscription(&sub, &format!("Subscription {s:02}")));
            for g in 0..10 {
                let rg = format!("rg-{s:02}-{g:02}");
                groups.push(group(&sub, &rg));
                let vm = resource(&sub, &rg, "vm-0", "microsoft.compute/virtualmachines");
                let nic = resource(&sub, &rg, "nic-0", "microsoft.network/networkinterfaces");
                let sql = resource(&sub, &rg, "sql-0", "microsoft.sql/servers");
                let pe = resource(&sub, &rg, "pe-0", "microsoft.network/privateendpoints");
                edges.push(edge(&nic, &vm, EdgeKind::AttachedTo));
                edges.push(edge(&pe, &sql, EdgeKind::PrivateEndpointFor));
                for r in 0..6 {
                    resources.push(resource(
                        &sub,
                        &rg,
                        &format!("st-{r}"),
                        "microsoft.storage/storageaccounts",
                    ));
                }
                resources.extend([vm, nic, sql, pe]);
            }
        }
        // A cross-group link per subscription so lanes have external traffic.
        for s in 0..10 {
            let sub = format!("sub-{s:02}");
            let a = resource(
                &sub,
                &format!("rg-{s:02}-00"),
                "pe-0",
                "microsoft.network/privateendpoints",
            );
            let b = resource(
                &sub,
                &format!("rg-{s:02}-01"),
                "sql-0",
                "microsoft.sql/servers",
            );
            edges.push(edge(&a, &b, EdgeKind::PrivateEndpointFor));
        }
        (subscriptions, groups, resources, edges)
    }

    fn input<'a>(
        subscriptions: &'a [Subscription],
        groups: &'a [ResourceGroup],
        resources: &'a [Resource],
        edges: &'a [Edge],
        findings: &'a BTreeMap<String, usize>,
    ) -> TopologyInput<'a> {
        TopologyInput {
            subscriptions,
            resource_groups: groups,
            resources,
            edges,
            finding_counts: findings,
        }
    }

    #[test]
    fn estate_counts_always_add_up_and_nothing_is_silently_dropped() {
        let (subs, groups, resources, edges) = large_estate();
        let findings = BTreeMap::new();
        let request = TopologyRequest {
            snapshot_id: None,
            mode: TopologyMode::Estate {
                expanded_subscriptions: Vec::new(),
            },
            scope: TopologyScope::default(),
        };

        let graph = build(
            &request,
            &input(&subs, &groups, &resources, &edges, &findings),
        );

        assert_eq!(graph.counts.total, 100, "all 100 groups in scope");
        assert_eq!(
            graph.counts.drawn + graph.counts.aggregated,
            graph.counts.total,
            "every group is a card or inside a collapsed lane"
        );
        assert_eq!(graph.lanes.len(), 10);
        assert!(
            graph.lanes.iter().any(|lane| !lane.expanded),
            "100 groups cannot all fit the card budget"
        );
        assert!(graph.counts.drawn <= ESTATE_CARD_BUDGET);
    }

    #[test]
    fn estate_output_is_deterministic() {
        let (subs, groups, resources, edges) = large_estate();
        let findings = BTreeMap::new();
        let request = TopologyRequest {
            snapshot_id: None,
            mode: TopologyMode::Estate {
                expanded_subscriptions: Vec::new(),
            },
            scope: TopologyScope::default(),
        };

        let first = build(
            &request,
            &input(&subs, &groups, &resources, &edges, &findings),
        );
        let second = build(
            &request,
            &input(&subs, &groups, &resources, &edges, &findings),
        );

        assert_eq!(
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap()
        );
    }

    #[test]
    fn group_view_represents_every_resource_exactly_once() {
        let (subs, groups, resources, edges) = large_estate();
        let findings = BTreeMap::new();
        let request = TopologyRequest {
            snapshot_id: None,
            mode: TopologyMode::Group {
                group_id: "/subscriptions/sub-00/resourcegroups/rg-00-00".to_owned(),
            },
            scope: TopologyScope::default(),
        };

        let graph = build(
            &request,
            &input(&subs, &groups, &resources, &edges, &findings),
        );

        assert_eq!(graph.counts.total, 10);
        assert_eq!(
            graph.counts.drawn + graph.counts.folded + graph.counts.aggregated,
            graph.counts.total,
            "drawn + folded + aggregated must cover the whole group"
        );
        assert_eq!(graph.counts.folded, 1, "the NIC folds into its VM");
        let aggregate = graph
            .nodes
            .iter()
            .find(|node| node.kind == "aggregate")
            .expect("6 unconnected storage accounts aggregate into one tile");
        assert_eq!(aggregate.count, 6);
        assert_eq!(aggregate.member_ids.len(), 6);
    }

    #[test]
    fn aggregate_tiles_use_the_friendly_type_name_when_labelled() {
        let (subs, groups, resources, edges) = large_estate();
        let findings = BTreeMap::new();
        let request = TopologyRequest {
            snapshot_id: None,
            mode: TopologyMode::Group {
                group_id: "/subscriptions/sub-00/resourcegroups/rg-00-00".to_owned(),
            },
            scope: TopologyScope::default(),
        };

        let graph = build(
            &request,
            &input(&subs, &groups, &resources, &edges, &findings),
        );

        let aggregate = graph
            .nodes
            .iter()
            .find(|node| node.kind == "aggregate")
            .expect("the unconnected storage accounts aggregate into one tile");
        // The tile used to be labelled with the raw ARM type, so the desktop
        // read "microsoft.storage/storageaccounts" where every CLI diagram read
        // "Storage Account" for the same resources.
        assert_eq!(aggregate.name, "Storage Account");
        assert_eq!(
            aggregate.azure_type.as_deref(),
            Some("microsoft.storage/storageaccounts"),
            "the machine-readable type stays on the DTO for filtering and icons"
        );
    }

    #[test]
    fn group_view_draws_external_neighbours_as_ghost_stubs() {
        let (subs, groups, resources, edges) = large_estate();
        let findings = BTreeMap::new();
        let request = TopologyRequest {
            snapshot_id: None,
            mode: TopologyMode::Group {
                group_id: "/subscriptions/sub-00/resourcegroups/rg-00-00".to_owned(),
            },
            scope: TopologyScope::default(),
        };

        let graph = build(
            &request,
            &input(&subs, &groups, &resources, &edges, &findings),
        );

        let stub = graph
            .nodes
            .iter()
            .find(|node| node.kind == "external")
            .expect("the cross-group private endpoint target appears as a stub");
        assert_eq!(graph.counts.external, 1);
        assert_eq!(stub.subtitle, "in rg-00-01");
        assert!(
            graph
                .links
                .iter()
                .any(|link| link.source_id.ends_with("/pe-0") && link.target_id == stub.id),
            "the cross-group link is drawn to the stub"
        );
    }

    #[test]
    fn group_view_keeps_cross_group_only_endpoints_in_the_connected_core() {
        let subs = vec![subscription("sub-a", "A")];
        let groups = vec![group("sub-a", "rg-a"), group("sub-a", "rg-shared")];
        let identity = resource(
            "sub-a",
            "rg-a",
            "app-identity",
            "microsoft.managedidentity/userassignedidentities",
        );
        let app = resource("sub-a", "rg-shared", "shared-app", "microsoft.web/sites");
        let resources = vec![identity.clone(), app.clone()];
        let edges = vec![edge(&app, &identity, EdgeKind::UsesIdentity)];
        let findings = BTreeMap::new();
        let request = TopologyRequest {
            snapshot_id: None,
            mode: TopologyMode::Group {
                group_id: "/subscriptions/sub-a/resourcegroups/rg-a".to_owned(),
            },
            scope: TopologyScope::default(),
        };

        let graph = build(
            &request,
            &input(&subs, &groups, &resources, &edges, &findings),
        );

        let identity_node = graph
            .nodes
            .iter()
            .find(|node| node.resource_id.as_deref() == Some(identity.id.as_str()))
            .expect("the identity is represented as its own node");
        assert_eq!(identity_node.zone.as_deref(), Some("core"));
        assert!(
            graph.links.iter().any(|link| link.target_id == identity.id),
            "the cross-group connector terminates in the visible core"
        );
    }

    #[test]
    fn neighbourhood_has_no_node_cap_and_folds_wide_fanouts() {
        let subs = vec![subscription("sub-a", "A")];
        let groups = vec![group("sub-a", "rg-a")];
        let hub = resource(
            "sub-a",
            "rg-a",
            "hub-vm",
            "microsoft.compute/virtualmachines",
        );
        let mut resources = vec![hub.clone()];
        let mut edges = Vec::new();
        for index in 0..20 {
            let disk = resource(
                "sub-a",
                "rg-a",
                &format!("disk-{index:02}"),
                "microsoft.compute/disks",
            );
            edges.push(edge(&disk, &hub, EdgeKind::AttachedTo));
            resources.push(disk);
        }
        let findings = BTreeMap::new();
        let request = TopologyRequest {
            snapshot_id: None,
            mode: TopologyMode::Neighbourhood {
                resource_id: hub.id.clone(),
                depth: 1,
                kind_classes: Vec::new(),
            },
            scope: TopologyScope::default(),
        };

        let graph = build(
            &request,
            &input(&subs, &groups, &resources, &edges, &findings),
        );

        assert_eq!(graph.counts.total, 21, "subject plus every neighbour");
        assert_eq!(
            graph.counts.drawn + graph.counts.aggregated,
            graph.counts.total
        );
        let aggregate = graph
            .nodes
            .iter()
            .find(|node| node.kind == "aggregate")
            .expect("20 disks fold into one ×N node");
        assert_eq!(aggregate.count, 20);
        let link = graph.links.first().expect("one merged link to the fold");
        assert_eq!(link.count, 20);
    }

    #[test]
    fn neighbourhood_kind_filter_hides_but_counts() {
        let subs = vec![subscription("sub-a", "A")];
        let groups = vec![group("sub-a", "rg-a")];
        let app = resource("sub-a", "rg-a", "app", "microsoft.web/sites");
        let plan = resource("sub-a", "rg-a", "plan", "microsoft.web/serverfarms");
        let law = resource(
            "sub-a",
            "rg-a",
            "law",
            "microsoft.operationalinsights/workspaces",
        );
        let edges = vec![
            edge(&app, &plan, EdgeKind::RunsOn),
            edge(&app, &law, EdgeKind::LogsTo),
        ];
        let resources = vec![app.clone(), plan, law];
        let findings = BTreeMap::new();
        let request = TopologyRequest {
            snapshot_id: None,
            mode: TopologyMode::Neighbourhood {
                resource_id: app.id.clone(),
                depth: 1,
                kind_classes: vec!["structure".to_owned()],
            },
            scope: TopologyScope::default(),
        };

        let graph = build(
            &request,
            &input(&subs, &groups, &resources, &edges, &findings),
        );

        assert_eq!(graph.counts.total, 2, "subject and the plan");
        assert_eq!(
            graph.counts.hidden_by_filter, 1,
            "the workspace is hidden but declared"
        );
    }

    #[test]
    fn scope_filter_reports_hidden_resources() {
        let (subs, groups, resources, edges) = large_estate();
        let findings = BTreeMap::new();
        let request = TopologyRequest {
            snapshot_id: None,
            mode: TopologyMode::Estate {
                expanded_subscriptions: Vec::new(),
            },
            scope: TopologyScope {
                subscriptions: vec!["sub-00".to_owned()],
                azure_types: Vec::new(),
                show_unconnected: true,
            },
        };

        let graph = build(
            &request,
            &input(&subs, &groups, &resources, &edges, &findings),
        );

        assert_eq!(graph.counts.hidden_by_filter, 900);
        assert!(
            graph
                .nodes
                .iter()
                .all(|node| node.lane.as_deref() == Some("sub-00"))
        );
    }
}

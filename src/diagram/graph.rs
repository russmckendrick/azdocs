pub(crate) mod assessment;

use std::collections::HashMap;

use crate::error::StoreError;
use crate::labels::{DiagramLabels, fill};
use crate::model::{Edge, EdgeKind, Resource, azure_types, network};
use crate::store::Store;

use super::page::DiagramDetail;

/// Diagram-neutral estate graph: typed nodes with parent containment plus
/// styled edges. Emitters (Mermaid, draw.io) consume this without touching the
/// store.
#[derive(Debug, Default)]
pub struct EstateGraph {
    pub title: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<DiagEdge>,
    /// How the boxes are arranged. Almost every graph here is a containment
    /// tree; the peering graph is the exception and says so.
    pub layout: LayoutMode,
}

/// The shape of a graph, which decides how it is laid out.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LayoutMode {
    /// Boxes nest — subnets inside a VNet, resources inside a subnet — and are
    /// gridded into rows within their parent.
    #[default]
    Containment,
    /// A network, not a tree: no box contains another and the *edges* are the
    /// content. Laid out around the best-connected node, because a peering
    /// graph in a single row makes every connector cross the boxes between its
    /// endpoints, which no amount of routing can fix.
    Relational,
}

#[derive(Debug)]
pub struct Node {
    pub label: String,
    pub sublabel: Option<String>,
    pub kind: NodeKind,
    pub parent: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Tenant,
    Subscription,
    ResourceGroup,
    Vnet,
    Subnet,
    /// A labelled aside for things outside the drawing's main structure —
    /// resources in no virtual network, VNets in no peering. Drawn apart so
    /// the reader can see at a glance that they are not part of the topology,
    /// and always labelled with what it is holding and how much.
    Zone,
    Resource {
        azure_type: String,
    },
}

impl NodeKind {
    pub fn is_container(&self) -> bool {
        !matches!(self, Self::Resource { .. })
    }
}

#[derive(Debug)]
pub struct DiagEdge {
    pub source: usize,
    pub target: usize,
    pub label: Option<String>,
    pub style: EdgeStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeStyle {
    /// Attachment/ownership.
    Solid,
    /// Peerings and other symmetric links.
    Dashed,
    /// Loose association (NSG on subnet, private link).
    Association,
}

/// Cap label length so long Azure resource names don't overlap neighbours.
pub fn truncate_label(value: &str) -> String {
    /// Long Azure names overlap their neighbours past this.
    const MAX: usize = 30;
    crate::model::truncate(value, MAX)
}

/// A node's drawn label, capped only where a long one would overlap a
/// neighbour.
///
/// A leaf tile is narrow and sits beside others, so its name is capped. A
/// container's label is drawn along its own wide edge and is often the only
/// place a count appears — "Not in a virtual network · 5 resources · 1 nested"
/// truncated to 30 characters loses exactly the information it exists to
/// carry. The SVG emitter already made this distinction; draw.io and Mermaid
/// capped everything.
pub fn node_label(node: &Node) -> String {
    if node.kind.is_container() {
        node.label.clone()
    } else {
        truncate_label(&node.label)
    }
}

/// Scoping filters shared by the builders.
#[derive(Debug, Default, Clone)]
pub struct DiagramScope {
    pub subscription: Option<String>,
    pub resource_group: Option<String>,
}

/// One graph of a fan-out set (per-VNet, per-resource-group), carrying the
/// file slug and draw.io sheet name alongside the graph itself.
#[derive(Debug)]
pub struct NamedGraph {
    pub slug: String,
    pub sheet_name: String,
    /// `<subscription id>/<lowercased group name>` on per-group graphs, so a
    /// report section can find its own diagram. `None` on the other fan-outs.
    pub group_key: Option<String>,
    /// Display name of the owning subscription. An id alone tells a reader
    /// nothing, and a group name is only unique within its subscription.
    pub subscription_name: Option<String>,
    pub graph: EstateGraph,
}

/// Filesystem-safe slug: ascii-lowercased alphanumerics, everything else
/// collapsed to single dashes.
pub fn slugify(input: &str) -> String {
    crate::model::slugify(input)
}

const VNET_TYPE: &str = "microsoft.network/virtualnetworks";

/// Tiles beyond this collapse into a single "Other resources" entry. Past
/// roughly a dozen the eye stops reading a grid and starts skimming it.
const MAX_TILES: usize = 11;

/// One drawn tile in an aggregated group: either a single resource or a whole
/// resource type collapsed to a count.
pub(crate) struct Tile {
    pub azure_type: String,
    /// What the tile *is*: the resource's name, or the type when the tile
    /// stands for many of them. Always the first line, everywhere — the two
    /// builders used to disagree, so a subnet member read name-then-type
    /// while an out-of-network tile of the same resource read type-then-name.
    pub label: String,
    /// The qualifier under it: the type, or `×N` for an aggregate.
    pub sublabel: String,
    /// `Some` only when the tile stands for exactly one resource.
    pub resource_id: Option<String>,
}

/// Tiles for a zone at the requested detail level: aggregated for a document
/// summary, one per resource for a standalone export.
pub(crate) fn tiles_for(
    resources: &[&Resource],
    detail: DiagramDetail,
    labels: &DiagramLabels,
) -> Vec<Tile> {
    if detail.aggregates() {
        return aggregate_by_type(resources, labels);
    }
    resources
        .iter()
        .map(|resource| Tile {
            azure_type: resource.azure_type.clone(),
            label: resource.name.clone(),
            sublabel: azure_types::display_name(&resource.azure_type).to_owned(),
            resource_id: Some(resource.id.clone()),
        })
        .collect()
}

/// Collapse a resource list into type tiles.
///
/// A resource group of 96 members drawn one icon per resource is a wall of
/// identical glyphs that says nothing; "Storage Account ×13" says the same
/// thing in one tile and leaves room for the group to fit a page. Singletons
/// keep their name, because for those the name *is* the information.
pub(crate) fn aggregate_by_type(resources: &[&Resource], labels: &DiagramLabels) -> Vec<Tile> {
    let mut order: Vec<&str> = Vec::new();
    let mut by_type: HashMap<&str, Vec<&Resource>> = HashMap::new();
    for resource in resources {
        let entry = by_type.entry(resource.azure_type.as_str()).or_default();
        if entry.is_empty() {
            order.push(resource.azure_type.as_str());
        }
        entry.push(resource);
    }
    // Busiest types first, then by name, so the ordering is stable across runs
    // and the tail that collapses is always the long thin one.
    order.sort_by(|a, b| {
        by_type[b]
            .len()
            .cmp(&by_type[a].len())
            .then_with(|| a.cmp(b))
    });

    let collapse = order.len() > MAX_TILES;
    let shown = if collapse { MAX_TILES - 1 } else { order.len() };
    let mut tiles: Vec<Tile> = order
        .iter()
        .take(shown)
        .map(|azure_type| {
            let members = &by_type[azure_type];
            let display = azure_types::display_name(azure_type).to_owned();
            match members.as_slice() {
                [only] => Tile {
                    azure_type: (*azure_type).to_owned(),
                    label: only.name.clone(),
                    sublabel: display,
                    resource_id: Some(only.id.clone()),
                },
                many => Tile {
                    azure_type: (*azure_type).to_owned(),
                    label: display,
                    sublabel: fill(&labels.aggregate, &[("count", &many.len())]),
                    resource_id: None,
                },
            }
        })
        .collect();

    if collapse {
        let rest: Vec<&&str> = order.iter().skip(shown).collect();
        let count: usize = rest
            .iter()
            .map(|azure_type| by_type[**azure_type].len())
            .sum();
        tiles.push(Tile {
            // The generic glyph: this tile stands for no one type.
            azure_type: "microsoft.resources/resourcegroups".to_owned(),
            label: labels.other_resources.clone(),
            sublabel: fill(
                &labels.other_resources_sub,
                &[("types", &rest.len()), ("count", &count)],
            ),
            resource_id: None,
        });
    }
    tiles
}

impl EstateGraph {
    fn add_node(
        &mut self,
        label: impl Into<String>,
        sublabel: Option<String>,
        kind: NodeKind,
        parent: Option<usize>,
    ) -> usize {
        self.nodes.push(Node {
            label: label.into(),
            sublabel,
            kind,
            parent,
        });
        self.nodes.len() - 1
    }

    /// Tenant → subscriptions → resource groups with resource-count badges.
    pub fn hierarchy(
        store: &Store,
        snapshot_id: &str,
        scope: &DiagramScope,
        labels: &DiagramLabels,
    ) -> Result<Self, StoreError> {
        let snapshot = store.get_snapshot(snapshot_id)?;
        let mut subscriptions = store.subscriptions(snapshot_id)?;
        let mut groups = store.resource_groups(snapshot_id)?;
        let resources = scoped_resources(store, snapshot_id, scope)?;
        // The same scope every other graph honours; a `--subscription` on the
        // hierarchy used to be silently ignored.
        subscriptions.retain(|sub| {
            scope
                .subscription
                .as_ref()
                .is_none_or(|wanted| wanted == &sub.subscription_id)
        });
        groups.retain(|rg| {
            scope
                .resource_group
                .as_ref()
                .is_none_or(|wanted| wanted.eq_ignore_ascii_case(&rg.name))
        });

        let mut graph = Self {
            title: fill(&labels.hierarchy_title, &[("tenant", &snapshot.tenant_id)]),
            ..Self::default()
        };
        let tenant = graph.add_node(
            fill(&labels.tenant_node, &[("tenant", &snapshot.tenant_id)]),
            None,
            NodeKind::Tenant,
            None,
        );
        for sub in &subscriptions {
            let count = resources
                .iter()
                .filter(|r| r.subscription_id == sub.subscription_id)
                .count();
            let sub_node = graph.add_node(
                &sub.display_name,
                Some(fill(&labels.resource_count, &[("count", &count)])),
                NodeKind::Subscription,
                Some(tenant),
            );
            for rg in groups
                .iter()
                .filter(|rg| rg.subscription_id == sub.subscription_id)
            {
                let rg_count = resources
                    .iter()
                    .filter(|r| {
                        r.subscription_id == sub.subscription_id
                            && r.resource_group.as_deref() == Some(&rg.name.to_lowercase())
                    })
                    .count();
                graph.add_node(
                    &rg.name,
                    Some(fill(&labels.resource_count, &[("count", &rg_count)])),
                    NodeKind::ResourceGroup,
                    Some(sub_node),
                );
            }
        }
        Ok(graph)
    }

    /// Resource-group containers with resource icon nodes and attachment edges.
    pub fn resources(
        store: &Store,
        snapshot_id: &str,
        scope: &DiagramScope,
        labels: &DiagramLabels,
    ) -> Result<Self, StoreError> {
        let subscriptions = store.subscriptions(snapshot_id)?;
        let groups = store.resource_groups(snapshot_id)?;
        let resources = scoped_resources(store, snapshot_id, scope)?;
        let edges = store.edges(snapshot_id)?;

        let mut graph = Self {
            title: labels.resources_title.clone(),
            ..Self::default()
        };
        let mut by_resource_id: HashMap<&str, usize> = HashMap::new();
        for sub in &subscriptions {
            if scope
                .subscription
                .as_ref()
                .is_some_and(|wanted| wanted != &sub.subscription_id)
            {
                continue;
            }
            let sub_node = graph.add_node(&sub.display_name, None, NodeKind::Subscription, None);
            for rg in groups
                .iter()
                .filter(|rg| rg.subscription_id == sub.subscription_id)
            {
                let rg_lower = rg.name.to_lowercase();
                if scope
                    .resource_group
                    .as_ref()
                    .is_some_and(|wanted| wanted.to_lowercase() != rg_lower)
                {
                    continue;
                }
                let members: Vec<&Resource> = resources
                    .iter()
                    .filter(|r| {
                        r.subscription_id == sub.subscription_id
                            && r.resource_group.as_deref() == Some(rg_lower.as_str())
                    })
                    .collect();
                if members.is_empty() {
                    continue;
                }
                let rg_node =
                    graph.add_node(&rg.name, None, NodeKind::ResourceGroup, Some(sub_node));
                for resource in members {
                    let node = graph.add_node(
                        &resource.name,
                        Some(azure_types::display_name(&resource.azure_type).to_owned()),
                        NodeKind::Resource {
                            azure_type: resource.azure_type.clone(),
                        },
                        Some(rg_node),
                    );
                    by_resource_id.insert(&resource.id, node);
                }
            }
        }
        graph.add_resource_edges(&edges, &by_resource_id, labels);
        Ok(graph)
    }

    /// VNet/subnet containers, NIC'd resources placed in their subnet, dashed
    /// peering edges, NSG associations, and private endpoint targets.
    pub fn network(
        store: &Store,
        snapshot_id: &str,
        scope: &DiagramScope,
        labels: &DiagramLabels,
    ) -> Result<Self, StoreError> {
        let resources = scoped_resources(store, snapshot_id, scope)?;
        let edges = store.edges(snapshot_id)?;

        let mut graph = Self {
            title: labels.network_title.clone(),
            ..Self::default()
        };
        let mut node_ids: HashMap<String, usize> = HashMap::new();

        // VNet containers with their subnets (from subnet_of edges).
        for vnet in resources.iter().filter(|r| r.azure_type == VNET_TYPE) {
            graph.add_vnet_with_subnets(vnet, None, &mut node_ids);
        }

        // Resources placed inside subnets via their nic_in_subnet edges.
        let by_id: HashMap<&str, &Resource> =
            resources.iter().map(|r| (r.id.as_str(), r)).collect();
        graph.place_subnet_resources(&by_id, &edges, &mut node_ids);

        // NSGs and private-link targets appear outside the vnets when needed.
        for edge in &edges {
            match edge.kind {
                EdgeKind::PeeredWith => {
                    if let (Some(&a), Some(&b)) =
                        (node_ids.get(&edge.source_id), node_ids.get(&edge.target_id))
                    {
                        // One edge per peering pair; the mirror image is skipped.
                        if a < b {
                            let state = edge
                                .properties
                                .as_ref()
                                .and_then(|p| p.get("state"))
                                .and_then(|s| s.as_str())
                                .unwrap_or(&labels.peered);
                            graph.edges.push(DiagEdge {
                                source: a,
                                target: b,
                                label: Some(state.to_owned()),
                                style: EdgeStyle::Dashed,
                            });
                        }
                    }
                }
                EdgeKind::NsgAttached => {
                    let Some(&target) = node_ids.get(&edge.source_id) else {
                        continue;
                    };
                    let Some(nsg) = by_id.get(edge.target_id.as_str()) else {
                        continue;
                    };
                    let nsg_node = *node_ids.entry(nsg.id.clone()).or_insert_with(|| {
                        graph.nodes.push(Node {
                            label: nsg.name.clone(),
                            sublabel: Some("NSG".to_owned()),
                            kind: NodeKind::Resource {
                                azure_type: nsg.azure_type.clone(),
                            },
                            parent: None,
                        });
                        graph.nodes.len() - 1
                    });
                    graph.edges.push(DiagEdge {
                        source: nsg_node,
                        target,
                        label: None,
                        style: EdgeStyle::Association,
                    });
                }
                EdgeKind::PrivateEndpointFor => {
                    let Some(&pe) = node_ids.get(&edge.source_id) else {
                        continue;
                    };
                    let Some(target) = by_id.get(edge.target_id.as_str()) else {
                        continue;
                    };
                    let target_node = *node_ids.entry(target.id.clone()).or_insert_with(|| {
                        graph.nodes.push(Node {
                            label: target.name.clone(),
                            sublabel: Some(
                                azure_types::display_name(&target.azure_type).to_owned(),
                            ),
                            kind: NodeKind::Resource {
                                azure_type: target.azure_type.clone(),
                            },
                            parent: None,
                        });
                        graph.nodes.len() - 1
                    });
                    graph.edges.push(DiagEdge {
                        source: pe,
                        target: target_node,
                        label: Some("private link".to_owned()),
                        style: EdgeStyle::Association,
                    });
                }
                _ => {}
            }
        }

        // NSGs and private-link targets arrive without a parent; group them in
        // one container instead of scattering them around the origin.
        let floating: Vec<usize> = graph
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.parent.is_none() && !node.kind.is_container())
            .map(|(index, _)| index)
            .collect();
        if !floating.is_empty() {
            let container = graph.add_node(
                labels.connected_services.as_str(),
                Some(labels.connected_services_sub.clone()),
                NodeKind::ResourceGroup,
                None,
            );
            for index in floating {
                graph.nodes[index].parent = Some(container);
            }
        }
        Ok(graph)
    }

    /// Add a VNet container with its subnets, registering ARM ids in
    /// `node_ids` so later passes can home resources and edges.
    fn add_vnet_with_subnets(
        &mut self,
        vnet: &Resource,
        parent: Option<usize>,
        node_ids: &mut HashMap<String, usize>,
    ) -> usize {
        let prefixes = network::vnet_address_prefixes(vnet);
        let label = (!prefixes.is_empty()).then(|| prefixes.join(", "));
        let vnet_node = self.add_node(&vnet.name, label, NodeKind::Vnet, parent);
        node_ids.insert(vnet.id.clone(), vnet_node);

        for subnet in network::vnet_subnets(vnet) {
            let subnet_node = self.add_node(
                &subnet.name,
                subnet.address_prefix,
                NodeKind::Subnet,
                Some(vnet_node),
            );
            node_ids.insert(subnet.id, subnet_node);
        }
        vnet_node
    }

    /// Home resources in their subnet via `nic_in_subnet` edges. Only subnets
    /// already present in `node_ids` receive resources, so callers control
    /// which VNets participate.
    fn place_subnet_resources(
        &mut self,
        by_id: &HashMap<&str, &Resource>,
        edges: &[Edge],
        node_ids: &mut HashMap<String, usize>,
    ) {
        for edge in edges.iter().filter(|e| e.kind == EdgeKind::NicInSubnet) {
            let Some(&subnet_node) = node_ids.get(&edge.target_id) else {
                continue;
            };
            let Some(resource) = by_id.get(edge.source_id.as_str()) else {
                continue;
            };
            let representative = host_representative(resource, edges, by_id);
            if node_ids.contains_key(&representative.id) {
                continue;
            }
            let node = self.add_node(
                &representative.name,
                Some(azure_types::display_name(&representative.azure_type).to_owned()),
                NodeKind::Resource {
                    azure_type: representative.azure_type.clone(),
                },
                Some(subnet_node),
            );
            node_ids.insert(representative.id.clone(), node);
        }
    }

    /// One graph per VNet: its subtree plus dashed peering edges to remote
    /// VNets, which appear as childless stub nodes.
    pub fn per_vnet(
        store: &Store,
        snapshot_id: &str,
        scope: &DiagramScope,
        labels: &DiagramLabels,
    ) -> Result<Vec<NamedGraph>, StoreError> {
        let resources = scoped_resources(store, snapshot_id, scope)?;
        let edges = store.edges(snapshot_id)?;
        let by_id: HashMap<&str, &Resource> =
            resources.iter().map(|r| (r.id.as_str(), r)).collect();

        let mut vnets: Vec<&Resource> = resources
            .iter()
            .filter(|r| r.azure_type == VNET_TYPE)
            .collect();
        vnets.sort_by_key(|a| a.name.to_lowercase());

        let mut named = Vec::new();
        for vnet in vnets {
            let sheet_name = fill(&labels.vnet_sheet, &[("name", &vnet.name)]);
            let mut graph = Self {
                title: sheet_name.clone(),
                ..Self::default()
            };
            let mut node_ids: HashMap<String, usize> = HashMap::new();
            let vnet_node = graph.add_vnet_with_subnets(vnet, None, &mut node_ids);
            graph.place_subnet_resources(&by_id, &edges, &mut node_ids);
            for edge in edges
                .iter()
                .filter(|e| e.kind == EdgeKind::PeeredWith && e.source_id == vnet.id)
            {
                let stub = *node_ids.entry(edge.target_id.clone()).or_insert_with(|| {
                    let label = by_id
                        .get(edge.target_id.as_str())
                        .map(|r| r.name.clone())
                        .unwrap_or_else(|| remote_vnet_label(&edge.target_id, labels));
                    graph.nodes.push(Node {
                        label,
                        sublabel: None,
                        kind: NodeKind::Vnet,
                        parent: None,
                    });
                    graph.nodes.len() - 1
                });
                graph.edges.push(DiagEdge {
                    source: vnet_node,
                    target: stub,
                    label: Some(peering_state(edge, labels)),
                    style: EdgeStyle::Dashed,
                });
            }
            named.push(NamedGraph {
                slug: slugify(&vnet.name),
                sheet_name,
                group_key: None,
                subscription_name: None,
                graph,
            });
        }
        finalize_named(&mut named);
        Ok(named)
    }

    /// One graph per non-empty resource group: VNet subtrees homed in the
    /// group plus a dashed "Standalone Resources" container for the rest.
    /// NICs represented by their VM are skipped.
    pub fn per_resource_group(
        store: &Store,
        snapshot_id: &str,
        scope: &DiagramScope,
        detail: DiagramDetail,
        labels: &DiagramLabels,
    ) -> Result<Vec<NamedGraph>, StoreError> {
        let groups = store.resource_groups(snapshot_id)?;
        let resources = scoped_resources(store, snapshot_id, scope)?;
        let edges = store.edges(snapshot_id)?;
        let by_id: HashMap<&str, &Resource> =
            resources.iter().map(|r| (r.id.as_str(), r)).collect();
        let subscriptions = store.subscriptions(snapshot_id)?;
        let subscription_names: HashMap<&str, &str> = subscriptions
            .iter()
            .map(|sub| (sub.subscription_id.as_str(), sub.display_name.as_str()))
            .collect();

        let mut groups: Vec<_> = groups
            .iter()
            .filter(|rg| {
                scope
                    .subscription
                    .as_ref()
                    .is_none_or(|wanted| wanted == &rg.subscription_id)
                    && scope
                        .resource_group
                        .as_ref()
                        .is_none_or(|wanted| wanted.to_lowercase() == rg.name.to_lowercase())
            })
            .collect();
        groups.sort_by(|a, b| {
            (a.name.to_lowercase(), &a.subscription_id)
                .cmp(&(b.name.to_lowercase(), &b.subscription_id))
        });

        let mut named = Vec::new();
        for rg in groups {
            let rg_lower = rg.name.to_lowercase();
            let mut members: Vec<&Resource> = resources
                .iter()
                .filter(|r| {
                    r.subscription_id == rg.subscription_id
                        && r.resource_group.as_deref() == Some(rg_lower.as_str())
                })
                .collect();
            if members.is_empty() {
                continue;
            }
            members.sort_by(|a, b| {
                (a.name.to_lowercase(), &a.id).cmp(&(b.name.to_lowercase(), &b.id))
            });

            let subscription_name = subscription_names
                .get(rg.subscription_id.as_str())
                .map(|name| (*name).to_owned());
            // Group names repeat across subscriptions, so a bare group name is
            // an ambiguous caption once the fan-out is spread over a directory.
            let sheet_name = match &subscription_name {
                Some(sub) => fill(
                    &labels.resource_group_sheet,
                    &[("name", &rg.name), ("subscription", sub)],
                ),
                None => fill(&labels.resource_group_sheet_bare, &[("name", &rg.name)]),
            };
            let mut graph = Self {
                title: sheet_name.clone(),
                ..Self::default()
            };
            let mut node_ids: HashMap<String, usize> = HashMap::new();
            let rg_node = graph.add_node(&rg.name, None, NodeKind::ResourceGroup, None);
            for vnet in members.iter().filter(|r| r.azure_type == VNET_TYPE) {
                graph.add_vnet_with_subnets(vnet, Some(rg_node), &mut node_ids);
            }
            graph.place_subnet_resources(&by_id, &edges, &mut node_ids);

            let standalone: Vec<&Resource> = members
                .iter()
                .copied()
                .filter(|r| {
                    r.azure_type != VNET_TYPE
                        && !node_ids.contains_key(&r.id)
                        && !is_represented_by_host(r, &edges, &by_id)
                        && !azure_types::is_child_type(&r.azure_type)
                })
                .collect();
            // Child resources (a SQL database inside its server) are not drawn
            // — they would only clutter the tiles — but the reader is told they
            // exist. A resource in the section's table and absent from the
            // diagram above it, with nothing saying so, is the defect this
            // count closes.
            let nested = members
                .iter()
                .filter(|r| {
                    azure_types::is_child_type(&r.azure_type) && !node_ids.contains_key(&r.id)
                })
                .count();
            if !standalone.is_empty() || nested > 0 {
                let mut label = fill(&labels.not_in_vnet, &[("count", &standalone.len())]);
                if nested > 0 {
                    label.push_str(&fill(&labels.nested_suffix, &[("count", &nested)]));
                }
                let container = graph.add_node(label, None, NodeKind::Zone, Some(rg_node));
                for tile in tiles_for(&standalone, detail, labels) {
                    let node = graph.add_node(
                        tile.label,
                        Some(tile.sublabel),
                        NodeKind::Resource {
                            azure_type: tile.azure_type,
                        },
                        Some(container),
                    );
                    // Only a tile standing for exactly one resource can carry
                    // that resource's edges; an aggregate has no single identity.
                    if let Some(id) = tile.resource_id {
                        node_ids.insert(id, node);
                    }
                }
            }

            let by_resource_id: HashMap<&str, usize> = node_ids
                .iter()
                .map(|(id, &node)| (id.as_str(), node))
                .collect();
            graph.add_resource_edges(&edges, &by_resource_id, labels);
            named.push(NamedGraph {
                slug: slugify(&rg.name),
                sheet_name,
                group_key: Some(crate::report::details::group_key(
                    &rg.subscription_id,
                    &rg.name,
                )),
                subscription_name,
                graph,
            });
        }
        finalize_named(&mut named);
        Ok(named)
    }

    /// Flat peering map: every VNet as a childless node, dashed edges
    /// labelled with the peering state. Remote VNets outside the snapshot
    /// appear as stubs.
    pub fn peerings(
        store: &Store,
        snapshot_id: &str,
        scope: &DiagramScope,
        labels: &DiagramLabels,
    ) -> Result<Self, StoreError> {
        let resources = scoped_resources(store, snapshot_id, scope)?;
        let edges = store.edges(snapshot_id)?;

        let mut graph = Self {
            title: labels.peerings_title.clone(),
            layout: LayoutMode::Relational,
            ..Self::default()
        };
        let mut node_ids: HashMap<String, usize> = HashMap::new();
        let mut vnets: Vec<&Resource> = resources
            .iter()
            .filter(|r| r.azure_type == VNET_TYPE)
            .collect();
        vnets.sort_by_key(|a| a.name.to_lowercase());
        for vnet in vnets {
            let node = graph.add_node(&vnet.name, None, NodeKind::Vnet, None);
            node_ids.insert(vnet.id.clone(), node);
        }

        let mut seen: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
        for edge in edges.iter().filter(|e| e.kind == EdgeKind::PeeredWith) {
            let Some(&source) = node_ids.get(&edge.source_id) else {
                continue;
            };
            let target = *node_ids.entry(edge.target_id.clone()).or_insert_with(|| {
                graph.nodes.push(Node {
                    label: remote_vnet_label(&edge.target_id, labels),
                    sublabel: None,
                    kind: NodeKind::Vnet,
                    parent: None,
                });
                graph.nodes.len() - 1
            });
            // One edge per peering pair; the mirror image is skipped.
            if !seen.insert((source.min(target), source.max(target))) {
                continue;
            }
            graph.edges.push(DiagEdge {
                source,
                target,
                label: Some(peering_state(edge, labels)),
                style: EdgeStyle::Dashed,
            });
        }

        // A VNet with no peering plays no part in the topology, so it is set
        // apart — and then has to say why. Four boxes sitting under a diagram
        // with nothing naming them read as part of it that failed to connect.
        let unpeered: Vec<usize> = (0..graph.nodes.len())
            .filter(|&i| {
                graph.nodes[i].kind == NodeKind::Vnet
                    && !graph.edges.iter().any(|e| e.source == i || e.target == i)
            })
            .collect();
        if !unpeered.is_empty() {
            let zone = graph.add_node(
                fill(&labels.not_peered, &[("count", &unpeered.len())]),
                None,
                NodeKind::Zone,
                None,
            );
            for node in unpeered {
                graph.nodes[node].parent = Some(zone);
            }
        }
        Ok(graph)
    }

    /// The graph immediately around one resource: the resource itself plus
    /// every resource one hop away, with the edges between them.
    ///
    /// Takes already-loaded resources and edges rather than the store, because
    /// the report emitters build one of these per resource and re-querying per
    /// resource would dominate the run.
    pub fn neighbourhood(
        resource: &Resource,
        by_id: &HashMap<&str, &Resource>,
        edges: &[Edge],
        labels: &DiagramLabels,
    ) -> Self {
        // No title: this diagram always sits directly under the resource's own
        // heading in the report, so a title band would just repeat it.
        let mut graph = Self::default();
        let mut node_ids: HashMap<String, usize> = HashMap::new();

        // Drawn inside the same dashed group frame as every other diagram.
        // Without it a relationships picture was a couple of icons loose on the
        // page, which read as an illustration rather than part of the estate.
        let frame = graph.add_node(
            resource
                .resource_group
                .clone()
                .unwrap_or_else(|| resource.subscription_id.clone()),
            None,
            NodeKind::ResourceGroup,
            None,
        );

        let add = |graph: &mut Self, node_ids: &mut HashMap<String, usize>, r: &Resource| {
            let node = graph.add_node(
                &r.name,
                Some(azure_types::display_name(&r.azure_type).to_owned()),
                NodeKind::Resource {
                    azure_type: r.azure_type.clone(),
                },
                Some(frame),
            );
            node_ids.insert(r.id.clone(), node);
        };
        // Sorted by name so the layout — and therefore the golden SVG — is
        // stable regardless of edge insertion order.
        let mut neighbours: Vec<&Resource> = edges
            .iter()
            .filter_map(|edge| {
                let other = if edge.source_id == resource.id {
                    &edge.target_id
                } else if edge.target_id == resource.id {
                    &edge.source_id
                } else {
                    return None;
                };
                by_id.get(other.as_str()).copied()
            })
            .collect();
        neighbours
            .sort_by(|a, b| (a.name.to_lowercase(), &a.id).cmp(&(b.name.to_lowercase(), &b.id)));
        neighbours.dedup_by(|a, b| a.id == b.id);
        // The subject goes in the middle of the row rather than at its head.
        // Nearly every edge here ends on it, and from one end a connector has
        // to cross the tiles in between — which is what made a VM's disk edge
        // run straight through its NIC.
        let (before, after) = neighbours.split_at(neighbours.len() / 2);
        for neighbour in before {
            add(&mut graph, &mut node_ids, neighbour);
        }
        add(&mut graph, &mut node_ids, resource);
        for neighbour in after {
            add(&mut graph, &mut node_ids, neighbour);
        }

        let by_resource_id: HashMap<&str, usize> = node_ids
            .iter()
            .map(|(id, &node)| (id.as_str(), node))
            .collect();
        graph.add_resource_edges(edges, &by_resource_id, labels);
        graph
    }

    fn add_resource_edges(
        &mut self,
        edges: &[Edge],
        by_resource_id: &HashMap<&str, usize>,
        labels: &DiagramLabels,
    ) {
        for edge in edges {
            let (Some(&source), Some(&target)) = (
                by_resource_id.get(edge.source_id.as_str()),
                by_resource_id.get(edge.target_id.as_str()),
            ) else {
                continue;
            };
            let (label, style) = match edge.kind {
                EdgeKind::AttachedTo => (None, EdgeStyle::Solid),
                EdgeKind::PrivateEndpointFor => {
                    (Some(labels.private_link.clone()), EdgeStyle::Association)
                }
                EdgeKind::NsgAttached => (None, EdgeStyle::Association),
                _ => continue,
            };
            self.edges.push(DiagEdge {
                source,
                target,
                label,
                style,
            });
        }
    }
}

/// The host an attachment is drawn as part of — a NIC's VM, or a private
/// endpoint's NIC — or the resource itself when nothing hosts it.
fn host_representative<'a>(
    resource: &'a Resource,
    edges: &[Edge],
    by_id: &HashMap<&str, &'a Resource>,
) -> &'a Resource {
    // The first attachment that *is* a host, not the first attachment filtered
    // by whether it happens to be one: an AKS VMSS NIC attaches to two load
    // balancers as well as its instance, and taking the first edge blind meant
    // the load balancer won and the NIC never folded.
    edges
        .iter()
        .filter(|e| e.kind == EdgeKind::AttachedTo && e.source_id == resource.id)
        .filter_map(|e| by_id.get(e.target_id.as_str()).copied())
        .find(|owner| azure_types::is_attachment_host(&owner.azure_type))
        .unwrap_or(resource)
}

/// Is this resource already represented by a host it is attached to?
///
/// NICs and disks fold (`azure_types::FOLDS_INTO_HOST`) into VMs and private
/// endpoints (`azure_types::ATTACHMENT_HOSTS`), matching the desktop topology
/// builder. They only fold when the attachment actually resolves to a drawn
/// host — an orphaned disk is still a resource, and stays a tile of its own.
fn is_represented_by_host(
    resource: &Resource,
    edges: &[Edge],
    by_id: &HashMap<&str, &Resource>,
) -> bool {
    azure_types::folds_into_host(&resource.azure_type)
        && !std::ptr::eq(host_representative(resource, edges, by_id), resource)
}

fn remote_vnet_label(arm_id: &str, labels: &DiagramLabels) -> String {
    let name = crate::model::short_name(arm_id);
    if name.is_empty() {
        labels.remote_vnet.clone()
    } else {
        name.to_owned()
    }
}

fn peering_state(edge: &Edge, labels: &DiagramLabels) -> String {
    edge.properties
        .as_ref()
        .and_then(|p| p.get("state"))
        .and_then(|s| s.as_str())
        .unwrap_or(&labels.peered)
        .to_owned()
}

/// Duplicate names (e.g. same RG name in two subscriptions) would collide on
/// disk; suffix later copies, then order by slug so fan-out output is stable.
fn finalize_named(named: &mut [NamedGraph]) {
    let mut seen: HashMap<String, usize> = HashMap::new();
    for item in named.iter_mut() {
        let count = seen.entry(item.slug.clone()).or_insert(0);
        *count += 1;
        if *count > 1 {
            item.slug = format!("{}-{}", item.slug, count);
        }
    }
    named.sort_by(|a, b| a.slug.cmp(&b.slug));
}

fn scoped_resources(
    store: &Store,
    snapshot_id: &str,
    scope: &DiagramScope,
) -> Result<Vec<Resource>, StoreError> {
    let mut resources = store.resources(snapshot_id)?;
    if let Some(sub) = &scope.subscription {
        resources.retain(|r| &r.subscription_id == sub);
    }
    if let Some(rg) = &scope.resource_group {
        let rg = rg.to_lowercase();
        resources.retain(|r| r.resource_group.as_deref() == Some(rg.as_str()));
    }
    Ok(resources)
}

#[cfg(test)]
mod label_tests {
    use super::{Node, NodeKind, node_label, truncate_label};

    fn node(kind: NodeKind, label: &str) -> Node {
        Node {
            label: label.to_owned(),
            sublabel: None,
            kind,
            parent: None,
        }
    }

    #[test]
    fn unit_keeps_a_container_label_whole_when_it_carries_a_count() {
        // The tail is the information: truncating it loses the very thing the
        // label exists to say.
        let long = "Not in a virtual network  ·  5 resources  ·  1 nested";
        assert_eq!(
            node_label(&node(NodeKind::Zone, long)),
            long,
            "a container label must survive whole"
        );
    }

    #[test]
    fn unit_caps_a_leaf_label_when_a_long_name_would_overlap() {
        let long = "an-extremely-long-azure-resource-name-that-runs-on";
        let capped = node_label(&node(
            NodeKind::Resource {
                azure_type: "microsoft.compute/virtualmachines".to_owned(),
            },
            long,
        ));
        assert_eq!(capped, truncate_label(long));
        assert!(capped.ends_with('…'));
    }
}

#[cfg(test)]
mod fold_tests {
    use super::*;

    fn resource(id: &str, azure_type: &str) -> Resource {
        Resource {
            id: id.to_owned(),
            display_id: id.to_owned(),
            name: id.rsplit('/').next().unwrap_or(id).to_owned(),
            azure_type: azure_type.to_owned(),
            kind: None,
            location: None,
            resource_group: Some("rg".to_owned()),
            subscription_id: "sub".to_owned(),
            tags: None,
            sku: None,
            identity: None,
            properties: None,
        }
    }

    fn attached(source: &str, target: &str) -> Edge {
        Edge {
            source_id: source.to_owned(),
            target_id: target.to_owned(),
            kind: EdgeKind::AttachedTo,
            properties: None,
        }
    }

    /// An AKS VMSS NIC attaches to two load balancers as well as its instance.
    /// Taking the first attachment and *then* asking whether it is a host let
    /// the load balancer win, and the NIC never folded.
    #[test]
    fn unit_a_nic_folds_into_its_vm_when_another_attachment_comes_first() {
        let nic = resource("/nic", "microsoft.network/networkinterfaces");
        let lb = resource("/lb", "microsoft.network/loadbalancers");
        let vm = resource("/vm", "microsoft.compute/virtualmachines");
        let by_id = HashMap::from([("/nic", &nic), ("/lb", &lb), ("/vm", &vm)]);
        let edges = vec![attached("/nic", "/lb"), attached("/nic", "/vm")];

        assert_eq!(host_representative(&nic, &edges, &by_id).id, "/vm");
        assert!(is_represented_by_host(&nic, &edges, &by_id));
    }

    /// A private endpoint's NIC is Azure's own plumbing and folds into the
    /// endpoint, which is what stopped every private-endpoint subnet drawing
    /// each of its endpoints twice.
    #[test]
    fn unit_a_private_endpoint_nic_folds_into_its_endpoint() {
        let nic = resource("/pe.nic", "microsoft.network/networkinterfaces");
        let pe = resource("/pe", "microsoft.network/privateendpoints");
        let by_id = HashMap::from([("/pe.nic", &nic), ("/pe", &pe)]);
        let edges = vec![attached("/pe.nic", "/pe")];

        assert_eq!(host_representative(&nic, &edges, &by_id).id, "/pe");
    }

    /// A NIC whose only attachment is a load balancer has nothing to be drawn
    /// as part of, so it stays a tile of its own.
    #[test]
    fn unit_a_nic_with_no_host_stays_its_own_tile() {
        let nic = resource("/nic", "microsoft.network/networkinterfaces");
        let lb = resource("/lb", "microsoft.network/loadbalancers");
        let by_id = HashMap::from([("/nic", &nic), ("/lb", &lb)]);
        let edges = vec![attached("/nic", "/lb")];

        assert!(!is_represented_by_host(&nic, &edges, &by_id));
    }
}

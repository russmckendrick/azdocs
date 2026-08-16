use std::collections::HashMap;

use crate::error::StoreError;
use crate::model::{Edge, EdgeKind, Resource, azure_types};
use crate::store::Store;

/// Diagram-neutral estate graph: typed nodes with parent containment plus
/// styled edges. Emitters (Mermaid, draw.io) consume this without touching the
/// store.
#[derive(Debug, Default)]
pub struct EstateGraph {
    pub title: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<DiagEdge>,
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
    Resource { azure_type: String },
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
    const MAX: usize = 30;
    if value.chars().count() <= MAX {
        return value.to_owned();
    }
    let mut out: String = value.chars().take(MAX - 1).collect();
    out.push('…');
    out
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
    pub graph: EstateGraph,
}

/// Filesystem-safe slug: ascii-lowercased alphanumerics, everything else
/// collapsed to single dashes.
pub fn slugify(input: &str) -> String {
    input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

const VNET_TYPE: &str = "microsoft.network/virtualnetworks";

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
    pub fn hierarchy(store: &Store, snapshot_id: &str) -> Result<Self, StoreError> {
        let snapshot = store.get_snapshot(snapshot_id)?;
        let subscriptions = store.subscriptions(snapshot_id)?;
        let groups = store.resource_groups(snapshot_id)?;
        let resources = store.resources(snapshot_id)?;

        let mut graph = Self {
            title: format!("Azure estate — tenant {}", snapshot.tenant_id),
            ..Self::default()
        };
        let tenant = graph.add_node(
            format!("Tenant {}", snapshot.tenant_id),
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
                Some(format!("{count} resources")),
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
                    Some(format!("{rg_count} resources")),
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
    ) -> Result<Self, StoreError> {
        let subscriptions = store.subscriptions(snapshot_id)?;
        let groups = store.resource_groups(snapshot_id)?;
        let resources = scoped_resources(store, snapshot_id, scope)?;
        let edges = store.edges(snapshot_id)?;

        let mut graph = Self {
            title: "Azure resources".to_owned(),
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
        graph.add_resource_edges(&edges, &by_resource_id);
        Ok(graph)
    }

    /// VNet/subnet containers, NIC'd resources placed in their subnet, dashed
    /// peering edges, NSG associations, and private endpoint targets.
    pub fn network(
        store: &Store,
        snapshot_id: &str,
        scope: &DiagramScope,
    ) -> Result<Self, StoreError> {
        let resources = scoped_resources(store, snapshot_id, scope)?;
        let edges = store.edges(snapshot_id)?;

        let mut graph = Self {
            title: "Azure network topology".to_owned(),
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
                                .unwrap_or("peered");
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
                "Connected services",
                Some("NSGs & private-link targets".to_owned()),
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
        let prefixes = vnet
            .properties
            .as_ref()
            .and_then(|p| p.get("addressSpace"))
            .and_then(|a| a.get("addressPrefixes"))
            .and_then(|p| p.as_array())
            .map(|list| {
                list.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            });
        let vnet_node = self.add_node(&vnet.name, prefixes, NodeKind::Vnet, parent);
        node_ids.insert(vnet.id.clone(), vnet_node);

        for subnet in vnet
            .properties
            .as_ref()
            .and_then(|p| p.get("subnets"))
            .and_then(|s| s.as_array())
            .into_iter()
            .flatten()
        {
            let Some(subnet_id) = subnet.get("id").and_then(|v| v.as_str()) else {
                continue;
            };
            let name = subnet
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| subnet_id.rsplit('/').next().unwrap_or("subnet"));
            let prefix = subnet
                .pointer("/properties/addressPrefix")
                .and_then(|v| v.as_str())
                .map(str::to_owned);
            let subnet_node = self.add_node(name, prefix, NodeKind::Subnet, Some(vnet_node));
            node_ids.insert(subnet_id.to_lowercase(), subnet_node);
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
            let representative = vm_representative(resource, edges, by_id);
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
            let sheet_name = format!("VNet - {}", vnet.name);
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
                        .unwrap_or_else(|| remote_vnet_label(&edge.target_id));
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
                    label: Some(peering_state(edge)),
                    style: EdgeStyle::Dashed,
                });
            }
            named.push(NamedGraph {
                slug: slugify(&vnet.name),
                sheet_name,
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
    ) -> Result<Vec<NamedGraph>, StoreError> {
        let groups = store.resource_groups(snapshot_id)?;
        let resources = scoped_resources(store, snapshot_id, scope)?;
        let edges = store.edges(snapshot_id)?;
        let by_id: HashMap<&str, &Resource> =
            resources.iter().map(|r| (r.id.as_str(), r)).collect();

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

            let sheet_name = format!("Resource Group - {}", rg.name);
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
                        && !is_nic_represented_by_vm(r, &edges, &by_id)
                        && !is_child_resource_type(&r.azure_type)
                })
                .collect();
            if !standalone.is_empty() {
                let container = graph.add_node(
                    "Standalone Resources",
                    None,
                    NodeKind::ResourceGroup,
                    Some(rg_node),
                );
                for resource in standalone {
                    let node = graph.add_node(
                        &resource.name,
                        Some(azure_types::display_name(&resource.azure_type).to_owned()),
                        NodeKind::Resource {
                            azure_type: resource.azure_type.clone(),
                        },
                        Some(container),
                    );
                    node_ids.insert(resource.id.clone(), node);
                }
            }

            let by_resource_id: HashMap<&str, usize> = node_ids
                .iter()
                .map(|(id, &node)| (id.as_str(), node))
                .collect();
            graph.add_resource_edges(&edges, &by_resource_id);
            named.push(NamedGraph {
                slug: slugify(&rg.name),
                sheet_name,
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
    ) -> Result<Self, StoreError> {
        let resources = scoped_resources(store, snapshot_id, scope)?;
        let edges = store.edges(snapshot_id)?;

        let mut graph = Self {
            title: "VNet peerings".to_owned(),
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
                    label: remote_vnet_label(&edge.target_id),
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
                label: Some(peering_state(edge)),
                style: EdgeStyle::Dashed,
            });
        }
        Ok(graph)
    }

    fn add_resource_edges(&mut self, edges: &[Edge], by_resource_id: &HashMap<&str, usize>) {
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
                    (Some("private link".to_owned()), EdgeStyle::Association)
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

/// A NIC is represented by its VM when one is attached.
fn vm_representative<'a>(
    resource: &'a Resource,
    edges: &[Edge],
    by_id: &HashMap<&str, &'a Resource>,
) -> &'a Resource {
    edges
        .iter()
        .find(|e| e.kind == EdgeKind::AttachedTo && e.source_id == resource.id)
        .and_then(|e| by_id.get(e.target_id.as_str()).copied())
        .filter(|owner| owner.azure_type == "microsoft.compute/virtualmachines")
        .unwrap_or(resource)
}

/// Child resources (`provider/parent/child`, e.g. VM extensions or SQL
/// databases) live inside their parent and would only clutter the
/// "Standalone Resources" container as free-floating nodes.
fn is_child_resource_type(azure_type: &str) -> bool {
    azure_type.matches('/').count() > 1
}

fn is_nic_represented_by_vm(
    resource: &Resource,
    edges: &[Edge],
    by_id: &HashMap<&str, &Resource>,
) -> bool {
    resource.azure_type == "microsoft.network/networkinterfaces"
        && !std::ptr::eq(vm_representative(resource, edges, by_id), resource)
}

fn remote_vnet_label(arm_id: &str) -> String {
    arm_id
        .rsplit('/')
        .next()
        .filter(|segment| !segment.is_empty())
        .unwrap_or("remote vnet")
        .to_owned()
}

fn peering_state(edge: &Edge) -> String {
    edge.properties
        .as_ref()
        .and_then(|p| p.get("state"))
        .and_then(|s| s.as_str())
        .unwrap_or("peered")
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

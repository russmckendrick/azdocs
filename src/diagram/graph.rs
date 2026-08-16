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

/// Scoping filters shared by the builders.
#[derive(Debug, Default, Clone)]
pub struct DiagramScope {
    pub subscription: Option<String>,
    pub resource_group: Option<String>,
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
        for vnet in resources
            .iter()
            .filter(|r| r.azure_type == "microsoft.network/virtualnetworks")
        {
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
            let vnet_node = graph.add_node(&vnet.name, prefixes, NodeKind::Vnet, None);
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
                let subnet_node = graph.add_node(name, prefix, NodeKind::Subnet, Some(vnet_node));
                node_ids.insert(subnet_id.to_lowercase(), subnet_node);
            }
        }

        // Resources placed inside subnets via their nic_in_subnet edges.
        let by_id: HashMap<&str, &Resource> =
            resources.iter().map(|r| (r.id.as_str(), r)).collect();
        for edge in edges.iter().filter(|e| e.kind == EdgeKind::NicInSubnet) {
            let Some(&subnet_node) = node_ids.get(&edge.target_id) else {
                continue;
            };
            let Some(resource) = by_id.get(edge.source_id.as_str()) else {
                continue;
            };
            // A NIC represents its VM when one is attached.
            let representative = edges
                .iter()
                .find(|e| e.kind == EdgeKind::AttachedTo && e.source_id == resource.id)
                .and_then(|e| by_id.get(e.target_id.as_str()))
                .filter(|owner| owner.azure_type == "microsoft.compute/virtualmachines")
                .unwrap_or(resource);
            if node_ids.contains_key(&representative.id) {
                continue;
            }
            let node = graph.add_node(
                &representative.name,
                Some(azure_types::display_name(&representative.azure_type).to_owned()),
                NodeKind::Resource {
                    azure_type: representative.azure_type.clone(),
                },
                Some(subnet_node),
            );
            node_ids.insert(representative.id.clone(), node);
        }

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

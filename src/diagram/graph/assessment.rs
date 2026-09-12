//! Focused connection figures for the assessment, separate from the exhaustive
//! report/reference diagrams. Every retained logical connection is paginated.

use std::collections::{BTreeMap, BTreeSet};

use crate::labels::{Labels, fill};
use crate::model::{EdgeKind, azure_types};
use crate::report::analysis::{GroupProfile, ReportAnalysis};

use crate::diagram::graph::NamedGraph;
use crate::diagram::graph::{DiagEdge, EdgeStyle, EstateGraph, Node, NodeKind};

#[derive(Clone)]
struct Tile {
    label: String,
    context: String,
    azure_type: String,
    members: BTreeSet<String>,
    hosts: BTreeSet<String>,
}

pub(crate) fn build(analysis: &ReportAnalysis, labels: &Labels) -> Vec<NamedGraph> {
    let mut assets = Vec::new();
    for (index, group) in analysis.studies().enumerate() {
        let (tiles, mut links) = group_graph(analysis, group, labels);
        // The estate figure already explains peerings. Select two further
        // connection types by boundary crossings, then recorded connection
        // count. Every edge in a selected type is still paginated, never capped.
        let mut priorities: BTreeMap<EdgeKind, (usize, usize)> = BTreeMap::new();
        for ((source, target, kind), count) in &links {
            if *kind != EdgeKind::PeeredWith {
                let score = priorities.entry(*kind).or_default();
                score.0 += usize::from(tiles[source].context != tiles[target].context) * count;
                score.1 += count;
            }
        }
        let mut priorities: Vec<_> = priorities.into_iter().collect();
        priorities.sort_by(|(ak, a), (bk, b)| b.cmp(a).then_with(|| ak.cmp(bk)));
        let selected: BTreeSet<_> = priorities.into_iter().take(2).map(|(k, _)| k).collect();
        links.retain(|(_, _, kind), _| selected.contains(kind));
        assets.extend(pages(
            &tiles,
            &links,
            &format!("assessment-group-{index}"),
            Some(&group.key),
            analysis,
            labels,
        ));
    }
    let mut tiles = BTreeMap::new();
    let mut links = BTreeMap::new();
    for edge in analysis
        .relationships
        .iter()
        .filter(|e| e.kind == EdgeKind::PeeredWith)
    {
        for id in [&edge.source, &edge.target] {
            tiles.entry(id.clone()).or_insert_with(|| Tile {
                label: analysis.display_name(id).to_owned(),
                context: String::new(),
                azure_type: "microsoft.network/virtualnetworks".to_owned(),
                members: BTreeSet::from([id.clone()]),
                hosts: BTreeSet::from([id.clone()]),
            });
        }
        links.insert(
            (
                edge.source.clone(),
                edge.target.clone(),
                EdgeKind::PeeredWith,
            ),
            1,
        );
    }
    assets.extend(pages(
        &tiles,
        &links,
        "assessment-network",
        None,
        analysis,
        labels,
    ));
    assets
}

type Links = BTreeMap<(String, String, EdgeKind), usize>;

fn group_graph(
    analysis: &ReportAnalysis,
    group: &GroupProfile,
    labels: &Labels,
) -> (BTreeMap<String, Tile>, Links) {
    let mut folded: BTreeMap<&str, &str> = BTreeMap::new();
    let by_id = analysis
        .resources
        .values()
        .map(|r| (r.id.as_str(), r))
        .collect();
    for resource in analysis
        .resources
        .values()
        .filter(|r| azure_types::folds_into_host(&r.azure_type))
    {
        let host = super::host_representative(resource, &analysis.edges, &by_id);
        if host.id != resource.id {
            folded.insert(resource.id.as_str(), host.id.as_str());
        }
    }
    for resource in analysis
        .resources
        .values()
        .filter(|r| azure_types::is_child_type(&r.azure_type))
    {
        if let Some((parent, _)) = resource.id.rsplit_once('/')
            && let Some((parent, _)) = parent.rsplit_once('/')
            && analysis.resources.contains_key(parent)
        {
            folded.insert(&resource.id, parent);
        }
    }
    let mut endpoint_keys = BTreeMap::new();
    let mut tiles: BTreeMap<String, Tile> = BTreeMap::new();
    for resource in analysis.resources.values() {
        let host_id = folded
            .get(resource.id.as_str())
            .copied()
            .unwrap_or(&resource.id);
        let host = analysis.resources.get(host_id).unwrap_or(resource);
        let key = crate::report::analysis::resource_group_key(host);
        let tile_key = format!("{key}/{}", host.azure_type);
        let tile = tiles.entry(tile_key.clone()).or_insert_with(|| Tile {
            label: azure_types::display_name(&host.azure_type).to_owned(),
            context: fill(
                &labels.diagram.resource_group_sheet,
                &[
                    (
                        "name",
                        &host
                            .resource_group
                            .as_deref()
                            .unwrap_or(&labels.common.subscription_scope),
                    ),
                    (
                        "subscription",
                        &analysis
                            .subscriptions
                            .get(&host.subscription_id)
                            .unwrap_or(&host.subscription_id),
                    ),
                ],
            ),
            azure_type: host.azure_type.clone(),
            members: BTreeSet::new(),
            hosts: BTreeSet::new(),
        });
        tile.members.insert(resource.id.clone());
        tile.hosts.insert(host.id.clone());
        endpoint_keys.insert(resource.id.clone(), tile_key);
    }
    let mut links = BTreeMap::new();
    for edge in &analysis.relationships {
        if edge.source_group.as_deref() != Some(&group.key)
            && edge.target_group.as_deref() != Some(&group.key)
        {
            continue;
        }
        let mut keys = Vec::new();
        for id in [&edge.source, &edge.target] {
            let key = endpoint_keys.get(id).or_else(|| {
                id.split_once("/subnets/")
                    .and_then(|(parent, _)| endpoint_keys.get(parent))
            });
            let key = key.cloned().unwrap_or_else(|| {
                tiles.entry(id.clone()).or_insert_with(|| Tile {
                    label: analysis.display_name(id).to_owned(),
                    context: labels.report.assessment.unresolved_endpoint.clone(),
                    azure_type: "microsoft.resources/resourcegroups".to_owned(),
                    members: BTreeSet::from([id.clone()]),
                    hosts: BTreeSet::from([id.clone()]),
                });
                id.clone()
            });
            keys.push(key);
        }
        if keys[0] != keys[1] {
            *links
                .entry((keys[0].clone(), keys[1].clone(), edge.kind))
                .or_default() += 1;
        }
    }
    // Only connected types belong in a relationship figure. The complete
    // group population is reconciled by the adjoining composition table.
    let used: BTreeSet<_> = links.keys().flat_map(|(a, b, _)| [a, b]).collect();
    tiles.retain(|id, _| used.contains(id));
    (tiles, links)
}

fn pages(
    tiles: &BTreeMap<String, Tile>,
    links: &Links,
    prefix: &str,
    group: Option<&String>,
    analysis: &ReportAnalysis,
    labels: &Labels,
) -> Vec<NamedGraph> {
    // One relationship family per figure gives each picture a clear question.
    // Each graph remains small enough for the shared A4 layout and density rung.
    const MAX_NODES: usize = 6;
    let mut batches = Vec::new();
    let families: BTreeSet<_> = links
        .keys()
        .map(|(_, target, kind)| {
            (
                *kind,
                if group.is_some() {
                    Some(target.as_str())
                } else {
                    None
                },
            )
        })
        .collect();
    for (kind, focus) in families {
        let mut batch = Vec::new();
        let mut nodes = BTreeSet::new();
        for entry @ ((source, target, _), _) in links
            .iter()
            .filter(|((_, target, k), _)| *k == kind && focus.is_none_or(|focus| focus == target))
        {
            let additional =
                usize::from(!nodes.contains(source)) + usize::from(!nodes.contains(target));
            if nodes.len() + additional > MAX_NODES && !batch.is_empty() {
                batches.push(std::mem::take(&mut batch));
                nodes.clear();
            }
            nodes.insert(source);
            nodes.insert(target);
            batch.push(entry);
        }
        if !batch.is_empty() {
            batches.push(batch);
        }
    }
    batches
        .into_iter()
        .enumerate()
        .map(|(index, edges)| {
            let mut graph = EstateGraph {
                layout: if group.is_none() {
                    super::LayoutMode::Relational
                } else {
                    super::LayoutMode::Containment
                },
                ..EstateGraph::default()
            };
            let ids: BTreeSet<_> = edges.iter().flat_map(|((a, b, _), _)| [a, b]).collect();
            let mut indices = BTreeMap::new();
            let mut frames = BTreeMap::new();
            for id in ids {
                if let Some(tile) = tiles.get(id) {
                    let parent = if tile.context.is_empty() {
                        None
                    } else {
                        Some(*frames.entry(tile.context.clone()).or_insert_with(|| {
                            let index = graph.nodes.len();
                            graph.nodes.push(Node {
                                label: tile.context.clone(),
                                sublabel: None,
                                kind: NodeKind::ResourceGroup,
                                parent: None,
                            });
                            index
                        }))
                    };
                    indices.insert(id, graph.nodes.len());
                    let count = fill(&labels.diagram.aggregate, &[("count", &tile.hosts.len())]);
                    let population = if tile.members.len() > tile.hosts.len() {
                        format!(
                            "{}{}",
                            count,
                            fill(
                                &labels.diagram.folded_suffix,
                                &[("count", &(tile.members.len() - tile.hosts.len()))]
                            )
                        )
                    } else {
                        count
                    };
                    graph.nodes.push(Node {
                        label: tile.label.clone(),
                        sublabel: Some(population),
                        kind: if tile.azure_type == "microsoft.network/virtualnetworks" {
                            NodeKind::Vnet
                        } else {
                            NodeKind::Resource {
                                azure_type: tile.azure_type.clone(),
                            }
                        },
                        parent,
                    });
                }
            }
            for ((source, target, kind), count) in &edges {
                let (source_tile, target_tile) = (&tiles[source], &tiles[target]);
                if let (Some(source), Some(target)) = (indices.get(source), indices.get(target)) {
                    let word = labels
                        .desktop
                        .topology
                        .edge_kinds
                        .get(kind.as_str())
                        .map(String::as_str)
                        .unwrap_or(kind.as_str());
                    graph.edges.push(DiagEdge {
                        source: *source,
                        target: *target,
                        label: Some(if *kind == EdgeKind::PeeredWith {
                            let states = analysis
                                .edges
                                .iter()
                                .filter(|e| e.kind == EdgeKind::PeeredWith)
                                .filter(|e| {
                                    (source_tile.members.contains(&e.source_id)
                                        && target_tile.members.contains(&e.target_id))
                                        || (target_tile.members.contains(&e.source_id)
                                            && source_tile.members.contains(&e.target_id))
                                })
                                .map(|e| super::peering_state(e, &labels.diagram))
                                .collect::<BTreeSet<_>>();
                            if states.len() == 1 {
                                states
                                    .into_iter()
                                    .next()
                                    .unwrap_or_else(|| labels.diagram.peered.clone())
                            } else {
                                labels.diagram.peered.clone()
                            }
                        } else {
                            format!("{word} ×{count}")
                        }),
                        style: if *kind == EdgeKind::PeeredWith {
                            EdgeStyle::Dashed
                        } else if matches!(
                            kind,
                            EdgeKind::PrivateEndpointFor
                                | EdgeKind::NsgAttached
                                | EdgeKind::DnsLinked
                        ) {
                            EdgeStyle::Association
                        } else {
                            EdgeStyle::Solid
                        },
                    });
                }
            }
            let title = edges
                .first()
                .map(|((_, target, kind), _)| {
                    fill(
                        &labels.report.assessment.connection_focus,
                        &[
                            (
                                "relationship",
                                &labels.desktop.topology.edge_kinds[kind.as_str()],
                            ),
                            ("target", &tiles[target].label),
                            (
                                "connections",
                                &edges.iter().map(|(_, count)| **count).sum::<usize>(),
                            ),
                        ],
                    )
                })
                .unwrap_or_default();
            NamedGraph {
                slug: format!("{prefix}-{index}"),
                sheet_name: title,
                group_key: group.cloned(),
                subscription_name: None,
                graph,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::{
        layout,
        page::{A4_PORTRAIT, DiagramDetail},
        route,
    };
    use crate::model::{Edge, Resource};

    fn resource(name: &str, group: &str, azure_type: &str) -> Resource {
        Resource {
            id: format!("/subscriptions/s/resourcegroups/{group}/providers/{azure_type}/{name}"),
            display_id: name.into(),
            name: name.into(),
            azure_type: azure_type.into(),
            subscription_id: "s".into(),
            resource_group: Some(group.into()),
            kind: None,
            location: None,
            tags: None,
            sku: None,
            identity: None,
            properties: None,
        }
    }

    #[test]
    fn unit_focus_graphs_reuse_shared_geometry_and_preserve_split_connection_counts() {
        let host = resource("vm", "group", "microsoft.compute/virtualmachines");
        let disk = resource("disk", "group", "microsoft.compute/disks");
        let mut resources = vec![host.clone(), disk.clone()];
        let mut edges = vec![Edge {
            source_id: disk.id.clone(),
            target_id: host.id.clone(),
            kind: EdgeKind::AttachedTo,
            properties: None,
        }];
        for i in 0..9 {
            let target = resource(
                &format!("law-{i}"),
                &format!("external-{i}"),
                "microsoft.operationalinsights/workspaces",
            );
            edges.push(Edge {
                source_id: host.id.clone(),
                target_id: target.id.clone(),
                kind: EdgeKind::LogsTo,
                properties: None,
            });
            resources.push(target);
        }
        let analysis = ReportAnalysis::build(&resources, &[], &[], &[], &edges, vec![]);
        let labels = Labels::default();
        let group = analysis
            .groups
            .iter()
            .find(|g| g.name.as_deref() == Some("group"))
            .unwrap();
        let (tiles, links) = group_graph(&analysis, group, &labels);
        assert_eq!(links.values().sum::<usize>(), 9);
        assert!(
            tiles
                .values()
                .any(|t| t.members.len() == 2 && t.hosts.len() == 1)
        );
        let graphs = pages(&tiles, &links, "test", Some(&group.key), &analysis, &labels);
        assert!(graphs.len() > 1);
        assert_eq!(graphs.iter().map(|g| g.graph.edges.len()).sum::<usize>(), 9);
        for named in graphs {
            let graph = &named.graph;
            assert_eq!(
                graph
                    .edges
                    .iter()
                    .map(|edge| edge.target)
                    .collect::<BTreeSet<_>>()
                    .len(),
                1,
                "separate monitoring targets must not share a confusing connector junction"
            );
            let placed =
                layout::absolutize(graph, &layout::layout_for(graph, DiagramDetail::Summary));
            let routes = route::route(graph, &placed, &layout::rung(graph));
            assert!(
                placed
                    .iter()
                    .all(|p| p.x + p.width <= A4_PORTRAIT.width + 0.1)
            );
            for edge in routes {
                assert!(
                    edge.points
                        .windows(2)
                        .all(|pair| pair[0].0 == pair[1].0 || pair[0].1 == pair[1].1)
                );
            }
            let svg =
                crate::diagram::svg::render_for(graph, DiagramDetail::Summary, &labels.diagram);
            assert!(svg.contains("width=\"680\""));
            assert!(svg.contains("data:image/svg+xml;base64,"));
            assert!(svg.contains("+ 1 attached/child"));
        }
    }
}

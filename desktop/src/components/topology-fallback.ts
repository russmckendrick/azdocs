// Browser-only stand-in for the Rust `topology_graph` command, used by the
// illustrative workspace (`npm run dev` outside Tauri). It mirrors the Rust
// contract — every resource drawn, folded or aggregated, with honest counts —
// on the small mock estate; the packaged app always uses the Rust builder.

import type {
  EdgeKind,
  EstateSnapshot,
  KindClass,
  Resource,
  TopologyGraph,
  TopologyLane,
  TopologyLink,
  TopologyNode,
  TopologyRequest,
} from "../types";
import { buildResourceGroupTopology, resourceGroupNodeId } from "./topology-model";
import { fill, plural, spaced } from "../format";
import { DEFAULT_LABELS } from "../labels";

// Built-ins, not the installed set: this file only runs in the browser
// preview, where the bootstrap labels are the built-ins by construction.
const NODE_WORDS = DEFAULT_LABELS.desktop.topology.nodes;
const EDGE_WORDS = DEFAULT_LABELS.desktop.topology.edge_kinds;

function kindLabel(kind: string) {
  return EDGE_WORDS[kind as keyof typeof EDGE_WORDS] ?? spaced(kind);
}

const NEIGHBOUR_FANOUT_LIMIT = 6;

/**
 * Mirrors `kind_class` in desktop/src-tauri/src/topology.rs.
 *
 * Written as a total Record rather than a switch with a `default`: the default
 * silently classified `monitors` as "structure" for as long as it existed,
 * because a missing case is invisible to a switch. A Record over the EdgeKind
 * union makes an unmapped kind a compile error, matching the exhaustive `match`
 * on the Rust side.
 */
const KIND_CLASSES: Record<EdgeKind, KindClass> = {
  peered_with: "network",
  nsg_attached: "network",
  nic_in_subnet: "network",
  subnet_of: "network",
  in_vnet: "network",
  dns_linked: "network",
  attached_to: "structure",
  runs_on: "structure",
  private_endpoint_for: "data",
  depends_on: "data",
  uses_identity: "identity",
  logs_to: "monitoring",
  monitors: "monitoring",
};

export function edgeKindClass(kind: EdgeKind): KindClass {
  return KIND_CLASSES[kind];
}

function classCounts(links: TopologyLink[]) {
  const counts = new Map<string, number>();
  for (const link of links) counts.set(link.kindClass, (counts.get(link.kindClass) ?? 0) + link.count);
  return [...counts.entries()]
    .sort(([left], [right]) => (left < right ? -1 : 1))
    .map(([kindClass, count]) => ({ class: kindClass, count }));
}

function resourceNode(resource: Resource, extra: Partial<TopologyNode> = {}): TopologyNode {
  return {
    id: resource.id,
    kind: "resource",
    name: resource.name,
    subtitle: "",
    azureType: resource.azureType,
    memberIds: [],
    count: 1,
    findingCount: resource.findingCount,
    resourceId: resource.id,
    groupIds: [],
    ...extra,
  };
}

export function buildFallbackTopology(estate: EstateSnapshot, request: TopologyRequest): TopologyGraph {
  const mode = request.mode;
  if (mode.kind === "group") return groupGraph(estate, mode.groupId);
  if (mode.kind === "neighbourhood") {
    return neighbourhoodGraph(estate, mode.resourceId, mode.depth ?? 1, mode.kindClasses ?? []);
  }
  return estateGraph(estate, mode.expandedSubscriptions ?? [], request.scope.showUnconnected);
}

function estateGraph(estate: EstateSnapshot, expandedSubscriptions: string[], showUnconnected: boolean): TopologyGraph {
  const topology = buildResourceGroupTopology(estate);
  // Mirrors `estate_graph` in topology.rs: only a connected group becomes a
  // card, and only the subscriptions the reader expanded are laid out.
  const connected = (group: { externalLinkCount: number }) => group.externalLinkCount > 0;
  const laneSummaries = estate.subscriptions
    .map((subscription) => {
      const groups = topology.groups.filter((group) => group.subscriptionId === subscription.id);
      return {
        subscriptionId: subscription.id,
        name: subscription.displayName,
        expanded: true,
        groupCount: groups.length,
        resourceCount: groups.reduce((total, group) => total + group.resourceCount, 0),
        findingCount: groups.reduce((total, group) => total + group.findingCount, 0),
        linkCount: groups.reduce((total, group) => total + group.externalLinkCount, 0),
      };
    })
    .filter((lane) => lane.groupCount > 0)
    .sort((left, right) => (left.name < right.name ? -1 : 1));
  const expanded = new Set(expandedSubscriptions);
  const lanes: TopologyLane[] = laneSummaries.map((lane) => ({
    subscriptionId: lane.subscriptionId,
    name: lane.name,
    expanded: expanded.has(lane.subscriptionId),
    groupCount: lane.groupCount,
    resourceCount: lane.resourceCount,
    findingCount: lane.findingCount,
  }));
  const groupById = new Map(topology.groups.map((group) => [group.id, group]));
  const nodeIdForGroup = (groupId: string) => {
    const group = groupById.get(groupId);
    if (!group || expanded.has(group.subscriptionId)) return resourceGroupNodeId(groupId);
    return `subscription:${group.subscriptionId}`;
  };

  const expandedGroups = topology.groups.filter((group) => expanded.has(group.subscriptionId));
  const unconnectedGroups = expandedGroups.filter((group) => !connected(group));
  const unconnectedByLane = new Map<string, typeof unconnectedGroups>();
  for (const group of unconnectedGroups) {
    unconnectedByLane.set(group.subscriptionId, [...(unconnectedByLane.get(group.subscriptionId) ?? []), group]);
  }
  const nodes: TopologyNode[] = [
    ...expandedGroups
      .filter(connected)
      .map((group) => ({
        id: resourceGroupNodeId(group.id),
        kind: "resource-group" as const,
        name: group.name,
        subtitle: fill(NODE_WORDS.group_resources, { count: group.resourceCount }),
        lane: group.subscriptionId,
        memberIds: group.resourceIds,
        count: group.resourceCount,
        findingCount: group.findingCount,
        groupId: group.id,
        groupIds: [],
      })),
    ...(showUnconnected
      ? [...unconnectedByLane.entries()]
          .sort(([left], [right]) => (left < right ? -1 : 1))
          .map(([subscriptionId, groups]) => ({
            id: `aggregate:unconnected:${subscriptionId}`,
            kind: "aggregate" as const,
            name: NODE_WORDS.unconnected_groups,
            subtitle: fill(NODE_WORDS.times_n, { count: groups.length }),
            lane: subscriptionId,
            zone: "unconnected" as const,
            memberIds: groups.flatMap((group) => group.resourceIds),
            count: groups.length,
            findingCount: groups.reduce((total, group) => total + group.findingCount, 0),
            groupIds: groups.map((group) => group.id),
          }))
      : []),
    ...lanes
      .filter((lane) => !lane.expanded)
      .map((lane) => ({
        id: `subscription:${lane.subscriptionId}`,
        kind: "subscription" as const,
        name: lane.name,
        subtitle: fill(NODE_WORDS.collapsed_subscription, { groups: lane.groupCount, resources: lane.resourceCount }),
        lane: lane.subscriptionId,
        memberIds: [],
        count: lane.groupCount,
        findingCount: lane.findingCount,
        groupIds: [],
      })),
  ];
  const mergedLinks = new Map<string, TopologyLink>();
  for (const link of topology.links) {
    const sourceId = nodeIdForGroup(link.sourceId);
    const targetId = nodeIdForGroup(link.targetId);
    if (sourceId === targetId) continue;
    const first = link.kinds[0];
    // An aggregated group link always carries at least one kind; "structure" is
    // the neutral family if a future caller ever hands over an empty set.
    const kindClass: KindClass = first ? edgeKindClass(first) : "structure";
    const key = `${sourceId}\u0000${targetId}\u0000${kindClass}`;
    const current = mergedLinks.get(key);
    const count = (current?.count ?? 0) + link.count;
    mergedLinks.set(key, {
      sourceId,
      targetId,
      label: plural(NODE_WORDS.links, count),
      kindClass,
      count,
    });
  }
  const links = [...mergedLinks.values()].sort((left, right) =>
    left.sourceId.localeCompare(right.sourceId)
      || left.targetId.localeCompare(right.targetId)
      || left.kindClass.localeCompare(right.kindClass),
  );
  const drawn = expandedGroups.filter(connected).length;
  const hiddenUnconnected = showUnconnected ? 0 : unconnectedGroups.length;
  const aggregated = topology.groups.length - drawn - hiddenUnconnected;

  return {
    level: "estate",
    lanes,
    nodes,
    links,
    kindClasses: classCounts(links),
    counts: {
      total: topology.groups.length,
      drawn,
      folded: 0,
      aggregated,
      external: 0,
      hiddenByFilter: hiddenUnconnected,
      totalLinks: topology.links.reduce((total, link) => total + link.count, 0),
      drawnLinks: links.length,
    },
  };
}

function groupGraph(estate: EstateSnapshot, groupId: string): TopologyGraph {
  const topology = buildResourceGroupTopology(estate);
  const group = topology.groups.find((candidate) => candidate.id === groupId);
  if (!group) {
    return {
      level: "group",
      lanes: [],
      nodes: [],
      links: [],
      kindClasses: [],
      counts: { total: 0, drawn: 0, folded: 0, aggregated: 0, external: 0, hiddenByFilter: 0, totalLinks: 0, drawnLinks: 0 },
    };
  }
  const memberIds = new Set(group.resourceIds);
  const members = estate.resources.filter((resource) => memberIds.has(resource.id));
  const byId = new Map(members.map((resource) => [resource.id, resource]));

  // Fold NICs and disks into their VM, mirroring the Rust builder.
  const foldedInto = new Map<string, string>();
  for (const edge of estate.edges) {
    if (edge.kind !== "attached_to") continue;
    const source = byId.get(edge.sourceId);
    const target = byId.get(edge.targetId);
    if (!source || !target) continue;
    const folds =
      (["microsoft.network/networkinterfaces", "microsoft.compute/disks"].includes(source.azureType) &&
        target.azureType === "microsoft.compute/virtualmachines") ||
      (source.azureType.split("/").length > 2 && source.id.startsWith(target.id));
    if (folds) foldedInto.set(source.id, target.id);
  }
  const resolve = (id: string) => {
    let current = id;
    for (let step = 0; step < 4; step += 1) {
      const next = foldedInto.get(current);
      if (!next) break;
      current = next;
    }
    return current;
  };

  const touched = new Set<string>();
  for (const edge of estate.edges) {
    if (memberIds.has(edge.sourceId)) touched.add(edge.sourceId);
    if (memberIds.has(edge.targetId)) touched.add(edge.targetId);
  }
  const foldedHosts = new Map<string, string[]>();
  for (const [child, host] of foldedInto) {
    foldedHosts.set(host, [...(foldedHosts.get(host) ?? []), child]);
  }

  const nodes: TopologyNode[] = [];
  const unconnectedByType = new Map<string, Resource[]>();
  let drawn = 0;
  for (const resource of members) {
    if (foldedInto.has(resource.id)) continue;
    const folded = foldedHosts.get(resource.id) ?? [];
    const connected = touched.has(resource.id) || folded.length > 0;
    if (!connected) {
      unconnectedByType.set(resource.azureType, [...(unconnectedByType.get(resource.azureType) ?? []), resource]);
      continue;
    }
    drawn += 1;
    nodes.push(
      resourceNode(resource, {
        subtitle: folded.length > 0 ? `+${folded.length} attached` : "",
        zone: "core",
        memberIds: folded,
        findingCount:
          resource.findingCount + folded.reduce((total, id) => total + (byId.get(id)?.findingCount ?? 0), 0),
      }),
    );
  }
  let aggregated = 0;
  for (const [azureType, unconnected] of [...unconnectedByType.entries()].sort(([left], [right]) => (left < right ? -1 : 1))) {
    if (unconnected.length === 1) {
      drawn += 1;
      nodes.push(resourceNode(unconnected[0], { zone: "unconnected" }));
      continue;
    }
    aggregated += unconnected.length;
    nodes.push({
      id: `aggregate:${groupId}:${azureType}`,
      kind: "aggregate",
      name: azureType,
      subtitle: fill(NODE_WORDS.times_n, { count: unconnected.length }),
      azureType,
      zone: "unconnected",
      memberIds: unconnected.map((resource) => resource.id),
      count: unconnected.length,
      findingCount: unconnected.reduce((total, resource) => total + resource.findingCount, 0),
      groupIds: [],
    });
  }

  const nodeIds = new Set(nodes.map((node) => node.id));
  const merged = new Map<string, TopologyLink>();
  let totalLinks = 0;
  for (const edge of estate.edges) {
    if (!memberIds.has(edge.sourceId) && !memberIds.has(edge.targetId)) continue;
    totalLinks += 1;
    if (["nic_in_subnet", "in_vnet", "subnet_of"].includes(edge.kind)) continue;
    const source = resolve(edge.sourceId);
    const target = resolve(edge.targetId);
    if (source === target || !nodeIds.has(source) || !nodeIds.has(target)) continue;
    const key = `${source}\0${target}\0${edge.kind}`;
    const existing = merged.get(key);
    if (existing) existing.count += 1;
    else {
      merged.set(key, {
        sourceId: source,
        targetId: target,
        label: kindLabel(edge.kind),
        kindClass: edgeKindClass(edge.kind),
        count: 1,
      });
    }
  }
  const links = [...merged.values()];

  return {
    level: "group",
    lanes: [],
    nodes,
    links,
    kindClasses: classCounts(links),
    counts: {
      total: members.length,
      drawn,
      folded: foldedInto.size,
      aggregated,
      external: 0,
      hiddenByFilter: 0,
      totalLinks,
      drawnLinks: links.length,
    },
  };
}

function neighbourhoodGraph(
  estate: EstateSnapshot,
  resourceId: string,
  depth: number,
  kindClasses: string[],
): TopologyGraph {
  const clampedDepth = Math.min(Math.max(depth, 1), 2);
  const byId = new Map(estate.resources.map((resource) => [resource.id, resource]));
  const subject = byId.get(resourceId);
  if (!subject) {
    return {
      level: "neighbourhood",
      lanes: [],
      nodes: [],
      links: [],
      kindClasses: [],
      counts: { total: 0, drawn: 0, folded: 0, aggregated: 0, external: 0, hiddenByFilter: 0, totalLinks: 0, drawnLinks: 0 },
    };
  }
  const allowed = (kind: EdgeKind) => kindClasses.length === 0 || kindClasses.includes(edgeKindClass(kind));
  const adjacency = new Map<string, Array<{ id: string; kind: EdgeKind }>>();
  for (const edge of estate.edges) {
    if (!byId.has(edge.sourceId) || !byId.has(edge.targetId)) continue;
    adjacency.set(edge.sourceId, [...(adjacency.get(edge.sourceId) ?? []), { id: edge.targetId, kind: edge.kind }]);
    adjacency.set(edge.targetId, [...(adjacency.get(edge.targetId) ?? []), { id: edge.sourceId, kind: edge.kind }]);
  }

  const hops = new Map<string, number>([[resourceId, 0]]);
  const hiddenNeighbours = new Set<string>();
  let frontier = [resourceId];
  for (let hop = 1; hop <= clampedDepth; hop += 1) {
    const next: string[] = [];
    for (const node of frontier) {
      for (const { id, kind } of adjacency.get(node) ?? []) {
        if (hops.has(id)) continue;
        if (!allowed(kind)) {
          hiddenNeighbours.add(id);
          continue;
        }
        hops.set(id, hop);
        next.push(id);
      }
    }
    frontier = next;
  }
  const hiddenByFilter = [...hiddenNeighbours].filter((id) => !hops.has(id)).length;

  const byHopType = new Map<string, string[]>();
  for (const [id, hop] of hops) {
    if (hop === 0) continue;
    const key = `${hop}\0${byId.get(id)?.azureType ?? ""}`;
    byHopType.set(key, [...(byHopType.get(key) ?? []), id]);
  }
  const aggregateOf = new Map<string, string>();
  const nodes: TopologyNode[] = [resourceNode(subject, { hop: 0 })];
  let drawn = 1;
  let aggregated = 0;
  for (const [key, ids] of [...byHopType.entries()].sort(([left], [right]) => (left < right ? -1 : 1))) {
    const [hopText, azureType] = key.split("\0");
    const hop = Number(hopText);
    if (ids.length > NEIGHBOUR_FANOUT_LIMIT) {
      const id = `aggregate:hop${hop}:${azureType}`;
      aggregated += ids.length;
      for (const member of ids) aggregateOf.set(member, id);
      nodes.push({
        id,
        kind: "aggregate",
        name: azureType,
        subtitle: fill(NODE_WORDS.times_n, { count: ids.length }),
        azureType,
        hop,
        memberIds: ids,
        count: ids.length,
        findingCount: ids.reduce((total, member) => total + (byId.get(member)?.findingCount ?? 0), 0),
        groupIds: [],
      });
    } else {
      for (const member of ids.sort()) {
        drawn += 1;
        const resource = byId.get(member);
        if (resource) nodes.push(resourceNode(resource, { hop }));
      }
    }
  }

  const representative = (id: string) => (hops.has(id) ? (aggregateOf.get(id) ?? id) : undefined);
  const merged = new Map<string, TopologyLink>();
  let totalLinks = 0;
  for (const edge of estate.edges) {
    if (!hops.has(edge.sourceId) && !hops.has(edge.targetId)) continue;
    totalLinks += 1;
    const source = representative(edge.sourceId);
    const target = representative(edge.targetId);
    if (!source || !target || source === target || !allowed(edge.kind)) continue;
    const key = `${source}\0${target}\0${edge.kind}`;
    const existing = merged.get(key);
    if (existing) existing.count += 1;
    else {
      merged.set(key, {
        sourceId: source,
        targetId: target,
        label: kindLabel(edge.kind),
        kindClass: edgeKindClass(edge.kind),
        count: 1,
      });
    }
  }
  const links = [...merged.values()].map((link) => ({
    ...link,
    label: link.count > 1 ? fill(NODE_WORDS.label_times, { name: link.label, count: link.count }) : link.label,
  }));

  return {
    level: "neighbourhood",
    lanes: [],
    nodes,
    links,
    kindClasses: classCounts(links),
    counts: {
      total: hops.size,
      drawn,
      folded: 0,
      aggregated,
      external: 0,
      hiddenByFilter,
      totalLinks,
      drawnLinks: links.length,
    },
  };
}

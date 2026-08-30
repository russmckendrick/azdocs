// Browser-only stand-in for the Rust `topology_graph` command, used by the
// illustrative workspace (`npm run dev` outside Tauri). It mirrors the Rust
// contract — every resource drawn, folded or aggregated, with honest counts —
// on the small mock estate; the packaged app always uses the Rust builder.

import type {
  EstateSnapshot,
  Resource,
  TopologyGraph,
  TopologyLane,
  TopologyLink,
  TopologyNode,
  TopologyRequest,
} from "../types";
import { buildResourceGroupTopology, resourceGroupNodeId } from "./topology-model";

const NEIGHBOUR_FANOUT_LIMIT = 6;
const ESTATE_CARD_BUDGET = 24;

export function edgeKindClass(kind: string): string {
  switch (kind) {
    case "peered_with":
    case "nsg_attached":
    case "nic_in_subnet":
    case "subnet_of":
    case "in_vnet":
    case "dns_linked":
      return "network";
    case "private_endpoint_for":
    case "depends_on":
      return "data";
    case "uses_identity":
      return "identity";
    case "logs_to":
      return "monitoring";
    default:
      return "structure";
  }
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
    ...extra,
  };
}

export function buildFallbackTopology(estate: EstateSnapshot, request: TopologyRequest): TopologyGraph {
  const mode = request.mode;
  if (mode.kind === "group") return groupGraph(estate, mode.groupId);
  if (mode.kind === "neighbourhood") {
    return neighbourhoodGraph(estate, mode.resourceId, mode.depth ?? 1, mode.kindClasses ?? []);
  }
  return estateGraph(estate, mode.expandedSubscriptions ?? []);
}

function estateGraph(estate: EstateSnapshot, expandedSubscriptions: string[]): TopologyGraph {
  const topology = buildResourceGroupTopology(estate);
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
  if (expanded.size === 0) {
    let budget = ESTATE_CARD_BUDGET;
    const ranked = [...laneSummaries].sort((left, right) =>
      right.linkCount - left.linkCount
        || right.resourceCount - left.resourceCount
        || (left.name < right.name ? -1 : 1),
    );
    for (const lane of ranked) {
      if (expanded.size === 0 || lane.groupCount <= budget) {
        expanded.add(lane.subscriptionId);
        budget = Math.max(0, budget - lane.groupCount);
      }
    }
  }
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

  const nodes: TopologyNode[] = [
    ...topology.groups
      .filter((group) => expanded.has(group.subscriptionId))
      .map((group) => ({
        id: resourceGroupNodeId(group.id),
        kind: "resource-group" as const,
        name: group.name,
        subtitle: `${group.resourceCount} resources`,
        lane: group.subscriptionId,
        memberIds: group.resourceIds,
        count: group.resourceCount,
        findingCount: group.findingCount,
        groupId: group.id,
      })),
    ...lanes
      .filter((lane) => !lane.expanded)
      .map((lane) => ({
        id: `subscription:${lane.subscriptionId}`,
        kind: "subscription" as const,
        name: lane.name,
        subtitle: `${lane.groupCount} groups · ${lane.resourceCount} resources`,
        lane: lane.subscriptionId,
        memberIds: [],
        count: lane.groupCount,
        findingCount: lane.findingCount,
      })),
  ];
  const mergedLinks = new Map<string, TopologyLink>();
  for (const link of topology.links) {
    const sourceId = nodeIdForGroup(link.sourceId);
    const targetId = nodeIdForGroup(link.targetId);
    if (sourceId === targetId) continue;
    const kindClass = edgeKindClass(link.kinds[0] ?? "");
    const key = `${sourceId}\u0000${targetId}\u0000${kindClass}`;
    const current = mergedLinks.get(key);
    const count = (current?.count ?? 0) + link.count;
    mergedLinks.set(key, {
      sourceId,
      targetId,
      label: `${count} link${count === 1 ? "" : "s"}`,
      kindClass,
      count,
    });
  }
  const links = [...mergedLinks.values()].sort((left, right) =>
    left.sourceId.localeCompare(right.sourceId)
      || left.targetId.localeCompare(right.targetId)
      || left.kindClass.localeCompare(right.kindClass),
  );
  const drawn = lanes.filter((lane) => lane.expanded).reduce((total, lane) => total + lane.groupCount, 0);
  const aggregated = topology.groups.length - drawn;

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
      hiddenByFilter: 0,
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
      subtitle: `×${unconnected.length}`,
      azureType,
      zone: "unconnected",
      memberIds: unconnected.map((resource) => resource.id),
      count: unconnected.length,
      findingCount: unconnected.reduce((total, resource) => total + resource.findingCount, 0),
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
        label: edge.kind.replaceAll("_", " "),
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
  const allowed = (kind: string) => kindClasses.length === 0 || kindClasses.includes(edgeKindClass(kind));
  const adjacency = new Map<string, Array<{ id: string; kind: string }>>();
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
        subtitle: `×${ids.length}`,
        azureType,
        hop,
        memberIds: ids,
        count: ids.length,
        findingCount: ids.reduce((total, member) => total + (byId.get(member)?.findingCount ?? 0), 0),
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
        label: edge.kind.replaceAll("_", " "),
        kindClass: edgeKindClass(edge.kind),
        count: 1,
      });
    }
  }
  const links = [...merged.values()].map((link) => ({
    ...link,
    label: link.count > 1 ? `${link.label} ×${link.count}` : link.label,
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

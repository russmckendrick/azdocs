import type { Edge, EstateSnapshot, Resource, ResourceGroup } from "../types";

export interface ResourceTypeCount {
  azureType: string;
  count: number;
}

export interface ResourceGroupSummary {
  id: string;
  name: string;
  subscriptionId: string;
  subscriptionName: string;
  location?: string;
  resourceIds: string[];
  resourceCount: number;
  findingCount: number;
  internalLinkCount: number;
  externalLinkCount: number;
  connectedGroupCount: number;
  resourceTypes: ResourceTypeCount[];
}

export interface ResourceGroupLink {
  sourceId: string;
  targetId: string;
  count: number;
  kinds: string[];
}

export interface ResourceGroupTopology {
  groups: ResourceGroupSummary[];
  links: ResourceGroupLink[];
  resourceGroupByResourceId: Map<string, string>;
}

function stableCompare(left: string, right: string) {
  return left < right ? -1 : left > right ? 1 : 0;
}

function groupMatchKey(subscriptionId: string, name: string) {
  return `${subscriptionId.toLowerCase()}\0${name.toLowerCase()}`;
}

function syntheticGroup(resource: Resource): ResourceGroup {
  const name = resource.resourceGroup || "Subscription scope";
  return {
    id: resource.resourceGroup
      ? `/subscriptions/${resource.subscriptionId}/resourcegroups/${resource.resourceGroup}`.toLowerCase()
      : `subscription-scope:${resource.subscriptionId}`.toLowerCase(),
    name,
    subscriptionId: resource.subscriptionId,
    location: resource.location,
  };
}

export function resourceGroupNodeId(groupId: string) {
  return `resource-group:${groupId}`;
}

export function buildResourceGroupTopology(estate: EstateSnapshot): ResourceGroupTopology {
  const groupsByKey = new Map<string, ResourceGroup>();
  for (const group of estate.resourceGroups) {
    groupsByKey.set(groupMatchKey(group.subscriptionId, group.name), group);
  }
  for (const resource of estate.resources) {
    const key = groupMatchKey(resource.subscriptionId, resource.resourceGroup ?? "Subscription scope");
    if (!groupsByKey.has(key)) groupsByKey.set(key, syntheticGroup(resource));
  }

  const subscriptionNames = new Map(
    estate.subscriptions.map((subscription) => [subscription.id, subscription.displayName]),
  );
  const resourcesByGroup = new Map<string, Resource[]>();
  const groupIdByKey = new Map<string, string>();
  for (const group of groupsByKey.values()) {
    const key = groupMatchKey(group.subscriptionId, group.name);
    groupIdByKey.set(key, group.id);
    resourcesByGroup.set(group.id, []);
  }

  const resourceGroupByResourceId = new Map<string, string>();
  for (const resource of estate.resources) {
    const key = groupMatchKey(resource.subscriptionId, resource.resourceGroup ?? "Subscription scope");
    const groupId = groupIdByKey.get(key);
    if (!groupId) continue;
    resourceGroupByResourceId.set(resource.id, groupId);
    resourcesByGroup.get(groupId)?.push(resource);
  }

  const internalLinks = new Map<string, number>();
  const externalLinks = new Map<string, number>();
  const connectedGroups = new Map<string, Set<string>>();
  const aggregatedLinks = new Map<string, { sourceId: string; targetId: string; count: number; kinds: Set<string> }>();
  for (const edge of estate.edges) {
    const sourceGroupId = resourceGroupByResourceId.get(edge.sourceId);
    const targetGroupId = resourceGroupByResourceId.get(edge.targetId);
    if (!sourceGroupId || !targetGroupId) continue;
    if (sourceGroupId === targetGroupId) {
      internalLinks.set(sourceGroupId, (internalLinks.get(sourceGroupId) ?? 0) + 1);
      continue;
    }
    externalLinks.set(sourceGroupId, (externalLinks.get(sourceGroupId) ?? 0) + 1);
    externalLinks.set(targetGroupId, (externalLinks.get(targetGroupId) ?? 0) + 1);
    if (!connectedGroups.has(sourceGroupId)) connectedGroups.set(sourceGroupId, new Set());
    if (!connectedGroups.has(targetGroupId)) connectedGroups.set(targetGroupId, new Set());
    connectedGroups.get(sourceGroupId)?.add(targetGroupId);
    connectedGroups.get(targetGroupId)?.add(sourceGroupId);
    const key = `${sourceGroupId}\0${targetGroupId}`;
    const aggregate = aggregatedLinks.get(key) ?? {
      sourceId: sourceGroupId,
      targetId: targetGroupId,
      count: 0,
      kinds: new Set<string>(),
    };
    aggregate.count += 1;
    aggregate.kinds.add(edge.kind);
    aggregatedLinks.set(key, aggregate);
  }

  const groups = [...groupsByKey.values()].map((group): ResourceGroupSummary => {
    const resources = [...(resourcesByGroup.get(group.id) ?? [])].sort((left, right) =>
      stableCompare(left.name, right.name) || stableCompare(left.id, right.id),
    );
    const typeCounts = new Map<string, number>();
    for (const resource of resources) {
      typeCounts.set(resource.azureType, (typeCounts.get(resource.azureType) ?? 0) + 1);
    }
    const resourceTypes = [...typeCounts.entries()]
      .map(([azureType, count]) => ({ azureType, count }))
      .sort((left, right) =>
        right.count - left.count || stableCompare(left.azureType, right.azureType),
      );
    return {
      id: group.id,
      name: group.name,
      subscriptionId: group.subscriptionId,
      subscriptionName: subscriptionNames.get(group.subscriptionId) ?? group.subscriptionId,
      location: group.location,
      resourceIds: resources.map((resource) => resource.id),
      resourceCount: resources.length,
      findingCount: resources.reduce((total, resource) => total + resource.findingCount, 0),
      internalLinkCount: internalLinks.get(group.id) ?? 0,
      externalLinkCount: externalLinks.get(group.id) ?? 0,
      connectedGroupCount: connectedGroups.get(group.id)?.size ?? 0,
      resourceTypes,
    };
  }).sort((left, right) =>
    stableCompare(left.subscriptionName, right.subscriptionName)
      || stableCompare(left.name, right.name)
      || stableCompare(left.id, right.id),
  );

  const links = [...aggregatedLinks.values()]
    .map((link): ResourceGroupLink => ({
      sourceId: link.sourceId,
      targetId: link.targetId,
      count: link.count,
      kinds: [...link.kinds].sort(stableCompare),
    }))
    .sort((left, right) =>
      stableCompare(left.sourceId, right.sourceId)
        || stableCompare(left.targetId, right.targetId),
    );

  return { groups, links, resourceGroupByResourceId };
}

export function resourcesInGroup(estate: EstateSnapshot, group?: ResourceGroupSummary) {
  if (!group) return [];
  const ids = new Set(group.resourceIds);
  return estate.resources
    .filter((resource) => ids.has(resource.id))
    .sort((left, right) => stableCompare(left.name, right.name) || stableCompare(left.id, right.id));
}

export function edgesWithinResources(edges: Edge[], resources: Resource[]) {
  const ids = new Set(resources.map((resource) => resource.id));
  return edges.filter((edge) => ids.has(edge.sourceId) && ids.has(edge.targetId));
}

import type { TopologyGraph } from "../types";

/** What happened to a run of items that are not drawn cards; the view maps it to `desktop.topology.counts`. */
export type CountsKind = "folded" | "collapsed" | "unconnected" | "tiles" | "external" | "hidden";

export interface CountsExtra {
  count: number;
  kind: CountsKind;
  emphasis?: boolean;
}

/**
 * The status rail's plain-language account of everything that is not a
 * drawn card. It has to add up: the counts contract is
 * `drawn + folded + aggregated == total`, and every aggregated item is named
 * by what happened to it (a collapsed subscription, an unconnected tile, a
 * ×N tile) rather than by the word "aggregated". Kinds, not phrases: the
 * wording lives in the labels file and this module stays pure.
 */
export function describeCounts(graph: TopologyGraph): { extras: CountsExtra[] } {
  const { counts } = graph;
  const extras: CountsExtra[] = [];
  if (counts.folded > 0) extras.push({ count: counts.folded, kind: "folded" });
  let accounted = 0;
  if (graph.level === "estate") {
    const collapsed = graph.nodes
      .filter((node) => node.kind === "subscription")
      .reduce((total, node) => total + node.count, 0);
    const unconnected = graph.nodes
      .filter((node) => node.kind === "aggregate" && node.groupIds.length > 0)
      .reduce((total, node) => total + node.count, 0);
    if (collapsed > 0) extras.push({ count: collapsed, kind: "collapsed" });
    if (unconnected > 0) extras.push({ count: unconnected, kind: "unconnected" });
    accounted = collapsed + unconnected;
  } else if (graph.level === "group") {
    const unconnected = graph.nodes
      .filter((node) => node.kind === "aggregate" && node.zone === "unconnected")
      .reduce((total, node) => total + node.count, 0);
    if (unconnected > 0) extras.push({ count: unconnected, kind: "unconnected" });
    accounted = unconnected;
  }
  const remainder = counts.aggregated - accounted;
  if (remainder > 0) extras.push({ count: remainder, kind: "tiles" });
  if (counts.external > 0) extras.push({ count: counts.external, kind: "external" });
  if (counts.hiddenByFilter > 0) extras.push({ count: counts.hiddenByFilter, kind: "hidden", emphasis: true });
  return { extras };
}

export interface RefreshPresentation {
  stale: boolean;
  error?: { message: string; hasPrevious: boolean };
}

export function refreshStarted(hasGraph: boolean): RefreshPresentation {
  return { stale: hasGraph };
}

export function refreshSucceeded(): RefreshPresentation {
  return { stale: false };
}

export function refreshFailed(hasGraph: boolean, message: string): RefreshPresentation {
  return { stale: hasGraph, error: { message, hasPrevious: hasGraph } };
}

export function toggleSubscriptionLane(
  override: readonly string[] | undefined,
  drawnExpanded: readonly string[],
  subscriptionId: string,
) {
  const source = override ?? drawnExpanded;
  return source.includes(subscriptionId)
    ? source.filter((id) => id !== subscriptionId)
    : [...source, subscriptionId].sort();
}

/** The trail's subscription crumb: make sure that lane is expanded, keep the rest. */
export function expandSubscriptionLane(
  override: readonly string[] | undefined,
  drawnExpanded: readonly string[],
  subscriptionId: string,
) {
  const source = override ?? drawnExpanded;
  return source.includes(subscriptionId) ? [...source] : [...source, subscriptionId].sort();
}

export type AggregateActivation =
  | { kind: "open-resource"; resourceId: string }
  | { kind: "open-group"; groupId: string }
  | { kind: "toggle"; expandedId?: string };

export function resolveAggregateActivation(
  expandedId: string | undefined,
  nodeId: string,
  memberIds: readonly string[],
  groupIds: readonly string[] = [],
): AggregateActivation {
  // An estate-level tile aggregates groups, not resources: a lone member opens
  // its group map, the way a lone resource member opens its record.
  if (groupIds.length === 1) {
    return { kind: "open-group", groupId: groupIds[0] };
  }
  if (groupIds.length === 0 && memberIds.length === 1) {
    return { kind: "open-resource", resourceId: memberIds[0] };
  }
  return { kind: "toggle", expandedId: expandedId === nodeId ? undefined : nodeId };
}

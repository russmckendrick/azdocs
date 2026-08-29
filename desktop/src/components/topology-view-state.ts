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

export type AggregateActivation =
  | { kind: "open-resource"; resourceId: string }
  | { kind: "toggle"; expandedId?: string };

export function resolveAggregateActivation(
  expandedId: string | undefined,
  nodeId: string,
  memberIds: readonly string[],
): AggregateActivation {
  if (memberIds.length === 1) {
    return { kind: "open-resource", resourceId: memberIds[0] };
  }
  return { kind: "toggle", expandedId: expandedId === nodeId ? undefined : nodeId };
}

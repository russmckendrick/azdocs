import { SEVERITIES, severityRank, stableCompare } from "../ordering";
import type {
  AppBootstrap,
  DashboardFilter,
  EstateSnapshot,
  Finding,
  Resource,
  SnapshotComparison,
} from "../types";

export function resourceMatchesDashboard(
  resource: Resource,
  filter?: DashboardFilter,
) {
  return (
    !filter ||
    ((!filter.subscriptionId ||
      resource.subscriptionId === filter.subscriptionId) &&
      (!filter.azureType || resource.azureType === filter.azureType) &&
      (filter.location === undefined ||
        (resource.location ?? "") === filter.location) &&
      (!filter.resourceIds || filter.resourceIds.includes(resource.id)))
  );
}

export function findingMatchesDashboard(
  finding: Finding,
  estate: EstateSnapshot,
  filter?: DashboardFilter,
) {
  return (
    !filter ||
    ((!filter.severity || finding.severity === filter.severity) &&
      (!filter.queryName || finding.queryName === filter.queryName) &&
      (!filter.category || finding.category === filter.category) &&
      (!filter.subscriptionId ||
        estate.resources.some(
          (resource) =>
            resource.id === finding.resourceId &&
            resource.subscriptionId === filter.subscriptionId,
        )) &&
      (!filter.resourceIds ||
        (!!finding.resourceId &&
          filter.resourceIds.includes(finding.resourceId))))
  );
}

export function dashboardData(estate: EstateSnapshot, subscriptionId?: string) {
  const resources = estate.resources.filter((resource) =>
    resourceMatchesDashboard(resource, { subscriptionId }),
  );
  const ids = new Set(resources.map((resource) => resource.id));
  // Findings without a resource retain estate scope; they cannot be assigned to a subscription.
  const findings = estate.findings.filter(
    (finding) =>
      !subscriptionId || (!!finding.resourceId && ids.has(finding.resourceId)),
  );
  const edges = estate.edges.filter(
    (edge) =>
      !subscriptionId || ids.has(edge.sourceId) || ids.has(edge.targetId),
  );
  const tagged = resources.filter(
    (resource) => Object.keys(resource.tags ?? {}).length > 0,
  );
  const types = estate.resourceTypes
    .map((type) => ({
      ...type,
      count: resources.filter(
        (resource) => resource.azureType === type.azureType,
      ).length,
    }))
    .filter((type) => type.count > 0)
    .sort(
      (a, b) => b.count - a.count || stableCompare(a.azureType, b.azureType),
    );
  const locationCounts = new Map<string, number>();
  for (const resource of resources)
    locationCounts.set(
      resource.location ?? "",
      (locationCounts.get(resource.location ?? "") ?? 0) + 1,
    );
  const locations = [...locationCounts]
    .map(([name, count]) => ({ name, count }))
    .sort((a, b) => b.count - a.count || stableCompare(a.name, b.name));
  const severity = Object.fromEntries(
    SEVERITIES.map((level) => [
      level,
      findings.filter((finding) => finding.severity === level).length,
    ]),
  ) as EstateSnapshot["severityCounts"];
  const checks = new Map<
    string,
    { queryName: string; severity: Finding["severity"]; count: number }
  >();
  for (const finding of findings) {
    const key = `${finding.queryName}:${finding.severity}`;
    const check = checks.get(key);
    if (check) check.count += 1;
    else
      checks.set(key, {
        queryName: finding.queryName,
        severity: finding.severity,
        count: 1,
      });
  }
  return {
    resources,
    findings,
    edges,
    tagged,
    types,
    locations,
    severity,
    groups: estate.resourceGroupSummaries.filter(
      (group) => !subscriptionId || group.subscriptionId === subscriptionId,
    ).length,
    percent: subscriptionId
      ? (estate.governance.subscriptions.find(
          (subscription) => subscription.subscriptionId === subscriptionId,
        )?.percent ?? 0)
      : estate.tagCoverage.percent,
    checks: [...checks.values()].sort(
      (a, b) =>
        severityRank(a.severity) - severityRank(b.severity) ||
        b.count - a.count ||
        stableCompare(a.queryName, b.queryName),
    ),
  };
}

export function dashboardHistory(
  bootstrap: AppBootstrap,
  estate: EstateSnapshot,
  days: number,
) {
  const end = Date.parse(estate.createdAt);
  return bootstrap.snapshots
    .filter(
      (snapshot) =>
        snapshot.status !== "running" &&
        snapshot.tenantId === estate.tenantId &&
        Date.parse(snapshot.createdAt) <= end &&
        (!days || Date.parse(snapshot.createdAt) >= end - days * 86400000),
    )
    .sort(
      (a, b) =>
        stableCompare(a.createdAt, b.createdAt) || stableCompare(a.id, b.id),
    );
}

export function dashboardComparison(
  bootstrap: AppBootstrap,
  estate: EstateSnapshot,
  diff: SnapshotComparison | undefined,
) {
  if (diff && diff.targetSnapshotId !== estate.id) return undefined;
  const base = bootstrap.snapshots.find(
    (snapshot) => snapshot.id === diff?.baseSnapshotId,
  );
  return diff &&
    base?.status === "complete" &&
    estate.status === "complete" &&
    base.tenantId === estate.tenantId
    ? { diff, base }
    : undefined;
}

export function queryCoverage(estate: EstateSnapshot) {
  const categories = [
    ...new Set(estate.queryRuns.map((run) => run.category)),
  ].sort(stableCompare);
  return categories.map((category) => {
    const runs = estate.queryRuns.filter((run) => run.category === category);
    return {
      category,
      runs,
      succeeded: runs.filter((run) => !run.error && run.rowCount != null)
        .length,
      total: runs.length,
    };
  });
}

/** A padded, whole-resource axis makes small changes legible without clipping extrema. */
export function resourceHistoryScale(values: number[]) {
  const valid = values.filter(value => Number.isFinite(value) && value >= 0);
  if (!valid.length) return { min: 0, max: 1, ticks: [0, 1] };
  const low = Math.min(...valid);
  const high = Math.max(...valid);
  const padding = Math.max(1, (high - low) * 0.15, high * 0.005);
  const roughStep = Math.max(1, (high - low + padding * 2) / 4);
  const magnitude = 10 ** Math.floor(Math.log10(roughStep));
  const multiple = [1, 2, 5, 10].find(value => value * magnitude >= roughStep) ?? 10;
  const step = multiple * magnitude;
  const min = Math.max(0, Math.floor((low - padding) / step) * step);
  const max = Math.max(min + step, Math.ceil((high + padding) / step) * step);
  const ticks = Array.from({ length: Math.round((max - min) / step) + 1 }, (_, index) => min + index * step);
  return { min, max, ticks };
}

import type { QueryDefMeta, QueryRun, Resource } from "../types";

type Row = Record<string, unknown>;

/**
 * Columns every stored row repeats that the resource beside it already shows.
 * `id` is always dropped: for the resource itself it duplicates the name, and
 * for child rows (subnets, peerings) it is a long ARM path the name says better.
 */
const REPEATED = ["id", "subscriptionId", "resourceGroup", "location", "type"];

/**
 * The inventory queries whose rows describe `azureType`, the pack's default
 * table first. Only queries that stored rows for this snapshot are offered, so
 * a column choice never opens onto an empty table.
 */
export function typeQueries(
  pack: QueryDefMeta[],
  runs: QueryRun[],
  azureType: string,
): QueryDefMeta[] {
  const stored = new Set(
    runs
      .filter((run) => !run.error && (run.rowCount ?? 0) > 0)
      .map((run) => run.queryName),
  );
  return pack
    .filter(
      (def) =>
        def.kind === "inventory" &&
        def.resourceColumn &&
        def.resourceTypes.includes(azureType) &&
        stored.has(def.name),
    )
    .sort(
      (a, b) =>
        Number(b.resourceTable) - Number(a.resourceTable) ||
        a.name.localeCompare(b.name),
    );
}

/**
 * The columns worth a cell once rows sit under their resource: never the join
 * column, and not the row's own name when the row *is* the resource.
 */
export function detailColumns(columns: string[], resourceColumn: string) {
  const hidden = new Set([
    ...REPEATED,
    resourceColumn,
    ...(resourceColumn === "id" ? ["name"] : []),
  ]);
  return columns.filter((column) => !hidden.has(column));
}

export interface JoinedRow {
  resource: Resource;
  /** Absent when no stored row describes the resource. */
  row?: Row;
  /** The first line for its resource; later lines are the same resource's children. */
  first: boolean;
}

/**
 * Rows placed under the resources they describe, in the resources' order.
 * ARG casing is inconsistent, so keys compare lowercased (resource ids already
 * are). A resource with no row keeps one empty line, after the rest, so the
 * table still accounts for every resource in view; rows for resources outside
 * the view are left out.
 */
export function joinRows(resources: Resource[], rows: Row[], column: string) {
  const byResource = new Map<string, Row[]>();
  for (const row of rows) {
    const key = String(row[column] ?? "").toLowerCase();
    if (!key) continue;
    const bucket = byResource.get(key);
    if (bucket) bucket.push(row);
    else byResource.set(key, [row]);
  }
  const joined: JoinedRow[] = [];
  const missing: JoinedRow[] = [];
  for (const resource of resources) {
    const matches = byResource.get(resource.id);
    if (!matches) missing.push({ resource, first: true });
    else matches.forEach((row, index) => joined.push({ resource, row, first: index === 0 }));
  }
  return { rows: [...joined, ...missing], missing: missing.length };
}

export function cellText(value: unknown, none: string): string {
  if (value === null || value === undefined || value === "") return none;
  if (Array.isArray(value) && value.every((item) => typeof item !== "object"))
    return value.length ? value.join(", ") : none;
  if (typeof value === "object") return JSON.stringify(value);
  return String(value);
}

export function isMachineShaped(column: string) {
  const needle = column.toLowerCase();
  return (
    needle === "location" ||
    needle.endsWith("id") ||
    needle.includes("version") ||
    needle.includes("address") ||
    needle.includes("prefix")
  );
}

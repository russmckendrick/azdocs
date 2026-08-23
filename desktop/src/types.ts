export type ViewId = "estate" | "topology" | "findings" | "history";
export type Severity = "high" | "medium" | "low" | "info";

export interface AppBootstrap {
  databasePath: string;
  configPath: string;
  configFound: boolean;
  hasCredentials: boolean;
  snapshots: SnapshotSummary[];
  latestSnapshotId?: string;
}

export interface SnapshotSummary {
  id: string;
  createdAt: string;
  tenantId: string;
  status: "running" | "complete" | "partial" | "failed";
  notes?: string;
  subscriptions: number;
  resources: number;
  findings: number;
}

export interface Totals {
  subscriptions: number;
  resourceGroups: number;
  resources: number;
  findings: number;
}

export interface ResourceType {
  azureType: string;
  displayName: string;
  count: number;
  icon: string;
  color: string;
}

export interface Subscription {
  id: string;
  displayName: string;
  state?: string;
  tags?: unknown;
}

export interface ResourceGroup {
  id: string;
  name: string;
  subscriptionId: string;
  location?: string;
  tags?: unknown;
}

export interface Resource {
  id: string;
  displayId: string;
  name: string;
  azureType: string;
  kind?: string;
  location?: string;
  resourceGroup?: string;
  subscriptionId: string;
  tags?: Record<string, unknown>;
  sku?: unknown;
  identity?: unknown;
  properties?: Record<string, unknown>;
  findingCount: number;
  edgeCount: number;
}

export interface Finding {
  queryName: string;
  category: string;
  severity: Severity;
  resourceId?: string;
  title: string;
  detail?: unknown;
}

export interface Edge {
  sourceId: string;
  targetId: string;
  kind: string;
  properties?: unknown;
}

export interface QueryRun {
  queryName: string;
  category: string;
  rowCount?: number;
  durationMs?: number;
  error?: string;
}

export interface SnapshotComparison {
  baseSnapshotId: string;
  targetSnapshotId: string;
  added: string[];
  removed: string[];
  changed: string[];
}

export interface EstateSnapshot {
  id: string;
  createdAt: string;
  tenantId: string;
  status: string;
  notes?: string;
  totals: Totals;
  tagCoverage: { tagged: number; untagged: number; percent: number };
  severityCounts: Record<Severity, number>;
  subscriptions: Subscription[];
  resourceGroups: ResourceGroup[];
  resources: Resource[];
  resourceTypes: ResourceType[];
  locations: Array<{ name: string; count: number }>;
  findings: Finding[];
  edges: Edge[];
  queryRuns: QueryRun[];
  previousDiff?: SnapshotComparison;
}

export interface ScopeSelection {
  subscriptionId?: string;
  resourceGroup?: string;
}

export type CollectionEvent =
  | { event: "phase"; data: { message: string } }
  | { event: "complete"; data: { snapshotId: string } }
  | { event: "failed"; data: { message: string } };

export interface CollectResult {
  snapshotId: string;
  status: string;
  queriesRun: number;
  queriesFailed: number;
  rowsIngested: number;
}

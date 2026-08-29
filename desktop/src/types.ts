export type ViewId =
  | "overview"
  | "estate"
  | "topology"
  | "inventory"
  | "findings"
  | "governance"
  | "history"
  | "settings";
export type Severity = "high" | "medium" | "low" | "info";
export type ThemePreference = "system" | "light" | "dark";

export interface AppBootstrap {
  databasePath: string;
  configPath: string;
  configFound: boolean;
  hasCredentials: boolean;
  requiredTags: string[];
  snapshots: SnapshotSummary[];
  latestSnapshotId?: string;
}

export interface QueryDefMeta {
  name: string;
  category: string;
  kind: "inventory" | "finding";
  description: string;
}

export interface QueryRows {
  queryName: string;
  columns: string[];
  rows: Array<Record<string, unknown>>;
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

export type TopologyNodeKind =
  | "resource"
  | "resource-group"
  | "subscription"
  | "vnet"
  | "subnet"
  | "aggregate"
  | "external";

export interface TopologyLane {
  subscriptionId: string;
  name: string;
  expanded: boolean;
  groupCount: number;
  resourceCount: number;
  findingCount: number;
}

export interface TopologyNode {
  id: string;
  kind: TopologyNodeKind;
  name: string;
  subtitle: string;
  azureType?: string;
  lane?: string;
  parentId?: string;
  zone?: "core" | "unconnected" | "external";
  hop?: number;
  memberIds: string[];
  count: number;
  findingCount: number;
  resourceId?: string;
  groupId?: string;
}

export interface TopologyLink {
  sourceId: string;
  targetId: string;
  label: string;
  kindClass: string;
  count: number;
}

export interface TopologyCounts {
  total: number;
  drawn: number;
  folded: number;
  aggregated: number;
  external: number;
  hiddenByFilter: number;
  totalLinks: number;
  drawnLinks: number;
}

export interface TopologyGraph {
  level: "estate" | "group" | "neighbourhood";
  lanes: TopologyLane[];
  nodes: TopologyNode[];
  links: TopologyLink[];
  kindClasses: Array<{ class: string; count: number }>;
  counts: TopologyCounts;
}

export type TopologyMode =
  | { kind: "estate"; expandedSubscriptions?: string[] }
  | { kind: "group"; groupId: string }
  | { kind: "neighbourhood"; resourceId: string; depth?: number; kindClasses?: string[] };

export interface TopologyScope {
  subscriptions?: string[];
  azureTypes?: string[];
  showUnconnected?: boolean;
}

export interface TopologyRequest {
  snapshotId?: string;
  mode: TopologyMode;
  scope?: TopologyScope;
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

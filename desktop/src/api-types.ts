/**
 * Closed string sets that cross the Tauri IPC boundary.
 *
 * Rust models each of these as an enum but serialises it through `as_str()`, so
 * the DTO field is a bare `String` and ts-rs cannot infer the union. The Rust
 * structs carry `#[ts(type = "...")]` pointing at the names below, so
 * `generated.ts` stays precise without the value sets being invented here.
 *
 * These are the only hand-written parts of the wire contract. Everything else in
 * `generated.ts` comes from `cargo test -p azdocs-desktop`.
 */

/** `Severity` in src/model/mod.rs, ordered high-first. */
export type Severity = "high" | "medium" | "low" | "info";

/**
 * `SnapshotStatus` in src/model/mod.rs.
 *
 * `api-types.test.ts` reads the Rust source and fails if this union and
 * `SnapshotStatus::as_str` fall out of step.
 */
export type SnapshotStatus =
  "running" | "complete" | "warnings" | "partial" | "failed" | "cancelled";

/** `CoverStyle`, `TableStyle` and `StatStyle` in src/report/theme/mod.rs. */
export type ReportCoverStyle = "band" | "editorial" | "block";
export type ReportTableStyle = "solid-header" | "hairline" | "banded";
export type ReportStatStyle = "card" | "outline" | "bare";

/** `QueryKind` in src/querypack/mod.rs. */
export type QueryKind = "inventory" | "finding";

/**
 * `EdgeKind` in src/model/mod.rs.
 *
 * `topology-fallback.test.ts` reads the Rust source and fails if this list and
 * the `kind_class` mapping fall out of step — the mirror is not compiler-checked
 * across the language boundary, and it had already drifted once.
 */
export type EdgeKind =
  | "subnet_of"
  | "in_vnet"
  | "peered_with"
  | "nsg_attached"
  | "nic_in_subnet"
  | "attached_to"
  | "private_endpoint_for"
  | "dns_linked"
  | "depends_on"
  | "runs_on"
  | "uses_identity"
  | "logs_to"
  | "monitors";

/** The coarse family an edge kind belongs to — `kind_class` in topology.rs. */
export type KindClass =
  "network" | "structure" | "data" | "identity" | "monitoring";

/** The `kind` discriminator on a topology node. */
export type TopologyNodeKind =
  | "resource"
  | "resource-group"
  | "subscription"
  | "vnet"
  | "subnet"
  | "aggregate"
  | "external";

/** Which shelf the group view lays a node in. */
export type TopologyZone = "core" | "unconnected" | "external";

/** The scope a topology graph was built at. */
export type TopologyLevel = "estate" | "group" | "neighbourhood";

/** `ExportKind` — which family of outputs the Exports workspace writes. */
export type ExportKind = "reports" | "diagrams";

/** `ReportFormat` in src/cli.rs. */
export type ReportExportFormat =
  "md" | "html" | "csv" | "xlsx" | "pdf" | "docx";

/** `DiagramType` in src/cli.rs. */
export type DiagramExportType =
  | "hierarchy"
  | "resources"
  | "network"
  | "vnets"
  | "resource-groups"
  | "workbook";

/** `DiagramFormat` in src/cli.rs. */
export type DiagramExportFormat = "drawio" | "mermaid" | "svg" | "png";

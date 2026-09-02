//! The typed shape of `data/labels/*.toml`. Every struct rejects unknown keys
//! and has no defaults: the built-in file is complete (a unit test proves it),
//! and a user file is deep-merged over it before deserialising, so a missing
//! key can only mean a hole in the built-in and a surplus key is a typo.
//!
//! Leaves are `String`, [`Plural`] or a flat string map — never `Option` or
//! `Vec` — because the desktop derives its TypeScript type from the
//! serialised JSON, where those shapes lose information.

use serde::{Deserialize, Serialize};

/// Singular and plural forms of a noun that follows a count.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plural {
    pub one: String,
    pub other: String,
}

impl Plural {
    /// The form for `count`: `one` for exactly one, `other` otherwise.
    pub fn pick(&self, count: usize) -> &str {
        if count == 1 { &self.one } else { &self.other }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Labels {
    pub common: CommonLabels,
    pub report: ReportLabels,
    pub diagram: DiagramLabels,
    pub cli: CliLabels,
    pub tui: TuiLabels,
    pub desktop: DesktopLabels,
}

// ------------------------------------------------------------------ common --

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommonLabels {
    pub subscription_scope: String,
    /// How a count and its noun are joined: `{count} {noun}`.
    pub counted: String,
    pub plurals: Plurals,
    pub severity: Severities,
    pub cover: CoverLabels,
    pub verdict: VerdictLabels,
    pub columns: ColumnLabels,
    pub governance: GovernanceLabels,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plurals {
    pub resource: Plural,
    pub finding: Plural,
    pub relationship: Plural,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Severities {
    pub high: SeverityLabels,
    pub medium: SeverityLabels,
    pub low: SeverityLabels,
    pub info: SeverityLabels,
}

impl Severities {
    /// Labels for a stored severity key; unknown keys read as `info`, the
    /// same fallback the theme palette applies.
    pub fn get(&self, severity: &str) -> &SeverityLabels {
        match severity {
            "high" => &self.high,
            "medium" => &self.medium,
            "low" => &self.low,
            _ => &self.info,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeverityLabels {
    /// Inline, lowercase: "3 high".
    pub name: String,
    /// Stat and badge label: "High".
    pub label: String,
    /// Section heading: "High priority".
    pub heading: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverLabels {
    pub tenant: String,
    pub snapshot: String,
    pub collected: String,
    pub status: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerdictLabels {
    pub healthy: String,
    pub below_threshold: String,
    pub called_out: String,
    pub none: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ColumnLabels {
    pub resource_group: String,
    pub subscription: String,
    pub subscriptions: String,
    pub resource_groups: String,
    pub resources: String,
    pub findings: String,
    pub non_compliant: String,
    pub missed_tags: String,
    pub tag_key: String,
    pub tag_coverage: String,
    pub distinct_keys: String,
    pub share: String,
    pub coverage: String,
    pub status: String,
    pub severity: String,
    pub title: String,
    pub category: String,
    pub check: String,
    pub resource: String,
    pub name: String,
    pub r#type: String,
    pub azure_type: String,
    pub kind: String,
    pub location: String,
    pub tags: String,
    pub count: String,
    pub metric: String,
    pub setting: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GovernanceLabels {
    pub coverage_by_key: String,
    pub coverage_by_subscription: String,
    pub least_compliant: String,
    pub no_tags: String,
    pub subscription_note: String,
    pub key_share: String,
    pub non_compliant_sentence: String,
}

// ------------------------------------------------------------------ report --

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportLabels {
    pub toc_title: String,
    pub summary: SummaryLabels,
    pub findings: FindingsLabels,
    pub governance: ReportGovernanceLabels,
    pub overview: ChapterLabels,
    pub type_index: ChapterLabels,
    pub estate: EstateLabels,
    pub evidence: EvidenceLabels,
    pub pdf: PdfLabels,
    pub markdown: MarkdownLabels,
    pub html: HtmlLabels,
    pub xlsx: XlsxLabels,
    pub csv: CsvLabels,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SummaryLabels {
    pub chapter: String,
    pub sentence: String,
    pub coverage_at_or_above: String,
    pub coverage_below: String,
    pub largest_types: String,
    pub geographic_footprint: String,
    pub priority_findings: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FindingsLabels {
    pub chapter: String,
    pub empty: String,
    pub intro: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportGovernanceLabels {
    pub chapter: String,
    pub flagged_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChapterLabels {
    pub chapter: String,
    pub intro: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EstateLabels {
    pub subscription_sentence: String,
    pub group_summary: String,
    pub group_findings: String,
    pub relationships: String,
    pub settings: String,
    pub no_settings: String,
    pub findings: String,
    pub related: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceLabels {
    pub chapter: String,
    pub intro: String,
    pub no_results: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfLabels {
    /// A Typst `counter.display` pattern, e.g. `1 / 1`.
    pub page_counter: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarkdownLabels {
    pub title: String,
    pub back_to_index: String,
    pub summary: String,
    pub findings_by_severity: String,
    pub see: String,
    pub tag_coverage_line: String,
    pub key_share: String,
    pub subscription_note: String,
    pub resources_by_type: String,
    pub resources_by_location: String,
    pub sections: String,
    pub subscription_link: String,
    pub subscription_title: String,
    pub id: String,
    pub state: String,
    pub resource_details: String,
    pub related: String,
    pub no_rows: String,
    pub no_resources: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HtmlLabels {
    pub collected: String,
    pub tenant: String,
    pub status: String,
    pub subscriptions: String,
    pub resource_groups: String,
    pub resources: String,
    pub findings: String,
    pub tag_coverage: String,
    pub distinct_keys: String,
    pub non_compliant: String,
    pub filter_placeholder: String,
    pub no_findings: String,
    pub share_of_tagged: String,
    pub subscriptions_heading: String,
    pub diagrams: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct XlsxLabels {
    pub sheet_summary: String,
    pub sheet_inventory: String,
    pub sheet_findings: String,
    pub sheet_governance: String,
    pub queries_sheet: String,
    pub high_findings: String,
    pub medium_findings: String,
    pub low_findings: String,
    pub info_findings: String,
    pub tag_coverage_percent: String,
    pub share_of_tagged_percent: String,
    pub inventory_columns: XlsxInventoryColumns,
    pub findings_columns: FindingsColumns,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct XlsxInventoryColumns {
    pub name: String,
    pub r#type: String,
    pub kind: String,
    pub location: String,
    pub resource_group: String,
    pub subscription: String,
    pub tags: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FindingsColumns {
    pub severity: String,
    pub category: String,
    pub check: String,
    pub title: String,
    pub resource: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CsvLabels {
    pub inventory_columns: CsvInventoryColumns,
    pub findings_columns: CsvFindingsColumns,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CsvInventoryColumns {
    pub id: String,
    pub name: String,
    pub r#type: String,
    pub kind: String,
    pub location: String,
    pub resource_group: String,
    pub subscription_id: String,
    pub tags: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CsvFindingsColumns {
    pub severity: String,
    pub category: String,
    pub check: String,
    pub title: String,
    pub resource_id: String,
}

// ----------------------------------------------------------------- diagram --

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagramLabels {
    pub hierarchy_title: String,
    pub tenant_node: String,
    pub resource_count: String,
    pub aggregate: String,
    pub other_resources: String,
    pub other_resources_sub: String,
    pub resources_title: String,
    pub network_title: String,
    pub connected_services: String,
    pub connected_services_sub: String,
    pub vnet_sheet: String,
    pub resource_group_sheet: String,
    pub resource_group_sheet_bare: String,
    pub not_in_vnet: String,
    pub nested_suffix: String,
    pub peerings_title: String,
    pub not_peered: String,
    pub peered: String,
    pub remote_vnet: String,
    pub private_link: String,
    pub legend: LegendLabels,
    pub workbook: WorkbookLabels,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegendLabels {
    pub vnet: String,
    pub subnet: String,
    pub zone: String,
    pub aggregate: String,
    pub watermark: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkbookLabels {
    pub network_topology: String,
    pub vnet_peerings: String,
}

// --------------------------------------------------------------------- cli --

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CliLabels {
    pub check: CheckLabels,
    pub collect: CollectLabels,
    pub diagram: CliDiagramLabels,
    pub init: InitLabels,
    pub query: QueryLabels,
    pub report: CliReportLabels,
    pub snapshots: SnapshotsLabels,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckLabels {
    pub config_ok: String,
    pub token_ok: String,
    pub visible_subscriptions: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectLabels {
    pub all_subscriptions: String,
    pub some_subscriptions: String,
    pub collecting: String,
    pub snapshot_written: String,
    pub partial_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CliDiagramLabels {
    pub large_warning: String,
    pub written: String,
    pub sheet_written: String,
    pub nothing_to_write: String,
    pub skipping_mermaid: String,
    pub workbook_written: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitLabels {
    pub tenant_prompt: String,
    pub client_prompt: String,
    pub secret_prompt: String,
    pub wrote: String,
    pub plaintext_note: String,
    pub next_step: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryLabels {
    pub rows_summary: String,
    pub user_queries_dir: String,
    pub no_rows: String,
    pub columns: QueryColumns,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryColumns {
    pub name: String,
    pub category: String,
    pub kind: String,
    pub severity: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CliReportLabels {
    pub markdown_written: String,
    pub html_written: String,
    pub site_written: String,
    pub csv_written: String,
    pub xlsx_written: String,
    pub pdf_written: String,
    pub docx_written: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotsLabels {
    pub none_yet: String,
    pub snapshot: String,
    pub created: String,
    pub tenant: String,
    pub status: String,
    pub tool: String,
    pub notes: String,
    pub comparing: String,
    pub nothing_to_prune: String,
    pub deleted: String,
    pub columns: SnapshotColumns,
    pub run_columns: RunColumns,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotColumns {
    pub id: String,
    pub created: String,
    pub status: String,
    pub subscriptions: String,
    pub resources: String,
    pub findings: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunColumns {
    pub query: String,
    pub category: String,
    pub rows: String,
    pub ms: String,
    pub error: String,
}

// --------------------------------------------------------------------- tui --

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TuiLabels {
    pub panes: TuiPanes,
    pub keys: TuiKeys,
    pub fields: TuiFields,
    pub units: TuiUnits,
    pub messages: TuiMessages,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TuiPanes {
    pub snapshots: String,
    pub estate: String,
    pub resources: String,
    pub detail: String,
    pub findings: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TuiKeys {
    pub snapshots: String,
    pub estate: String,
    pub filtering: String,
    pub filtered: String,
    pub findings: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TuiFields {
    pub name: String,
    pub r#type: String,
    pub kind: String,
    pub location: String,
    pub subscription: String,
    pub id: String,
    pub tags: String,
    pub related: String,
    pub properties: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TuiUnits {
    pub subscriptions: String,
    pub resources: String,
    pub findings: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TuiMessages {
    pub no_selection: String,
}

// ----------------------------------------------------------------- desktop --
//
// Typed here rather than in the desktop crate so one loader validates the
// whole user file and `azdocs check` can report a typo anywhere in it. The
// frontend's TypeScript type is inferred from the serialised JSON, so keep
// every leaf a `String`, a [`Plural`] or a `BTreeMap<String, String>`.

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopLabels {
    pub app: DesktopAppLabels,
    pub nav: DesktopNavLabels,
    pub shell: DesktopShellLabels,
    pub errors: DesktopErrorLabels,
    pub dialogs: DesktopDialogLabels,
    pub topology: DesktopTopologyLabels,
    pub backend: DesktopBackendLabels,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopNavLabels {
    pub overview: String,
    pub estate: String,
    pub topology: String,
    pub inventory: String,
    pub findings: String,
    pub governance: String,
    pub history: String,
    pub exports: String,
    pub settings: String,
    pub previous_view: String,
    pub resource: String,
    pub resource_group: String,
    pub neighbourhood: String,
    pub neighbourhood_of: String,
    pub back_to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopShellLabels {
    pub tagline: String,
    pub preview_badge: String,
    pub snapshot: String,
    pub snapshot_option: String,
    pub snapshot_picker: String,
    pub search_placeholder: String,
    pub search_aria: String,
    pub search_results: String,
    pub search_no_matches: String,
    pub shortcut_mac: String,
    pub shortcut_other: String,
    pub open_data: String,
    pub collect: String,
    pub collecting: String,
    pub collect_hint: String,
    pub configure_credentials: String,
    pub primary_navigation: String,
    pub preparing_collection: String,
    pub snapshot_stored: String,
    pub collected: String,
    pub source_note: String,
    pub dismiss: String,
    pub loading_title: String,
    pub loading_detail: String,
    pub empty_title: String,
    pub empty_detail: String,
    pub open_database: String,
    pub collect_first: String,
    pub status_aria: String,
    pub status_snapshot: String,
    pub status_none: String,
    pub status_resolving: String,
    pub status_selection: String,
    pub status_stored_relationships: String,
    pub status_export_ready: String,
    pub status_no_selection: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopErrorLabels {
    pub generic: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopDialogLabels {
    pub open_database_title: String,
    pub sqlite_filter: String,
    pub export_directory_title: String,
    pub preview_collecting: String,
    pub preview_exporting: String,
    pub collect_notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopAppLabels {
    pub window_title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopTopologyLabels {
    pub nodes: TopologyNodeLabels,
    /// Keyed by `EdgeKind::as_str()`.
    pub edge_kinds: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TopologyNodeLabels {
    pub group_resources: String,
    pub unconnected_groups: String,
    pub times_n: String,
    pub collapsed_subscription: String,
    pub links: Plural,
    pub attached_suffix: String,
    pub in_other_groups: String,
    pub in_group: String,
    pub label_times: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopBackendLabels {
    pub phases: BackendPhaseLabels,
    pub errors: BackendErrorLabels,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackendPhaseLabels {
    pub loading_pack: String,
    pub running_queries: String,
    pub composing_reports: String,
    pub rendering_diagram: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackendErrorLabels {
    pub no_report_format: String,
    pub no_diagram_type: String,
    pub no_diagram_format: String,
    pub no_destination: String,
    pub destination_not_dir: String,
    pub unsupported_kind: String,
}

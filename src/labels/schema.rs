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
    pub access: AccessLabels,
    pub websites: WebsiteLabels,
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
    pub posture: PostureLabels,
    pub assessment: AssessmentLabels,
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
    pub folded_suffix: String,
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
    pub failed_hint: String,
    pub no_subscriptions: String,
    pub dry_run_header: String,
    pub dry_run_columns: DryRunColumns,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DryRunColumns {
    pub name: String,
    pub category: String,
    pub kind: String,
    pub severity: String,
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
    pub default_name: String,
    pub tenant_prompt: String,
    pub client_prompt: String,
    pub secret_prompt: String,
    pub wrote: String,
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
    pub schema: String,
    pub interrupted: String,
    pub notes: String,
    pub comparing: String,
    pub nothing_to_prune: String,
    pub deleted: String,
    pub vacuumed: String,
    pub verify_ok: String,
    pub verify_problems: String,
    pub reconciled: String,
    pub columns: SnapshotColumns,
    pub run_columns: RunColumns,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotColumns {
    pub id: String,
    pub created: String,
    pub tenant: String,
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
    pub dropped: String,
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
    pub overview: DesktopOverviewLabels,
    pub findings: DesktopFindingsLabels,
    pub history: DesktopHistoryLabels,
    pub inventory: DesktopInventoryLabels,
    pub settings: DesktopSettingsLabels,
    pub governance: DesktopGovernanceLabels,
    pub estate: DesktopEstateLabels,
    pub record: DesktopRecordLabels,
    pub data_view: DesktopDataViewLabels,
    pub progressive: DesktopProgressiveLabels,
    pub exports: DesktopExportsLabels,
    pub backend: DesktopBackendLabels,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopEstateLabels {
    pub hierarchy_aria: String,
    pub title: String,
    pub summary: String,
    pub entire_estate: String,
    pub collapse: String,
    pub expand: String,
    pub no_stored_resources: String,
    pub inventory_aria: String,
    pub of_total: String,
    pub matching: String,
    pub type_filter_aria: String,
    pub all_types: String,
    pub location_filter_aria: String,
    pub all_locations: String,
    pub location_not_stored: String,
    pub location_global: String,
    pub sort_aria: String,
    pub sort_name: String,
    pub sort_type: String,
    pub sort_location: String,
    pub sort_findings: String,
    pub clear_filters: String,
    pub signals: String,
    pub list_aria: String,
    pub empty_title: String,
    pub empty_detail: String,
    pub quiet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopRecordLabels {
    pub unknown_subscription: String,
    pub source_note: String,
    pub summary_aria: String,
    pub relationships: String,
    pub review_findings: String,
    pub explore_relationships: Plural,
    pub no_relationships: String,
    pub findings_title: String,
    pub findings_detail: String,
    pub finding_evidence: String,
    pub no_findings: String,
    pub context_title: String,
    pub context_detail: String,
    pub properties_title: String,
    pub properties_detail: String,
    pub properties: String,
    pub sku: String,
    pub identity: String,
    pub relationships_title: String,
    pub relationships_detail: String,
    pub outbound: String,
    pub inbound: String,
    pub relationship_evidence: String,
    pub no_relationship_rows: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopDataViewLabels {
    pub no_value: String,
    pub items: Plural,
    pub fields: Plural,
    pub stored_value: String,
    pub not_stored: String,
    pub no_items: String,
    pub no_fields: String,
    pub r#true: String,
    pub r#false: String,
    pub empty_string: String,
    pub item: String,
    pub value: String,
    pub items_path: String,
    pub col_item: String,
    pub col_field: String,
    pub col_stored_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopProgressiveLabels {
    pub show_more: String,
    pub loaded: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopExportsLabels {
    pub include_reference: String,
    pub reference_detail: String,
    pub title: String,
    pub description: String,
    pub source_aria: String,
    pub snapshot: String,
    pub resources: String,
    pub choose_title: String,
    pub choose_detail: String,
    pub legend: String,
    pub advanced_note: String,
    pub run_title: String,
    pub destination: String,
    pub not_selected: String,
    pub choose_folder: String,
    pub source: String,
    pub source_value: String,
    pub deliverable: String,
    pub format: String,
    pub access: String,
    pub access_value: String,
    pub generating: String,
    pub run_action: String,
    pub preview_action: String,
    pub choose_directory: String,
    pub exported: Plural,
    pub preparing: String,
    pub failed: String,
    pub incomplete: String,
    pub complete: String,
    pub preview_note: String,
    /// Keyed by the preset ids in `ExportsView.tsx`.
    pub presets: std::collections::BTreeMap<String, ExportPresetLabels>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportPresetLabels {
    pub label: String,
    pub action: String,
    pub detail: String,
    pub includes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopOverviewLabels {
    pub dashboard: DesktopDashboardLabels,
    pub title: String,
    pub description: String,
    pub snapshot_stamp: String,
    pub resources: String,
    pub resource_groups: String,
    pub findings: String,
    pub tag_coverage: String,
    pub relationships: String,
    pub types_caption: String,
    pub growth_aria: String,
    pub growth_caption: String,
    pub location_not_stored: String,
    pub regions_caption: String,
    pub attention: String,
    pub estate_level: String,
    pub no_findings: String,
    pub review_all: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopDashboardLabels {
    pub collapse_sidebar: String,
    pub expand_sidebar: String,
    pub scope: String,
    pub all_subscriptions: String,
    pub audit: String,
    pub groups_context: Plural,
    pub high_context: String,
    pub tags_context: String,
    pub links_context: String,
    pub history: String,
    pub history_note: String,
    pub history_empty: String,
    pub history_range: String,
    pub range_month: String,
    pub range_quarter: String,
    pub range_all: String,
    pub severity: String,
    pub types: String,
    pub regions: String,
    pub subscriptions: String,
    pub tags_note: String,
    pub coverage: String,
    pub coverage_note: String,
    pub coverage_count: String,
    pub coverage_empty: String,
    pub changes: String,
    pub changes_note: String,
    pub changes_empty: String,
    pub baseline: String,
    pub details: String,
    pub open_results: String,
    pub close: String,
    pub clear_filter: String,
    pub back: String,
    pub filter: String,
    pub empty: String,
    pub all_types: String,
    pub all_regions: String,
    pub all_queries: String,
    pub open_map: String,
    pub open_snapshot: String,
    pub open_governance: String,
    pub tagged: String,
    pub untagged: String,
    pub relationships_note: String,
    pub snapshot_note: String,
    pub query_result: Plural,
    pub query_failed: String,
    pub query_unknown: String,
    pub more: String,
    pub attention: String,
    pub check_count: Plural,
    pub show_all: String,
    pub recorded: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopFindingsLabels {
    pub title: String,
    pub description: String,
    pub severity_counts_aria: String,
    pub count: String,
    pub all_severities: String,
    pub list_aria: String,
    pub empty_title: String,
    pub empty_detail: String,
    pub estate_level: String,
    pub close_evidence: String,
    pub affected_resource: String,
    pub stored_evidence: String,
    pub no_detail: String,
    pub audit_context: String,
    pub query: String,
    pub category: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopHistoryLabels {
    pub title: String,
    pub description: String,
    pub source_stamp: String,
    pub captured: String,
    pub estate: String,
    pub findings: String,
    pub status: String,
    pub resources: String,
    pub subscriptions: String,
    pub audit_signals: String,
    pub what_changed: String,
    pub earliest: String,
    pub base: String,
    pub base_aria: String,
    pub base_option: String,
    pub target: String,
    pub comparing: String,
    pub added: String,
    pub changed: String,
    pub removed: String,
    pub no_older: String,
    pub health: String,
    pub health_summary: String,
    pub run_detail: String,
    /// Keyed by change kind (`added`, `changed`, `removed`).
    pub kinds: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopInventoryLabels {
    pub summaries: String,
    pub provenance: String,
    pub provenance_missing: String,
    pub source: String,
    pub scope: String,
    pub subscriptions: String,
    pub all_visible: String,
    pub query_hash: String,
    pub source_reviewed: String,
    pub title: String,
    pub description: String,
    pub pack_unreadable: String,
    pub pack_failed: String,
    pub collected_stamp: String,
    pub collected_value: String,
    pub empty_title: String,
    pub empty_detail: String,
    pub categories_aria: String,
    pub table: String,
    pub query_option: String,
    pub position: String,
    pub failed: String,
    pub reading: String,
    pub no_search_rows: String,
    pub no_rows: String,
    pub row_count: Plural,
    pub visible_of: String,
    pub matching: String,
    pub collected_in: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopSettingsLabels {
    pub editor: std::collections::BTreeMap<String, String>,
    pub title: String,
    pub description: String,
    pub appearance: String,
    pub theme: String,
    pub theme_detail: String,
    pub theme_aria: String,
    pub resolving: String,
    pub following_system: String,
    pub resolved_end: String,
    pub database: String,
    pub store: String,
    pub store_detail: String,
    pub open_another: String,
    pub snapshots_held: String,
    pub prune_hint: String,
    pub prune_command: String,
    pub snapshot_count: Plural,
    pub snapshot_range: String,
    pub collection: String,
    pub configuration: String,
    pub configuration_detail: String,
    pub credentials_found: String,
    pub credentials_missing: String,
    pub required_tags: String,
    pub required_tags_detail: String,
    pub none_configured: String,
    pub none_configured_key: String,
    pub active_snapshot: String,
    pub active_snapshot_detail: String,
    /// Keyed by theme preference (`system`, `light`, `dark`).
    pub theme_options: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopGovernanceLabels {
    pub title: String,
    pub required_tags_from: String,
    pub not_enforced: String,
    pub governance_findings: String,
    pub key_caption: String,
    pub subscription_caption: String,
    pub all_clear: String,
    pub worst_caption: String,
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
    pub choose_resource: String,
    pub select_prompt: String,
    pub reach_subtitle: String,
    pub group_subtitle: Plural,
    pub estate_subtitle: String,
    pub subscription_count: Plural,
    pub hops: Plural,
    pub unit_groups: String,
    pub unit_resources: String,
    pub include_unconnected_resources: String,
    pub include_unconnected_groups: String,
    pub neighbourhood_stage: String,
    pub no_selected_resource: String,
    pub group_stage: String,
    pub estate_stage: String,
    pub back: String,
    pub trail_aria: String,
    pub depth_aria: String,
    pub reach: String,
    pub unconnected: String,
    pub refresh_failed: String,
    pub error_title: String,
    pub error_previous: String,
    pub retry: String,
    pub revert: String,
    pub no_map_title: String,
    pub no_map_detail: String,
    pub building: String,
    pub status_aria: String,
    pub drawn_caption: String,
    pub building_short: String,
    pub cross_group: String,
    pub relationship: Plural,
    pub connector: Plural,
    pub drawn_as: String,
    pub stale: String,
    pub legend: String,
    pub legend_aria: String,
    /// Keyed by `describeCounts` kind.
    pub counts: std::collections::BTreeMap<String, String>,
    pub regions: TopologyRegionLabels,
    /// Keyed by the `kind_class` family names.
    pub kind_classes: std::collections::BTreeMap<String, String>,
    pub tools: TopologyToolLabels,
    pub graph: TopologyGraphLabels,
    pub graph_extra: TopologyGraphExtraLabels,
    pub nodes: TopologyNodeLabels,
    /// Keyed by `EdgeKind::as_str()`.
    pub edge_kinds: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TopologyRegionLabels {
    pub external: String,
    pub services: String,
    pub unconnected: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TopologyToolLabels {
    pub controls_aria: String,
    pub fit_all_aria: String,
    pub fit_all: String,
    pub zoom_in: String,
    pub zoom_in_tip: String,
    pub zoom_out: String,
    pub zoom_out_tip: String,
    pub motion_reduced_aria: String,
    pub motion_pause_aria: String,
    pub motion_play_aria: String,
    pub motion_reduced: String,
    pub motion_pause: String,
    pub motion_play: String,
    pub reset_lanes: String,
    pub lanes_default: String,
    pub help_aria: String,
    pub help: String,
    pub help_title: String,
    pub help_body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TopologyGraphLabels {
    pub init_failed: String,
    pub layout_failed: String,
    pub resource_fallback: String,
    pub relationship_fallback: String,
    pub links: String,
    pub summary_none: String,
    pub summary_out_of_scope: String,
    pub summary_no_connectors: String,
    pub summary_connectors: String,
    pub connectors: Plural,
    pub connector_sentence: String,
    pub outbound: String,
    pub inbound: String,
    pub to: String,
    pub from: String,
    pub other_item: String,
    pub findings_suffix: Plural,
    pub findings_badge: String,
    pub lane_expanded: String,
    pub expand: String,
    pub vnet_card: String,
    pub subscription_collapsed: String,
    pub aggregate_groups: String,
    pub aggregate_resources: String,
    pub collapse: String,
    pub show: String,
    pub no_cross_group: String,
    pub unconnected_groups_aria: String,
    pub no_drawn_connectors: String,
    pub hops_away: Plural,
    pub open_record: String,
    pub external_card: String,
    pub group_card: String,
    pub resource_card: String,
    pub explore_hint: String,
    pub no_relationships_hint: String,
    pub explore_aria: String,
    pub no_relationships_aria: String,
    pub explore: String,
    pub no_relationships: String,
    pub nothing_title: String,
    pub nothing_detail: String,
}

/// Card and lane copy that arrived after the main graph table was laid out;
/// kept as its own table so an override file stays readable.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TopologyGraphExtraLabels {
    pub group_count: Plural,
    pub lane_summary: String,
    pub collapse_lane: String,
    pub member_group: String,
    pub members_aria: String,
    pub renderer_unavailable: String,
    pub renderer_retry_detail: String,
    pub renderer_retry: String,
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
    pub rendering_report: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssessmentLabels {
    pub unresolved_endpoint: String,
    pub study_mix: String,
    pub study_dependency_summary: String,
    pub issue_group_scope: String,
    pub family_caption: String,
    pub diagram_population: String,
    pub diagram_external: String,
    pub executive: String,
    pub composition: String,
    pub architecture: String,
    pub profiles: String,
    pub security: String,
    pub governance: String,
    pub actions: String,
    pub coverage: String,
    pub unknown: String,
    pub none_recorded: String,
    pub occurrences: String,
    pub affected: String,
    pub issue_patterns: String,
    pub relationships: String,
    pub region: String,
    pub service_mix: String,
    pub geography: String,
    pub subscription_comparison: String,
    pub main_observations: String,
    pub review_first: String,
    pub executive_intro: String,
    pub executive_empty: String,
    pub executive_location: String,
    pub executive_subscription: String,
    pub executive_issue: String,
    pub executive_tags: String,
    pub composition_intro: String,
    pub geography_note: String,
    pub architecture_intro: String,
    pub architecture_summary: String,
    pub connection_families: String,
    pub shared_dependencies: String,
    pub dependency: String,
    pub consumers: String,
    pub groups: String,
    pub source: String,
    pub target: String,
    pub relationship: String,
    pub external: String,
    pub shared_note: String,
    pub network_detail: String,
    pub private_access: String,
    pub network_note: String,
    pub profile_intro: String,
    pub profile_summary: String,
    pub profile_concentration: String,
    pub group_register: String,
    pub study: String,
    pub study_findings: String,
    pub study_connections: String,
    pub study_population: String,
    pub study_summary: String,
    pub configuration: String,
    pub configuration_note: String,
    pub configuration_absent: String,
    pub group_dependencies: String,
    pub group_dependencies_none: String,
    pub study_findings_heading: String,
    pub study_findings_none: String,
    pub study_issue: String,
    pub connection_focus: String,
    pub diagram_caption: String,
    pub network_caption: String,
    pub security_intro: String,
    pub issue_observation: String,
    pub issue_significance: String,
    pub issue_verification: String,
    pub issue_policy: String,
    pub issue_scope: String,
    pub issue_examples: String,
    pub issue_summary: String,
    pub severity_mix: String,
    pub example_limit: String,
    pub guidance_scope: String,
    pub governance_intro: String,
    pub governance_coverage: String,
    pub tag_scope_unknown: String,
    pub tag_scope_known: String,
    pub tag_focus: String,
    pub operations: String,
    pub operations_note: String,
    pub operations_summary: String,
    pub operational_questions: String,
    pub operational_review: String,
    pub actions_intro: String,
    pub action: String,
    pub action_verification: String,
    pub coverage_intro: String,
    pub coverage_summary: String,
    pub coverage_missing: String,
    pub coverage_limits: String,
    pub query: String,
    pub result: String,
    pub rows: String,
    pub successful: String,
    pub empty_query: String,
    pub failed: String,
    pub outcome_unknown: String,
    pub scope_unavailable: String,
    pub reference_title: String,
    pub reference_intro: String,
    pub reference_reductions: String,
    pub reference_properties: String,
    pub reference_fields: std::collections::BTreeMap<String, String>,
    pub reference_findings_note: String,
    pub reference_coverage_note: String,
    pub reference_scope: String,
    pub reference_occurrence_count: String,
    pub reference_identity: String,
    pub reference_resource_id: String,
    pub reference_sku: String,
    pub reference_auth_identity: String,
    pub reference_relationships: String,
    pub reference_record_key: String,
    pub reference_record_value: String,
    pub original_evidence: String,
    pub reference_occurrences: String,
    pub reference_query_note: String,
    pub full_rows: String,
    pub full_row: String,
    pub findings_reference: String,
    pub reference_available: String,
    pub family_compute: String,
    pub family_network: String,
    pub family_data: String,
    pub family_identity: String,
    pub family_monitoring: String,
    pub family_integration: String,
    pub family_other: String,
    pub compute: String,
    pub storage: String,
    pub databases: String,
    pub identity_logging: String,
    pub vm_size: String,
    pub operating_system: String,
    pub zones: String,
    pub replication: String,
    pub public_network: String,
    pub default_action: String,
    pub https_only: String,
    pub minimum_tls: String,
    pub blob_access: String,
    pub identity_type: String,
    pub logging_targets: String,
    pub retention: String,
    pub vault_configuration: String,
    pub showing_rows: String,
    pub checks: std::collections::BTreeMap<String, CheckGuidance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckGuidance {
    pub title: String,
    pub observation: String,
    pub significance: String,
    pub verification: String,
    pub policy: String,
    pub action: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebsiteLabels {
    pub links_title: String,
    pub website: String,
    pub capture_time: String,
    pub attempt_time: String,
    pub capture_note: String,
    pub failure_detail: String,
    pub collect_again: String,
    pub stage_inventory: String,
    pub stage_discovery: String,
    pub stages: String,
    pub elapsed: String,
    pub finished: String,
    pub query_progress: String,
    pub count_progress: String,
    pub rows_count: String,
    pub saved_count: String,
    pub failed_count: String,
    pub remaining_count: String,
    pub current_url: String,
    pub latest_query: String,
    pub preparing: String,
    pub new_snapshot: String,
    pub collection_detail: String,
    pub saved_snapshot: String,
    pub saved_detail: String,
    pub close_collection: String,
    pub background_note: String,
    pub show_collection: String,
    pub title: String,
    pub detail: String,
    pub capture_all: String,
    pub retry_all: String,
    pub refresh: String,
    pub capture: String,
    pub cancel: String,
    pub view: String,
    pub save: String,
    pub close: String,
    pub empty: String,
    pub progress: String,
    pub complete: String,
    pub cancelled: String,
    pub discovering: String,
    pub capture_window: String,
    pub captured_at: String,
    pub attempted_at: String,
    pub final_url: String,
    pub renderer: String,
    pub no_image: String,
    pub missing_evidence: String,
    pub caption: String,
    pub previous_image: String,
    pub preview_only: String,
    pub saved: String,
    pub collect_note: String,
    pub states: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostureLabels {
    pub chapter: String,
    pub intro: String,
    pub unknown: String,
    pub status: String,
    pub sheet: String,
    pub values: std::collections::BTreeMap<String, String>,
    pub datasets: std::collections::BTreeMap<String, PostureDatasetLabels>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostureDatasetLabels {
    pub title: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccessLabels {
    pub title: String,
    pub detail: String,
    pub checked: String,
    pub verdicts: std::collections::BTreeMap<String, String>,
    pub reasons: std::collections::BTreeMap<String, String>,
}

//! Generates `desktop/src/generated.ts` from the DTO types.
//!
//! The frontend used to hand-mirror ~32 interfaces against these structs, kept
//! in step only by `#[serde(rename_all = "camelCase")]` and care. Now the Rust
//! types are the single source of truth and the check is mechanical: CI runs
//! `cargo test -p azdocs-desktop` and then `git diff --exit-code`, so a field
//! added, removed or renamed here fails the build until the file is regenerated.
//!
//! Not every TypeScript type is generated. Closed string sets that Rust models
//! as an enum but serialises through `as_str()` (severity, edge kind, snapshot
//! status, …) stay hand-written in `src/api-types.ts` and are pulled in by the
//! `#[ts(type = "...")]` overrides on the fields that carry them. The labels
//! type is the other exception: `src/labels.ts` infers it from
//! `generated-labels.json`, which this module also writes.

use std::fmt::Write as _;

use ts_rs::{Config, TS};

/// Hand-written types the generated declarations may reference via
/// `#[ts(type = "...")]`, as (name, module). Only the ones a declaration
/// actually mentions are imported, so the file does not trip `no-unused-vars`.
const MANUAL_IMPORTS: &[(&str, &str)] = &[
    ("DiagramExportType", "./api-types"),
    ("EdgeKind", "./api-types"),
    ("ExportKind", "./api-types"),
    ("KindClass", "./api-types"),
    ("QueryKind", "./api-types"),
    ("Severity", "./api-types"),
    ("SnapshotStatus", "./api-types"),
    ("TopologyLevel", "./api-types"),
    ("TopologyNodeKind", "./api-types"),
    ("TopologyZone", "./api-types"),
    ("Labels", "./labels"),
];

/// Push one type's declaration, exported.
fn decl<T: TS>(out: &mut String, cfg: &Config) {
    let declaration = T::decl(cfg)
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");
    let _ = writeln!(out, "export {declaration}\n");
}

/// The full contents of `desktop/src/generated.ts`.
pub fn generated_typescript() -> String {
    // serde_json emits plain JSON numbers, so u64 must map to `number`, not the
    // `bigint` ts-rs defaults to — nothing on the wire is ever a BigInt.
    let cfg = Config::default().with_large_int("number");
    let mut out = String::new();

    // Ordered by hand rather than alphabetically so related types read together
    // and the diff stays small when one is edited.
    use crate::dto::*;
    use crate::topology::*;

    use crate::settings::*;
    use azdocs::auth::diagnostics::*;
    use azdocs::config::document::*;
    use azdocs::config::{
        AuditConfig, BrandingConfig, CollectConfig, ReportConfig, RetryConfig, StorageConfig,
    };
    decl::<azdocs::cloud::Cloud>(&mut out, &cfg);
    decl::<SettingsValues>(&mut out, &cfg);
    decl::<TenantProfile>(&mut out, &cfg);
    decl::<CollectOverrides>(&mut out, &cfg);
    decl::<AuditOverrides>(&mut out, &cfg);
    decl::<BrandingOverrides>(&mut out, &cfg);
    decl::<CollectConfig>(&mut out, &cfg);
    decl::<RetryConfig>(&mut out, &cfg);
    decl::<AuditConfig>(&mut out, &cfg);
    decl::<StorageConfig>(&mut out, &cfg);
    decl::<ReportConfig>(&mut out, &cfg);
    decl::<BrandingConfig>(&mut out, &cfg);
    decl::<TenantSummary>(&mut out, &cfg);
    decl::<SettingsDocumentDto>(&mut out, &cfg);
    decl::<SettingsSaveRequest>(&mut out, &cfg);
    decl::<SettingsTestResult>(&mut out, &cfg);
    decl::<SettingsTestRequest>(&mut out, &cfg);
    decl::<ConnectionCheck>(&mut out, &cfg);
    decl::<VisibleSubscription>(&mut out, &cfg);
    decl::<PermissionGrant>(&mut out, &cfg);
    decl::<PermissionVerdict>(&mut out, &cfg);
    decl::<AccessIssue>(&mut out, &cfg);
    decl::<AccessIssueKind>(&mut out, &cfg);
    decl::<AppBootstrap>(&mut out, &cfg);
    decl::<SnapshotSummary>(&mut out, &cfg);
    decl::<SnapshotComparison>(&mut out, &cfg);
    decl::<FieldChangeDto>(&mut out, &cfg);
    decl::<FindingRefDto>(&mut out, &cfg);
    decl::<EdgeRefDto>(&mut out, &cfg);
    decl::<ComparisonCountsDto>(&mut out, &cfg);
    decl::<TrendPointDto>(&mut out, &cfg);
    decl::<EstateSnapshot>(&mut out, &cfg);
    decl::<TotalsDto>(&mut out, &cfg);
    decl::<TagCoverageDto>(&mut out, &cfg);
    decl::<SeverityCountsDto>(&mut out, &cfg);
    decl::<AzureMetadataDto>(&mut out, &cfg);
    decl::<RegionPointDto>(&mut out, &cfg);
    decl::<GovernanceDto>(&mut out, &cfg);
    decl::<TagKeyCoverageDto>(&mut out, &cfg);
    decl::<SubscriptionCoverageDto>(&mut out, &cfg);
    decl::<GroupComplianceDto>(&mut out, &cfg);
    decl::<SubscriptionDto>(&mut out, &cfg);
    decl::<ResourceGroupDto>(&mut out, &cfg);
    decl::<ResourceGroupSummaryDto>(&mut out, &cfg);
    decl::<ResourceDto>(&mut out, &cfg);
    decl::<ResourceDetailDto>(&mut out, &cfg);
    decl::<ResourceTypeDto>(&mut out, &cfg);
    decl::<NameCountDto>(&mut out, &cfg);
    decl::<FindingDto>(&mut out, &cfg);
    decl::<EdgeDto>(&mut out, &cfg);
    decl::<QueryRunDto>(&mut out, &cfg);
    decl::<QueryProvenanceDto>(&mut out, &cfg);
    decl::<EvidenceTableDto>(&mut out, &cfg);
    decl::<QueryDefDto>(&mut out, &cfg);
    decl::<QueryRowsDto>(&mut out, &cfg);

    decl::<TopologyRequest>(&mut out, &cfg);
    decl::<TopologyMode>(&mut out, &cfg);
    decl::<TopologyScope>(&mut out, &cfg);
    decl::<TopologyGraphDto>(&mut out, &cfg);
    decl::<LaneDto>(&mut out, &cfg);
    decl::<TopologyNodeDto>(&mut out, &cfg);
    decl::<TopologyLinkDto>(&mut out, &cfg);
    decl::<KindClassCountDto>(&mut out, &cfg);
    decl::<TopologyCountsDto>(&mut out, &cfg);

    decl::<CollectRequestDto>(&mut out, &cfg);
    decl::<CollectResultDto>(&mut out, &cfg);
    decl::<CollectionStage>(&mut out, &cfg);
    decl::<CollectionQueryProgress>(&mut out, &cfg);
    decl::<CollectionEvent>(&mut out, &cfg);
    decl::<WebsiteState>(&mut out, &cfg);
    decl::<WebsiteEndpointDto>(&mut out, &cfg);
    decl::<WebsiteCaptureDto>(&mut out, &cfg);
    decl::<WebsiteCaptureRequest>(&mut out, &cfg);
    decl::<WebsiteProgress>(&mut out, &cfg);
    decl::<WebsiteBatchResult>(&mut out, &cfg);
    decl::<ExportRequestDto>(&mut out, &cfg);
    decl::<ExportResultDto>(&mut out, &cfg);
    decl::<ExportEvent>(&mut out, &cfg);

    let used = |module: &str| -> Vec<&str> {
        MANUAL_IMPORTS
            .iter()
            .filter(|(_, from)| *from == module)
            .map(|(name, _)| *name)
            .filter(|name| {
                // Word-boundary match so `KindClass` does not count as a use of
                // `KindClassCount`, and vice versa.
                out.split(|c: char| !c.is_alphanumeric() && c != '_')
                    .any(|word| word == *name)
            })
            .collect()
    };

    let mut header = String::from(
        "// @generated by `cargo test -p azdocs-desktop` — do not edit by hand.\n\
         //\n\
         // Source of truth: desktop/src-tauri/src/dto.rs and topology.rs.\n\
         // Regenerate with `cargo test -p azdocs-desktop`; CI fails if this file\n\
         // is stale. Hand-written unions live in ./api-types; the labels type\n\
         // is inferred from generated-labels.json in ./labels.\n\n",
    );
    for module in ["./api-types", "./labels"] {
        let names = used(module);
        if names.is_empty() {
            continue;
        }
        let _ = writeln!(
            header,
            "import type {{\n{}\n}} from \"{module}\";\n",
            names
                .iter()
                .map(|name| format!("  {name},"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
    header.push_str(&out);
    header
}

/// The built-in labels the frontend types itself against and falls back to
/// before bootstrap. Built-ins only, so the output is the same on every host.
pub fn generated_labels_json() -> String {
    let mut json = serde_json::to_string_pretty(&crate::labels::AppLabels::builtin())
        .expect("built-in labels serialise; guaranteed by the labels unit test");
    json.push('\n');
    json
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::generated_typescript;

    fn target() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src/generated.ts")
    }

    fn labels_target() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src/generated-labels.json")
    }

    /// Writing from a test (rather than build.rs) keeps codegen out of the
    /// normal build and makes staleness a CI diff rather than a silent rebuild.
    #[test]
    fn writes_the_typescript_bindings_when_run() {
        let contents = generated_typescript();
        std::fs::write(target(), &contents).expect("write desktop/src/generated.ts");
        assert!(
            contents.contains("export type Resource = "),
            "the DTO declarations should be present"
        );
        std::fs::write(labels_target(), super::generated_labels_json())
            .expect("write desktop/src/generated-labels.json");
    }

    #[test]
    fn keeps_hand_written_unions_on_the_fields_that_carry_them() {
        let contents = generated_typescript();
        for expected in [
            "kind: EdgeKind,",
            "severity: Severity,",
            "status: SnapshotStatus,",
            "kind: TopologyNodeKind,",
            "kindClass: KindClass,",
            "exportKind: ExportKind,",
            "labels: Labels,",
        ] {
            assert!(
                contents.contains(expected),
                "generated bindings lost the `{expected}` union override"
            );
        }
    }
}

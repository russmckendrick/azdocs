# Architecture

Single binary crate with a lib/bin split (`lib.rs` + thin `main.rs`) so
integration tests can call internals.

## Dependency flow

Strictly one-way:

```mermaid
flowchart TD
    cli[cli.rs<br/>clap surface] --> commands[commands/*]
    commands --> collect[collect/*]
    commands --> report[report/*]
    commands --> diagram[diagram/*]
    commands --> tui[tui/*]
    desktop[desktop/src-tauri<br/>typed Tauri IPC] --> store
    desktop --> collect
    collect --> arg[arg/*<br/>Resource Graph client]
    arg --> auth[auth/*<br/>TokenProvider]
    collect --> store[(store/*<br/>SQLite)]
    report --> store
    diagram --> store
    tui --> store
    collect --> qp[querypack/*]
    store --- model[model/*]
    report --> context[ReportContext]
    context --> print[PrintDocument<br/>PDF + DOCX composition]
    print --> pdf[Typst PDF renderer]
    print --> docx[OOXML DOCX renderer]
```

The load-bearing rule: **report, diagram, and TUI code read only from SQLite,
never the network.** Everything after `collect` works offline — that's what
makes golden-file testing and air-gapped report generation possible.

## The collect pipeline

```mermaid
flowchart LR
    qp[Query pack<br/>TOML, embedded] --> runner[Bounded-concurrency<br/>runner]
    runner -->|rows| ingest[ingest.rs<br/>route to tables]
    ingest --> db[(SQLite)]
    db --> extract[extractors.rs<br/>edges from properties]
    db --> audit[audit.rs<br/>required-tags findings]
    extract --> db
    audit --> db
```

## Module map

| Path | Purpose |
|---|---|
| `src/cli.rs` | The whole clap surface — the contract for every command |
| `src/config.rs` | TOML config, platform paths, env overrides (`deny_unknown_fields`) |
| `src/auth/` | `TokenProvider` trait + OAuth2 client-credentials flow, cached single-flight refresh |
| `src/arg/` | ARG client: `$skipToken` pagination, 429 backoff, quota-header pacing |
| `src/querypack/` | `QueryDef` TOML model; built-ins embedded, user `queries.d/` merged by name |
| `src/labels/` | Every user-facing string, typed from `data/labels/en.toml`; user files deep-merged by name |
| `src/store/` | All SQL. Versioned migrations, snapshot-scoped tables, cascade delete |
| `src/model/` | Plain data types + `azure_types.rs` display names |
| `src/collect/` | Runner, `ingest.rs`, `extractors.rs`, `audit.rs` |
| `src/report/` | `ReportContext` → all report data; `document.rs` composes one semantic `PrintDocument` for PDF + DOCX, while markdown / html / site / csv / xlsx consume `ReportContext` directly. Styled outputs use `BrandingContext`. `governance.rs` owns the tag thresholds and the analysis applying them, shared with the desktop. |
| `src/diagram/` | `EstateGraph` builders (incl. per-VNet/per-RG fan-out) → `page` (A4 fractions, density rungs) → `layout` (measure/justify) → `route` (orthogonal connectors) → mermaid / drawio (single + workbook) / svg / png emitters |
| `src/tui/` | ratatui browse; `App` is a pure state machine, `ui.rs` renders it |
| `desktop/src-tauri/` | Thin Tauri v2 boundary; opens the shared `Store` per command and maps core models to serialisable DTOs. `topology.rs` builds the explorer's view-ready relationship graphs (estate lanes, group drill-in with folding, ×N aggregation and cross-group ghost stubs, bounded-depth neighbourhoods) with honest drawn/folded/aggregated counts |
| `desktop/src/` | React/TypeScript estate explorer and lazy-loaded Cytoscape.js relationship canvas; `topology-layout.ts` owns deterministic zones/cameras, `topology-presentation.ts` owns synthetic boundary ports and DOM label placement, and `CytoscapeResourceGraph.tsx` coordinates paint and interaction. No direct file, database, credential, or Azure access. See [Desktop map](desktop-relationships.md). |

## Design decisions

**ARM ids are lowercased at every join.** ARG returns inconsistent casing
across queries; every key id goes through `model::normalize_arm_id`, original
casing kept in `display_id`. Skipping this creates phantom duplicate
resources — the classic Azure inventory bug.

**Edges are derived in Rust, not queried.** `collect/extractors.rs` walks the
already-stored properties JSON: per-type handlers for the network/compute
chain (VNet → subnets/peerings, NIC → subnet/NSG/VM, private endpoint →
target, load balancer, application gateway, VMSS, AKS, App Service, storage
and key-vault network ACLs) plus two generic passes that apply to every
resource — child types link to the ARM parent their id nests under, and
`identity.userAssignedIdentities` links to the managed identity. Zero extra
API calls, retroactive on old snapshots, and each extractor is a pure
`fn(&Resource) -> Vec<Edge>`.

**Auth is ~80 hand-rolled lines on purpose.** The client-credentials flow is
one POST. `azure_identity` was rejected for API churn and unneeded surface.
The `TokenProvider` trait (static dispatch) keeps it swappable and testable.

**Queries are data, not code.** A new audit check is a TOML file. Routing in
`collect/ingest.rs`: the three core inventory queries fill typed tables, other
inventory rows land in `query_results` for report tables, finding rows become
`findings`. Display names, themes and labels follow the same rule: embedded
TOML, user overrides from the config dir, no branch on a name in Rust.

**One graph, many emitters.** Diagram builders produce a single `EstateGraph`
(typed nodes with parent containment + styled edges); the Mermaid and draw.io
emitters both consume it. Report emitters share one `ReportContext`. The two
native print formats go one step further: `report/document.rs` turns that data,
branding and diagram bundle into an ordered `PrintDocument`, then the Typst and
OOXML backends render the same semantic block stream. Content, hierarchy,
labels, captions and asset placement therefore have one edit point; only
native layout mechanics remain renderer-specific. `report/analysis.rs` groups
stored findings without losing occurrences or severities, resolves typed
relationships and selects distinct group studies. The main assessment and the
optional technical reference have separate compositions in
`report/assessment.rs`. Non-print report content is unchanged.

**Report selection is not a rendering engine.** `diagram/graph/assessment.rs`
selects and aggregates connection-focused `EstateGraph`s. `diagram/assets.rs`
renders them through the established `page`, `layout`, `route`, `icons`, `svg`
and `png` modules. Fix geometry and label fitting there, never in PDF/DOCX or a
parallel SVG/card renderer.

**A judgement is computed once, and travels as a verdict.** Tag governance is
the worked example: `report/governance.rs` holds `HEALTHY_TAG_COVERAGE_PERCENT`
and `FLAGGED_NON_COMPLIANT_SHARE` *and* the analysis that applies them, over the
stored `missing_required_tags` findings rather than whatever `required_tags`
happens to say today. `ReportContext.governance` renders the print and Markdown
Governance chapter; `EstateSnapshot.governance` renders the explorer's
workspace. Both receive `healthy` and `flagged` booleans, not the thresholds —
sending the numbers is what let the frontend keep its own copy of the rule.

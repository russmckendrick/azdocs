# azdocs development guide

## Quick start

```sh
cargo build
cargo test              # full suite: unit + integration + golden files; no Azure needed
cargo clippy --all-targets --locked -- -D warnings
cargo fmt --check
```

Everything except `azdocs check`/`collect`/`query run` works offline. The test
suite never touches the network: HTTP is mocked with wiremock, ARG responses
are fixtures, and reports/diagrams are golden-tested from a canonical seeded
estate.

## Architecture

Single binary crate with a lib/bin split (`lib.rs` + thin `main.rs`) so
integration tests can call internals. Dependency flow is strictly one-way:

```
cli → commands → collect / report / diagram / tui → store / model ← arg / auth
```

The load-bearing rule: **report, diagram, and TUI code read only from SQLite,
never the network.** Everything after `collect` works offline, which is what
makes golden-file testing (and air-gapped report generation) possible.

### Module map

| Path | Purpose |
|---|---|
| `src/cli.rs` | The whole clap surface — the contract for every command |
| `src/config.rs` | TOML config, platform paths, env overrides (`deny_unknown_fields` — typos fail loudly) |
| `src/auth/` | `TokenProvider` trait + hand-rolled OAuth2 client-credentials flow with cached, single-flight refresh |
| `src/arg/` | Resource Graph client: `$skipToken` pagination, 429/`Retry-After` backoff, proactive quota-header pacing |
| `src/querypack/` | `QueryDef` TOML model; built-ins embedded via `include_dir`, user `queries.d/` merged over them by name |
| `src/store/` | All SQL lives here. Versioned migrations, snapshot-scoped tables, cascade delete |
| `src/model/` | Plain data types (`Resource`, `Edge`, `Finding`, …) + `azure_types.rs` display names |
| `src/collect/` | Bounded-concurrency query runner + `ingest.rs` (rows → tables), `extractors.rs` (edges), `audit.rs` (tag findings) |
| `src/report/` | `ReportContext` built once → markdown / html / site / csv / xlsx / details emitters |
| `src/diagram/` | `EstateGraph` builders → layout → mermaid / drawio emitters, azure2 icon map |
| `src/tui/` | ratatui browse mode; `App` is a pure state machine, `ui.rs` renders it |
| `queries/` | Built-in query pack (TOML, compiled into the binary) |
| `templates/` | minijinja templates for markdown pages and the HTML report |

### Design decisions worth knowing

**ARM ids are lowercased everywhere they join.** ARG returns inconsistent id
casing across queries; every id used as a key (`resources.id`, edge endpoints,
finding resource ids) goes through `model::normalize_arm_id`, with the
original casing kept in `display_id` for display. Skipping this creates
phantom duplicate resources — it's the classic Azure inventory bug.

**Edges are derived in Rust, not queried.** `collect/extractors.rs` walks the
already-stored properties JSON (vnet → subnets/peerings, NIC → subnet/NSG/VM,
private endpoint → target, …). Zero extra API calls, works retroactively on
old snapshots, and each extractor is a pure `fn(&Resource) -> Vec<Edge>` —
trivially unit-testable.

**Auth is ~80 lines on purpose.** The OAuth2 client-credentials flow is one
POST; `azure_identity` was rejected for API churn and unneeded surface. The
`TokenProvider` trait (static dispatch, `impl Future`) keeps it swappable and
lets tests inject `StaticTokenProvider`.

**Queries are data, not code.** A new audit check is a TOML file, not Rust.
The three "core" inventory queries (`all_resources`, `subscriptions`,
`resource_groups`) load typed tables; other inventory queries keep raw rows in
`query_results` for report tables; finding queries become `findings` rows.
Routing lives in `collect/ingest.rs`.

## SQLite schema

One database, snapshot-scoped rows, `ON DELETE CASCADE` from `snapshots`:

- `snapshots` — id (uuid), created_at, tenant, status (`running|complete|partial|failed`), notes
- `subscriptions`, `resource_groups` — per snapshot
- `resources` — PK `(snapshot_id, id)`; lowercased id + `display_id`; tags/sku/identity/properties as JSON text
- `edges` — `(source_id, target_id, edge_type)` + JSON properties
- `findings` — provenance (`query_name`), category, severity, resource_id, title, detail
- `query_runs` — per-snapshot audit trail (row counts, durations, errors)
- `query_results` — raw rows of shaped inventory queries, for report tables

Migrations are ordered SQL strings in `store/schema.rs`; `meta.schema_version`
records progress. **Append new migrations, never edit old ones.** Snapshot
diff is a single `FULL OUTER JOIN` in `store/snapshots.rs`.

## Testing

```sh
cargo test                          # everything
cargo test --test collect_test     # one integration suite
cargo insta review                  # accept intentional golden-output changes
```

Layers, and where to add tests:

| Layer | Approach | Files |
|---|---|---|
| HTTP (auth, ARG) | wiremock: token flow, pagination chains, 429/5xx retry | `tests/arg_client_test.rs` |
| Ingest/store | Fixture JSON rows → in-memory SQLite → assertions | `tests/collect_test.rs` |
| Edge extractors | Pure-function unit tests incl. adversarial inputs (nulls, mixed-case, cross-sub ids) | `src/collect/extractors.rs` |
| Reports & diagrams | insta golden files from the **canonical fixture estate** | `tests/report_golden_test.rs`, `tests/diagram_golden_test.rs` |
| draw.io XML | Structural re-parse: well-formedness, unique ids, resolving parent/source/target refs | `tests/diagram_golden_test.rs` |
| TUI | `TestBackend` buffer snapshots + key-event sequences against the pure `App` | `tests/tui_test.rs` |

The canonical fixture estate lives in `tests/common/mod.rs`: two
subscriptions, peered hub/spoke VNets, a VM with NIC + public IP, a storage
account with findings, a private endpoint → SQL. If you add a feature, extend
the fixture so it's exercised, then `INSTA_UPDATE=always cargo test` (or
`cargo insta review`) to regenerate goldens — and eyeball the diff before
committing.

Golden tests filter out volatile values (snapshot uuids, timestamps) via insta
filters; keep output deterministic (sort everything) or the goldens will flap.

## Common tasks

### Add a query to the pack

1. Create `queries/<category>/<name>.toml` (see [usage.md](usage.md#custom-queries)
   for the schema and KQL rules — deterministic `order by`, no `count` column).
2. `cargo build` embeds it; `azdocs query list` should show it.
3. Test the KQL live: `azdocs query run <name>`.
4. If it's a new category, it automatically becomes a new report section and
   XLSX sheet.

### Add an edge kind

1. Add the variant to `model::EdgeKind` (+ `as_str`/`parse`).
2. Write the extractor in `collect/extractors.rs` and register the type in
   `extract()`'s match.
3. Unit-test it with realistic property JSON, including a malformed variant.
4. Optionally surface it in diagrams (`diagram/graph.rs`) and the docs
   "Related" lists (automatic).

### Add a resource type's polish

- Display name: `model/azure_types.rs` (`DISPLAY_NAMES`).
- draw.io icon: `diagram/icons.rs` (`ICONS` — azure2 category + SVG name).
  Unmapped types fall back to a generic icon, so this is cosmetic only.

### Add a report format

Implement an emitter in `src/report/` that consumes `ReportContext` (never the
store directly), wire it into `commands/report.rs` and the `ReportFormat`
enum, and golden-test it from the fixture estate.

### Schema change

Append a new migration string to `MIGRATIONS` in `store/schema.rs`. The
`migrate_is_idempotent` test plus opening any pre-existing db covers the
upgrade path. Cascade-delete new tables from `snapshots`.

## Release

CI (`.github/workflows/ci.yml`) runs fmt/clippy/test on Linux, macOS, and
Windows for every push. Tagging `v*` triggers `release.yml`, which builds:

- macOS `aarch64` + `x86_64`
- Linux musl `x86_64` + `aarch64` (fully static)
- Windows `x86_64`

and attaches archives to a GitHub release with generated notes.

```sh
git tag v0.2.0 && git push origin v0.2.0
```

## Crate-choice notes (a.k.a. things that will bite you)

- **reqwest 0.13** renamed the TLS feature: it's `rustls`, not `rustls-tls`.
- **comfy-table 8** uses `load_style(...)`, not `load_preset(...)`.
- **ratatui 0.30**: `ratatui::init()`/`restore()`, crossterm re-exported at
  `ratatui::crossterm`, `TestBackend` under `ratatui::backend`.
- **serde_json** needs the `preserve_order` feature — column order in query
  output and report tables follows ARG's projection order because of it.
- **rusqlite** is `bundled` (SQLite compiled in) — that's what keeps the
  binary dependency-free; don't switch it to a system lib.
- **Excel sheet names** are case-insensitive and capped at 31 chars — the
  category sheets are suffixed `" queries"` to dodge collisions with the fixed
  `Inventory` sheet.

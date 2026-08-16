# azdocs

Rust CLI that audits an Azure estate via Azure Resource Graph (read-only
service principal), stores snapshots in SQLite, and exports reports/diagrams
offline. Full docs: [docs/](docs/README.md) — usage/, development/, reference/
(GitHub-style: folder READMEs as indexes, Mermaid diagrams; keep that format).

## Commands

```sh
cargo test                                        # full suite, no Azure/network needed
cargo clippy --all-targets --locked -- -D warnings
cargo fmt --check
cargo insta review                                # accept intended golden-output changes
INSTA_UPDATE=always cargo test                    # regenerate all goldens (then eyeball the diff)
cargo run -- <subcommand>                         # check/collect need real credentials
```

CI gates on fmt + clippy `-D warnings` + tests across Linux/macOS/Windows.
Always run fmt and clippy before committing.

## Architecture (one-way dependency flow)

```
cli → commands → collect / report / diagram / tui → store / model ← arg / auth
```

- **report/diagram/tui read ONLY from SQLite, never the network.** Never break
  this — it's what makes offline export and golden-file testing work.
- All SQL lives in `src/store/`. Migrations are append-only strings in
  `store/schema.rs`; never edit an existing migration.
- `ReportContext` (`src/report/mod.rs`) is built once from the store and
  consumed by every report emitter; emitters never query the store directly.
- Diagrams: builders in `diagram/graph.rs` produce `EstateGraph`s (single or
  per-VNet/per-RG fan-out), consumed by the mermaid, drawio (single sheet +
  workbook), and svg emitters; png rasterises the svg. Layout math in
  `diagram/layout.rs`; `diagram/assets.rs` feeds diagrams to the HTML/PDF
  reports.
- Report emitters: md/html/site/csv/xlsx plus pdf (embedded Typst,
  `templates/typst/report.typ`) and docx (docx-rs). All take
  `&BrandingContext` (`report/branding.rs`, from `[branding]` config).

## Non-negotiable invariants

- **Lowercase ARM ids for all join keys** (via `model::normalize_arm_id`);
  original casing lives in `display_id`. ARG returns inconsistent casing —
  skipping this creates phantom duplicate resources.
- **Edges and config-driven audits are Rust post-passes**
  (`collect/extractors.rs`, `collect/audit.rs`), never extra ARG queries.
- **Deterministic output everywhere** (sort by name/id): golden tests and
  `$skipToken` pagination both depend on it.
- KQL queries that can paginate must end `| order by id asc`. Never name a
  projected column `count` (KQL reserved word → ARG HTTP 400).
- Auth stays hand-rolled behind the `TokenProvider` trait (static dispatch);
  do not introduce `azure_identity`.
- Config structs use `deny_unknown_fields`; keep it that way so typos fail.

## Queries are data

Built-in query pack = TOML files in `queries/<category>/`, embedded via
`include_dir`. New audit check = new TOML file (kind `finding` requires
`severity`), not Rust. Routing of rows to tables is in `collect/ingest.rs`:
`all_resources`/`subscriptions`/`resource_groups` fill typed tables, other
inventory rows go to `query_results`, findings to `findings`. Resource
display names are data too: `data/display_names.toml` (embedded), user
overrides at `<config dir>/azdocs/display_names.toml`.

## Testing layout

- `tests/common/mod.rs` — canonical fixture estate (2 subs, peered VNets,
  VM+NIC+PIP, findings, private endpoint). Extend it when adding features so
  goldens exercise them.
- `tests/arg_client_test.rs` — wiremock (token, pagination, 429/5xx).
- `tests/report_golden_test.rs`, `tests/diagram_golden_test.rs` — insta
  goldens + drawio structural re-parse (unique ids, resolving refs).
- `tests/tui_test.rs` — ratatui `TestBackend` buffers; `tui::App` is a pure
  state machine, test it via `handle_key` sequences.
- Extractor/audit unit tests live next to the code with adversarial JSON.

## Crate gotchas (already fought, don't refight)

- reqwest 0.13: TLS feature is `rustls` (not `rustls-tls`).
- comfy-table 8: `load_style`, not `load_preset`.
- ratatui 0.30: `ratatui::init()`/`restore()`, crossterm at
  `ratatui::crossterm`, `TestBackend` in `ratatui::backend`.
- serde_json `preserve_order` feature is load-bearing (column order).
- rusqlite stays `bundled`; Excel sheet names are case-insensitive/31-char
  (category sheets are suffixed `" queries"` for this reason).
- typst/typst-pdf/typst-assets are pinned to the same minor (0.13); the World
  impl in `report/pdf.rs` derives today()/timestamps from the snapshot so PDF
  bytes stay deterministic. The document face is vendored in `data/fonts`
  (typst-assets ships no proportional sans); its two families stay loaded
  behind it purely as a glyph fallback, and font-book insertion order is
  load-bearing.
- resvg and usvg are lockstep-released — always bump them together.
- drawio single-sheet output must stay byte-identical (bare n{i}/e{i} cell
  ids, sheet id `azdocs-0`); workbook sheets prefix ids `s{i}-`.

## Paths & outputs

- Config/db live in platform dirs (`directories::ProjectDirs` — on macOS
  `~/Library/Application Support/azdocs/`); overridable with `--config`/`--db`.
- All exports default under `./output/` (gitignored). `azdocs.toml` contains a
  client secret — it and `azdocs.db` must never be committed.
- User query overrides: `<config dir>/azdocs/queries.d/*.toml`, merged over
  built-ins by name.

## Style

- thiserror enums per layer (`error.rs`); `anyhow` only at the CLI boundary.
- No `unwrap`/`expect` outside tests (except statically-infallible cases with
  a message saying why).
- Comments explain *why*/constraints only; tests are named
  `unit_does_x_when_y` with one behaviour per test.

# Contributing

## Recipes

### Add a query to the pack

1. Create `queries/<category>/<name>.toml` — schema and KQL rules in
   [usage/queries.md](../usage/queries.md#custom-queries) (deterministic
   `order by`, no column named `count`).
2. `cargo build` embeds it; `azdocs query list` should show it.
3. Test live: `azdocs query run <name>`.
4. A new category automatically becomes a report section and an XLSX sheet.

### Add an edge kind

1. Add the variant to `model::EdgeKind` (+ `as_str`/`parse`).
2. Write the extractor in `collect/extractors.rs`; register the resource type
   in `extract()`.
3. Unit-test with realistic property JSON, including a malformed variant.
4. Diagrams (`diagram/graph.rs`) and detail-page "Related" lists pick it up.

### Polish a resource type

- Display name: `model/azure_types.rs` → `DISPLAY_NAMES`.
- draw.io icon: `diagram/icons.rs` → `ICONS` (azure2 category + SVG name).
  Unmapped types get a generic icon; this is cosmetic only.

### Add a report format

Write an emitter in `src/report/` consuming `ReportContext` (never the store
directly), wire it into `commands/report.rs` and the `ReportFormat` enum,
golden-test it from the fixture estate.

### Change the schema

Append a migration to `MIGRATIONS` in `store/schema.rs`; never edit existing
ones. Cascade-delete new tables from `snapshots`.

## Style

- `thiserror` enums per layer (`error.rs`); `anyhow` only at the CLI boundary.
- No `unwrap`/`expect` outside tests, except statically-infallible cases with
  a message saying why.
- Comments explain *why* and constraints — not what the next line does.
- Tests: one behaviour per test, named like `unit_does_x_when_y`.
- Everything user-visible is deterministic (sorted) — goldens depend on it.

## Crate gotchas (already fought — don't refight)

| Crate | Gotcha |
|---|---|
| reqwest 0.13 | TLS feature is `rustls`, not `rustls-tls` |
| comfy-table 8 | `load_style(...)`, not `load_preset(...)` |
| ratatui 0.30 | `ratatui::init()`/`restore()`; crossterm at `ratatui::crossterm`; `TestBackend` in `ratatui::backend` |
| serde_json | `preserve_order` feature is load-bearing (column order everywhere) |
| rusqlite | stays `bundled` — that's what keeps the binary dependency-free |
| rust_xlsxwriter | Excel sheet names: case-insensitive, 31-char cap — category sheets are suffixed `" queries"` to avoid colliding with `Inventory` |

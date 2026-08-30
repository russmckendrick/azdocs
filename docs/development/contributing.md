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

1. Add the variant to `model::EdgeKind` (+ `as_str`/`parse` and the
   round-trip test).
2. Write the extractor in `collect/extractors.rs`; register the resource type
   in `extract()`.
3. Unit-test with realistic property JSON, including a malformed variant.
4. The desktop's `kind_class` match (`desktop/src-tauri/src/topology.rs`) is
   exhaustive, so the compiler makes you assign the kind to a filter family
   (network / structure / data / identity / monitoring); mirror it in
   `desktop/src/components/topology-fallback.ts` for the browser preview.
5. Detail-page "Related" lists and the desktop graphs pick it up
   automatically; the CLI diagram (`diagram/graph.rs::add_resource_edges`)
   draws only its own whitelist — extend that deliberately if the kind should
   appear in printed diagrams.

### Change the desktop relationship renderer

Read [Desktop relationship maps](desktop-relationships.md) before editing the
layout or Cytoscape component. Keep graph semantics in Rust, deterministic
placement in `topology-layout.ts`, and pure connector/label decisions in
`topology-presentation.ts`. A rendered relationship uses two invisible
boundary-port nodes, while tracing and camera logic continue to use its
logical source and target ids. Never move relationship text back into
Cytoscape edge labels.

Add or update pure-function tests for every geometry change, including peer
ordering at shared endpoints and behaviour after a node crosses its peer.
Run `pnpm test` and `pnpm run build` in `desktop/`, then review estate, group,
and neighbourhood scopes at all documented viewport widths in both themes.

### Polish a resource type

- Display names: `data/display_names.toml`, `data/azure_locations.toml`, and
  `data/azure_kinds.toml` (embedded, no recompile needed for
  users — they can override it in their config dir).
- Icon: `data/icon_mapping.toml`, resolved against the vendored pack in
  `data/icons/`. Unmapped types fall back to the pack's generic icon and then
  to a generated monogram tile; this is cosmetic only. The icon appears in
  diagrams *and* on the level-2 resource-type index headings in the PDF/DOCX.

### Add a report format

Write an emitter in `src/report/` consuming `ReportContext` (never the store
directly), wire it into `commands/report.rs` and the `ReportFormat` enum,
golden-test it from the fixture estate. If it is a *styled* format, take
`&BrandingContext` and read every colour and size from `branding.tokens` —
never hardcode a palette.

### Change the PDF/DOCX report

Edit the composition once in `src/report/document.rs`. Its `PrintDocument`
builder owns the cover metadata, TOC depth, chapter order, shared labels,
display-ready fact and table values, icon references, captions and diagram
placement.
The Typst and DOCX backends are exhaustive renderers of those blocks and must
not reconstruct report-specific loops or lookup maps.

Reordering content or adding content made from an existing block changes only
the shared builder. Adding a genuinely new visual primitive requires a new
exhaustive block variant plus implementations in both
`templates/typst/report.typ`/`theme.typ` and `report/docx/sections.rs`/`style.rs`.
Add a semantic-model test and extend the rendered parity/OOXML tests whenever
the block stream changes.

### Add a document theme

Drop a TOML file into `data/themes/`. Do not add Rust: if a theme needs
something the schema cannot express, add a new *variant* to one of the layout
strategy enums in `report/theme/mod.rs` and implement it in every emitter
(`templates/typst/theme.typ`, `report/docx/style.rs`, `report/site.rs`,
`templates/html/report.html.j2`). See
[reference/themes.md](../reference/themes.md).

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

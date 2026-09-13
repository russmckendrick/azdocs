# Contributing

See the root [contribution guide](../../CONTRIBUTING.md) for issue reports,
licensing and handling sensitive evidence. The [development index](README.md)
lists setup and validation commands.

## Recipes

### Add a query to the pack

1. Create `queries/<category>/<name>.toml` — schema and KQL rules in
   [usage/queries.md](../usage/queries.md#custom-queries) (deterministic
   `order by`, no column named `count`).
2. `cargo run --locked -- query list` compiles the current pack and should show it.
3. Test live with authorised credentials: `cargo run --locked -- query run <name>`.
   Add mocked or fixture tests for interpretation and ingestion as appropriate.
4. A new category becomes an inventory evidence section and an XLSX query sheet.
   Update the [query reference](../reference/queries.md), its counts and source
   notes. Custom checks use generic assessment guidance unless the shared
   assessment analysis explicitly supports them.

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

Read [Desktop map](desktop-relationships.md) before editing the
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
- draw.io icon: the `ICONS` table in `src/diagram/icons.rs`, naming a stencil
  in draw.io's own bundled `azure2` library. That is a *different* set from the
  vendored pack above, and draw.io renders a path it cannot resolve as a
  broken-image placeholder rather than failing — so every path is checked
  against `data/azure2_stencils.toml`, which also carries each stencil's aspect
  (they are not square). Regenerate it after upgrading draw.io:

  ```sh
  python3 data/tools/build_azure2_manifest.py          # rewrite the manifest
  python3 data/tools/build_azure2_manifest.py --check  # is it stale?
  ```

  The script reads draw.io's `app.asar`, so it needs the app installed; the
  tests only need the checked-in manifest.

### Add a report format

Write an emitter in `src/report/` consuming `ReportContext` (never the store
directly), wire it into `commands/report.rs` and the `ReportFormat` enum,
golden-test it from the fixture estate. If it is a *styled* format, take
`&BrandingContext` and read every colour and size from `branding.tokens` —
never hardcode a palette.

### Change the PDF/DOCX report

Edit chapter composition in `src/report/assessment.rs`: `build` creates the
main assessment and `reference` creates the optional technical companion.
`src/report/document.rs` defines their shared `PrintDocument` and block types,
cover metadata and supporting builders. `src/report/metadata.rs` selects
reference settings from `data/reference_fields.toml`.
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

### Change wording

Edit `data/labels/en.toml`, never a literal: no user-facing string lives in
Rust, the templates or the desktop TSX. Adding a string is a TOML key plus a
field on the matching struct in `src/labels/schema.rs`; the desktop reads its
type from `desktop/src/generated-labels.json`, regenerated by
`cargo test -p azdocs-desktop`. Report goldens pin the shipped wording, so a
change there is reviewed through the snapshot diff. Note that a local override
in `<config dir>/azdocs/labels/` is read by the tests too, so it will make
them drift. See [reference/labels.md](../reference/labels.md).

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
| rusqlite | stays `bundled` — no separately installed SQLite library is needed |
| rust_xlsxwriter | Excel sheet names: case-insensitive, 31-char cap — category sheets are suffixed `" queries"` to avoid colliding with `Inventory` |

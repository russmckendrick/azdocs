# azdocs

Rust CLI that audits an Azure estate via Azure Resource Graph (read-only
service principal), stores snapshots in SQLite, and exports reports/diagrams
offline. Full docs: [docs/](docs/README.md) — usage/, development/, reference/
(GitHub-style: folder READMEs as indexes, Mermaid diagrams; keep that format).

## Commands

This is a Cargo workspace: the CLI (`azdocs`, at the root) and the Tauri backend
(`azdocs-desktop`, in `desktop/src-tauri`) share one lockfile and one `target/`.
Bare `cargo test` runs the root package only — use `--workspace` for both.

```sh
cargo test                                        # CLI crate, no Azure/network needed
cargo test --workspace                            # CLI + desktop backend
cargo clippy --all-targets --locked -- -D warnings
cargo fmt --all --check
cargo insta review                                # accept intended golden-output changes
INSTA_UPDATE=always cargo test                    # regenerate all goldens (then eyeball the diff)
cargo run -- <subcommand>                         # check/collect need real credentials
```

The frontend uses **pnpm**, not npm (`tauri.conf.json` shells out to it):

```sh
cd desktop && pnpm install
pnpm run lint && pnpm run typecheck && pnpm test  # gated in CI
pnpm run tauri dev                                # run the desktop app
```

CI gates on fmt + clippy `-D warnings` + tests across Linux/macOS/Windows, and
on ESLint + tsc + vitest + `cargo test -p azdocs-desktop` for the desktop.
Always run fmt and clippy before committing.

A graphify knowledge graph of this repo lives in `graphify-out/` (gitignored).
For architecture or "what calls/uses X" questions, `/graphify query` it before
grepping; when rebuilding, exclude `data/icons/` (700+ stock SVGs).

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
  `diagram/layout.rs` (measure then justify), page geometry in `diagram/page.rs`,
  orthogonal connector routing in `diagram/route.rs`. `diagram/assets.rs` feeds
  diagrams to the reports — overviews and one summarised graph per resource
  group (capped by `[report] max_group_diagrams`, default `MAX_GROUP_DIAGRAMS`),
  each rendered once as SVG and once as Mermaid; truncation always logged.
  Every emitter keys its legend off `graph::legend_kinds`. drawio legend cells
  use their own `l{i}` id family, so the frozen `n{i}`/`e{i}` scheme is untouched.
  Full standards: [docs/reference/diagrams.md](docs/reference/diagrams.md).
- Report emitters: md/csv plus the themed set — html/site/xlsx, pdf (embedded
  Typst, `templates/typst/`) and docx (`report/docx/`). The themed ones take
  `&BrandingContext` (`report/branding.rs`), which carries the resolved
  `ThemeTokens`. md and csv stay deliberately unstyled.

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
- **Diagrams are sized for A4 portrait, not for their content.** A summary
  diagram is exactly the width of the text column and no taller than the sheet
  (`diagram::page`); `PX_PER_MM` is 4.0 so a layout pixel is exactly 0.25mm.
  The width sets the scale, so every diagram in a report prints its labels at
  the same size. Never let a diagram grow *wider* to fit its content — that is
  the defect this replaced, and it made every label unreadable once scaled to
  the page. Report diagrams carry no title band: the section heading above and
  the figure caption below already name them (`DiagramDetail::shows_title`).
- **Connectors are orthogonal and anchored on boundaries**, never straight
  centre-to-centre; containment edges are not drawn at all. Sides and channels
  are picked by scoring a fixed set of candidates against the other boxes —
  closed-form, no path search, because a report renders hundreds of diagrams.

## Queries are data

Built-in query pack = TOML files in `queries/<category>/`, embedded via
`include_dir`. New audit check = new TOML file (kind `finding` requires
`severity`), not Rust. Routing of rows to tables is in `collect/ingest.rs`:
`all_resources`/`subscriptions`/`resource_groups` fill typed tables, other
inventory rows go to `query_results`, findings to `findings`. Resource
display names are data too: `data/display_names.toml` (embedded), user
overrides at `<config dir>/azdocs/display_names.toml`. Region
names and coordinates too: `data/azure_locations.toml` and
`data/azure_regions.toml`, generated from Microsoft\'s published lists by
`cargo run --example update_azure_locations`. Region
names and coordinates too: `data/azure_locations.toml` and
`data/azure_regions.toml`, generated from Microsoft\'s published lists by
`cargo run --example update_azure_locations`.

## Themes are data

Same split as the queries, for document design. `data/themes/*.toml` supply
**values** — palette expressions over the two `[branding]` colours, a type
scale, and a choice from a closed set of layout strategies (`cover`, `table`,
`stat`). Each emitter implements every strategy once and branches only on
those values. **A new theme is a new TOML file; there must be no branch
anywhere on a theme's name.** User overrides live in
`<config dir>/azdocs/themes/`, merged by file stem.

- `report/theme/mod.rs` parses and resolves; `theme/color.rs` holds the colour
  maths and the `lighten/darken/mix/readable_on` expression language.
- `docs/reference/labels.md` documents the sibling pattern for wording.
- Anything drawn on top of a brand colour must go through `readable_on`, or a
  pale `primary_color` produces white-on-white.
- Field Report's PDF and Word use Charter / Arial / Courier New. The theme's
  `pdf_use_docx_fonts` prefers installed `docx_*` faces, loaded offline during
  branding resolution. Missing PDF faces fall back to the bundled IBM Plex
  families (`data/fonts`, OFL). DOCX names fonts and cannot embed them.

## Labels are data

The third instance of the pattern, for wording. `data/labels/en.toml` holds
every user-facing string — report headings and columns, diagram legends, CLI
progress lines, TUI labels, the desktop's copy — typed by `src/labels/schema.rs`
(`deny_unknown_fields`, no defaults). `[branding] labels` names the set; a
partial `<config dir>/azdocs/labels/<name>.toml` is deep-merged over the
built-in. **No user-facing literal in Rust, the templates or the desktop TSX**;
name a key. `{snake_case}` placeholders are filled by `labels::fill` (Rust) and
`fill` in `desktop/src/format.ts`; plurals are `{ one, other }` tables. Exempt:
clap `///` help, thiserror messages, `[branding] title`/`footer`.

- `BrandingContext.labels` reaches every report emitter; diagram builders,
  the TUI and commands take it explicitly. No global.
- The desktop ships `common` + `desktop` on `AppBootstrap` and types itself
  from `desktop/src/generated-labels.json`, **generated** by
  `cargo test -p azdocs-desktop` alongside `generated.ts` (CI checks both).
  `desktop.topology.edge_kinds` must name every `EdgeKind`.
- The built-in wording must reproduce today's output byte for byte: a wording
  change is a golden change, reviewed through the snapshots.

## Report structure

The default PDF and DOCX are assessments: executive observations, composition,
architecture, subscription profiles and selected group studies, recurring-check
explanations, governance/operational evidence, review actions and collection
coverage. `ReportContext.analysis` owns grouping, historical evidence and study
selection; `report/assessment.rs` composes both through `PrintDocument`.

`--include-reference` additionally writes `technical-reference.pdf`/`.docx`.
The reference owns the full type register, subscription → group → resource
body, selected operational settings, relationships, findings grouped by check /
resource / severity with complete occurrence counts, and collection coverage.
`data/reference_fields.toml` selects settings; field labels live in the labels
pack. Keep provider catalogues, raw query rows, scripts and repeated evidence in
SQLite and data exports, not print. Selection and value reductions must be
explicit. Do not restore exhaustive inventory to the main report or interpret
missing evidence as a passed control.
Exports do not create a separate provenance JSON file; exact queries and scope
remain in SQLite, with embedded provenance views in the data-oriented formats.

**Use the shared diagram libraries.** `diagram/graph/assessment.rs` only selects
and aggregates `EstateGraph`s. `diagram/assets.rs` renders them using the same
`page`, `layout`, `route`, `icons`, `svg` and `png` pipeline as existing diagrams.
No report-specific SVG renderer, geometry, connector router, palette, text
wrapper or substitute card system. Correct shared components and their tests
when a figure fails the standards. Split complex graphs; never shrink labels to
make an inventory picture fit. Generate only the assets used by selected exports.

## Diagram detail levels

The two consumers of a diagram want opposite things, and one flag
(`page::DiagramDetail`) settles it. `Summary` — the report path
(`diagram/assets.rs` → pdf/docx/html) — aggregates resources by type into
`×N` tiles at a fixed page width, with content-driven height capped by the page budget. `Full` — the
`azdocs diagram` CLI — draws every resource on a natural 1400px-wide canvas.
Adding a consumer means picking a level, not adding a branch.

Density rungs (`comfortable`/`compact`/`dense`) are chosen once per graph from
its box count, so one resource type is never drawn at two sizes in a picture.
Chrome decays with nesting depth; fonts and leaf sizes never do.

## Desktop design

The desktop app is a premium desktop data application: a dark navy
navigation frame in both modes, a cool neutral workspace, restrained Azure
blue, Inter throughout, and colour reserved for data and signals.
`desktop/src/styles.css` is the **only** place a colour value is written down
(light canonical; dark re-tunes the same `--az-*` roles in two blocks that
must agree; tri-state theme — System/Light/Dark — persisted from Settings).

- [docs/reference/design.md](docs/reference/design.md) — the authoritative sheet.
- [docs/reference/design.html](docs/reference/design.html) — swatches, type
  scale and marks, **generated** from `styles.css` and `docs/marks/manifest.json`
  by `docs/reference/tools/build_design_sheet.py`. Never edit it; re-run the
  script, which also fails if the two dark blocks in `styles.css` disagree.
- [DESIGN.md](DESIGN.md) — a pointer to the above, plus the rules most often
  broken. It used to restate the whole palette; three copies is how they drift.

Hard rules: no numbering chrome, selection is a quiet `--az-selected` fill
(never a coloured bar), blue only for actions/selection/links/series and
otherwise colour only for data (category set), signals (severity set) and the
three named accents, type never below 11px, cards only on Overview, Azure
artwork in the workspace and Lucide outline icons in navigation. The
Cytoscape stage reads `--graph-*`/`--kind-*` tokens at build time and
rebuilds on theme change — never hardcode a canvas colour. On macOS the
title bar overlays the navy frame (`titleBarStyle: Overlay`); the shell sets
`data-titlebar="overlay"` and the stylesheet pads and widens the rail for it.

## The wire contract is generated

`desktop/src/generated.ts` is emitted from the DTO structs by
`cargo test -p azdocs-desktop` (`desktop/src-tauri/src/bindings.rs`, ts-rs).
**Never edit it** — change the Rust struct and re-run the tests. CI fails if the
checked-in file is stale.

Three layers, and it matters which one a type belongs in:

- `generated.ts` — the wire contract. Source of truth is `dto.rs`/`topology.rs`.
- `api-types.ts` — closed string sets Rust models as an enum but serialises via
  `as_str()`, so the field is a bare `String` and ts-rs cannot infer the union.
  Hand-written, and pointed at from the struct with `#[ts(type = "...")]`.
- `types.ts` — UI-only types with no Rust counterpart. Re-exports the other two;
  everything imports from `./types`.
- `labels.ts` — `Labels`, inferred from `generated-labels.json` (also written by
  the bindings test). Wording only; never a wire shape.

Two traps this replaced, both of which had shipped:

- `#[serde(rename_all)]` on an **enum** renames the variants, not the fields of
  a struct variant. Struct-variant fields need `rename_all_fields`, or the
  payload goes out snake_case while every other field is camelCase.
- `Option<T>` serialises to `null`, not an absent key. The generated type is
  `field?: T | null`; do not "simplify" it to `field?: T`.

## Desktop topology

The Tauri explorer's relationship graphs are built in Rust
(`desktop/src-tauri/src/topology.rs`, command `topology_graph`), not in the
frontend — Cytoscape only renders the DTO. The invariant, enforced by unit
tests over a synthetic 1,000-resource estate: **drawn + folded + aggregated
== total, never a silent cap** (the old slice-based node limits are gone for
good). NICs/disks fold into their VM and child types into their ARM parent;
unlinked resources aggregate into ×N shelf tiles, and at estate level groups
with no cross-group edge aggregate into one ×N tile per subscription
(`group_ids` on the node) so only connected groups cost a card; other groups' neighbours
appear as ghost "external" stubs; subscriptions open collapsed to bars and
expand into lanes only on request; anything a filter hides is counted in
`counts.hidden_by_filter`. Edge kinds map to filter families via the
exhaustive `kind_class` match — a new `EdgeKind` forces a classification
(mirror it in `desktop/src/components/topology-fallback.ts`, the
browser-preview stand-in; `topology-fallback.test.ts` fails if you don't).
Subnets are not resource rows, so `desktop/src-tauri/src/dto.rs` collapses
subnet-ended edges onto the owning VNet before they reach the frontend.

**Group membership is Rust's too.** `desktop/src-tauri/src/groups.rs` decides
which resource group a resource belongs to, including synthesising a group when
the snapshot has no row for one, and names a group-less resource
`SUBSCRIPTION_SCOPE`. It feeds both the topology builder and
`EstateSnapshot.resourceGroupSummaries`, so the map and the estate view cannot
disagree. `topology-model.ts` still implements the same rule, but **only the
browser preview may reach it** — it is stripped from a Tauri build, and a
production component importing it puts a second, drifting implementation back
in the app. That is what it was doing before.

**So is the governance analysis.** `src/report/governance.rs` owns the tag
thresholds *and* the analysis applying them — key and subscription coverage,
who is missing a required tag, which groups are worst, and which of those are
past the flagged share. It reads the stored `missing_required_tags` findings,
not today's `required_tags`, so a snapshot always describes the estate as it
was audited. `ReportContext.governance` feeds the print/markdown Governance
chapter and `EstateSnapshot.governance` feeds `GovernanceView.tsx`, which now
only draws it. The wire contract carries **verdicts** (`healthy`, `flagged`),
never the raw thresholds for a surface to compare against — that shape is what
let the explorer keep its own `>= 60` and its own `Math.round`.

Frontend rendering has another hard boundary. Read
`docs/development/desktop-relationships.md` before changing it.
`topology-layout.ts` owns deterministic zones, connection-aware rails, entry
sets and camera profiles; `topology-presentation.ts` owns pure connector-port,
taxi-channel and label-placement decisions; `CytoscapeResourceGraph.tsx`
coordinates paint and interaction. Every logical link renders through its own
invisible source and target boundary-port nodes, ordered on each side by the
opposite endpoint's spatial position. The edge retains `logicalSource` and
`logicalTarget`; tracing, counts, cameras, navigation and accessibility must
never use the synthetic endpoints. Relationship text is a DOM overlay above
all connector paint, never a Cytoscape edge label. Group entry includes the
connected core and external rail but excludes the unconnected shelf;
neighbourhood entry excludes second hops; estate entry includes collapsed
subscription bars. Do not replace these rules with direct logical-node edges,
shared ports, name-only grids or indiscriminate Fit all.

## Testing layout

- `tests/common/mod.rs` — canonical fixture estate (2 subs, peered VNets,
  VM+NIC+PIP, findings, private endpoint). Extend it when adding features so
  goldens exercise them.
- `tests/arg_client_test.rs` — wiremock (token, pagination, 429/5xx).
- `tests/report_golden_test.rs`, `tests/diagram_golden_test.rs` — insta
  goldens + drawio structural re-parse (unique ids, resolving refs).
- `tests/report_pdf_test.rs` / `report_docx_test.rs` — no snapshots: the PDF is
  checked by compiling every theme with zero Typst warnings, extracting text
  with lopdf and re-rendering for byte equality; the DOCX by unzipping and
  matching the OOXML parts. Both assert themes differ in *layout*, not just
  colour.
- `tests/report_preview.rs` — `#[ignore]`d; writes every theme in every format
  to `output/preview/<theme>/` for eyeballing:
  `cargo test --test report_preview -- --ignored --nocapture`.
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
  impl in `report/pdf.rs` derives today()/timestamps and the document id from
  the snapshot so PDF bytes stay deterministic within a process for a fixed
  snapshot, theme and font files. Across processes typst-pdf orders font and
  image objects from `HashMap`s, so bytes can differ while content does not;
  do not promise cross-process PDF byte identity.
- docx-rs numbers hyperlinks, pictures, bookmarks and paragraphs from
  process-global counters: a second render in one process gets higher ids.
  `report/docx/mod.rs` sorts the relationship part so separate processes
  produce identical bytes; in-process tests compare modulo those numbers.
  Bundled fallback faces live in `data/fonts`; typst-assets' two families stay loaded
  behind it purely as a glyph fallback, and font-book insertion order is
  load-bearing.
- resvg and usvg are lockstep-released — always bump them together.
- drawio's **id scheme** is frozen: single-sheet cells stay bare n{i}/e{i} with
  sheet id `azdocs-0`, workbook sheets prefix ids `s{i}-`. Geometry is not —
  any layout change moves every `<mxGeometry>`, and that is reviewed through
  the goldens rather than forbidden.

## Paths & outputs

- Config/db live in platform dirs (`directories::ProjectDirs` — on macOS
  `~/Library/Application Support/azdocs/`); overridable with `--config`/`--db`.
- All exports default under `./output/` (gitignored). `azdocs.toml` uses native secret references or explicit environment references;
  legacy files and protected migration backups may contain plaintext secrets.
  Local configs, backups and `azdocs.db` must never be committed.
- User query overrides: `<config dir>/azdocs/queries.d/*.toml`, merged over
  built-ins by name; user themes: `<config dir>/azdocs/themes/*.toml`, merged
  by file stem.

## Style

- thiserror enums per layer (`error.rs`); `anyhow` only at the CLI boundary.
- No `unwrap`/`expect` outside tests (except statically-infallible cases with
  a message saying why).
- Comments explain *why*/constraints only; tests are named
  `unit_does_x_when_y` with one behaviour per test.

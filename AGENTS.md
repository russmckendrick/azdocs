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
  `diagram/layout.rs` (measure then justify), page geometry in `diagram/page.rs`,
  orthogonal connector routing in `diagram/route.rs`. `diagram/assets.rs` feeds
  diagrams to the reports — overviews, one summarised graph per resource group
  (capped at `MAX_GROUP_DIAGRAMS`) and one `neighbourhood` graph per connected
  resource (capped at `MAX_RESOURCE_DIAGRAMS`); truncation always logged.
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
  diagram is emitted at exactly a quarter, third, half or full page
  (`diagram::page`); `PX_PER_MM` is 4.0 so a layout pixel is exactly 0.25mm.
  Never let a diagram grow to fit its content — that is the defect this
  replaced, and it made every label unreadable once scaled to the page.
- **Connectors are orthogonal and anchored on boundaries**, never straight
  centre-to-centre; containment edges are not drawn at all. Routing is
  closed-form with no obstacle search — a report renders hundreds of diagrams.

## Queries are data

Built-in query pack = TOML files in `queries/<category>/`, embedded via
`include_dir`. New audit check = new TOML file (kind `finding` requires
`severity`), not Rust. Routing of rows to tables is in `collect/ingest.rs`:
`all_resources`/`subscriptions`/`resource_groups` fill typed tables, other
inventory rows go to `query_results`, findings to `findings`. Resource
display names are data too: `data/display_names.toml` (embedded), user
overrides at `<config dir>/azdocs/display_names.toml`.

## Themes are data

Same split as the queries, for document design. `data/themes/*.toml` supply
**values** — palette expressions over the two `[branding]` colours, a type
scale, and a choice from a closed set of layout strategies (`cover`, `table`,
`stat`). Each emitter implements every strategy once and branches only on
those values. **A new theme is a new TOML file; there must be no branch
anywhere on a theme's name.** User overrides live in
`<config dir>/azdocs/themes/`, merged by file stem.

- `report/theme.rs` parses and resolves; `theme/color.rs` holds the colour
  maths and the `lighten/darken/mix/readable_on` expression language.
- Anything drawn on top of a brand colour must go through `readable_on`, or a
  pale `primary_color` produces white-on-white.
- The PDF's type is vendored (`data/fonts`, IBM Plex, OFL). DOCX cannot embed
  fonts, so themes carry separate `docx_sans`/`docx_mono` names that Word can
  resolve locally — do not point those at the vendored family.

## Diagram detail levels

The two consumers of a diagram want opposite things, and one flag
(`page::DiagramDetail`) settles it. `Summary` — the report path
(`diagram/assets.rs` → pdf/docx/html) — aggregates resources by type into
`×N` tiles and snaps the canvas to a page fraction. `Full` — the
`azdocs diagram` CLI — draws every resource on a natural 1400px-wide canvas.
Adding a consumer means picking a level, not adding a branch.

Density rungs (`comfortable`/`compact`/`dense`) are chosen once per graph from
its box count, so one resource type is never drawn at two sizes in a picture.
Chrome decays with nesting depth; fonts and leaf sizes never do.

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
  impl in `report/pdf.rs` derives today()/timestamps from the snapshot so PDF
  bytes stay deterministic. The document face is vendored in `data/fonts`
  (typst-assets ships no proportional sans); its two families stay loaded
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
- All exports default under `./output/` (gitignored). `azdocs.toml` contains a
  client secret — it and `azdocs.db` must never be committed.
- User query overrides: `<config dir>/azdocs/queries.d/*.toml`, merged over
  built-ins by name; user themes: `<config dir>/azdocs/themes/*.toml`, merged
  by file stem.

## Style

- thiserror enums per layer (`error.rs`); `anyhow` only at the CLI boundary.
- No `unwrap`/`expect` outside tests (except statically-infallible cases with
  a message saying why).
- Comments explain *why*/constraints only; tests are named
  `unit_does_x_when_y` with one behaviour per test.

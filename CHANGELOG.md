# Changelog

Notable changes to azdocs. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/). Release notes for GitHub Releases
are cut from the Unreleased section at tag time; see
[releasing](docs/development/releasing.md#cut-the-release-notes).

## [Unreleased]

Nothing yet.

## [0.3.0] - 2026-09-22

### Added

- Regions explorer with an interactive world map, Azure region metadata,
  resource counts, zoom controls and region details.
- Overview resource-history chart with selectable snapshots, clearer date
  labels and markers that remain usable when collections are close together.
- Collection `warnings` status for complete inventory with failed finding
  checks. These snapshots remain available through `latest` and history;
  missing audit evidence remains recorded, and `--fail-on partial` still
  returns a non-zero exit code for warnings.
- Inline website capture previews in the collection panel, using an isolated
  native child webview without opening another window.

### Changed

- Desktop layout with a navy navigation frame, denser data views, Inter
  typography, restrained selection fills and consistent light/dark tokens.
- Overview and Regions share the world-map backdrop; Overview keeps a static
  summary while Regions provides the interactive exploration controls.
- Collection progress groups inventory, endpoint discovery and screenshots,
  with compact permission results and expandable collection options.
- Dependabot groups updates and limits concurrent routine pull requests.

### Fixed

- Region map clicks select the nearest marker when hit areas overlap.
- The unused managed identity check uses an Azure Resource Graph-supported
  join, and nested API errors retain the underlying query diagnostics.
- Dropped inventory rows mark a collection partial instead of complete.

## [0.2.0] - 2026-09-19

### Added

- Snapshot lifecycle: heartbeat and reconciliation of abandoned collects,
  `cancelled` status, read-only opens for every explorer and export path,
  `snapshots delete/prune/verify`, `--fail-on`, `--dry-run`, `--quiet`,
  `--format json` on `check`, `snapshots list/show/verify` and `query list`,
  `init --secret-env`, `NO_COLOR`.
- Bounded retries with jitter for Entra, Resource Graph and ARM, configurable
  under `[collect.retry]`; sovereign clouds through `cloud`.
- Field-level snapshot comparison (`snapshots diff` as table, Markdown or
  JSON) with a noise list in `data/diff_ignore.toml`, a snapshot trend, and
  cancellation of a running collect from the CLI (Ctrl-C) and the desktop.
- 34 new queries: edge services, container platforms, monitoring and
  security checks; relationships for NAT gateways, firewalls, VPN gateways
  and connections, container apps, flexible servers, disk encryption sets,
  availability sets, application security groups, scale sets and backup
  vaults; resource-group and subscription tag audits.
- Report scoping (`--subscription`, `--resource-group`, `--severity`),
  diagrams in Markdown and HTML with Mermaid source, legends in every
  diagram format, a derived dark palette per theme, a "changes since the
  previous snapshot" chapter and trend, website evidence in the assessment,
  honest caps under `[report]`, and broader CSV/XLSX exports.
- Desktop: reads off the main thread, on-demand comparison and resource
  detail, cancel and scope for collection, cancel and reveal for exports,
  findings multi-select with copy and CSV, field-level history, tag facet,
  remembered filters, About and keyboard shortcuts, text scaling, single
  instance, window state, clipboard and file-manager commands.
- Fixture with every relationship kind and an older sibling snapshot;
  command-layer, TUI and desktop component tests; TUI keys overlay,
  severity floor and relationship following.
- CI gates for dependency policy, frontend audit, licence notices, the
  minimum Rust version, musl targets and coverage; pinned actions; SBOMs
  and build-provenance attestations on release assets; optional Windows
  Trusted Signing; Dependabot, CODEOWNERS and issue and PR templates.

### Changed

- Implicit `latest` and the "previous snapshot" resolve only to complete or
  partial snapshots.
- Excel cell text is bounded to Excel's limit rather than failing the
  workbook; workbook document properties come from the snapshot, the PDF's
  document id from the snapshot and document kind, and the DOCX's
  relationship part is written in id order. Markdown, HTML, CSV, XLSX and
  DOCX exports of one snapshot are byte-identical run to run; the PDF's
  content is identical but typst-pdf numbers its font and image objects
  from hash maps, so its bytes can differ between processes.
- Desktop type and spacing are rem-based; only the referenced icons and
  fonts are bundled.

## [0.1.1] - 2026-09-13

Softer Azure desktop theme and clearer Settings surfaces. See
[docs/releases/0.1.1.md](docs/releases/0.1.1.md).

## [0.1.0] - 2026-09-12

First public release: CLI, TUI and desktop explorer. See
[docs/releases/0.1.0.md](docs/releases/0.1.0.md).

[Unreleased]: https://github.com/russmckendrick/azdocs/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/russmckendrick/azdocs/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/russmckendrick/azdocs/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/russmckendrick/azdocs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/russmckendrick/azdocs/releases/tag/v0.1.0

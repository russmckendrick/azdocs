# Testing

```sh
cargo test --locked                 # CLI crate; HTTP tests use local mocks
cargo test --workspace --locked     # both Rust packages (CI tests them in separate jobs)
cargo test -p azdocs-desktop        # desktop topology, DTO and command helpers
cargo test --test collect_test      # one integration suite
cargo insta review                  # accept intended golden changes
INSTA_UPDATE=always cargo test      # regenerate all goldens (eyeball the diff!)
cd desktop && pnpm test             # relationship UI helpers and Rust↔TS mirrors
```

## Layers

```mermaid
flowchart TD
    subgraph mocked["HTTP mocked (wiremock)"]
        auth_t[token flow]
        arg_t[pagination, 429/5xx retry]
    end
    subgraph fixtures["Fixture-driven (in-memory SQLite)"]
        ingest_t[ingest + store]
        extract_t[edge extractors]
    end
    subgraph golden["Golden files (insta)"]
        report_t[report pages]
        diagram_t[mermaid + drawio + SVG/PNG]
    end
    tui_t[TUI TestBackend buffers]
```

| Layer | Approach | Where |
|---|---|---|
| HTTP (auth, ARG) | wiremock: token flow, `$skipToken` chains, 429/5xx | `tests/arg_client_test.rs` |
| Ingest/store | Fixture JSON → in-memory SQLite → assertions | `tests/collect_test.rs` |
| Edge extractors | Pure-function tests incl. adversarial inputs (nulls, mixed-case ids, cross-sub peerings) | `src/collect/extractors.rs` |
| Reports/diagrams | insta goldens from the fixture estate | `tests/report_golden_test.rs`, `tests/diagram_golden_test.rs` |
| draw.io XML | Structural re-parse: well-formed, unique ids, resolving refs | `tests/diagram_golden_test.rs` |
| Diagram geometry | Summary canvases obey the page-width/height budget; every connector segment is axis-aligned; routes are deterministic | `src/diagram/svg.rs`, `src/diagram/route.rs`, `src/diagram/page.rs` |
| TUI | `TestBackend` buffer snapshots + key-event sequences | `tests/tui_test.rs` |
| Desktop topology | Pure-function tests over a synthetic 1,000-resource estate: drawn + folded + aggregated always equals total, deterministic output, fan-out folding, scope filters counted | `desktop/src-tauri/src/topology.rs` |
| Desktop relationship UI | Vitest at 1440/1060/800px: deterministic zones, rail alignment, directional neighbourhoods, camera targets, spatial navigation, per-edge boundary ports, taxi channels, label placement, trace priority, and retained-error state | `desktop/src/components/topology-layout.test.ts`, `desktop/src/components/topology-presentation.test.ts`, `desktop/src/components/topology-view-state.test.ts` |
| Rust↔TypeScript mirrors | Reads `src/model/mod.rs` and `topology.rs` and asserts the browser preview classifies every `EdgeKind` into the same family the Rust `match` does | `desktop/src/components/topology-fallback.test.ts` |
| Desktop navigation | History-aware drill-down and the relationship workspace reducer | `desktop/src/navigation-state.test.ts` |
| Wire contract | Regenerates `desktop/src/generated.ts` from the DTOs; CI then fails on `git diff` if it is stale. Also pins the serialised bytes of the event enums | `desktop/src-tauri/src/bindings.rs`, `dto.rs` |
| Display metadata | Asserts the Rust and TypeScript `humanize_identifier` agree on a shared case list — both are live, one renders the reports and one the desktop | `desktop/src/azure-values.test.ts`, `src/model/azure_values.rs` |

The full relationship rendering and screenshot review contract is in
[Desktop map](desktop-relationships.md#tests-and-review).

## The fixture estate

`tests/common/mod.rs` seeds the canonical estate every golden test renders:
two subscriptions, peered hub/spoke VNets, a VM with NIC + public IP, a
storage account with findings, a private endpoint → SQL server with a
database child, and a web app on its plan carrying a user-assigned identity.

**When you add a feature, extend the fixture so goldens exercise it**, then
regenerate and review:

```sh
INSTA_UPDATE=always cargo test
git diff tests/snapshots/    # the diff IS the review
```

## Rules for stable goldens

- Sort every output collection (by name, then id).
- Volatile values (snapshot uuids, timestamps) are normalised by insta filters
  in the test setup — extend the filters rather than embedding volatility.

## Configuration and tenant regression tests

- `tests/configuration_test.rs`: comment preservation, external revision conflicts,
  migration/backups, rollback on secret failures, redaction, inheritance and paths.
- `tests/tenant_store_test.rs`: mixed-tenant latest/history/deletion, explicit
  offline snapshot IDs and cross-tenant comparison rejection.
- `tests/permission_diagnostics_test.rs`: mocked Reader/custom roles, mixed grants,
  per-block exclusions, data-plane grants, inherited/group/narrower scopes,
  conditions, pagination, partial failures, throttling and authentication errors.
- `desktop/src/settings-model.test.ts`: profile editing and sparse override rules.
- `desktop/src-tauri/src/settings.rs`: bootstrap/settings/export redaction,
  missing and invalid config recovery, check revision invalidation and database
  selection failures that preserve the previous config and history.
- `tests/credential_store_test.rs`: ignored by default; writes, verifies and
  deletes an isolated UUID entry in the native store. It never uses user secrets.

```sh
cargo test --test credential_store_test -- --ignored
```

Run native storage smoke tests on each supported OS with an unlocked native
store. Linux needs a session D-Bus and Secret Service (for example GNOME Keyring).
CI exercises these separately from the offline suite.

Settings review covers 1440×900 and 980×680, light and dark modes, first-run and
invalid-file states, draft testing, inherited/empty overrides, Save/Discard,
keyboard focus and scrolling all the way to advanced branding fields. The
header and action bar must remain reachable and no real secret may be captured.

Run `cargo fmt --all --check`, workspace Clippy with warnings denied, workspace
tests, frontend lint/typecheck/Vitest, and the generated-contract checks. Config
changes should not change report goldens unless report content was intended.

## Documentation and generated files

Python 3.11+ is required for the documentation checker. From the repository root:

```sh
python3 docs/development/tools/check_docs.py
python3 docs/reference/tools/build_design_sheet.py --check
```

The checker validates local Markdown/HTML links and anchors, parses TOML examples,
and compares the query catalogue's names, counts and category chart with the
actual TOML pack. It is offline; it does not claim external URLs or live KQL were
tested. CI runs both documentation checks on Linux.

The desktop backend tests regenerate both `desktop/src/generated.ts` and
`desktop/src/generated-labels.json`. After a contract or label change, run:

```sh
cargo test -p azdocs-desktop --locked
git diff -- desktop/src/generated.ts desktop/src/generated-labels.json
```

Review and commit intended generated changes. CI uses `git diff --exit-code`
after generation to detect missing updates. For a documentation-only change,
these files should remain unchanged.

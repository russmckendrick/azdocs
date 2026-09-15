# Product

What azdocs is for, and the constraints that shape it. Architecture lives in
[AGENTS.md](AGENTS.md) and [docs/](docs/README.md); this is the "why".

## The problem

Understanding an unfamiliar Azure estate often means moving between portal
blades, queries and service-specific tools. Engineers need a durable account of
what was visible, how resources were connected and which findings need review,
so they can compare collections and share the evidence offline.

## What azdocs does

One read-only collection through Azure Resource Graph becomes a durable local
SQLite snapshot. Exploring saved evidence, tracing relationships and exporting
diagrams and reports all run against that snapshot offline. Desktop collection
can also capture website images; collecting or refreshing that evidence needs
network access.

That single decision is the product. It makes the output reproducible, makes it
diffable against last month's collection, and means the tool needs nothing more
than Reader to do its job.

## Who it is for

Azure, cloud-platform and infrastructure engineers who need to understand an
estate they did not necessarily build: onboarding to an unfamiliar tenant,
preparing a review, chasing a misconfiguration, or producing documentation
somebody else will read.

They move between estate-wide orientation, search and filtering, finding
triage, relationship tracing, snapshot comparison, and formal export — and the
job is usually to get from a broad signal to the exact resource and the
evidence behind it.

## Two interfaces, one engine

| | |
|---|---|
| **CLI** (`azdocs`) | Collect, report, diagram, and a terminal browser. Scriptable, CI-friendly, one CLI executable per platform. |
| **Desktop** (Tauri) | Interactive exploration of the same snapshot: estate explorer, the estate map, findings, history, exports. |

Neither is a subset of the other, and both read the same SQLite database
through the same Rust library. A resource type is named the same way, a
location is spelled the same way, and a diagram is laid out the same way in
both, because there is one implementation of each.

## Constraints that are not negotiable

These are product constraints, not implementation details — breaking one
changes what azdocs *is*:

- **Collection is read-only.** A service principal with Reader, and no write
  path to Azure anywhere in the codebase.
- **Everything downstream of collection is offline.** Reports, diagrams, the
  TUI and desktop exploration read stored evidence from SQLite. Connection tests,
  new collection and explicit website capture/refresh are online actions. This is what makes output
  reproducible and golden-testable, and what lets the tool run somewhere the
  tenant is not reachable.
- **Output is deterministic.** Stable ordering and snapshot-derived dates make exports reproducible for a
  fixed application version, configuration, theme, labels and font files.
  Changing these inputs can change an export without a new Azure collection.
- **Nothing is silently dropped.** Where a view cannot draw everything — a
  crowded map, a capped diagram set — it states the arithmetic.
  `drawn + folded + aggregated == total`, and truncation is logged.
- **Stored credentials never reach the webview.** Settings supports write-only
  entry of a new client secret into Rust; the field is cleared after submission.
  Rust owns OS credential storage and environment resolution. Returned settings,
  diagnostics and logs exclude secrets and OAuth tokens. Offline work never
  unlocks the credential store.
- **One active tenant, shared storage.** Named profiles share defaults and one
  SQLite database. Tenant ID isolates history and comparisons. Selecting a
  profile for editing does not switch the active estate; removing it keeps
  snapshot history available offline.

## Extending it is usually data, not code

A new audit check is a TOML file in `queries/`. A new document theme is a TOML
file in `data/themes/`. A friendlier name for a resource type, region or kind
is a line in `data/`. Each has a user-override directory, so an estate with
local conventions does not need a fork. Adding Rust should be the exception.

## Design

The desktop combines a balanced Overview dashboard with focused evidence
workspaces. IBM Plex provides the type hierarchy; soft cool-grey-to-Azure
surfaces frame the content in light mode, with charcoal-to-deep-Azure surfaces
in dark mode. Saturated colour identifies data, signals and primary actions.
This atmosphere is desktop-only; document exports keep their selected report
theme. Dashboard details lead to the exact stored results behind each measure.
See [DESIGN.md](DESIGN.md).

The product voice matches it: technical, calm, direct, evidence-led. Say what
was collected and when; do not imply live state; do not decorate.

## Accessibility

The design targets complete keyboard operation, visible focus, WCAG 2.2 AA
contrast, honoured
reduced-motion preferences, semantic controls, and layouts that survive text
scaling and narrow windows. Colour never carries meaning alone — severity is
always accompanied by its word. Automated checks do not establish complete
accessibility conformance; keyboard, text scaling and assistive-technology
acceptance need review on the actual desktop platforms.

## What azdocs does not claim

Resource Graph cannot see everything, and the product should not pretend
otherwise: no resolved effective-access graph, no data-plane contents, no complete
activity log and no measured utilisation. A snapshot includes resource configuration,
derived relationships and the accessible policy, RBAC, security and operational
evidence exposed by ARG. Retention windows and missing records limit that evidence. There are no customer claims, benchmarks
or pricing here, and none should be invented.

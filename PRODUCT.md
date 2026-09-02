# Product

What azdocs is for, and the constraints that shape it. Architecture lives in
[AGENTS.md](AGENTS.md) and [docs/](docs/README.md); this is the "why".

## The problem

An Azure estate is only legible through the portal, one blade at a time, and
only while you are connected to it. Answering "what is in this subscription,
what talks to what, and what is misconfigured" means clicking through it — and
the answer cannot be filed, diffed, or handed to someone else.

## What azdocs does

One read-only collection through Azure Resource Graph becomes a durable local
SQLite snapshot. Everything after that — exploring, auditing, tracing
relationships, diagrams, reports — runs against the stored snapshot, offline.

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
| **CLI** (`azdocs`) | Collect, report, diagram, and a terminal browser. Scriptable, CI-friendly, static binaries. |
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
  TUI and the desktop read only from SQLite. This is what makes output
  reproducible and golden-testable, and what lets the tool run somewhere the
  tenant is not reachable.
- **Output is deterministic.** The same snapshot renders byte-identically. A
  diff between two reports is a diff between two estates, never noise.
- **Nothing is silently dropped.** Where a view cannot draw everything — a
  crowded map, a capped diagram set — it states the arithmetic.
  `drawn + folded + aggregated == total`, and truncation is logged.
- **Credentials never reach the webview.** The desktop's frontend has no
  Azure access, no filesystem access and no database handle; it renders DTOs
  the Rust side produced.

## Extending it is usually data, not code

A new audit check is a TOML file in `queries/`. A new document theme is a TOML
file in `data/themes/`. A friendlier name for a resource type, region or kind
is a line in `data/`. Each has a user-override directory, so an estate with
local conventions does not need a fork. Adding Rust should be the exception.

## Design

The desktop wears the **Field Report** language — the printed report made
interactive, IBM Plex throughout, colour reserved for data and signals. See
[DESIGN.md](DESIGN.md).

The product voice matches it: technical, calm, direct, evidence-led. Say what
was collected and when; do not imply live state; do not decorate.

## Accessibility

Complete keyboard operation, visible focus, WCAG 2.2 AA contrast, honoured
reduced-motion preferences, semantic controls, and layouts that survive text
scaling and narrow windows. Colour never carries meaning alone — severity is
always accompanied by its word.

## What azdocs does not claim

Resource Graph cannot see everything, and the product should not pretend
otherwise: no RBAC assignments, no data-plane contents, no activity logs, no
metrics. A snapshot is a point-in-time inventory of resource configuration and
the relationships derivable from it. There are no customer claims, benchmarks
or pricing here, and none should be invented.

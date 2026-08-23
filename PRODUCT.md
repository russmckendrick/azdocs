# Product

<!-- impeccable:product-schema 1 -->

> The desktop-specific product decisions below are inferred from the initial
> request and the repository's existing behavior, pending user confirmation.

## Platform

web

## Stack

Inferred: a Tauri v2 desktop shell with a React and TypeScript web interface,
backed directly by the existing Rust library and SQLite store. The existing
Rust CLI remains supported as a peer interface rather than being replaced.

## Users

The primary user is an Azure, cloud-platform, or infrastructure engineer who
needs to understand an estate from periodic read-only snapshots. They may be
investigating one resource, reviewing audit findings, tracing relationships,
or comparing the present estate with an earlier collection.

## Product Purpose

azdocs collects an Azure estate through Azure Resource Graph, stores a durable
local snapshot, and makes that snapshot useful for exploration, auditing,
diagrams, and reports. Success means a user can move from an estate-wide signal
to the exact resource and evidence behind it without returning to the network.

## Positioning

One read-only collection becomes a deterministic offline source for interactive
exploration, relationship analysis, compliance findings, diagrams, and document
exports. The desktop app explores the same stored evidence as every other
azdocs output rather than maintaining a separate cloud-side model.

## Operating Context

Users work with Azure subscriptions, resource groups, ARM resource types,
regions, tags, findings, resource properties, and derived relationships. They
typically move between broad estate review, search and filtering, finding
triage, topology tracing, snapshot history, and formal report export.

## Capabilities and Constraints

- Azure collection uses the existing hand-rolled client-credentials provider
  and a read-only service principal.
- Reports, diagrams, the TUI, and the desktop explorer read only from SQLite;
  they never query Azure directly.
- Lowercase ARM IDs remain the join key; original casing is display-only.
- Edges and configuration-driven audits remain Rust post-passes over stored
  data, with no additional Resource Graph queries.
- Output and UI ordering must be deterministic.
- Existing configuration and database locations remain compatible with the
  CLI, with an explicit database picker available in the desktop app.
- The desktop interface must remain usable with large estates, keyboard input,
  reduced motion, and high-contrast operating-system preferences.

## Brand Commitments

The product name is `azdocs`. Azure resource iconography already vendored in
`data/icons/` is the factual visual asset for resource types. The product voice
is technical, calm, direct, and evidence-led. The desktop visual world is an
Azure observatory: resource icons appear in their native artwork without white
button backplates, relationship space is deep and luminous, and interaction
state is expressed with line, light, and motion instead of filled UI chrome.

The relationship explorer uses Cytoscape.js for a deterministic resource-group
overview, group-to-resource drill-down, and one-hop resource neighbourhoods,
with node dragging, pan, zoom, selection, boundary-anchored orthogonal edges,
and directional flow dashes. Nonessential motion is user-controllable, stops
while the window is hidden, and is removed when the operating system requests
reduced motion.

## Evidence on Hand

- The canonical two-subscription fixture in `tests/common/mod.rs` provides
  representative resources, peerings, findings, and private connectivity.
- The embedded Azure icon pack and `data/icon_mapping.toml` provide real
  resource imagery.
- Existing report themes provide palette evidence but are not a desktop UI
  design system.
- No customer claims, usage benchmarks, pricing, or testimonials are present
  and none should be fabricated.

## Product Principles

- Start broad, then preserve context while drilling into exact evidence.
- Keep offline snapshot truth visibly distinct from live Azure state.
- Make relationships and findings navigable, not merely reportable.
- Preserve one engine and one data model across every interface.
- Favor dense clarity and fast keyboard workflows over decorative analytics.

## Accessibility & Inclusion

The desktop app targets WCAG 2.2 AA contrast and interaction behavior, complete
keyboard operation, visible focus, reduced-motion support, semantic controls,
and layouts that tolerate text scaling and narrower windows.

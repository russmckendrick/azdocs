import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { buildFallbackTopology, edgeKindClass } from "./topology-fallback";
import { mockEstate } from "../mock-data";
import type { EdgeKind, KindClass } from "../types";

/**
 * The browser-preview fallback mirrors `kind_class` in
 * desktop/src-tauri/src/topology.rs by hand. The Rust side is an exhaustive
 * `match`, so adding a variant there is a compile error there — but was
 * silently absorbed here until `monitors` was found classified as "structure"
 * instead of "monitoring". These tests read the Rust source so the mirror
 * cannot drift again without a red test.
 */
const rustSource = readFileSync(
  fileURLToPath(new URL("../../src-tauri/src/topology.rs", import.meta.url)),
  "utf8",
);
const modelSource = readFileSync(
  fileURLToPath(new URL("../../../src/model/mod.rs", import.meta.url)),
  "utf8",
);

/**
 * Every `Self::Variant => "snake_name"` arm of `EdgeKind::as_str`.
 *
 * Anchored to `impl EdgeKind` on purpose: mod.rs holds several `as_str`
 * implementations and an unanchored match picks up `SnapshotStatus` instead.
 */
function rustEdgeKinds(): string[] {
  const impl = modelSource.match(/impl EdgeKind \{[\s\S]*?\n\}/);
  if (!impl) throw new Error("could not locate `impl EdgeKind` in src/model/mod.rs");
  const block = impl[0].match(/pub fn as_str\(self\) -> &'static str \{[\s\S]*?\n {8}\}/);
  if (!block) throw new Error("could not locate EdgeKind::as_str in src/model/mod.rs");
  return [...block[0].matchAll(/Self::\w+ => "(\w+)"/g)].map((match) => match[1]).sort();
}

/** The `kind_class` match arms, as edge-kind name -> family. */
function rustKindClasses(): Map<string, string> {
  const block = rustSource.match(/pub fn kind_class\(kind: EdgeKind\) -> &'static str \{[\s\S]*?\n {4}\}/);
  if (!block) throw new Error("could not locate kind_class in topology.rs");
  const pascalToSnake = (name: string) =>
    name.replace(/([a-z0-9])([A-Z])/g, "$1_$2").toLowerCase();
  const mapping = new Map<string, string>();
  for (const arm of block[0].matchAll(/((?:\s*(?:\|\s*)?EdgeKind::\w+)+)\s*=>\s*"(\w+)"/g)) {
    const family = arm[2];
    for (const variant of arm[1].matchAll(/EdgeKind::(\w+)/g)) {
      mapping.set(pascalToSnake(variant[1]), family);
    }
  }
  return mapping;
}

describe("edgeKindClass", () => {
  it("unit_covers_every_rust_edge_kind_when_mirroring_the_enum", () => {
    const rustKinds = rustEdgeKinds();
    const mapped = [...rustKindClasses().keys()].sort();
    // Guard the guard: if the regexes stop matching, fail loudly rather than
    // passing on two empty lists.
    expect(rustKinds.length).toBeGreaterThan(0);
    expect(mapped).toEqual(rustKinds);
  });

  it("unit_returns_the_same_family_as_rust_when_given_each_edge_kind", () => {
    for (const [kind, family] of rustKindClasses()) {
      expect(edgeKindClass(kind as EdgeKind)).toBe(family as KindClass);
    }
  });

  it("unit_classifies_monitors_as_monitoring_when_called", () => {
    // The specific regression: a switch `default` made this "structure".
    expect(edgeKindClass("monitors")).toBe("monitoring");
  });
});

describe("buildFallbackTopology estate", () => {
  const everySubscription = mockEstate.subscriptions.map((subscription) => subscription.id);
  const request = (showUnconnected: boolean, expandedSubscriptions = everySubscription) => ({
    snapshotId: mockEstate.id,
    mode: { kind: "estate" as const, expandedSubscriptions },
    scope: { subscriptions: [], azureTypes: [], showUnconnected },
  });

  it("unit_opens_with_every_subscription_collapsed_when_nothing_is_expanded", () => {
    const graph = buildFallbackTopology(mockEstate, request(true, []));
    expect(graph.lanes.every((lane) => !lane.expanded)).toBe(true);
    expect(graph.nodes.every((node) => node.kind === "subscription")).toBe(true);
    expect(graph.counts.drawn).toBe(0);
    expect(graph.counts.aggregated).toBe(graph.counts.total);
  });

  it("unit_draws_connected_groups_and_tiles_the_rest_per_subscription_when_mirroring_rust", () => {
    const graph = buildFallbackTopology(mockEstate, request(true));
    const cards = graph.nodes.filter((node) => node.kind === "resource-group");
    const tiles = graph.nodes.filter((node) => node.kind === "aggregate");
    expect(graph.counts.drawn).toBe(cards.length);
    expect(graph.counts.drawn + graph.counts.folded + graph.counts.aggregated).toBe(graph.counts.total);
    expect(tiles.length).toBeGreaterThan(0);
    for (const tile of tiles) {
      expect(tile.id).toBe(`aggregate:unconnected:${tile.lane}`);
      expect(tile.groupIds.length).toBe(tile.count);
      expect(tile.zone).toBe("unconnected");
    }
    // No card is unconnected and no connector reaches a tile.
    const tileIds = new Set(tiles.map((tile) => tile.id));
    expect(graph.links.some((link) => tileIds.has(link.sourceId) || tileIds.has(link.targetId))).toBe(false);
  });

  it("unit_counts_hidden_groups_when_the_unconnected_toggle_is_off", () => {
    const shown = buildFallbackTopology(mockEstate, request(true));
    const hidden = buildFallbackTopology(mockEstate, request(false));
    const tiled = shown.nodes.filter((node) => node.kind === "aggregate").reduce((total, node) => total + node.count, 0);
    expect(hidden.nodes.some((node) => node.kind === "aggregate")).toBe(false);
    expect(hidden.counts.hiddenByFilter).toBe(tiled);
    expect(hidden.counts.drawn).toBe(shown.counts.drawn);
  });
});

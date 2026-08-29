import { describe, expect, it } from "vitest";
import type { TopologyGraph, TopologyNode } from "../types";
import {
  GRAPH_SIZE,
  cameraEntryZoom,
  cameraProfile,
  cameraTargetIds,
  layoutTopology,
  nearestNodeInDirection,
} from "./topology-layout";

function node(id: string, kind: TopologyNode["kind"], overrides: Partial<TopologyNode> = {}): TopologyNode {
  return {
    id,
    kind,
    name: id,
    subtitle: "",
    memberIds: [],
    count: 1,
    findingCount: 0,
    ...overrides,
  };
}

function graph(level: TopologyGraph["level"], nodes: TopologyNode[], overrides: Partial<TopologyGraph> = {}): TopologyGraph {
  return {
    level,
    nodes,
    lanes: [],
    links: [],
    kindClasses: [],
    counts: {
      total: nodes.length,
      drawn: nodes.length,
      folded: 0,
      aggregated: 0,
      external: 0,
      hiddenByFilter: 0,
      totalLinks: 0,
      drawnLinks: 0,
    },
    ...overrides,
  };
}

function entries(plan: ReturnType<typeof layoutTopology>) {
  return [...plan.positions.entries()];
}

describe.each([1440, 1060, 800])("topology layout at %ipx", (width) => {
  it("lays out expanded estate cards without collisions and keeps collapsed lanes secondary", () => {
    const estate = graph("estate", [
      node("g-a", "resource-group", { lane: "sub-a" }),
      node("g-b", "resource-group", { lane: "sub-a" }),
      node("g-c", "resource-group", { lane: "sub-a" }),
      node("g-d", "resource-group", { lane: "sub-a" }),
      node("sub-b", "subscription", { lane: "sub-b" }),
    ], {
      lanes: [
        { subscriptionId: "sub-a", name: "A", expanded: true, groupCount: 4, resourceCount: 40, findingCount: 0 },
        { subscriptionId: "sub-b", name: "B", expanded: false, groupCount: 3, resourceCount: 30, findingCount: 0 },
      ],
    });
    const first = layoutTopology(estate, { width, height: 760 });
    const second = layoutTopology(estate, { width, height: 760 });

    expect(entries(first)).toEqual(entries(second));
    expect(first.coreNodeIds).toContain("lane:sub-a");
    expect(first.secondaryNodeIds).toEqual(["sub-b"]);
    const cards = ["g-a", "g-b", "g-c", "g-d"].map((id) => first.positions.get(id));
    for (let left = 0; left < cards.length; left += 1) {
      for (let right = left + 1; right < cards.length; right += 1) {
        const a = cards[left];
        const b = cards[right];
        expect(a && b && (
          Math.abs(a.x - b.x) >= GRAPH_SIZE.groupWidth
          || Math.abs(a.y - b.y) >= GRAPH_SIZE.groupHeight
        )).toBe(true);
      }
    }
    const collapsed = first.positions.get("sub-b");
    const cardBottom = Math.max(...cards.map((position) => (position?.y ?? 0) + GRAPH_SIZE.groupHeight));
    if (width < 1040) expect(collapsed?.y).toBeGreaterThan(cardBottom);
    else expect(collapsed?.x).toBeGreaterThan(Math.max(...cards.map((position) => position?.x ?? 0)));
  });

  it("places group neighbours on a boundary rail and isolated aggregates below the core", () => {
    const groupView = graph("group", [
      node("core-a", "resource", { zone: "core" }),
      node("core-b", "resource", { zone: "core" }),
      node("external", "external", { zone: "external" }),
      node("isolated", "aggregate", { zone: "unconnected", count: 8 }),
    ], {
      links: [{ sourceId: "external", targetId: "core-a", label: "peered", kindClass: "network", count: 1 }],
    });
    const plan = layoutTopology(groupView, { width, height: 760 });
    const core = plan.positions.get("core-a");
    const external = plan.positions.get("external");
    const isolated = plan.positions.get("isolated");

    expect(external?.x).toBeLessThan(core?.x ?? 0);
    expect(external?.y).toBe(core?.y);
    expect(isolated?.y).toBeGreaterThan(
      Math.max(...["core-a", "core-b", "external"].map((id) => plan.positions.get(id)?.y ?? 0))
        + GRAPH_SIZE.resourceHeight,
    );
    expect(plan.coreNodeIds).toEqual(expect.arrayContaining(["core-a", "core-b", "external"]));
    expect(plan.secondaryNodeIds).toEqual(["isolated"]);
  });

  it("centres a neighbourhood subject with inbound left, outbound right, and second hops outside", () => {
    const neighbourhood = graph("neighbourhood", [
      node("subject", "resource", { hop: 0 }),
      node("inbound", "resource", { hop: 1 }),
      node("outbound", "resource", { hop: 1 }),
      node("second", "resource", { hop: 2 }),
    ], {
      links: [
        { sourceId: "inbound", targetId: "subject", label: "feeds", kindClass: "data", count: 1 },
        { sourceId: "subject", targetId: "outbound", label: "uses", kindClass: "identity", count: 1 },
        { sourceId: "outbound", targetId: "second", label: "alerts", kindClass: "monitoring", count: 1 },
      ],
    });
    const plan = layoutTopology(neighbourhood, { width, height: 760 });

    expect(plan.positions.get("subject")).toEqual({ x: 0, y: 0 });
    expect(plan.positions.get("inbound")?.x).toBeLessThan(0);
    expect(plan.positions.get("outbound")?.x).toBeGreaterThan(0);
    expect(plan.positions.get("second")?.x).toBeGreaterThan(plan.positions.get("outbound")?.x ?? 0);
    expect(plan.coreNodeIds).toEqual(expect.arrayContaining(["subject", "inbound", "outbound"]));
    expect(plan.secondaryNodeIds).toEqual(["second"]);
  });
});

describe("spatial keyboard navigation", () => {
  const points = new Map([
    ["centre", { x: 100, y: 100 }],
    ["left", { x: 20, y: 100 }],
    ["right", { x: 180, y: 100 }],
    ["up", { x: 100, y: 20 }],
    ["down", { x: 100, y: 180 }],
    ["far-right", { x: 260, y: 100 }],
  ]);

  it.each([
    ["ArrowLeft", "left"],
    ["ArrowRight", "right"],
    ["ArrowUp", "up"],
    ["ArrowDown", "down"],
  ] as const)("moves %s to the nearest item in that direction", (direction, expected) => {
    expect(nearestNodeInDirection("centre", direction, points)).toBe(expected);
  });
});

describe("camera targets", () => {
  const plan = {
    positions: new Map(),
    coreNodeIds: ["core-a", "core-b"],
    secondaryNodeIds: ["shelf"],
  };

  it("distinguishes readable core, explicit selection, and Fit all", () => {
    const all = ["core-a", "core-b", "shelf"];
    expect(cameraTargetIds(plan, all, "core")).toEqual(["core-a", "core-b"]);
    expect(cameraTargetIds(plan, all, "selection", "shelf")).toEqual(["shelf"]);
    expect(cameraTargetIds(plan, all, "all", "shelf")).toEqual(all);
  });

  it("falls back to the readable core when selection is outside the current graph", () => {
    expect(cameraTargetIds(plan, ["core-a", "core-b", "shelf"], "selection", "missing")).toEqual([
      "core-a",
      "core-b",
    ]);
  });
});

describe.each([1440, 1060, 800])("camera motion at %ipx", (width) => {
  const viewport = { width, height: 900 };

  it("lets a sparse group occupy more of the canvas than the estate overview", () => {
    const estate = cameraProfile("estate", "core", 3, viewport);
    const group = cameraProfile("group", "core", 6, viewport);

    expect(group.maxZoom).toBeGreaterThan(1);
    expect(group.maxZoom).toBeGreaterThanOrEqual(estate.maxZoom);
    expect(group.minZoom).toBeLessThanOrEqual(group.maxZoom);
  });

  it("keeps Fit all an overview and uses opposite entry motion for parent and child scopes", () => {
    const fitAll = cameraProfile("group", "all", 14, viewport);
    expect(fitAll.maxZoom).toBe(1);
    expect(cameraEntryZoom("estate", 1)).toBeGreaterThan(1);
    expect(cameraEntryZoom("group", 1)).toBeLessThan(1);
    expect(cameraEntryZoom("neighbourhood", 1)).toBeLessThan(1);
  });
});

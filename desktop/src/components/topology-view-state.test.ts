import { describe, expect, it } from "vitest";
import type { TopologyGraph } from "../types";
import {
  describeCounts,
  expandSubscriptionLane,
  refreshFailed,
  refreshStarted,
  refreshSucceeded,
  resolveAggregateActivation,
  toggleSubscriptionLane,
} from "./topology-view-state";

describe("retained topology refresh state", () => {
  it("keeps an existing graph stale while refreshing and after a failure", () => {
    expect(refreshStarted(true)).toEqual({ stale: true });
    expect(refreshFailed(true, "offline")).toEqual({
      stale: true,
      error: { message: "offline", hasPrevious: true },
    });
    expect(refreshSucceeded()).toEqual({ stale: false });
  });

  it("reports an unrecoverable first request without calling it stale", () => {
    expect(refreshFailed(false, "missing snapshot")).toEqual({
      stale: false,
      error: { message: "missing snapshot", hasPrevious: false },
    });
  });
});

describe("relationship view controls", () => {
  it("toggles an individual lane from the drawn state and preserves deterministic order", () => {
    expect(toggleSubscriptionLane(undefined, ["b"], "a")).toEqual(["a", "b"]);
    expect(toggleSubscriptionLane(["a", "b"], ["b"], "a")).toEqual(["b"]);
  });

  it("opens a single aggregate member and toggles a multi-member list", () => {
    expect(resolveAggregateActivation(undefined, "aggregate-a", ["resource-a"])).toEqual({
      kind: "open-resource",
      resourceId: "resource-a",
    });
    expect(resolveAggregateActivation(undefined, "aggregate-a", ["resource-a", "resource-b"])).toEqual({
      kind: "toggle",
      expandedId: "aggregate-a",
    });
    expect(resolveAggregateActivation("aggregate-a", "aggregate-a", ["resource-a", "resource-b"])).toEqual({
      kind: "toggle",
      expandedId: undefined,
    });
  });

  it("unit_opens_the_group_map_when_an_unconnected_groups_tile_holds_one_group", () => {
    // The tile's memberIds are that group's resources; they must not be
    // mistaken for a single-resource aggregate.
    expect(resolveAggregateActivation(undefined, "aggregate:unconnected:sub-a", ["resource-a"], ["group-a"])).toEqual({
      kind: "open-group",
      groupId: "group-a",
    });
    expect(resolveAggregateActivation(undefined, "aggregate:unconnected:sub-a", ["resource-a", "resource-b"], ["group-a", "group-b"])).toEqual({
      kind: "toggle",
      expandedId: "aggregate:unconnected:sub-a",
    });
  });
});

describe("expandSubscriptionLane", () => {
  it("unit_adds_the_subscription_when_it_is_collapsed_and_keeps_it_when_expanded", () => {
    expect(expandSubscriptionLane(undefined, [], "sub-b")).toEqual(["sub-b"]);
    expect(expandSubscriptionLane(["sub-c", "sub-a"], [], "sub-b")).toEqual(["sub-a", "sub-b", "sub-c"]);
    expect(expandSubscriptionLane(["sub-a"], [], "sub-a")).toEqual(["sub-a"]);
    expect(expandSubscriptionLane(undefined, ["sub-a"], "sub-b")).toEqual(["sub-a", "sub-b"]);
  });
});

describe("describeCounts", () => {
  function estateGraph(): TopologyGraph {
    const base = { subtitle: "", memberIds: [], findingCount: 0, groupIds: [] as string[] };
    return {
      level: "estate",
      lanes: [],
      links: [],
      kindClasses: [],
      nodes: [
        { ...base, id: "g-a", kind: "resource-group", name: "a", lane: "sub-a", count: 5 },
        { ...base, id: "g-b", kind: "resource-group", name: "b", lane: "sub-a", count: 2 },
        { ...base, id: "aggregate:unconnected:sub-a", kind: "aggregate", name: "Unconnected groups", lane: "sub-a", zone: "unconnected", count: 9, groupIds: ["x", "y"] },
        { ...base, id: "subscription:sub-b", kind: "subscription", name: "B", lane: "sub-b", count: 17 },
        { ...base, id: "subscription:sub-c", kind: "subscription", name: "C", lane: "sub-c", count: 11 },
      ],
      counts: { total: 39, drawn: 2, folded: 0, aggregated: 37, external: 0, hiddenByFilter: 0, totalLinks: 14, drawnLinks: 7 },
    };
  }

  it("unit_names_collapsed_and_unconnected_groups_when_they_cover_the_aggregated_count", () => {
    expect(describeCounts(estateGraph()).extras).toEqual([
      { count: 28, kind: "collapsed" },
      { count: 9, kind: "unconnected" },
    ]);
  });

  it("unit_keeps_the_arithmetic_honest_when_an_aggregate_is_unnamed", () => {
    const graph = estateGraph();
    graph.counts.aggregated = 40;
    graph.counts.hiddenByFilter = 3;
    expect(describeCounts(graph).extras).toEqual([
      { count: 28, kind: "collapsed" },
      { count: 9, kind: "unconnected" },
      { count: 3, kind: "tiles" },
      { count: 3, kind: "hidden", emphasis: true },
    ]);
  });
});

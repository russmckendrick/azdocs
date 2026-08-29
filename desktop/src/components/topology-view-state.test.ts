import { describe, expect, it } from "vitest";
import {
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
});

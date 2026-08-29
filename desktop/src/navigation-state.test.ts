import { describe, expect, it } from "vitest";
import {
  initialNavigationState,
  navigationReducer,
  type NavigationState,
} from "./navigation-state";

function inGroup(groupId = "group-a"): NavigationState {
  return navigationReducer(initialNavigationState(), {
    type: "update-relationships",
    workspace: {
      location: { kind: "group", groupId },
      depth: 2,
      excludedClasses: ["data"],
      expandedSubscriptions: ["subscription-a"],
      showUnconnected: false,
    },
  });
}

describe("workspace navigation", () => {
  it("returns through resource and relationship surfaces without losing the group", () => {
    let state = navigationReducer(inGroup(), { type: "open-section", section: "topology" });
    state = navigationReducer(state, { type: "open-resource", resourceId: "resource-a" });
    state = navigationReducer(state, { type: "open-relationships", resourceId: "resource-a" });

    expect(state.section).toBe("topology");
    expect(state.relationships.location).toEqual({ kind: "neighbourhood", resourceId: "resource-a" });

    state = navigationReducer(state, { type: "back" });
    expect(state.surface).toEqual({ kind: "resource", resourceId: "resource-a" });
    expect(state.relationships.location).toEqual({ kind: "group", groupId: "group-a" });

    state = navigationReducer(state, { type: "back" });
    expect(state.surface).toEqual({ kind: "section" });
    expect(state.relationships).toMatchObject({
      location: { kind: "group", groupId: "group-a" },
      depth: 2,
      excludedClasses: ["data"],
      expandedSubscriptions: ["subscription-a"],
      showUnconnected: false,
    });
  });

  it("treats related resource records as history entries", () => {
    let state = navigationReducer(initialNavigationState(), { type: "open-resource", resourceId: "resource-a" });
    state = navigationReducer(state, { type: "open-resource", resourceId: "resource-b" });
    state = navigationReducer(state, { type: "back" });

    expect(state.surface).toEqual({ kind: "resource", resourceId: "resource-a" });
  });

  it("resumes the relationship location from primary navigation and clears temporal history", () => {
    let state = navigationReducer(inGroup(), { type: "open-resource", resourceId: "resource-a" });
    state = navigationReducer(state, { type: "open-section", section: "topology" });

    expect(state.surface).toEqual({ kind: "section" });
    expect(state.relationships.location).toEqual({ kind: "group", groupId: "group-a" });
    expect(state.history).toEqual([]);
  });

  it("adds estate, group, and neighbourhood drill-downs to Back history", () => {
    let state = navigationReducer(initialNavigationState(), { type: "open-section", section: "topology" });
    state = navigationReducer(state, {
      type: "navigate-relationships",
      workspace: { ...state.relationships, location: { kind: "group", groupId: "group-a" } },
    });
    state = navigationReducer(state, {
      type: "navigate-relationships",
      workspace: { ...state.relationships, location: { kind: "neighbourhood", resourceId: "resource-a" } },
    });

    state = navigationReducer(state, { type: "back" });
    expect(state.relationships.location).toEqual({ kind: "group", groupId: "group-a" });
    state = navigationReducer(state, { type: "back" });
    expect(state.relationships.location).toEqual({ kind: "estate" });
  });

  it("resets relationship context when the snapshot changes", () => {
    const state = navigationReducer(inGroup(), { type: "reset-snapshot" });

    expect(state.relationships.location).toEqual({ kind: "estate" });
    expect(state.history).toEqual([]);
  });
});

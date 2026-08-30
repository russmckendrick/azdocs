import { describe, expect, it } from "vitest";
import type { TopologyGraph } from "../types";
import {
  activeRelationshipLabels,
  buildLinkPresentations,
  connectorGeometries,
  connectorPortAssignments,
  connectorPortPosition,
  placeRelationshipLabel,
  relationshipLabelSide,
  resolveTraceNode,
} from "./topology-presentation";

function graph(links: TopologyGraph["links"]): TopologyGraph {
  return {
    level: "neighbourhood",
    nodes: [],
    lanes: [],
    links,
    kindClasses: [],
    counts: {
      total: 0,
      drawn: 0,
      folded: 0,
      aggregated: 0,
      external: 0,
      hiddenByFilter: 0,
      totalLinks: links.length,
      drawnLinks: links.length,
    },
  };
}

describe("relationship presentation", () => {
  const sharedTarget = graph([
    { sourceId: "a", targetId: "subject", label: "attached to", kindClass: "structure", count: 1 },
    { sourceId: "b", targetId: "subject", label: "attached to", kindClass: "structure", count: 1 },
    { sourceId: "c", targetId: "subject", label: "depends on", kindClass: "data", count: 1 },
  ]);

  it("fans shared endpoint routes into deterministic taxi channels", () => {
    const first = buildLinkPresentations(sharedTarget);
    const second = buildLinkPresentations(sharedTarget);

    expect(first).toEqual(second);
    expect(new Set(first.map((link) => link.taxiTurn)).size).toBe(3);
  });

  it("keeps shared channels deterministic and follows the dominant route axis", () => {
    const presentations = buildLinkPresentations(sharedTarget);
    const positions = new Map([
      ["a", { x: -300, y: -90 }],
      ["b", { x: -300, y: 0 }],
      ["c", { x: 0, y: 300 }],
      ["subject", { x: 0, y: 0 }],
    ]);
    const geometry = connectorGeometries(presentations, positions);

    expect(geometry.map((edge) => edge.taxiDirection)).toEqual(["horizontal", "horizontal", "vertical"]);
    expect(new Set(geometry.map((edge) => edge.taxiTurn)).size).toBe(3);
    expect(connectorGeometries(presentations, positions)).toEqual(geometry);
  });

  it("orders shared boundary ports by the opposite endpoint position", () => {
    const presentations = buildLinkPresentations(sharedTarget);
    const positions = new Map([
      ["a", { x: -300, y: 120 }],
      ["b", { x: -300, y: -120 }],
      ["c", { x: -300, y: 0 }],
      ["subject", { x: 0, y: 0 }],
    ]);
    const assignments = connectorPortAssignments(presentations, positions);
    const targetOffsets = new Map(assignments.map((assignment) => [
      assignment.sourceId,
      assignment.targetPort.offset,
    ]));

    expect(targetOffsets.get("b") ?? 0).toBeLessThan(targetOffsets.get("c") ?? 0);
    expect(targetOffsets.get("c") ?? 0).toBeLessThan(targetOffsets.get("a") ?? 0);
    expect(new Set(assignments.map((assignment) => assignment.targetPort.offset)).size).toBe(3);
    expect(connectorPortAssignments(presentations, positions)).toEqual(assignments);
  });

  it("projects compound-frame ports towards their peers instead of spanning the whole frame", () => {
    const presentations = buildLinkPresentations(sharedTarget);
    const positions = new Map([
      ["a", { x: -300, y: -120 }],
      ["b", { x: -300, y: 0 }],
      ["c", { x: -300, y: 120 }],
      ["subject", { x: 0, y: 0 }],
    ]);
    const assignments = connectorPortAssignments(presentations, positions, new Map([
      ["subject", { x1: -100, y1: -300, x2: 100, y2: 300 }],
    ]));

    expect(assignments.map((assignment) => assignment.targetPort.offset)).toEqual([0.3, 0.5, 0.7]);
  });

  it("keeps reciprocal routes paired around their projected frame approach", () => {
    const reciprocal = graph([
      { sourceId: "peer", targetId: "frame", label: "peered with", kindClass: "network", count: 1 },
      { sourceId: "frame", targetId: "peer", label: "peered with", kindClass: "network", count: 1 },
    ]);
    const assignments = connectorPortAssignments(
      buildLinkPresentations(reciprocal),
      new Map([
        ["peer", { x: -300, y: 100 }],
        ["frame", { x: 0, y: 0 }],
      ]),
      new Map([
        ["frame", { x1: -100, y1: -300, x2: 100, y2: 300 }],
      ]),
    );
    const frameOffsets = assignments.map((assignment) => (
      assignment.sourceId === "frame" ? assignment.sourcePort.offset : assignment.targetPort.offset
    ));

    expect((frameOffsets[0] + frameOffsets[1]) / 2).toBeCloseTo(2 / 3);
    expect(Math.abs(frameOffsets[0] - frameOffsets[1])).toBeGreaterThan(0.02);
  });

  it("places invisible ports directly on the selected node boundary", () => {
    const bounds = { x1: 100, y1: 200, x2: 300, y2: 400 };

    expect(connectorPortPosition(bounds, { side: "left", offset: 0.25 })).toEqual({ x: 100, y: 250 });
    expect(connectorPortPosition(bounds, { side: "right", offset: 0.75 })).toEqual({ x: 300, y: 350 });
    expect(connectorPortPosition(bounds, { side: "top", offset: 0.25 })).toEqual({ x: 150, y: 200 });
    expect(connectorPortPosition(bounds, { side: "bottom", offset: 0.75 })).toEqual({ x: 250, y: 400 });
  });

  it("moves a connector to the opposite boundary after a node crosses its peer", () => {
    const presentations = buildLinkPresentations(graph([
      { sourceId: "peer", targetId: "subject", label: "attached to", kindClass: "structure", count: 1 },
    ]));
    const before = connectorPortAssignments(presentations, new Map([
      ["peer", { x: -200, y: 0 }],
      ["subject", { x: 0, y: 0 }],
    ]));
    const after = connectorPortAssignments(presentations, new Map([
      ["peer", { x: 200, y: 0 }],
      ["subject", { x: 0, y: 0 }],
    ]));

    expect(before[0].targetPort.side).toBe("left");
    expect(after[0].targetPort.side).toBe("right");
  });

  it("anchors labels at the endpoint opposite the traced node", () => {
    const presentations = buildLinkPresentations(sharedTarget);
    const labels = activeRelationshipLabels(presentations, "subject");

    expect(labels.map((label) => label.endpointId)).toEqual(["a", "b", "c"]);
    expect(labels.map((label) => label.label)).toEqual(["Attached to", "Attached to", "Depends on"]);
  });

  it("gives parallel labels at one endpoint unique stable slots", () => {
    const parallel = graph([
      { sourceId: "subject", targetId: "peer", label: "attached to", kindClass: "structure", count: 1 },
      { sourceId: "subject", targetId: "peer", label: "monitors", kindClass: "monitoring", count: 1 },
    ]);
    const labels = activeRelationshipLabels(buildLinkPresentations(parallel), "subject");

    expect(new Set(labels.map((label) => label.endpointSlot)).size).toBe(2);
  });

  it("uses focus, pointer and selection as an explicit trace priority", () => {
    expect(resolveTraceNode("focus", "pointer", "selected")).toBe("focus");
    expect(resolveTraceNode(undefined, "pointer", "selected")).toBe("pointer");
    expect(resolveTraceNode(undefined, undefined, "selected")).toBe("selected");
  });

  it("chooses the side of the remote endpoint away from the traced node", () => {
    expect(relationshipLabelSide({ x: 0, y: 0 }, { x: 100, y: 0 })).toBe("right");
    expect(relationshipLabelSide({ x: 100, y: 0 }, { x: 0, y: 0 })).toBe("left");
    expect(relationshipLabelSide({ x: 0, y: 0 }, { x: 0, y: 100 })).toBe("bottom");
    expect(relationshipLabelSide({ x: 0, y: 100 }, { x: 0, y: 0 })).toBe("top");
  });

  it("moves a relationship label away from an occupied preferred side", () => {
    const placement = placeRelationshipLabel(
      { x: 0, y: 100 },
      { x1: 100, y1: 70, x2: 180, y2: 130 },
      { width: 90, height: 24 },
      { x1: 0, y1: 0, x2: 360, y2: 240 },
      [{ x1: 190, y1: 60, x2: 300, y2: 140 }],
      0,
    );

    expect(placement.side).toBe("top");
    expect(placement.y).toBeLessThan(70);
  });
});

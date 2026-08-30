import type { TopologyGraph, TopologyNode } from "../types";

export interface GraphViewport {
  width: number;
  height: number;
}

export interface Placement {
  x: number;
  y: number;
}

export interface TopologyLayoutPlan {
  positions: Map<string, Placement>;
  coreNodeIds: string[];
  secondaryNodeIds: string[];
  entryNodeIds: string[];
}

export type TopologyCameraTarget = "core" | "selection" | "all";

export interface TopologyCameraProfile {
  minZoom: number;
  maxZoom: number;
  padding: number;
  duration: number;
}

export function cameraProfile(
  level: TopologyGraph["level"],
  mode: TopologyCameraTarget,
  targetCount: number,
  viewport: GraphViewport,
): TopologyCameraProfile {
  const shortEdge = Math.min(viewport.width, viewport.height);
  const narrow = viewport.width < 900;
  if (mode === "all") {
    return {
      minZoom: 0.1,
      maxZoom: 1,
      padding: clamp(shortEdge * 0.07, 48, 80),
      duration: 320,
    };
  }
  if (mode === "selection") {
    return {
      minZoom: narrow ? 0.9 : 1.05,
      maxZoom: narrow ? 1.25 : 1.55,
      padding: clamp(shortEdge * 0.16, 86, 150),
      duration: 260,
    };
  }

  const roomyMaximum = level === "estate"
    ? targetCount <= 4 ? 1.22 : targetCount <= 12 ? 1.08 : 0.96
    : level === "group"
      ? targetCount <= 3 ? 1.55 : targetCount <= 7 ? 1.38 : targetCount <= 12 ? 1.22 : 1.06
      : targetCount <= 3 ? 1.52 : targetCount <= 7 ? 1.32 : 1.14;
  const narrowMaximum = level === "estate" ? 1.05 : 1.18;
  return {
    minZoom: narrow ? 0.72 : 0.82,
    maxZoom: narrow ? Math.min(roomyMaximum, narrowMaximum) : roomyMaximum,
    padding: clamp(shortEdge * 0.085, 56, 92),
    duration: 340,
  };
}

export function cameraEntryZoom(level: TopologyGraph["level"], targetZoom: number) {
  const multiplier = level === "estate" ? 0.88 : 0.8;
  return clamp(targetZoom * multiplier, 0.1, 2.2);
}

export function cameraTargetIds(
  plan: TopologyLayoutPlan,
  allNodeIds: readonly string[],
  mode: TopologyCameraTarget,
  selectedNodeId?: string,
) {
  if (mode === "selection" && selectedNodeId && allNodeIds.includes(selectedNodeId)) return [selectedNodeId];
  if (mode === "all") return [...allNodeIds];
  const entry = plan.entryNodeIds.filter((id) => allNodeIds.includes(id));
  if (entry.length > 0) return entry;
  const core = plan.coreNodeIds.filter((id) => allNodeIds.includes(id));
  return core.length > 0 ? core : [...allNodeIds];
}

export const GRAPH_SIZE = {
  resourceWidth: 172,
  resourceHeight: 76,
  groupWidth: 244,
  groupHeight: 118,
  aggregateWidth: 188,
  aggregateHeight: 60,
  laneBarWidth: 320,
  laneBarHeight: 58,
} as const;

const clamp = (value: number, minimum: number, maximum: number) =>
  Math.min(maximum, Math.max(minimum, value));

function stableCompare(left: string, right: string) {
  return left < right ? -1 : left > right ? 1 : 0;
}

function gridPosition(
  count: number,
  columns: number,
  index: number,
  width: number,
  height: number,
  gapX: number,
  gapY: number,
) {
  const column = index % columns;
  const row = Math.floor(index / columns);
  return {
    x: column * (width + gapX),
    y: row * (height + gapY),
    rows: Math.ceil(count / columns),
  };
}

function columnCount(availableWidth: number, itemWidth: number, gap: number, maximum: number) {
  return clamp(Math.floor((availableWidth + gap) / (itemWidth + gap)), 1, maximum);
}

export function layoutTopology(graph: TopologyGraph, viewport: GraphViewport): TopologyLayoutPlan {
  if (graph.level === "estate") return layoutEstate(graph, viewport);
  if (graph.level === "group") return layoutGroup(graph, viewport);
  return layoutNeighbourhood(graph, viewport);
}

function layoutEstate(graph: TopologyGraph, viewport: GraphViewport): TopologyLayoutPlan {
  const positions = new Map<string, Placement>();
  const coreNodeIds: string[] = [];
  const secondaryNodeIds: string[] = [];
  const expandedLanes = graph.lanes.filter((lane) => lane.expanded);
  const hasCollapsedLanes = graph.lanes.some((lane) => !lane.expanded);
  const railAllowance = hasCollapsedLanes && viewport.width >= 1040 ? GRAPH_SIZE.laneBarWidth + 96 : 0;
  const availableWidth = Math.max(620, viewport.width - railAllowance - 120);
  const columns = columnCount(availableWidth, GRAPH_SIZE.groupWidth, 44, 6);
  const laneWidth = columns * GRAPH_SIZE.groupWidth + Math.max(0, columns - 1) * 44;
  let cursorY = 0;

  for (const lane of expandedLanes) {
    const cards = graph.nodes
      .filter((node) => node.kind === "resource-group" && node.lane === lane.subscriptionId)
      .sort((left, right) => stableCompare(left.name, right.name) || stableCompare(left.id, right.id));
    if (cards.length === 0) continue;
    let rows = 1;
    for (const [index, card] of cards.entries()) {
      const cell = gridPosition(cards.length, columns, index, GRAPH_SIZE.groupWidth, GRAPH_SIZE.groupHeight, 44, 42);
      rows = cell.rows;
      positions.set(card.id, { x: cell.x, y: cursorY + cell.y });
      coreNodeIds.push(card.id);
    }
    coreNodeIds.push(`lane:${lane.subscriptionId}`);
    cursorY += rows * (GRAPH_SIZE.groupHeight + 42) + 68;
  }

  const collapsed = graph.nodes
    .filter((node) => node.kind === "subscription")
    .sort((left, right) => stableCompare(left.name, right.name) || stableCompare(left.id, right.id));
  const railX = viewport.width >= 1040 && coreNodeIds.length > 0 ? laneWidth + 92 : 0;
  const railY = viewport.width >= 1040 && coreNodeIds.length > 0 ? 0 : cursorY;
  for (const [index, node] of collapsed.entries()) {
    positions.set(node.id, {
      x: railX,
      y: railY + index * (GRAPH_SIZE.laneBarHeight + 20),
    });
    secondaryNodeIds.push(node.id);
  }

  if (coreNodeIds.length === 0) coreNodeIds.push(...secondaryNodeIds);
  return {
    positions,
    coreNodeIds,
    secondaryNodeIds,
    entryNodeIds: [...coreNodeIds, ...secondaryNodeIds],
  };
}

function childMap(graph: TopologyGraph) {
  const children = new Map<string, TopologyNode[]>();
  for (const node of graph.nodes) {
    if (!node.parentId) continue;
    children.set(node.parentId, [...(children.get(node.parentId) ?? []), node]);
  }
  return children;
}

function positionExternalRail(
  graph: TopologyGraph,
  positions: Map<string, Placement>,
  externalNodes: TopologyNode[],
) {
  const anchors = withContainerAnchors(graph, positions);
  const desired = externalNodes.map((node) => {
    const neighbours = graph.links
      .filter((link) => link.sourceId === node.id || link.targetId === node.id)
      .map((link) => anchors.get(link.sourceId === node.id ? link.targetId : link.sourceId)?.y)
      .filter((value): value is number => value !== undefined);
    const y = neighbours.length > 0
      ? neighbours.reduce((total, value) => total + value, 0) / neighbours.length
      : 0;
    return { node, desiredY: y };
  }).sort((left, right) => left.desiredY - right.desiredY || stableCompare(left.node.id, right.node.id));

  let nextY = 0;
  for (const entry of desired) {
    const y = Math.max(entry.desiredY, nextY);
    positions.set(entry.node.id, { x: -(GRAPH_SIZE.resourceWidth + 144), y });
    nextY = y + GRAPH_SIZE.resourceHeight + 30;
  }
}

function withContainerAnchors(
  graph: TopologyGraph,
  positions: ReadonlyMap<string, Placement>,
) {
  const anchors = new Map(positions);
  for (const kind of ["subnet", "vnet"] as const) {
    for (const node of graph.nodes.filter((candidate) => candidate.kind === kind)) {
      if (anchors.has(node.id)) continue;
      const childPositions = graph.nodes
        .filter((candidate) => candidate.parentId === node.id)
        .map((candidate) => anchors.get(candidate.id))
        .filter((position): position is Placement => position !== undefined);
      if (childPositions.length === 0) continue;
      anchors.set(node.id, {
        x: childPositions.reduce((total, position) => total + position.x, 0) / childPositions.length,
        y: childPositions.reduce((total, position) => total + position.y, 0) / childPositions.length,
      });
    }
  }
  return anchors;
}

function neighbourAverageY(
  graph: TopologyGraph,
  positions: ReadonlyMap<string, Placement>,
  nodeId: string,
) {
  const neighbours = graph.links
    .filter((link) => link.sourceId === nodeId || link.targetId === nodeId)
    .map((link) => positions.get(link.sourceId === nodeId ? link.targetId : link.sourceId)?.y)
    .filter((value): value is number => value !== undefined);
  if (neighbours.length === 0) return Number.POSITIVE_INFINITY;
  return neighbours.reduce((total, value) => total + value, 0) / neighbours.length;
}

function layoutGroup(graph: TopologyGraph, viewport: GraphViewport): TopologyLayoutPlan {
  const positions = new Map<string, Placement>();
  const children = childMap(graph);
  const coreNodeIds: string[] = [];
  const secondaryNodeIds: string[] = [];
  const coreWidth = Math.max(620, Math.min(1320, viewport.width - 144));
  const hasFreeResources = graph.nodes.some(
    (node) => node.kind === "resource" && !node.parentId && node.zone === "core",
  );

  let vnetY = 0;
  let vnetColumnWidth = 0;
  const vnets = graph.nodes
    .filter((node) => node.kind === "vnet")
    .sort((left, right) => stableCompare(left.name, right.name) || stableCompare(left.id, right.id));
  for (const vnet of vnets) {
    coreNodeIds.push(vnet.id);
    let subnetY = vnetY + 60;
    const directChildren = (children.get(vnet.id) ?? [])
      .sort((left, right) => stableCompare(left.name, right.name) || stableCompare(left.id, right.id));
    const subnets = directChildren.filter((node) => node.kind === "subnet");
    for (const subnet of subnets) {
      coreNodeIds.push(subnet.id);
      const members = (children.get(subnet.id) ?? [])
        .sort((left, right) => stableCompare(left.name, right.name) || stableCompare(left.id, right.id));
      if (members.length === 0) {
        positions.set(subnet.id, { x: 168, y: subnetY + 20 });
        subnetY += 78;
        continue;
      }
      const columns = Math.min(
        members.length,
        columnCount(
          Math.min(coreWidth * 0.58, 720),
          GRAPH_SIZE.resourceWidth,
          34,
          hasFreeResources ? 2 : 3,
        ),
      );
      let rows = 1;
      for (const [index, member] of members.entries()) {
        const cell = gridPosition(members.length, columns, index, GRAPH_SIZE.resourceWidth, GRAPH_SIZE.resourceHeight, 34, 28);
        rows = cell.rows;
        positions.set(member.id, { x: 58 + cell.x, y: subnetY + 42 + cell.y });
        coreNodeIds.push(member.id);
      }
      const width = 58 + columns * GRAPH_SIZE.resourceWidth + Math.max(0, columns - 1) * 34 + 40;
      vnetColumnWidth = Math.max(vnetColumnWidth, width);
      subnetY += rows * (GRAPH_SIZE.resourceHeight + 28) + 88;
    }
    const directResources = directChildren.filter((node) => node.kind === "resource");
    for (const [index, resource] of directResources.entries()) {
      positions.set(resource.id, { x: 58 + index * (GRAPH_SIZE.resourceWidth + 34), y: subnetY + 36 });
      coreNodeIds.push(resource.id);
    }
    if (directResources.length > 0) subnetY += GRAPH_SIZE.resourceHeight + 82;
    vnetY = subnetY + 82;
  }
  if (vnets.length > 0 && vnetColumnWidth === 0) vnetColumnWidth = 440;

  const freeX = vnetColumnWidth > 0 ? vnetColumnWidth + 110 : 0;
  const coreAnchors = withContainerAnchors(graph, positions);
  const free = graph.nodes
    .filter((node) => node.kind === "resource" && !node.parentId && node.zone === "core")
    .sort((left, right) => {
      const vertical = neighbourAverageY(graph, coreAnchors, left.id)
        - neighbourAverageY(graph, coreAnchors, right.id);
      return Number.isNaN(vertical) || vertical === 0
        ? stableCompare(left.name, right.name) || stableCompare(left.id, right.id)
        : vertical;
    });
  const freeWidth = Math.max(GRAPH_SIZE.resourceWidth, coreWidth - freeX);
  const freeColumns = Math.min(free.length || 1, columnCount(freeWidth, GRAPH_SIZE.resourceWidth, 44, 3));
  const columnBottoms = Array.from({ length: freeColumns }, () => Number.NEGATIVE_INFINITY);
  for (const node of free) {
    const desiredY = neighbourAverageY(graph, coreAnchors, node.id);
    let column = 0;
    let y = Number.POSITIVE_INFINITY;
    for (let candidate = 0; candidate < freeColumns; candidate += 1) {
      const nextY = Number.isFinite(desiredY)
        ? Math.max(desiredY, columnBottoms[candidate] + 40)
        : Math.max(0, columnBottoms[candidate] + 40);
      if (nextY < y) {
        column = candidate;
        y = nextY;
      }
    }
    positions.set(node.id, {
      x: freeX + column * (GRAPH_SIZE.resourceWidth + 44),
      y,
    });
    columnBottoms[column] = y + GRAPH_SIZE.resourceHeight;
    coreNodeIds.push(node.id);
  }

  const external = graph.nodes
    .filter((node) => node.zone === "external")
    .sort((left, right) => stableCompare(left.name, right.name) || stableCompare(left.id, right.id));
  positionExternalRail(graph, positions, external);
  coreNodeIds.push(...external.map((node) => node.id));

  const coreBottom = Math.max(
    0,
    ...[...positions.entries()]
      .filter(([id]) => !graph.nodes.some((node) => node.id === id && node.zone === "unconnected"))
      .map(([, placement]) => placement.y + GRAPH_SIZE.resourceHeight),
    vnetY,
  );
  const shelf = graph.nodes
    .filter((node) => node.zone === "unconnected")
    .sort((left, right) => stableCompare(left.azureType ?? left.name, right.azureType ?? right.name) || stableCompare(left.id, right.id));
  const shelfColumns = Math.min(
    shelf.length || 1,
    columnCount(coreWidth, GRAPH_SIZE.aggregateWidth, 34, 5),
  );
  for (const [index, node] of shelf.entries()) {
    const cell = gridPosition(shelf.length, shelfColumns, index, GRAPH_SIZE.aggregateWidth, GRAPH_SIZE.aggregateHeight, 34, 24);
    positions.set(node.id, { x: cell.x, y: coreBottom + 320 + cell.y });
    secondaryNodeIds.push(node.id);
  }

  if (coreNodeIds.length === 0) coreNodeIds.push(...secondaryNodeIds);
  return { positions, coreNodeIds, secondaryNodeIds, entryNodeIds: [...coreNodeIds] };
}

function directSide(graph: TopologyGraph, subjectId: string, nodeId: string) {
  let score = 0;
  for (const link of graph.links) {
    if (link.sourceId === subjectId && link.targetId === nodeId) score += link.count;
    if (link.targetId === subjectId && link.sourceId === nodeId) score -= link.count;
  }
  return score === 0 ? undefined : score > 0 ? 1 : -1;
}

function neighbourhoodSides(graph: TopologyGraph, subjectId: string) {
  const sides = new Map<string, -1 | 1>();
  const hopOne = graph.nodes.filter((node) => node.hop === 1);
  for (const node of hopOne) {
    const direct = directSide(graph, subjectId, node.id);
    sides.set(node.id, direct ?? (stableCompare(node.id, subjectId) < 0 ? -1 : 1));
  }
  for (const node of graph.nodes.filter((candidate) => (candidate.hop ?? 0) > 1)) {
    const neighbours = graph.links
      .filter((link) => link.sourceId === node.id || link.targetId === node.id)
      .map((link) => sides.get(link.sourceId === node.id ? link.targetId : link.sourceId))
      .filter((side): side is -1 | 1 => side !== undefined);
    const score = neighbours.reduce((total: number, side) => total + side, 0);
    sides.set(node.id, score === 0 ? (stableCompare(node.id, subjectId) < 0 ? -1 : 1) : score > 0 ? 1 : -1);
  }
  return sides;
}

function layoutNeighbourhood(graph: TopologyGraph, viewport: GraphViewport): TopologyLayoutPlan {
  const positions = new Map<string, Placement>();
  const coreNodeIds: string[] = [];
  const secondaryNodeIds: string[] = [];
  const subject = graph.nodes.find((node) => node.hop === 0);
  if (!subject) return { positions, coreNodeIds, secondaryNodeIds, entryNodeIds: [] };
  positions.set(subject.id, { x: 0, y: 0 });
  coreNodeIds.push(subject.id);
  const sides = neighbourhoodSides(graph, subject.id);
  const columnGap = clamp(viewport.width / 3.2, 300, 410);

  for (const hop of [1, 2]) {
    for (const side of [-1, 1] as const) {
      const members = graph.nodes
        .filter((node) => node.hop === hop && sides.get(node.id) === side)
        .sort((left, right) => stableCompare(left.name, right.name) || stableCompare(left.id, right.id));
      for (const [index, node] of members.entries()) {
        positions.set(node.id, {
          x: side * hop * columnGap,
          y: (index - (members.length - 1) / 2) * (GRAPH_SIZE.resourceHeight + 38),
        });
        if (hop === 1) coreNodeIds.push(node.id);
        else secondaryNodeIds.push(node.id);
      }
    }
  }
  return { positions, coreNodeIds, secondaryNodeIds, entryNodeIds: [...coreNodeIds] };
}

export type SpatialDirection = "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown";

export function nearestNodeInDirection(
  currentId: string,
  direction: SpatialDirection,
  points: ReadonlyMap<string, Placement>,
) {
  const current = points.get(currentId);
  if (!current) return undefined;
  const horizontal = direction === "ArrowLeft" || direction === "ArrowRight";
  const sign = direction === "ArrowLeft" || direction === "ArrowUp" ? -1 : 1;
  let best: { id: string; score: number } | undefined;
  for (const [id, point] of points) {
    if (id === currentId) continue;
    const primary = horizontal ? point.x - current.x : point.y - current.y;
    if (primary * sign <= 0) continue;
    const secondary = horizontal ? Math.abs(point.y - current.y) : Math.abs(point.x - current.x);
    const score = Math.abs(primary) + secondary * 1.8;
    if (!best || score < best.score || (score === best.score && stableCompare(id, best.id) < 0)) {
      best = { id, score };
    }
  }
  return best?.id;
}

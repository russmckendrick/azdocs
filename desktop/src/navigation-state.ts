import type { DashboardDestination, ViewId } from "./types";

export type RelationshipLocation =
  | { kind: "estate" }
  | { kind: "group"; groupId: string }
  | { kind: "neighbourhood"; resourceId: string };

export interface RelationshipWorkspaceState {
  location: RelationshipLocation;
  depth: 1 | 2;
  excludedClasses: string[];
  expandedSubscriptions?: string[];
  showUnconnected: boolean;
  expandedAggregateId?: string;
}

export type WorkspaceSurface =
  | { kind: "section" }
  | { kind: "resource"; resourceId: string };

export interface NavigationFrame {
  result?: DashboardDestination;
  section: ViewId;
  surface: WorkspaceSurface;
  relationships: RelationshipWorkspaceState;
}

export interface NavigationState extends NavigationFrame {
  history: NavigationFrame[];
}

export type NavigationAction =
  | { type: "open-results"; destination: DashboardDestination }
  | { type: "clear-results" }
  | { type: "open-section"; section: ViewId }
  | { type: "open-resource"; resourceId: string }
  | { type: "open-relationships"; resourceId: string }
  | { type: "navigate-relationships"; workspace: RelationshipWorkspaceState }
  | { type: "update-relationships"; workspace: RelationshipWorkspaceState }
  | { type: "back" }
  | { type: "reset-snapshot" };

const MAX_HISTORY = 40;

export function initialRelationshipWorkspace(): RelationshipWorkspaceState {
  return {
    location: { kind: "estate" },
    depth: 1,
    excludedClasses: [],
    showUnconnected: true,
  };
}

export function initialNavigationState(): NavigationState {
  return {
    section: "overview",
    surface: { kind: "section" },
    relationships: initialRelationshipWorkspace(),
    history: [],
  };
}

/**
 * A deep-enough copy for the history stack and for capturing the controls that
 * produced a successful graph.
 *
 * TopologyView had this spread out by hand twice; the arrays must be copied or
 * a later mutation would rewrite a frame the user has already navigated past.
 */
export function cloneRelationshipWorkspace(
  workspace: RelationshipWorkspaceState,
): RelationshipWorkspaceState {
  return {
    ...workspace,
    location: { ...workspace.location },
    excludedClasses: [...workspace.excludedClasses],
    expandedSubscriptions: workspace.expandedSubscriptions
      ? [...workspace.expandedSubscriptions]
      : undefined,
  };
}

function currentFrame(state: NavigationState): NavigationFrame {
  return {
    result: state.result,
    section: state.section,
    surface: { ...state.surface },
    relationships: cloneRelationshipWorkspace(state.relationships),
  };
}

function pushCurrent(state: NavigationState) {
  return [...state.history, currentFrame(state)].slice(-MAX_HISTORY);
}

/**
 * Stable identity for "which graph is on screen".
 *
 * The reducer and TopologyView derived this independently, so a new location
 * kind had to be added in two places for history and camera resets to agree.
 */
export function locationKey(location: RelationshipLocation) {
  if (location.kind === "estate") return "estate";
  return location.kind === "group"
    ? `group:${location.groupId}`
    : `neighbourhood:${location.resourceId}`;
}

export function navigationReducer(
  state: NavigationState,
  action: NavigationAction,
): NavigationState {
  switch (action.type) {
    case "open-results":
      return { ...state, section: action.destination.view, result: action.destination,
        surface: { kind: "section" }, relationships: action.destination.view === "topology" ? initialRelationshipWorkspace() : state.relationships, history: pushCurrent(state) };
    case "clear-results":
      return { ...state, result: undefined };
    case "open-section":
      return {
        ...state,
        section: action.section,
        result: undefined,
        surface: { kind: "section" },
        history: [],
      };
    case "open-resource":
      if (state.surface.kind === "resource" && state.surface.resourceId === action.resourceId) {
        return state;
      }
      return {
        ...state,
        surface: { kind: "resource", resourceId: action.resourceId },
        history: pushCurrent(state),
      };
    case "open-relationships":
      return {
        ...state,
        section: "topology",
        surface: { kind: "section" },
        relationships: {
          ...state.relationships,
          location: { kind: "neighbourhood", resourceId: action.resourceId },
          expandedAggregateId: undefined,
        },
        history: pushCurrent(state),
      };
    case "navigate-relationships":
      if (locationKey(state.relationships.location) === locationKey(action.workspace.location)) {
        return {
          ...state,
          relationships: cloneRelationshipWorkspace(action.workspace),
        };
      }
      return {
        ...state,
        section: "topology",
        surface: { kind: "section" },
        relationships: cloneRelationshipWorkspace(action.workspace),
        history: pushCurrent(state),
      };
    case "update-relationships":
      return {
        ...state,
        relationships: cloneRelationshipWorkspace(action.workspace),
      };
    case "back": {
      const previous = state.history.at(-1);
      if (!previous) return state;
      return {
        section: previous.section,
        result: previous.result,
        surface: { ...previous.surface },
        relationships: cloneRelationshipWorkspace(previous.relationships),
        history: state.history.slice(0, -1),
      };
    }
    case "reset-snapshot":
      return {
        section: state.section,
        surface: { kind: "section" },
        relationships: initialRelationshipWorkspace(),
        history: [],
      };
  }
}

import { useEffect, useMemo, useRef, useState, Fragment } from "react";
import {
  Activity,
  AlertTriangle,
  ArrowLeft,
  ChevronDown,
  ChevronRight,
  CircleHelp,
  PauseCircle,
  PlayCircle,
  RotateCcw,
  Scan,
  Unplug,
  Waypoints,
  ZoomIn,
  ZoomOut,
} from "lucide-react";
import type { EstateSnapshot, TopologyGraph, TopologyRequest } from "../types";
import {
  cloneRelationshipWorkspace,
  locationKey,
  type RelationshipWorkspaceState,
} from "../navigation-state";
import { getTopology } from "../api";
import { RESOURCE_GROUP_ICON } from "../azure-icons";
import {
  CytoscapeResourceGraph,
  kindClassColor,
  type GraphActivation,
  type GraphCameraMode,
  type GraphCameraRequest,
} from "./CytoscapeResourceGraph";
import {
  refreshFailed,
  refreshStarted,
  refreshSucceeded,
  resolveAggregateActivation,
  toggleSubscriptionLane,
  describeCounts,
  expandSubscriptionLane,
} from "./topology-view-state";
import { errorMessage } from "../format";
import { EmptyState } from "./view-chrome";

const ALL_KIND_CLASSES = ["network", "structure", "data", "identity", "monitoring"] as const;

interface TopologyError {
  message: string;
  hasPrevious: boolean;
}

export function TopologyView({
  estate,
  theme,
  workspace,
  active,
  backLabel,
  onBack,
  onNavigate,
  onWorkspaceChange,
  onInspect,
}: {
  estate: EstateSnapshot;
  theme: "light" | "dark";
  workspace: RelationshipWorkspaceState;
  active: boolean;
  backLabel?: string;
  onBack?: () => void;
  onNavigate: (workspace: RelationshipWorkspaceState) => void;
  onWorkspaceChange: (workspace: RelationshipWorkspaceState) => void;
  onInspect: (id: string) => void;
}) {
  const location = workspace.location;
  const mode = location.kind === "neighbourhood" ? "neighbourhood" : "estate";
  const selectedResourceId = location.kind === "neighbourhood" ? location.resourceId : undefined;
  const selected = selectedResourceId
    ? estate.resources.find((resource) => resource.id === selectedResourceId)
    : undefined;
  const activeResourceGroupId = location.kind === "group"
    ? location.groupId
    : undefined;
  const {
    depth,
    excludedClasses,
    expandedSubscriptions,
    showUnconnected,
    expandedAggregateId,
  } = workspace;
  const [motionEnabled, setMotionEnabled] = useState(true);
  const [motionReduced, setMotionReduced] = useState(false);
  const [helpOpen, setHelpOpen] = useState(false);
  const [legendOpen, setLegendOpen] = useState(false);
  const [camera, setCamera] = useState<GraphCameraRequest>({ mode: "core", nonce: 0 });
  const [topology, setTopology] = useState<TopologyGraph>();
  const [topologyStale, setTopologyStale] = useState(false);
  const [topologyError, setTopologyError] = useState<TopologyError>();
  const [retryNonce, setRetryNonce] = useState(0);
  const lastLocationRef = useRef("");
  const requestTokenRef = useRef(0);
  const successfulControlsRef = useRef<RelationshipWorkspaceState | undefined>(undefined);

  // Group membership is decided in Rust and arrives on the snapshot; deriving
  // it again here is what let the two drift apart on synthetic-group ids.
  const resourceGroups = estate.resourceGroupSummaries;
  const groupByResourceId = useMemo(() => {
    const map = new Map<string, string>();
    for (const group of resourceGroups) {
      for (const id of group.resourceIds) map.set(id, group.id);
    }
    return map;
  }, [resourceGroups]);
  const activeResourceGroup = resourceGroups.find((group) => group.id === activeResourceGroupId);
  const selectedResourceGroupId = selected ? groupByResourceId.get(selected.id) : undefined;
  const selectedResourceGroup = resourceGroups.find((group) => group.id === selectedResourceGroupId);
  const selectedNodeId = mode === "neighbourhood" ? selected?.id : undefined;

  function nextWorkspace(update: Partial<RelationshipWorkspaceState>) {
    return { ...workspace, ...update };
  }

  function replaceWorkspace(update: Partial<RelationshipWorkspaceState>) {
    onWorkspaceChange(nextWorkspace(update));
  }

  function navigateWorkspace(update: Partial<RelationshipWorkspaceState>) {
    onNavigate(nextWorkspace(update));
  }

  useEffect(() => {
    if (workspace.location.kind === "neighbourhood" && !selected) {
      replaceWorkspace({ location: { kind: "estate" }, expandedAggregateId: undefined });
    }
    // The workspace callback is intentionally driven by the controlled location.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selected, workspace.location]);

  // Keep the previous successful graph visible while a replacement is being
  // built. A failed replacement can then be retried or reverted without
  // losing spatial context.
  useEffect(() => {
    if (mode === "neighbourhood" && !selected) return;
    const controls = cloneRelationshipWorkspace(workspace);
    const request: TopologyRequest = {
      snapshotId: estate.id,
      mode:
        mode === "neighbourhood"
          ? {
              kind: "neighbourhood",
              resourceId: selected?.id ?? "",
              depth,
              kindClasses: excludedClasses.length > 0
                ? ALL_KIND_CLASSES.filter((kindClass) => !excludedClasses.includes(kindClass))
                : [],
            }
          : activeResourceGroupId
            ? { kind: "group", groupId: activeResourceGroupId }
            : { kind: "estate", expandedSubscriptions: expandedSubscriptions ?? [] },
      // TopologyScope is a wire type: Rust defaults every field, but the
      // request carries them explicitly so the shape matches the DTO.
      scope: { subscriptions: [], azureTypes: [], showUnconnected },
    };
    const token = requestTokenRef.current + 1;
    requestTokenRef.current = token;
    const started = refreshStarted(Boolean(topology));
    setTopologyStale(started.stale);
    setTopologyError(started.error);
    getTopology(request)
      .then((graph) => {
        if (requestTokenRef.current !== token) return;
        successfulControlsRef.current = controls;
        setTopology(graph);
        const succeeded = refreshSucceeded();
        setTopologyStale(succeeded.stale);
        setTopologyError(succeeded.error);
      })
      .catch((error: unknown) => {
        if (requestTokenRef.current !== token) return;
        const failed = refreshFailed(Boolean(topology), errorMessage(error, "The map could not be refreshed."));
        setTopologyStale(failed.stale);
        setTopologyError(failed.error);
      });
    // The retained topology is deliberately read at request start only.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    activeResourceGroupId,
    depth,
    estate.id,
    excludedClasses,
    expandedSubscriptions,
    mode,
    retryNonce,
    selected?.id,
    showUnconnected,
  ]);

  useEffect(() => {
    if (workspace.location.kind !== "group") return;
    const groupId = workspace.location.groupId;
    if (resourceGroups.some((group) => group.id === groupId)) return;
    replaceWorkspace({ location: { kind: "estate" }, expandedAggregateId: undefined });
    // The workspace callback is intentionally driven by the controlled location.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [resourceGroups, workspace.location]);

  useEffect(() => {
    const preference = window.matchMedia("(prefers-reduced-motion: reduce)");
    const update = () => setMotionReduced(preference.matches);
    update();
    preference.addEventListener("change", update);
    return () => preference.removeEventListener("change", update);
  }, []);

  useEffect(() => {
    const key = locationKey(workspace.location);
    if (!lastLocationRef.current) {
      lastLocationRef.current = key;
      requestCamera("core");
      return;
    }
    if (lastLocationRef.current === key) return;
    lastLocationRef.current = key;
    requestCamera("core");
  }, [workspace.location]);

  useEffect(() => {
    if (!active) return;
    requestCamera("core");
    // Reopening the relationship surface must refit after the record overlay is removed.
  }, [active]);

  const counts = topology?.counts;
  const graphUnit = topology?.level === "estate" ? "groups" : "resources";
  const classCounts = new Map((topology?.kindClasses ?? []).map((entry) => [entry.class, entry.count]));
  const countsCaption = topology ? describeCounts(topology) : undefined;
  // The trail is the title: Map › subscription › group › resource, every
  // crumb but the last a link, the last the page's own name. "Estate" as a
  // separate crumb pointed at the same place as Map and named nothing the
  // reader had seen, so it is gone.
  const title = mode === "neighbourhood"
    ? selected?.name ?? "Choose a resource"
    : activeResourceGroup?.name ?? "Map";
  const subtitle = mode === "neighbourhood"
    ? selected
      ? `${depth} hop${depth === 1 ? "" : "s"} · ${counts?.total ?? "…"} resources in reach`
      : "Select a resource to explore its neighbourhood"
    : activeResourceGroup
      ? `${activeResourceGroup.resourceCount} resource${activeResourceGroup.resourceCount === 1 ? "" : "s"}`
      : `${estate.subscriptions.length} subscription${estate.subscriptions.length === 1 ? "" : "s"} · ${resourceGroups.length} groups · ${estate.resources.length.toLocaleString()} resources`;
  const trailGroup = mode === "neighbourhood" ? selectedResourceGroup : activeResourceGroup;
  const trail: Array<{ key: string; label: string; open: () => void }> = [];
  if (trailGroup) {
    trail.push({ key: "map", label: "Map", open: showResourceGroups });
    trail.push({
      key: "subscription",
      label: trailGroup.subscriptionName,
      open: () => openSubscription(trailGroup.subscriptionId),
    });
  }
  if (mode === "neighbourhood" && selectedResourceGroup) {
    trail.push({
      key: "group",
      label: selectedResourceGroup.name,
      open: () => openResourceGroup(selectedResourceGroup.id),
    });
  }

  const plural = (count: number, noun: string) => (count === 1 ? noun : `${noun}s`);
  const unconnectedLabel = activeResourceGroup
    ? "Include resources without drawn relationships"
    : "Include groups without cross-group relationships";

  function requestCamera(cameraMode: GraphCameraMode) {
    setCamera((current) => ({ mode: cameraMode, nonce: current.nonce + 1 }));
  }

  function openResourceGroup(id: string) {
    const group = resourceGroups.find((candidate) => candidate.id === id);
    if (!group) return;
    navigateWorkspace({ location: { kind: "group", groupId: id }, expandedAggregateId: undefined });
    setHelpOpen(false);
  }

  function showResourceGroups() {
    navigateWorkspace({ location: { kind: "estate" }, expandedAggregateId: undefined });
  }

  function openSubscription(subscriptionId: string) {
    const drawnExpanded = topology?.lanes
      .filter((lane) => lane.expanded)
      .map((lane) => lane.subscriptionId) ?? [];
    navigateWorkspace({
      location: { kind: "estate" },
      expandedAggregateId: undefined,
      expandedSubscriptions: expandSubscriptionLane(expandedSubscriptions, drawnExpanded, subscriptionId),
    });
  }

  function openNeighbourhood(resourceId: string) {
    navigateWorkspace({
      location: { kind: "neighbourhood", resourceId },
      expandedAggregateId: undefined,
    });
    setHelpOpen(false);
  }

  function toggleLane(subscriptionId: string) {
    const drawnExpanded = topology?.lanes
      .filter((lane) => lane.expanded)
      .map((lane) => lane.subscriptionId) ?? [];
    replaceWorkspace({
      expandedSubscriptions: toggleSubscriptionLane(
        expandedSubscriptions,
        drawnExpanded,
        subscriptionId,
      ),
    });
    requestCamera("core");
  }

  function activateGraphItem(activation: GraphActivation) {
    if (activation.kind === "subscription") {
      toggleLane(activation.subscriptionId);
      return;
    }
    if (activation.kind === "resource-group") {
      openResourceGroup(activation.groupId);
      return;
    }
    if (activation.kind === "resource-neighbourhood") {
      openNeighbourhood(activation.resourceId);
      return;
    }
    if (activation.kind === "aggregate") {
      const aggregate = topology?.nodes.find((node) => node.id === activation.nodeId);
      if (!aggregate) return;
      const next = resolveAggregateActivation(expandedAggregateId, activation.nodeId, aggregate.memberIds, aggregate.groupIds);
      if (next.kind === "open-resource") {
        onInspect(next.resourceId);
        return;
      }
      if (next.kind === "open-group") {
        openResourceGroup(next.groupId);
        return;
      }
      replaceWorkspace({ expandedAggregateId: next.expandedId });
      return;
    }
    onInspect(activation.resourceId);
  }

  function toggleClass(kindClass: string) {
    replaceWorkspace({
      excludedClasses: excludedClasses.includes(kindClass)
        ? excludedClasses.filter((candidate) => candidate !== kindClass)
        : [...excludedClasses, kindClass],
    });
  }

  function handleDepthKeyDown(event: React.KeyboardEvent<HTMLDivElement>) {
    if (!["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End"].includes(event.key)) return;
    event.preventDefault();
    const nextDepth = event.key === "ArrowLeft" || event.key === "ArrowUp" || event.key === "Home" ? 1 : 2;
    replaceWorkspace({ depth: nextDepth });
    event.currentTarget.querySelector<HTMLButtonElement>(`[data-depth="${nextDepth}"]`)?.focus();
  }

  function revertGraph() {
    const controls = successfulControlsRef.current;
    if (!controls) return;
    onWorkspaceChange(cloneRelationshipWorkspace(controls));
    setTopologyError(undefined);
    setTopologyStale(false);
    requestCamera("core");
  }

  return (
    <div className={active ? "topology-workspace" : "topology-workspace covered"} aria-hidden={!active}>
      <section className="topology-stage" aria-label={mode === "neighbourhood"
        ? `Neighbourhood of ${selected?.name ?? "no selected resource"}`
        : activeResourceGroup
          ? `Map of resource group ${activeResourceGroup.name}`
          : "Estate map"}>
        <header className="topology-commandbar">
          {backLabel && onBack ? (
            <button className="topology-return" onClick={onBack} aria-label={backLabel} title={backLabel}>
              <ArrowLeft size={15} /> <span>Back</span>
            </button>
          ) : null}
          <nav className="topology-trail" aria-label="Map location">
            {trail.map((crumb) => (
              <Fragment key={crumb.key}>
                <button className="topology-trail-crumb" title={crumb.label} onClick={crumb.open}>{crumb.label}</button>
                <ChevronRight size={12} aria-hidden="true" />
              </Fragment>
            ))}
            <h1 title={title}>{title}</h1>
            <span className="topology-trail-note">{subtitle}</span>
          </nav>

          {mode === "neighbourhood" ? (
            <div className="graph-depth-switch" role="radiogroup" aria-label="Neighbourhood depth" onKeyDown={handleDepthKeyDown}>
              <span className="graph-depth-label" aria-hidden="true"><Waypoints size={14} /> Reach</span>
              {[1, 2].map((value) => (
                <button key={value} data-depth={value} role="radio" aria-checked={depth === value} tabIndex={depth === value ? 0 : -1} className={depth === value ? "active" : ""} onClick={() => replaceWorkspace({ depth: value as 1 | 2 })}>
                  {value} hop{value === 1 ? "" : "s"}
                </button>
              ))}
            </div>
          ) : null}

          {mode === "estate" ? (
            <button className={showUnconnected ? "topology-inline-toggle active" : "topology-inline-toggle"} aria-pressed={showUnconnected} aria-label={unconnectedLabel} title={unconnectedLabel} onClick={() => replaceWorkspace({ showUnconnected: !showUnconnected })}>
              <Unplug size={15} /><span>Unconnected</span>
            </button>
          ) : null}

          <div className="topology-tools" role="group" aria-label="Graph view controls">
            <button className="topology-tool-button" aria-label="Fit all represented regions" data-tooltip="Fit all" onClick={() => requestCamera("all")}>
              <Scan size={16} />
            </button>
            <button className="topology-tool-button" aria-label="Zoom in" data-tooltip="Zoom in · +" onClick={() => requestCamera("zoom-in")}>
              <ZoomIn size={16} />
            </button>
            <button className="topology-tool-button" aria-label="Zoom out" data-tooltip="Zoom out · −" onClick={() => requestCamera("zoom-out")}>
              <ZoomOut size={16} />
            </button>
            <button className="topology-tool-button" aria-label={motionReduced ? "Motion reduced by system preference" : motionEnabled ? "Pause selected path motion" : "Play selected path motion"} aria-pressed={motionEnabled && !motionReduced} data-tooltip={motionReduced ? "Motion reduced" : motionEnabled ? "Pause path motion" : "Play path motion"} onClick={() => setMotionEnabled((current) => !current)} disabled={motionReduced}>
              {motionEnabled && !motionReduced ? <PauseCircle size={16} /> : <PlayCircle size={16} />}
            </button>
            <button className="topology-tool-button" aria-label="Reset subscription lanes" data-tooltip={expandedSubscriptions ? "Reset subscription lanes" : "Subscription lanes at default"} onClick={() => replaceWorkspace({ expandedSubscriptions: undefined })} disabled={!expandedSubscriptions}>
              <RotateCcw size={16} />
            </button>
            <button className="topology-tool-button" aria-label="Graph interaction help" data-tooltip="Interaction help" onClick={() => setHelpOpen((current) => !current)} aria-expanded={helpOpen} aria-controls="topology-help-popover">
              <CircleHelp size={16} />
            </button>
            {helpOpen ? (
              <div id="topology-help-popover" className="topology-help-popover" role="note">
                <strong>Graph controls</strong>
                <span>Enter opens a record; R explores its relationships. Aggregate tiles expand in place. Arrow keys move spatially; drag to pan; scroll to zoom; 0 fits the readable core.</span>
              </div>
            ) : null}
          </div>
        </header>

        {topologyError ? (
          <div className="topology-error-rail" role="alert" aria-live="assertive">
            <AlertTriangle size={16} />
            <span><strong>Map refresh failed.</strong> {topologyError.message}{topologyError.hasPrevious ? " The last successful graph is still shown." : ""}</span>
            <button onClick={() => setRetryNonce((value) => value + 1)}>Retry</button>
            {topologyError.hasPrevious ? <button onClick={revertGraph}>Revert controls</button> : null}
          </div>
        ) : null}

        {topology ? (
          <CytoscapeResourceGraph graph={topology} estate={estate} theme={theme} selectedNodeId={selectedNodeId} expandedAggregateId={expandedAggregateId} motionEnabled={active && motionEnabled && !motionReduced} camera={camera} onActivate={activateGraphItem} />
        ) : topologyError ? (
          <EmptyState
            className="graph-empty-state"
            role="status"
            icon={<AlertTriangle size={24} />}
            title="No map is available"
            detail="Retry the request or return to the estate after checking the stored snapshot."
          />
        ) : (
          <EmptyState
            className="graph-empty-state"
            role="status"
            icon={<img src={RESOURCE_GROUP_ICON} alt="" />}
            title="Building the map…"
          />
        )}

        <footer className="topology-status-rail" aria-label="Map status and legend">
          <div className="topology-counts" role="status">
            <Activity size={14} />
            {counts && countsCaption ? (
              <span><strong>{counts.drawn}</strong> of <strong>{counts.total}</strong> {graphUnit} drawn{countsCaption.extras.map((extra) => <span key={extra.label}> · {extra.emphasis ? <strong>{extra.count} {extra.label}</strong> : <>{extra.count} {extra.label}</>}</span>)}</span>
            ) : <span>Building map…</span>}
            <i />
            <span><strong>{counts?.totalLinks ?? 0}</strong> {topology?.level === "estate" ? "cross-group " : ""}{plural(counts?.totalLinks ?? 0, "relationship")} drawn as <strong>{counts?.drawnLinks ?? 0}</strong> {plural(counts?.drawnLinks ?? 0, "connector")}</span>
            {topologyStale ? <b className="topology-stale">Stale map</b> : null}
          </div>
          <button className="topology-legend-toggle" onClick={() => setLegendOpen((current) => !current)} aria-expanded={legendOpen} aria-controls="topology-kind-legend">Legend <ChevronDown size={13} /></button>
          <div id="topology-kind-legend" className={legendOpen ? "topology-kind-legend open" : "topology-kind-legend"} aria-label="Relationship kinds">
            {ALL_KIND_CLASSES.map((kindClass) => {
              const active = !excludedClasses.includes(kindClass);
              const count = classCounts.get(kindClass);
              return mode === "neighbourhood" ? (
                <button key={kindClass} aria-pressed={active} className={active ? "active" : ""} onClick={() => toggleClass(kindClass)}><i style={{ background: kindClassColor(kindClass) }} />{kindClass}{count !== undefined ? <b>{count}</b> : null}</button>
              ) : (
                <span key={kindClass}><i style={{ background: kindClassColor(kindClass) }} />{kindClass}{count !== undefined ? <b>{count}</b> : null}</span>
              );
            })}
          </div>
        </footer>
      </section>
    </div>
  );
}

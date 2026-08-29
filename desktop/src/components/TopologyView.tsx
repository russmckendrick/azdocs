import { useEffect, useMemo, useRef, useState } from "react";
import {
  Activity,
  AlertTriangle,
  ArrowLeft,
  ChevronDown,
  ChevronRight,
  CircleHelp,
  GitBranch,
  LocateFixed,
  PauseCircle,
  PlayCircle,
  RotateCcw,
  Scan,
  SlidersHorizontal,
  Unplug,
  Waypoints,
  ZoomIn,
  ZoomOut,
} from "lucide-react";
import type { EstateSnapshot, TopologyGraph, TopologyRequest } from "../types";
import type { RelationshipWorkspaceState } from "../navigation-state";
import { getTopology } from "../api";
import { RESOURCE_GROUP_ICON } from "../azure-icons";
import {
  CytoscapeResourceGraph,
  kindClassColor,
  type GraphActivation,
  type GraphCameraMode,
  type GraphCameraRequest,
} from "./CytoscapeResourceGraph";
import { buildResourceGroupTopology } from "./topology-model";
import {
  refreshFailed,
  refreshStarted,
  refreshSucceeded,
  resolveAggregateActivation,
  toggleSubscriptionLane,
} from "./topology-view-state";

const ALL_KIND_CLASSES = ["network", "structure", "data", "identity", "monitoring"] as const;

interface TopologyError {
  message: string;
  hasPrevious: boolean;
}

function errorMessage(error: unknown) {
  return error instanceof Error && error.message
    ? error.message
    : "The relationship graph could not be refreshed.";
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
  const [toolsOpen, setToolsOpen] = useState(false);
  const [legendOpen, setLegendOpen] = useState(false);
  const [camera, setCamera] = useState<GraphCameraRequest>({ mode: "core", nonce: 0 });
  const [topology, setTopology] = useState<TopologyGraph>();
  const [topologyStale, setTopologyStale] = useState(false);
  const [topologyError, setTopologyError] = useState<TopologyError>();
  const [retryNonce, setRetryNonce] = useState(0);
  const lastLocationRef = useRef("");
  const requestTokenRef = useRef(0);
  const successfulControlsRef = useRef<RelationshipWorkspaceState | undefined>(undefined);

  const resourceGroupTopology = useMemo(() => buildResourceGroupTopology(estate), [estate]);
  const activeResourceGroup = resourceGroupTopology.groups.find((group) => group.id === activeResourceGroupId);
  const selectedResourceGroupId = selected
    ? resourceGroupTopology.resourceGroupByResourceId.get(selected.id)
    : undefined;
  const selectedResourceGroup = resourceGroupTopology.groups.find(
    (group) => group.id === selectedResourceGroupId,
  );
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
    const controls: RelationshipWorkspaceState = {
      ...workspace,
      location: { ...workspace.location },
      excludedClasses: [...excludedClasses],
      expandedSubscriptions: expandedSubscriptions ? [...expandedSubscriptions] : undefined,
    };
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
      scope: { showUnconnected },
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
        const failed = refreshFailed(Boolean(topology), errorMessage(error));
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
    if (resourceGroupTopology.groups.some((group) => group.id === groupId)) return;
    replaceWorkspace({ location: { kind: "estate" }, expandedAggregateId: undefined });
    // The workspace callback is intentionally driven by the controlled location.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [resourceGroupTopology, workspace.location]);

  useEffect(() => {
    const preference = window.matchMedia("(prefers-reduced-motion: reduce)");
    const update = () => setMotionReduced(preference.matches);
    update();
    preference.addEventListener("change", update);
    return () => preference.removeEventListener("change", update);
  }, []);

  useEffect(() => {
    const locationKey = workspace.location.kind === "estate"
      ? "estate"
      : workspace.location.kind === "group"
        ? `group:${workspace.location.groupId}`
        : `neighbourhood:${workspace.location.resourceId}`;
    if (!lastLocationRef.current) {
      lastLocationRef.current = locationKey;
      requestCamera("core");
      return;
    }
    if (lastLocationRef.current === locationKey) return;
    lastLocationRef.current = locationKey;
    requestCamera("core");
  }, [workspace.location]);

  useEffect(() => {
    if (!active) return;
    requestCamera("core");
    // Reopening the relationship surface must refit after the record overlay is removed.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active]);

  const counts = topology?.counts;
  const graphUnit = topology?.level === "estate" ? "groups" : "resources";
  const classCounts = new Map((topology?.kindClasses ?? []).map((entry) => [entry.class, entry.count]));
  const contextTitle = mode === "neighbourhood"
    ? selected?.name ?? "Choose a resource"
    : activeResourceGroup?.name ?? "Estate map";
  const contextSubtitle = mode === "neighbourhood"
    ? selected
      ? `${depth} hop${depth === 1 ? "" : "s"} · ${counts?.total ?? "…"} resources in reach`
      : "Select a resource to explore its neighbourhood"
    : activeResourceGroup
      ? `${activeResourceGroup.subscriptionName} · ${activeResourceGroup.resourceCount} resources`
      : `${resourceGroupTopology.groups.length} groups · ${estate.resources.length.toLocaleString()} resources`;

  function requestCamera(cameraMode: GraphCameraMode) {
    setCamera((current) => ({ mode: cameraMode, nonce: current.nonce + 1 }));
  }

  function openResourceGroup(id: string) {
    const group = resourceGroupTopology.groups.find((candidate) => candidate.id === id);
    if (!group) return;
    navigateWorkspace({ location: { kind: "group", groupId: id }, expandedAggregateId: undefined });
    setToolsOpen(false);
  }

  function showResourceGroups() {
    navigateWorkspace({ location: { kind: "estate" }, expandedAggregateId: undefined });
  }

  function openNeighbourhood(resourceId: string) {
    navigateWorkspace({
      location: { kind: "neighbourhood", resourceId },
      expandedAggregateId: undefined,
    });
    setToolsOpen(false);
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
      const next = resolveAggregateActivation(expandedAggregateId, activation.nodeId, aggregate.memberIds);
      if (next.kind === "open-resource") {
        onInspect(next.resourceId);
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
    onWorkspaceChange({
      ...controls,
      location: { ...controls.location },
      excludedClasses: [...controls.excludedClasses],
      expandedSubscriptions: controls.expandedSubscriptions
        ? [...controls.expandedSubscriptions]
        : undefined,
    });
    setTopologyError(undefined);
    setTopologyStale(false);
    requestCamera("core");
  }

  return (
    <div className={active ? "topology-workspace" : "topology-workspace covered"} aria-hidden={!active}>
      <section className="topology-stage" aria-label={mode === "neighbourhood"
        ? `Relationship neighbourhood for ${selected?.name ?? "no selected resource"}`
        : activeResourceGroup
          ? `Relationship map for resource group ${activeResourceGroup.name}`
          : "Azure estate relationship map"}>
        <header className="topology-commandbar">
          {backLabel && onBack ? (
            <button className="topology-return" onClick={onBack}>
              <ArrowLeft size={15} /> <span>{backLabel}</span>
            </button>
          ) : null}
          <div className="topology-title">
            <span className="topology-title-icon"><GitBranch size={18} /></span>
            <div>
              <nav className="topology-scope-line" aria-label="Relationship location">
                <button onClick={showResourceGroups}>Relationships</button>
                {workspace.location.kind !== "estate" ? (
                  <>
                    <ChevronRight size={12} aria-hidden="true" />
                    <button onClick={showResourceGroups}>Estate</button>
                  </>
                ) : null}
                {activeResourceGroup ? (
                  <>
                    <ChevronRight size={12} aria-hidden="true" />
                    <span>{activeResourceGroup.subscriptionName}</span>
                    <ChevronRight size={12} aria-hidden="true" />
                    <span aria-current="page">{activeResourceGroup.name}</span>
                  </>
                ) : null}
                {mode === "neighbourhood" && selectedResourceGroup ? (
                  <>
                    <ChevronRight size={12} aria-hidden="true" />
                    <span>{selectedResourceGroup.subscriptionName}</span>
                    <ChevronRight size={12} aria-hidden="true" />
                    <button onClick={() => openResourceGroup(selectedResourceGroup.id)}>{selectedResourceGroup.name}</button>
                    <ChevronRight size={12} aria-hidden="true" />
                    <span aria-current="page">{selected?.name}</span>
                  </>
                ) : null}
              </nav>
              <h1>{contextTitle}</h1>
              <span>{contextSubtitle}</span>
            </div>
          </div>

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

          {mode === "estate" && activeResourceGroup ? (
            <button className={showUnconnected ? "topology-inline-toggle active" : "topology-inline-toggle"} aria-pressed={showUnconnected} aria-label="Include resources without drawn relationships" title="Include resources without drawn relationships" onClick={() => replaceWorkspace({ showUnconnected: !showUnconnected })}>
              <Unplug size={15} /><span>Unconnected</span>
            </button>
          ) : null}

          <div className="topology-tools">
            <button className="topology-recenter" aria-label={selectedNodeId ? "Recenter the selected item" : "Recenter the readable core"} onClick={() => requestCamera(selectedNodeId ? "selection" : "core")} title={selectedNodeId ? "Recenter the selected item" : "Recenter the readable core"}>
              <LocateFixed size={16} /><span>Recenter</span>
            </button>
            <div className="topology-more">
              <button className="topology-more-trigger" aria-label="More graph controls" onClick={() => setToolsOpen((current) => !current)} aria-haspopup="menu" aria-expanded={toolsOpen} aria-controls="topology-more-menu">
                <SlidersHorizontal size={16} /><span>Controls</span>
              </button>
              {toolsOpen ? (
                <div id="topology-more-menu" className="topology-more-menu" role="menu">
                  <button role="menuitem" onClick={() => { requestCamera("all"); setToolsOpen(false); }}><Scan size={16} /><span><strong>Fit all</strong><small>Show every represented region</small></span></button>
                  <button role="menuitem" onClick={() => requestCamera("zoom-in")}><ZoomIn size={16} /><span><strong>Zoom in</strong><small>Keyboard: +</small></span></button>
                  <button role="menuitem" onClick={() => requestCamera("zoom-out")}><ZoomOut size={16} /><span><strong>Zoom out</strong><small>Keyboard: −</small></span></button>
                  <button role="menuitem" onClick={() => setMotionEnabled((current) => !current)} disabled={motionReduced}>
                    {motionEnabled && !motionReduced ? <PauseCircle size={16} /> : <PlayCircle size={16} />}
                    <span><strong>{motionReduced ? "Motion reduced" : motionEnabled ? "Pause selected path" : "Play selected path"}</strong><small>{motionReduced ? "Uses your system preference" : "Only the selected path animates"}</small></span>
                  </button>
                  {expandedSubscriptions ? <button role="menuitem" onClick={() => { replaceWorkspace({ expandedSubscriptions: undefined }); setToolsOpen(false); }}><RotateCcw size={16} /><span><strong>Reset subscription lanes</strong><small>Restore the snapshot default</small></span></button> : null}
                  <div className="topology-help" role="note"><CircleHelp size={16} /><p><strong>Graph controls</strong><span>Enter opens a record; R explores its relationships. Aggregate tiles expand in place. Arrow keys move spatially; drag to pan; scroll to zoom; 0 recentres.</span></p></div>
                </div>
              ) : null}
            </div>
          </div>
        </header>

        {topologyError ? (
          <div className="topology-error-rail" role="alert" aria-live="assertive">
            <AlertTriangle size={16} />
            <span><strong>Relationship refresh failed.</strong> {topologyError.message}{topologyError.hasPrevious ? " The last successful graph is still shown." : ""}</span>
            <button onClick={() => setRetryNonce((value) => value + 1)}>Retry</button>
            {topologyError.hasPrevious ? <button onClick={revertGraph}>Revert controls</button> : null}
          </div>
        ) : null}

        {topology ? (
          <CytoscapeResourceGraph graph={topology} estate={estate} theme={theme} selectedNodeId={selectedNodeId} expandedAggregateId={expandedAggregateId} motionEnabled={active && motionEnabled && !motionReduced} camera={camera} onActivate={activateGraphItem} />
        ) : topologyError ? (
          <div className="graph-empty-state" role="status"><AlertTriangle size={24} /><strong>No relationship graph is available</strong><span>Retry the request or return to the estate after checking the stored snapshot.</span></div>
        ) : (
          <div className="graph-empty-state" role="status"><img src={RESOURCE_GROUP_ICON} alt="" /><strong>Building the relationship graph…</strong></div>
        )}

        <footer className="topology-status-rail" aria-label="Relationship graph status and legend">
          <div className="topology-counts" role="status">
            <Activity size={14} />
            {counts ? (
              <span><strong>{counts.total}</strong> {graphUnit} represented{counts.folded > 0 ? <> · {counts.folded} folded</> : null}{counts.aggregated > 0 ? <> · {counts.aggregated} aggregated</> : null}{counts.external > 0 ? <> · {counts.external} external</> : null}{counts.hiddenByFilter > 0 ? <> · <strong>{counts.hiddenByFilter} filtered</strong></> : null}</span>
            ) : <span>Building graph…</span>}
            <i />
            <span><strong>{counts?.totalLinks ?? 0}</strong> source relationships · <strong>{counts?.drawnLinks ?? 0}</strong> connectors</span>
            {topologyStale ? <b className="topology-stale">Stale graph</b> : null}
          </div>
          <button className="topology-legend-toggle" onClick={() => setLegendOpen((current) => !current)} aria-expanded={legendOpen} aria-controls="topology-kind-legend">Kinds <ChevronDown size={13} /></button>
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

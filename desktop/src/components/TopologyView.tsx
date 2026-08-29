import { useEffect, useMemo, useRef, useState } from "react";
import {
  Activity,
  AlertTriangle,
  ChevronDown,
  ChevronRight,
  Focus,
  GitBranch,
  HelpCircle,
  Layers3,
  Maximize2,
  Minus,
  MoreHorizontal,
  Pause,
  Play,
  Plus,
  RotateCcw,
  ScanSearch,
} from "lucide-react";
import type { EstateSnapshot, TopologyGraph, TopologyRequest } from "../types";
import { getTopology } from "../api";
import { RESOURCE_GROUP_ICON } from "../azure-icons";
import {
  CytoscapeResourceGraph,
  kindClassColor,
  type GraphActivation,
  type GraphCameraMode,
  type GraphCameraRequest,
  type GraphMode,
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

interface ViewControls {
  mode: GraphMode;
  activeResourceGroupId?: string;
  depth: 1 | 2;
  excludedClasses: string[];
  expandedOverride?: string[];
  showUnconnected: boolean;
  selectedResourceId?: string;
}

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
  selectedResourceId,
  focusRequestNonce,
  onSelectResource,
  onInspect,
}: {
  estate: EstateSnapshot;
  theme: "light" | "dark";
  selectedResourceId?: string;
  focusRequestNonce: number;
  onSelectResource: (id: string) => void;
  onInspect: (id: string) => void;
}) {
  const selected = estate.resources.find((resource) => resource.id === selectedResourceId);
  const [mode, setMode] = useState<GraphMode>("estate");
  const [motionEnabled, setMotionEnabled] = useState(true);
  const [motionReduced, setMotionReduced] = useState(false);
  const [depth, setDepth] = useState<1 | 2>(1);
  const [excludedClasses, setExcludedClasses] = useState<string[]>([]);
  const [expandedOverride, setExpandedOverride] = useState<string[]>();
  const [showUnconnected, setShowUnconnected] = useState(true);
  const [activeResourceGroupId, setActiveResourceGroupId] = useState<string>();
  const [expandedAggregateId, setExpandedAggregateId] = useState<string>();
  const [toolsOpen, setToolsOpen] = useState(false);
  const [legendOpen, setLegendOpen] = useState(false);
  const [camera, setCamera] = useState<GraphCameraRequest>({ mode: "core", nonce: 0 });
  const [topology, setTopology] = useState<TopologyGraph>();
  const [topologyStale, setTopologyStale] = useState(false);
  const [topologyError, setTopologyError] = useState<TopologyError>();
  const [retryNonce, setRetryNonce] = useState(0);
  const lastFocusRequestRef = useRef(0);
  const requestTokenRef = useRef(0);
  const successfulControlsRef = useRef<ViewControls | undefined>(undefined);

  const resourceGroupTopology = useMemo(() => buildResourceGroupTopology(estate), [estate]);
  const activeResourceGroup = resourceGroupTopology.groups.find((group) => group.id === activeResourceGroupId);
  const selectedNodeId = mode === "neighbourhood" ? selected?.id : undefined;

  useEffect(() => {
    if (mode === "neighbourhood" && !selected) setMode("estate");
  }, [mode, selected]);

  // Keep the previous successful graph visible while a replacement is being
  // built. A failed replacement can then be retried or reverted without
  // losing spatial context.
  useEffect(() => {
    if (mode === "neighbourhood" && !selected) return;
    const controls: ViewControls = {
      mode,
      activeResourceGroupId,
      depth,
      excludedClasses: [...excludedClasses],
      expandedOverride: expandedOverride ? [...expandedOverride] : undefined,
      showUnconnected,
      selectedResourceId: selected?.id,
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
            : { kind: "estate", expandedSubscriptions: expandedOverride ?? [] },
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
    expandedOverride,
    mode,
    retryNonce,
    selected?.id,
    showUnconnected,
  ]);

  useEffect(() => {
    setActiveResourceGroupId((current) =>
      current && resourceGroupTopology.groups.some((group) => group.id === current)
        ? current
        : undefined,
    );
  }, [resourceGroupTopology]);

  useEffect(() => {
    const preference = window.matchMedia("(prefers-reduced-motion: reduce)");
    const update = () => setMotionReduced(preference.matches);
    update();
    preference.addEventListener("change", update);
    return () => preference.removeEventListener("change", update);
  }, []);

  useEffect(() => {
    if (focusRequestNonce === 0 || focusRequestNonce === lastFocusRequestRef.current || !selected) return;
    lastFocusRequestRef.current = focusRequestNonce;
    setMode("neighbourhood");
    setActiveResourceGroupId(undefined);
    setExpandedAggregateId(undefined);
    requestCamera("selection");
  }, [focusRequestNonce, selected]);

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
    setMode("estate");
    setActiveResourceGroupId(id);
    setExpandedAggregateId(undefined);
    setToolsOpen(false);
    requestCamera("core");
  }

  function showResourceGroups() {
    setActiveResourceGroupId(undefined);
    setExpandedAggregateId(undefined);
    requestCamera("core");
  }

  function toggleLane(subscriptionId: string) {
    const drawnExpanded = topology?.lanes
      .filter((lane) => lane.expanded)
      .map((lane) => lane.subscriptionId) ?? [];
    setExpandedOverride((current) => toggleSubscriptionLane(current, drawnExpanded, subscriptionId));
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
    if (activation.kind === "aggregate") {
      const aggregate = topology?.nodes.find((node) => node.id === activation.nodeId);
      if (!aggregate) return;
      const next = resolveAggregateActivation(expandedAggregateId, activation.nodeId, aggregate.memberIds);
      if (next.kind === "open-resource") {
        onInspect(next.resourceId);
        return;
      }
      setExpandedAggregateId(next.expandedId);
      return;
    }
    onInspect(activation.resourceId);
  }

  function toggleClass(kindClass: string) {
    setExcludedClasses((current) => current.includes(kindClass)
      ? current.filter((candidate) => candidate !== kindClass)
      : [...current, kindClass]);
  }

  function setGraphMode(nextMode: GraphMode) {
    if (nextMode === "neighbourhood" && !selected) return;
    setMode(nextMode);
    setExpandedAggregateId(undefined);
    setToolsOpen(false);
    requestCamera(nextMode === "neighbourhood" ? "selection" : "core");
  }

  function handleGraphModeKeyDown(event: React.KeyboardEvent<HTMLDivElement>) {
    if (!["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End"].includes(event.key)) return;
    event.preventDefault();
    const wantsNeighbourhood = event.key === "ArrowRight" || event.key === "ArrowDown" || event.key === "End";
    const nextMode: GraphMode = wantsNeighbourhood && selected ? "neighbourhood" : "estate";
    setGraphMode(nextMode);
    event.currentTarget.querySelector<HTMLButtonElement>(`[data-graph-mode="${nextMode}"]`)?.focus();
  }

  function handleDepthKeyDown(event: React.KeyboardEvent<HTMLDivElement>) {
    if (!["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End"].includes(event.key)) return;
    event.preventDefault();
    const nextDepth = event.key === "ArrowLeft" || event.key === "ArrowUp" || event.key === "Home" ? 1 : 2;
    setDepth(nextDepth);
    event.currentTarget.querySelector<HTMLButtonElement>(`[data-depth="${nextDepth}"]`)?.focus();
  }

  function revertGraph() {
    const controls = successfulControlsRef.current;
    if (!controls) return;
    setMode(controls.mode);
    setActiveResourceGroupId(controls.activeResourceGroupId);
    setDepth(controls.depth);
    setExcludedClasses([...controls.excludedClasses]);
    setExpandedOverride(controls.expandedOverride ? [...controls.expandedOverride] : undefined);
    setShowUnconnected(controls.showUnconnected);
    if (controls.selectedResourceId) onSelectResource(controls.selectedResourceId);
    setTopologyError(undefined);
    setTopologyStale(false);
    requestCamera("core");
  }

  return (
    <div className="topology-workspace">
      <section className="topology-stage" aria-label={mode === "neighbourhood"
        ? `Relationship neighbourhood for ${selected?.name ?? "no selected resource"}`
        : activeResourceGroup
          ? `Relationship map for resource group ${activeResourceGroup.name}`
          : "Azure estate relationship map"}>
        <header className="topology-commandbar">
          <div className="topology-title">
            <span className="topology-title-icon"><GitBranch size={18} /></span>
            <div>
              <div className="topology-scope-line">
                <strong>Relationships</strong>
                {activeResourceGroup && mode === "estate" ? (
                  <>
                    <ChevronRight size={12} aria-hidden="true" />
                    <button onClick={showResourceGroups}>Estate map</button>
                    <ChevronRight size={12} aria-hidden="true" />
                    <span>{activeResourceGroup.subscriptionName}</span>
                  </>
                ) : null}
              </div>
              <h1>{contextTitle}</h1>
              <span>{contextSubtitle}</span>
            </div>
          </div>

          <div className="graph-mode-switch" role="radiogroup" aria-label="Graph scope" onKeyDown={handleGraphModeKeyDown}>
            <button data-graph-mode="estate" role="radio" aria-checked={mode === "estate"} tabIndex={mode === "estate" ? 0 : -1} className={mode === "estate" ? "active" : ""} onClick={() => setGraphMode("estate")}>
              <Layers3 size={14} /> Estate map
            </button>
            <button data-graph-mode="neighbourhood" role="radio" aria-checked={mode === "neighbourhood"} tabIndex={mode === "neighbourhood" ? 0 : -1} className={mode === "neighbourhood" ? "active" : ""} onClick={() => setGraphMode("neighbourhood")} disabled={!selected} title={!selected ? "Select a resource before opening its neighbourhood" : undefined}>
              <ScanSearch size={14} /> Neighbourhood
            </button>
          </div>

          {mode === "neighbourhood" ? (
            <div className="graph-depth-switch" role="radiogroup" aria-label="Neighbourhood depth" onKeyDown={handleDepthKeyDown}>
              {[1, 2].map((value) => (
                <button key={value} data-depth={value} role="radio" aria-checked={depth === value} tabIndex={depth === value ? 0 : -1} className={depth === value ? "active" : ""} onClick={() => setDepth(value as 1 | 2)}>
                  {value} hop{value === 1 ? "" : "s"}
                </button>
              ))}
            </div>
          ) : null}

          {mode === "estate" && activeResourceGroup ? (
            <button className={showUnconnected ? "topology-inline-toggle active" : "topology-inline-toggle"} aria-pressed={showUnconnected} onClick={() => setShowUnconnected((current) => !current)}>
              Include resources without drawn relationships
            </button>
          ) : null}

          <div className="topology-tools">
            <button aria-label={selectedNodeId ? "Recenter the selected item" : "Recenter the readable core"} onClick={() => requestCamera(selectedNodeId ? "selection" : "core")} title={selectedNodeId ? "Recenter the selected item" : "Recenter the readable core"}>
              <Focus size={15} /><span>Recenter</span>
            </button>
            <div className="topology-more">
              <button className="topology-more-trigger" aria-label="More graph controls" onClick={() => setToolsOpen((current) => !current)} aria-haspopup="menu" aria-expanded={toolsOpen} aria-controls="topology-more-menu">
                <MoreHorizontal size={17} /><span>More</span>
              </button>
              {toolsOpen ? (
                <div id="topology-more-menu" className="topology-more-menu" role="menu">
                  <button role="menuitem" onClick={() => { requestCamera("all"); setToolsOpen(false); }}><Maximize2 size={15} /><span><strong>Fit all</strong><small>Show every represented region</small></span></button>
                  <button role="menuitem" onClick={() => requestCamera("zoom-in")}><Plus size={15} /><span><strong>Zoom in</strong><small>Keyboard: +</small></span></button>
                  <button role="menuitem" onClick={() => requestCamera("zoom-out")}><Minus size={15} /><span><strong>Zoom out</strong><small>Keyboard: −</small></span></button>
                  <button role="menuitem" onClick={() => setMotionEnabled((current) => !current)} disabled={motionReduced}>
                    {motionEnabled && !motionReduced ? <Pause size={15} /> : <Play size={15} />}
                    <span><strong>{motionReduced ? "Motion reduced" : motionEnabled ? "Pause selected path" : "Play selected path"}</strong><small>{motionReduced ? "Uses your system preference" : "Only the selected path animates"}</small></span>
                  </button>
                  {expandedOverride ? <button role="menuitem" onClick={() => { setExpandedOverride(undefined); setToolsOpen(false); }}><RotateCcw size={15} /><span><strong>Reset subscription lanes</strong><small>Restore the snapshot default</small></span></button> : null}
                  <div className="topology-help" role="note"><HelpCircle size={15} /><p><strong>Graph controls</strong><span>Click or press Enter to open an item. Aggregate tiles expand in place. Arrow keys move spatially; drag to pan; scroll to zoom; 0 recentres.</span></p></div>
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
          <CytoscapeResourceGraph graph={topology} estate={estate} theme={theme} selectedNodeId={selectedNodeId} expandedAggregateId={expandedAggregateId} motionEnabled={motionEnabled && !motionReduced} camera={camera} onActivate={activateGraphItem} />
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

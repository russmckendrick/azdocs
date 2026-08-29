import { useEffect, useMemo, useRef, useState } from "react";
import {
  Activity,
  ArrowDownLeft,
  ArrowUpRight,
  ChevronRight,
  ExternalLink,
  Focus,
  GitBranch,
  Layers3,
  Pause,
  Play,
  ScanSearch,
  X,
} from "lucide-react";
import type { EstateSnapshot, TopologyGraph, TopologyNode, TopologyRequest } from "../types";
import { getTopology } from "../api";
import { ALL_RESOURCES_ICON, RESOURCE_GROUP_ICON } from "../azure-icons";
import { CytoscapeResourceGraph, kindClassColor, type GraphMode } from "./CytoscapeResourceGraph";
import {
  buildResourceGroupTopology,
  resourceGroupNodeId,
  resourcesInGroup,
  type ResourceGroupSummary,
} from "./topology-model";

const ALL_KIND_CLASSES = ["network", "structure", "data", "identity", "monitoring"] as const;

function relationLabel(kind: string) {
  return kind.replaceAll("_", " ");
}

function stableCompare(left: string, right: string) {
  return left < right ? -1 : left > right ? 1 : 0;
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
  const selected = estate.resources.find((resource) => resource.id === selectedResourceId) ?? estate.resources[0];
  const [mode, setMode] = useState<GraphMode>("estate");
  const [motionEnabled, setMotionEnabled] = useState(true);
  const [focusNonce, setFocusNonce] = useState(0);
  const [selectedResourceGroupId, setSelectedResourceGroupId] = useState<string>();
  const [activeResourceGroupId, setActiveResourceGroupId] = useState<string>();
  const [inspectorOpen, setInspectorOpen] = useState(false);
  const [motionReduced, setMotionReduced] = useState(false);
  const [depth, setDepth] = useState<1 | 2>(1);
  const [excludedClasses, setExcludedClasses] = useState<string[]>([]);
  const [expandedOverride, setExpandedOverride] = useState<string[]>();
  const [showUnconnected, setShowUnconnected] = useState(true);
  const [topology, setTopology] = useState<TopologyGraph>();
  const [topologyStale, setTopologyStale] = useState(false);
  const [aggregateNodeId, setAggregateNodeId] = useState<string>();
  const detailsTriggerRef = useRef<HTMLButtonElement>(null);
  const closeButtonRef = useRef<HTMLButtonElement>(null);
  const returnFocusRef = useRef<HTMLElement | undefined>(undefined);
  const lastFocusRequestRef = useRef(0);
  const requestTokenRef = useRef(0);
  const typeMap = useMemo(() => new Map(estate.resourceTypes.map((type) => [type.azureType, type])), [estate.resourceTypes]);
  const resourceGroupTopology = useMemo(() => buildResourceGroupTopology(estate), [estate]);
  const selectedResourceGroup = resourceGroupTopology.groups.find(
    (group) => group.id === selectedResourceGroupId,
  ) ?? resourceGroupTopology.groups[0];
  const activeResourceGroup = resourceGroupTopology.groups.find(
    (group) => group.id === activeResourceGroupId,
  );
  const selectedResourceGroupForResource = selected
    ? resourceGroupTopology.resourceGroupByResourceId.get(selected.id)
    : undefined;
  const selectedIsInActiveGroup = Boolean(
    selected && activeResourceGroup?.resourceIds.includes(selected.id),
  );
  const aggregateNode = aggregateNodeId
    ? topology?.nodes.find((node) => node.id === aggregateNodeId)
    : undefined;
  const groupInspectorVisible = !aggregateNode && mode === "estate" && (!activeResourceGroup || !selectedIsInActiveGroup);
  const inspectorResourceGroup = activeResourceGroup ?? selectedResourceGroup;
  const relationships = useMemo(() => {
    if (!selected) return [];
    return estate.edges
      .filter((edge) => edge.sourceId === selected.id || edge.targetId === selected.id)
      .map((edge) => {
        const outbound = edge.sourceId === selected.id;
        const resource = estate.resources.find((candidate) => candidate.id === (outbound ? edge.targetId : edge.sourceId));
        return { edge, outbound, resource };
      });
  }, [estate.edges, estate.resources, selected]);

  // Fetch the graph whenever anything that shapes it changes. The previous
  // graph stays on screen (marked stale) so the canvas never flashes empty.
  useEffect(() => {
    if (!selected) return;
    const request: TopologyRequest = {
      snapshotId: estate.id,
      mode:
        mode === "neighbourhood"
          ? {
              kind: "neighbourhood",
              resourceId: selected.id,
              depth,
              kindClasses:
                excludedClasses.length > 0
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
    setTopologyStale(true);
    getTopology(request)
      .then((graph) => {
        if (requestTokenRef.current !== token) return;
        setTopology(graph);
        setTopologyStale(false);
      })
      .catch(() => {
        if (requestTokenRef.current !== token) return;
        setTopologyStale(false);
      });
  }, [
    activeResourceGroupId,
    depth,
    estate.id,
    excludedClasses,
    expandedOverride,
    mode,
    selected,
    showUnconnected,
  ]);

  useEffect(() => {
    setAggregateNodeId(undefined);
  }, [mode, activeResourceGroupId, selectedResourceId]);

  useEffect(() => {
    setSelectedResourceGroupId((current) => {
      if (current && resourceGroupTopology.groups.some((group) => group.id === current)) return current;
      return selectedResourceGroupForResource ?? resourceGroupTopology.groups[0]?.id;
    });
    setActiveResourceGroupId((current) =>
      current && resourceGroupTopology.groups.some((group) => group.id === current)
        ? current
        : undefined,
    );
  }, [resourceGroupTopology, selectedResourceGroupForResource]);

  useEffect(() => {
    const preference = window.matchMedia("(prefers-reduced-motion: reduce)");
    const update = () => setMotionReduced(preference.matches);
    update();
    preference.addEventListener("change", update);
    return () => preference.removeEventListener("change", update);
  }, []);

  useEffect(() => {
    if (focusRequestNonce === 0 || focusRequestNonce === lastFocusRequestRef.current) return;
    lastFocusRequestRef.current = focusRequestNonce;
    if (mode === "estate" && selectedResourceGroupForResource) {
      setSelectedResourceGroupId(selectedResourceGroupForResource);
      setActiveResourceGroupId(selectedResourceGroupForResource);
    }
    rememberReturnFocus();
    setInspectorOpen(true);
    setFocusNonce((value) => value + 1);
  }, [focusRequestNonce, mode, selectedResourceGroupForResource]);

  useEffect(() => {
    if (!inspectorOpen) return;
    const frame = window.requestAnimationFrame(() => {
      const closeButton = closeButtonRef.current;
      if (closeButton && window.getComputedStyle(closeButton).display !== "none") closeButton.focus();
    });
    function handleEscape(event: KeyboardEvent) {
      if (event.key !== "Escape") return;
      event.preventDefault();
      closeInspector();
    }
    window.addEventListener("keydown", handleEscape);
    return () => {
      window.cancelAnimationFrame(frame);
      window.removeEventListener("keydown", handleEscape);
    };
  }, [inspectorOpen]);

  if (!selected) return null;
  const selectedType = typeMap.get(selected.azureType);
  const focusedNodeId = mode === "estate" && !activeResourceGroup
    ? (selectedResourceGroup ? resourceGroupNodeId(selectedResourceGroup.id) : "")
    : selected.id;
  const counts = topology?.counts;
  const graphUnit = topology?.level === "estate" ? "groups" : "resources";

  function selectGraphResource(id: string) {
    rememberReturnFocus();
    onSelectResource(id);
    setInspectorOpen(false);
  }

  function selectInspectorResource(id: string) {
    onSelectResource(id);
    setInspectorOpen(true);
  }

  function selectResourceGroup(id: string) {
    setSelectedResourceGroupId(id);
  }

  function openResourceGroup(id: string) {
    const group = resourceGroupTopology.groups.find((candidate) => candidate.id === id);
    if (!group) return;
    setSelectedResourceGroupId(id);
    setActiveResourceGroupId(id);
    setInspectorOpen(false);
    if (!selected || !group.resourceIds.includes(selected.id)) {
      const nextResource = resourcesInGroup(estate, group)
        .sort((left, right) =>
          right.edgeCount - left.edgeCount
            || right.findingCount - left.findingCount
            || stableCompare(left.name, right.name)
            || stableCompare(left.id, right.id),
        )[0];
      if (nextResource) onSelectResource(nextResource.id);
    }
  }

  function showResourceGroups() {
    setActiveResourceGroupId(undefined);
    setInspectorOpen(false);
  }

  function expandLane(subscriptionId: string) {
    const alreadyExpanded = topology?.lanes
      .filter((lane) => lane.expanded)
      .map((lane) => lane.subscriptionId) ?? [];
    setExpandedOverride([...new Set([...alreadyExpanded, subscriptionId])]);
  }

  function toggleClass(kindClass: string) {
    setExcludedClasses((current) =>
      current.includes(kindClass)
        ? current.filter((candidate) => candidate !== kindClass)
        : [...current, kindClass],
    );
  }

  function rememberReturnFocus(candidate?: HTMLElement) {
    const active = document.activeElement instanceof HTMLElement ? document.activeElement : undefined;
    returnFocusRef.current = candidate ?? active ?? detailsTriggerRef.current ?? undefined;
  }

  function openInspector(trigger: HTMLButtonElement) {
    rememberReturnFocus(trigger);
    setInspectorOpen(true);
  }

  function closeInspector() {
    const returnTarget = returnFocusRef.current ?? detailsTriggerRef.current;
    setInspectorOpen(false);
    setAggregateNodeId(undefined);
    window.requestAnimationFrame(() => returnTarget?.focus());
  }

  function handleGraphModeKeyDown(event: React.KeyboardEvent<HTMLDivElement>) {
    if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
    event.preventDefault();
    const nextMode: GraphMode = event.key === "ArrowLeft" || event.key === "Home" ? "neighbourhood" : "estate";
    setMode(nextMode);
    event.currentTarget.querySelector<HTMLButtonElement>(`[data-graph-mode="${nextMode}"]`)?.focus();
  }

  const classCounts = new Map((topology?.kindClasses ?? []).map((entry) => [entry.class, entry.count]));

  return (
    <div className="topology-workspace">
      <section
        className="topology-stage"
        aria-label={mode === "estate" && !activeResourceGroup
          ? "Interactive Azure estate map grouped by subscription and resource group"
          : `Interactive relationship graph centered on ${selected.name}`}
      >
        <div className="topology-overlays">
        <header className="topology-commandbar">
          <div className="topology-title">
            <span className="topology-title-icon"><GitBranch size={18} /></span>
            <div>
              <h1>{mode === "estate" && !activeResourceGroup ? "Estate map" : activeResourceGroup && mode === "estate" ? activeResourceGroup.name : "Neighbourhood"}</h1>
              <span>{mode === "estate" && !activeResourceGroup
                ? `${resourceGroupTopology.groups.length} groups · ${estate.resources.length.toLocaleString()} resources`
                : activeResourceGroup && mode === "estate"
                  ? `${activeResourceGroup.subscriptionName} · ${activeResourceGroup.resourceCount} resources`
                  : `${selected.name} · ${counts ? `${counts.total} in reach` : "…"}`}</span>
            </div>
          </div>
          {mode === "neighbourhood" ? (
            <div className="graph-depth-switch" role="radiogroup" aria-label="Neighbourhood depth">
              {[1, 2].map((value) => (
                <button
                  key={value}
                  role="radio"
                  aria-checked={depth === value}
                  className={depth === value ? "active" : ""}
                  onClick={() => setDepth(value as 1 | 2)}
                >
                  {value} hop{value === 1 ? "" : "s"}
                </button>
              ))}
            </div>
          ) : null}
          <div className="graph-mode-switch" role="radiogroup" aria-label="Graph scope" onKeyDown={handleGraphModeKeyDown}>
            <button data-graph-mode="neighbourhood" role="radio" aria-checked={mode === "neighbourhood"} tabIndex={mode === "neighbourhood" ? 0 : -1} className={mode === "neighbourhood" ? "active" : ""} onClick={() => setMode("neighbourhood")}>
              <ScanSearch size={14} /> Neighbourhood
            </button>
            <button data-graph-mode="estate" role="radio" aria-checked={mode === "estate"} tabIndex={mode === "estate" ? 0 : -1} className={mode === "estate" ? "active" : ""} onClick={() => setMode("estate")}>
              <Layers3 size={14} /> Estate map
            </button>
          </div>
          <div className="topology-tools">
            <button onClick={() => setMotionEnabled((current) => !current)} aria-pressed={motionEnabled && !motionReduced} disabled={motionReduced} title={motionReduced ? "Motion is disabled by your operating-system preference" : undefined}>
              {motionEnabled && !motionReduced ? <Pause size={14} /> : <Play size={14} />}
              {motionReduced ? "Motion reduced" : motionEnabled ? "Pause flow" : "Play flow"}
            </button>
            <button onClick={() => setFocusNonce((value) => value + 1)}>
              <Focus size={14} /> Fit selection
            </button>
            <button
              ref={detailsTriggerRef}
              className="mobile-inspector-toggle"
              onClick={(event) => openInspector(event.currentTarget)}
              aria-expanded={inspectorOpen}
              aria-controls="topology-inspector"
            >
              {groupInspectorVisible
                ? <img src={RESOURCE_GROUP_ICON} alt="" />
                : selectedType
                  ? <img src={selectedType.icon} alt="" />
                  : <GitBranch size={14} />} Details
            </button>
          </div>
        </header>

        {mode === "estate" && activeResourceGroup ? (
          <nav className="topology-breadcrumb" aria-label="Topology scope">
            <button onClick={showResourceGroups}><Layers3 size={13} /> Resource groups</button>
            <ChevronRight size={13} aria-hidden="true" />
            <span>{activeResourceGroup.subscriptionName}</span>
            <ChevronRight size={13} aria-hidden="true" />
            <strong>{activeResourceGroup.name}</strong>
          </nav>
        ) : null}

        {mode === "neighbourhood" ? (
          <div className="topology-kind-chips" role="group" aria-label="Relationship kinds">
            {ALL_KIND_CLASSES.map((kindClass) => {
              const active = !excludedClasses.includes(kindClass);
              const count = classCounts.get(kindClass);
              return (
                <button
                  key={kindClass}
                  className={active ? "kind-chip active" : "kind-chip"}
                  aria-pressed={active}
                  onClick={() => toggleClass(kindClass)}
                >
                  <i style={{ background: kindClassColor(kindClass) }} />
                  {kindClass}
                  {count !== undefined ? <b>{count}</b> : null}
                </button>
              );
            })}
          </div>
        ) : null}
        {mode === "estate" && activeResourceGroup ? (
          <div className="topology-kind-chips" role="group" aria-label="Group view options">
            <button
              className={showUnconnected ? "kind-chip active" : "kind-chip"}
              aria-pressed={showUnconnected}
              onClick={() => setShowUnconnected((current) => !current)}
            >
              Unlinked shelf
            </button>
            {expandedOverride ? null : null}
          </div>
        ) : null}
        {mode === "estate" && !activeResourceGroup && expandedOverride ? (
          <div className="topology-kind-chips" role="group" aria-label="Lane options">
            <button className="kind-chip" onClick={() => setExpandedOverride(undefined)}>
              Reset lanes
            </button>
          </div>
        ) : null}
        </div>

        {topology ? (
          <CytoscapeResourceGraph
            graph={topology}
            estate={estate}
            theme={theme}
            focusedNodeId={focusedNodeId}
            motionEnabled={motionEnabled}
            focusNonce={focusNonce}
            onSelectResource={selectGraphResource}
            onSelectResourceGroup={selectResourceGroup}
            onOpenResourceGroup={openResourceGroup}
            onExpandLane={expandLane}
            onSelectAggregate={(nodeId) => {
              setAggregateNodeId(nodeId);
              setInspectorOpen(true);
            }}
          />
        ) : (
          <div className="graph-empty-state" role="status">
            <img src={RESOURCE_GROUP_ICON} alt="" />
            <strong>Building the relationship graph…</strong>
          </div>
        )}

        <div className="graph-snapshot-chip" role="status">
          <Activity size={13} />
          {counts ? (
            <span>
              All <strong>{counts.total}</strong> {graphUnit} represented
              {counts.folded > 0 ? <> · {counts.folded} folded</> : null}
              {counts.aggregated > 0 ? <> · {counts.aggregated} aggregated</> : null}
              {counts.external > 0 ? <> · {counts.external} neighbours in other groups</> : null}
              {counts.hiddenByFilter > 0 ? <> · <strong>{counts.hiddenByFilter} hidden by filters</strong></> : null}
            </span>
          ) : (
            <span>Building…</span>
          )}
          <i />
          <span><strong>{counts?.drawnLinks ?? 0}</strong> of {counts?.totalLinks ?? 0} links drawn</span>
          <i />
          <span>Snapshot {estate.id.slice(0, 8)}{topologyStale ? " · updating…" : ""}</span>
        </div>
      </section>

      {aggregateNode ? (
        <AggregateInspector
          node={aggregateNode}
          estate={estate}
          open={inspectorOpen}
          typeMap={typeMap}
          closeButtonRef={closeButtonRef}
          onClose={closeInspector}
          onSelect={selectInspectorResource}
        />
      ) : groupInspectorVisible && inspectorResourceGroup ? (
        <ResourceGroupInspector
          group={inspectorResourceGroup}
          estate={estate}
          open={inspectorOpen}
          active={activeResourceGroup?.id === inspectorResourceGroup.id}
          typeMap={typeMap}
          closeButtonRef={closeButtonRef}
          onClose={closeInspector}
          onOpen={openResourceGroup}
        />
      ) : (
        <aside
          id="topology-inspector"
          className={inspectorOpen ? "topology-inspector open" : "topology-inspector"}
          aria-labelledby="topology-inspector-title"
        >
          <div className="topology-inspector-glow" style={{ "--service-color": selectedType?.color ?? "#45b6fe" } as React.CSSProperties} />
          <header className="topology-resource-hero">
            <button ref={closeButtonRef} className="topology-inspector-close" onClick={closeInspector} aria-label="Close relationship details"><X size={17} /></button>
            <div className="topology-resource-icon">
              <img src={selectedType?.icon ?? ALL_RESOURCES_ICON} alt="" />
            </div>
            <div>
              <span className="inspector-eyebrow">{selectedType?.displayName ?? selected.azureType}</span>
              <h2 id="topology-inspector-title">{selected.name}</h2>
              <p>{selected.resourceGroup} · {selected.location ?? "global"}</p>
            </div>
          </header>

          <div className="topology-resource-status">
            <span className="status-live"><i /> Stored resource</span>
            {selected.findingCount > 0 ? <span className="status-risk">{selected.findingCount} finding{selected.findingCount === 1 ? "" : "s"}</span> : <span className="status-clear">No findings</span>}
          </div>

          <button className="inspect-resource" onClick={() => onInspect(selected.id)}>
            <span><strong>Open resource record</strong><small>Properties, tags, findings and raw evidence</small></span>
            <ExternalLink size={16} />
          </button>

          <section className="relationship-section">
            <div className="relationship-heading">
              <div><small>Direct connections</small><strong>{relationships.length}</strong></div>
              <span>Derived offline</span>
            </div>
            <div className="relationship-list">
              {relationships.length === 0 ? (
                <div className="relationship-empty"><GitBranch size={20} /><span>No derived relationships for this resource.</span></div>
              ) : relationships.map(({ edge, outbound, resource }) => {
                const type = resource ? typeMap.get(resource.azureType) : undefined;
                return (
                  <button key={`${edge.sourceId}-${edge.targetId}-${edge.kind}`} onClick={() => resource && selectInspectorResource(resource.id)}>
                    <span className="relationship-direction" title={outbound ? "Outbound relationship" : "Inbound relationship"}>
                      {outbound ? <ArrowUpRight size={13} /> : <ArrowDownLeft size={13} />}
                    </span>
                    <span className="relationship-resource-icon"><img src={type?.icon ?? ALL_RESOURCES_ICON} alt="" /></span>
                    <span className="relationship-copy">
                      <strong>{resource?.name ?? "Unknown resource"}</strong>
                      <small>{relationLabel(edge.kind)}</small>
                    </span>
                    <span className="relationship-type">{type?.displayName ?? "Resource"}</span>
                  </button>
                );
              })}
            </div>
          </section>

          <footer className="topology-legend">
            <div><i className="legend-flow" /><span>Arrow shows direction; colours group relationship kinds</span></div>
            <p>Relationships are deterministic post-passes over the selected SQLite snapshot. No live Azure calls are made here.</p>
          </footer>
        </aside>
      )}
    </div>
  );
}

function AggregateInspector({
  node,
  estate,
  open,
  typeMap,
  closeButtonRef,
  onClose,
  onSelect,
}: {
  node: TopologyNode;
  estate: EstateSnapshot;
  open: boolean;
  typeMap: Map<string, EstateSnapshot["resourceTypes"][number]>;
  closeButtonRef: React.RefObject<HTMLButtonElement | null>;
  onClose: () => void;
  onSelect: (id: string) => void;
}) {
  const type = node.azureType ? typeMap.get(node.azureType) : undefined;
  const members = node.memberIds
    .map((id) => estate.resources.find((resource) => resource.id === id))
    .filter((resource): resource is NonNullable<typeof resource> => Boolean(resource))
    .sort((left, right) => stableCompare(left.name, right.name));
  return (
    <aside
      id="topology-inspector"
      className={open ? "topology-inspector topology-group-inspector open" : "topology-inspector topology-group-inspector"}
      aria-labelledby="topology-inspector-title"
    >
      <div className="topology-inspector-glow" style={{ "--service-color": type?.color ?? "#47c8ff" } as React.CSSProperties} />
      <header className="topology-resource-hero topology-group-hero">
        <button ref={closeButtonRef} className="topology-inspector-close" onClick={onClose} aria-label="Close aggregated tile details"><X size={17} /></button>
        <div className="topology-resource-icon"><img src={type?.icon ?? ALL_RESOURCES_ICON} alt="" /></div>
        <div>
          <span className="inspector-eyebrow">Aggregated tile</span>
          <h2 id="topology-inspector-title">{type?.displayName ?? node.name}</h2>
          <p>{node.count} resources drawn as one tile</p>
        </div>
      </header>

      <section className="relationship-section">
        <div className="relationship-heading">
          <div><small>Members</small><strong>{members.length}</strong></div>
          <span>Select one to focus it</span>
        </div>
        <div className="relationship-list">
          {members.map((resource) => (
            <button key={resource.id} onClick={() => onSelect(resource.id)}>
              <span className="relationship-resource-icon">
                <img src={type?.icon ?? ALL_RESOURCES_ICON} alt="" />
              </span>
              <span className="relationship-copy">
                <strong>{resource.name}</strong>
                <small>{resource.location ?? "global"}{resource.findingCount > 0 ? ` · ${resource.findingCount} findings` : ""}</small>
              </span>
            </button>
          ))}
        </div>
      </section>

      <footer className="topology-legend">
        <p>These resources have no drawn relationships yet, so they share one tile instead of being hidden.</p>
      </footer>
    </aside>
  );
}

function ResourceGroupInspector({
  group,
  estate,
  open,
  active,
  typeMap,
  closeButtonRef,
  onClose,
  onOpen,
}: {
  group: ResourceGroupSummary;
  estate: EstateSnapshot;
  open: boolean;
  active: boolean;
  typeMap: Map<string, EstateSnapshot["resourceTypes"][number]>;
  closeButtonRef: React.RefObject<HTMLButtonElement | null>;
  onClose: () => void;
  onOpen: (id: string) => void;
}) {
  return (
    <aside
      id="topology-inspector"
      className={open ? "topology-inspector topology-group-inspector open" : "topology-inspector topology-group-inspector"}
      aria-labelledby="topology-inspector-title"
    >
      <div className="topology-inspector-glow" style={{ "--service-color": "#47c8ff" } as React.CSSProperties} />
      <header className="topology-resource-hero topology-group-hero">
        <button ref={closeButtonRef} className="topology-inspector-close" onClick={onClose} aria-label="Close resource group details"><X size={17} /></button>
        <div className="topology-resource-icon"><img src={RESOURCE_GROUP_ICON} alt="" /></div>
        <div>
          <h2 id="topology-inspector-title">{group.name}</h2>
          <p>Resource group · {group.subscriptionName}</p>
        </div>
      </header>

      <div className="topology-resource-status">
        <span className="status-live"><i /> {group.resourceCount} resource{group.resourceCount === 1 ? "" : "s"}</span>
        {group.findingCount > 0
          ? <span className="status-risk">{group.findingCount} finding{group.findingCount === 1 ? "" : "s"}</span>
          : <span className="status-clear">No findings</span>}
      </div>

      <button className="inspect-resource" onClick={() => onOpen(group.id)} disabled={active && group.resourceCount === 0}>
        <span>
          <strong>{active ? "Resource group map" : "Open resource group"}</strong>
          <small>{active ? "This group contains no stored resources" : "Explore contained resources and internal links"}</small>
        </span>
        <ChevronRight size={16} />
      </button>

      <div className="topology-group-metrics" aria-label="Resource group totals">
        <span><small>Location</small><strong>{group.location ?? "global"}</strong></span>
        <span><small>Internal links</small><strong>{group.internalLinkCount}</strong></span>
        <span><small>Connected groups</small><strong>{group.connectedGroupCount}</strong></span>
        <span><small>Cross-group links</small><strong>{group.externalLinkCount}</strong></span>
      </div>

      <section className="relationship-section">
        <div className="relationship-heading">
          <div><small>Resource types</small><strong>{group.resourceTypes.length}</strong></div>
          <span>{group.resourceCount} total</span>
        </div>
        <div className="topology-group-type-list">
          {group.resourceTypes.length === 0 ? (
            <div className="relationship-empty"><img src={ALL_RESOURCES_ICON} alt="" /><span>No resource records are stored in this group.</span></div>
          ) : group.resourceTypes.map(({ azureType, count }) => {
            const type = typeMap.get(azureType);
            return (
              <div key={azureType}>
                <span className="relationship-resource-icon">
                  <img src={type?.icon ?? ALL_RESOURCES_ICON} alt="" />
                </span>
                <span><strong>{type?.displayName ?? azureType}</strong><small>{azureType}</small></span>
                <b>{count}</b>
              </div>
            );
          })}
        </div>
      </section>

      <footer className="topology-legend">
        <div><i className="legend-flow" /><span>Group connectors aggregate cross-group resource links</span></div>
        <p>{estate.resources.length.toLocaleString()} resources are organised into {estate.resourceGroups.length} stored resource groups. Select a group, then open it to drill into resource relationships.</p>
      </footer>
    </aside>
  );
}

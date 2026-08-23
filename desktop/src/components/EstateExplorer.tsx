import { useMemo, useRef, useState } from "react";
import {
  AlertTriangle,
  ArrowDownAZ,
  Braces,
  ChevronDown,
  ChevronRight,
  CircleDot,
  ExternalLink,
  GitBranch,
  Layers3,
  MapPin,
  Network,
  SearchX,
  Server,
  Tags,
  X,
} from "lucide-react";
import type { EstateSnapshot, Resource, ResourceType, ScopeSelection } from "../types";

type SortKey = "name" | "type" | "location" | "findings";
type InspectorTab = "overview" | "properties" | "relationships";

interface EstateExplorerProps {
  estate: EstateSnapshot;
  search: string;
  scope: ScopeSelection;
  onScopeChange: (scope: ScopeSelection) => void;
  selectedResourceId?: string;
  onSelectResource: (id?: string) => void;
  onOpenTopology: (id: string) => void;
  onOpenFinding: () => void;
}

function includesSearch(resource: Resource, search: string) {
  if (!search.trim()) return true;
  const value = search.toLowerCase();
  return [
    resource.name,
    resource.azureType,
    resource.location,
    resource.resourceGroup,
    resource.subscriptionId,
    JSON.stringify(resource.tags ?? {}),
  ].some((candidate) => candidate?.toLowerCase().includes(value));
}

function prettyRelation(kind: string) {
  return kind.replaceAll("_", " ");
}

export function EstateExplorer({
  estate,
  search,
  scope,
  onScopeChange,
  selectedResourceId,
  onSelectResource,
  onOpenTopology,
  onOpenFinding,
}: EstateExplorerProps) {
  const [typeFilter, setTypeFilter] = useState("");
  const [locationFilter, setLocationFilter] = useState("");
  const [sortKey, setSortKey] = useState<SortKey>("name");
  const [inspectorTab, setInspectorTab] = useState<InspectorTab>("overview");
  const resourceListRef = useRef<HTMLDivElement>(null);

  const resourceTypeMap = useMemo(
    () => new Map(estate.resourceTypes.map((item) => [item.azureType, item])),
    [estate.resourceTypes],
  );
  const subscriptionMap = useMemo(
    () => new Map(estate.subscriptions.map((item) => [item.id, item.displayName])),
    [estate.subscriptions],
  );

  const filtered = useMemo(() => {
    const matches = estate.resources.filter((resource) => {
      const inSubscription = !scope.subscriptionId || resource.subscriptionId === scope.subscriptionId;
      const inGroup = !scope.resourceGroup || resource.resourceGroup === scope.resourceGroup;
      const hasType = !typeFilter || resource.azureType === typeFilter;
      const inLocation = !locationFilter || resource.location === locationFilter;
      return inSubscription && inGroup && hasType && inLocation && includesSearch(resource, search);
    });
    return matches.sort((a, b) => {
      if (sortKey === "findings") return b.findingCount - a.findingCount || a.name.localeCompare(b.name);
      if (sortKey === "type") return a.azureType.localeCompare(b.azureType) || a.name.localeCompare(b.name);
      if (sortKey === "location") return (a.location ?? "").localeCompare(b.location ?? "") || a.name.localeCompare(b.name);
      return a.name.localeCompare(b.name);
    });
  }, [estate.resources, locationFilter, scope, search, sortKey, typeFilter]);

  const selected = estate.resources.find((resource) => resource.id === selectedResourceId);
  const activeScopeName = scope.resourceGroup
    ? scope.resourceGroup
    : scope.subscriptionId
      ? subscriptionMap.get(scope.subscriptionId)
      : "Entire estate";

  function focusResource(index: number) {
    const resource = filtered[index];
    if (!resource) return;
    onSelectResource(resource.id);
    const option = resourceListRef.current?.querySelector<HTMLButtonElement>(`[data-resource-index="${index}"]`);
    option?.focus();
  }

  function handleResourceListKeyDown(event: React.KeyboardEvent<HTMLDivElement>) {
    if (filtered.length === 0) return;
    const selectedIndex = Math.max(filtered.findIndex((resource) => resource.id === selectedResourceId), 0);
    let nextIndex: number | undefined;
    if (event.key === "ArrowDown") nextIndex = Math.min(selectedIndex + 1, filtered.length - 1);
    if (event.key === "ArrowUp") nextIndex = Math.max(selectedIndex - 1, 0);
    if (event.key === "Home") nextIndex = 0;
    if (event.key === "End") nextIndex = filtered.length - 1;
    if (nextIndex === undefined) return;
    event.preventDefault();
    focusResource(nextIndex);
  }

  return (
    <div className="estate-explorer">
      <aside className="scope-pane" aria-label="Estate hierarchy">
        <div className="pane-heading">
          <div>
            <h1>Azure estate</h1>
            <span>{estate.totals.subscriptions} subscriptions · {estate.totals.resourceGroups} groups</span>
          </div>
          <Layers3 size={18} />
        </div>

        <div className="scope-tree">
          <button
            className={!scope.subscriptionId ? "scope-all active" : "scope-all"}
            onClick={() => onScopeChange({})}
          >
            <span><Network size={16} /> Entire estate</span>
            <b>{estate.totals.resources}</b>
          </button>
          {estate.subscriptions.map((subscription) => {
            const subResources = estate.resources.filter((resource) => resource.subscriptionId === subscription.id);
            const groups = estate.resourceGroups.filter((group) => group.subscriptionId === subscription.id);
            const subActive = scope.subscriptionId === subscription.id && !scope.resourceGroup;
            return (
              <div className="subscription-branch" key={subscription.id}>
                <button
                  className={subActive ? "subscription-row active" : "subscription-row"}
                  onClick={() => onScopeChange({ subscriptionId: subscription.id })}
                >
                  <ChevronDown size={14} />
                  <span>{subscription.displayName}</span>
                  <b>{subResources.length}</b>
                </button>
                <div className="group-branches">
                  {groups.map((group) => {
                    const count = subResources.filter((resource) => resource.resourceGroup === group.name.toLowerCase()).length;
                    const active = scope.subscriptionId === subscription.id && scope.resourceGroup === group.name.toLowerCase();
                    return (
                      <button
                        key={group.id}
                        className={active ? "group-row active" : "group-row"}
                        onClick={() => onScopeChange({ subscriptionId: subscription.id, resourceGroup: group.name.toLowerCase() })}
                      >
                        <ChevronRight size={12} />
                        <span>{group.name}</span>
                        <b>{count}</b>
                      </button>
                    );
                  })}
                </div>
              </div>
            );
          })}
        </div>

        <div className="type-index">
          <div className="minor-heading">
            <span>Resource types</span>
            <button onClick={() => setTypeFilter("")} disabled={!typeFilter}>Clear</button>
          </div>
          <div className="type-index-list">
            {estate.resourceTypes.map((type) => (
              <button
                key={type.azureType}
                className={typeFilter === type.azureType ? "type-index-row active" : "type-index-row"}
                onClick={() => setTypeFilter(typeFilter === type.azureType ? "" : type.azureType)}
              >
                <img src={type.icon} alt="" />
                <span>{type.displayName}</span>
                <b>{type.count}</b>
              </button>
            ))}
          </div>
        </div>
      </aside>

      <section className="resource-pane" aria-label="Resource inventory">
        <header className="resource-header">
          <div>
            <h2>{activeScopeName}</h2>
            <p>
              <strong>{filtered.length}</strong> of {estate.totals.resources} resources
              {search ? <> matching “{search}”</> : null}
            </p>
          </div>
          <div className="estate-pulse" aria-label="Estate summary">
            <span><i className="pulse-dot finding" />{estate.totals.findings} findings</span>
            <span><i className="pulse-dot tag" />{estate.tagCoverage.percent}% tagged</span>
            <span><i className="pulse-dot region" />{estate.locations.length} regions</span>
          </div>
        </header>

        <div className="resource-controls">
          <label>
            <MapPin size={14} />
            <select value={locationFilter} onChange={(event) => setLocationFilter(event.target.value)} aria-label="Filter by location">
              <option value="">All locations</option>
              {estate.locations.map((location) => <option key={location.name} value={location.name}>{location.name} ({location.count})</option>)}
            </select>
          </label>
          <label>
            <ArrowDownAZ size={14} />
            <select value={sortKey} onChange={(event) => setSortKey(event.target.value as SortKey)} aria-label="Sort resources">
              <option value="name">Name</option>
              <option value="type">Resource type</option>
              <option value="location">Location</option>
              <option value="findings">Findings first</option>
            </select>
          </label>
          {(typeFilter || locationFilter || search) ? (
            <button className="clear-filters" onClick={() => { setTypeFilter(""); setLocationFilter(""); }}>
              <X size={13} /> Clear local filters
            </button>
          ) : null}
        </div>

        <div className="resource-table-head" aria-hidden="true">
          <span>Resource</span>
          <span>Resource group</span>
          <span>Location</span>
          <span>Signals</span>
        </div>
        <div ref={resourceListRef} className="resource-list" role="listbox" aria-label="Resources" onKeyDown={handleResourceListKeyDown}>
          {filtered.map((resource, index) => (
            <ResourceRow
              key={resource.id}
              resource={resource}
              type={resourceTypeMap.get(resource.azureType)}
              selected={selectedResourceId === resource.id}
              index={index}
              tabIndex={selectedResourceId === resource.id || (!selectedResourceId && index === 0) ? 0 : -1}
              onSelect={() => onSelectResource(resource.id)}
            />
          ))}
          {!filtered.length ? (
            <div className="no-results">
              <SearchX size={28} />
              <strong>No resources match this view</strong>
              <span>Clear a filter or search the whole snapshot.</span>
            </div>
          ) : null}
        </div>
      </section>

      {selected ? (
        <ResourceInspector
          resource={selected}
          type={resourceTypeMap.get(selected.azureType)}
          estate={estate}
          activeTab={inspectorTab}
          onTabChange={setInspectorTab}
          onClose={() => onSelectResource(undefined)}
          onSelectResource={onSelectResource}
          onOpenTopology={() => onOpenTopology(selected.id)}
          onOpenFinding={onOpenFinding}
        />
      ) : (
        <aside className="inspector-pane inspector-empty">
          <CircleDot size={30} />
          <strong>Select a resource</strong>
          <span>Properties, findings, and derived relationships will appear here.</span>
        </aside>
      )}
    </div>
  );
}

function ResourceRow({
  resource,
  type,
  selected,
  index,
  tabIndex,
  onSelect,
}: {
  resource: Resource;
  type?: ResourceType;
  selected: boolean;
  index: number;
  tabIndex: number;
  onSelect: () => void;
}) {
  return (
    <button className={selected ? "resource-row selected" : "resource-row"} data-resource-index={index} tabIndex={tabIndex} onClick={onSelect} role="option" aria-selected={selected}>
      <span className="resource-identity">
        {type ? <img src={type.icon} alt="" /> : <Server size={25} />}
        <span>
          <strong>{resource.name}</strong>
          <small>{type?.displayName ?? resource.azureType}</small>
        </span>
      </span>
      <span className="resource-group-cell">{resource.resourceGroup ?? "—"}</span>
      <span className="resource-location-cell">{resource.location ?? "global"}</span>
      <span className="signal-cell">
        {resource.findingCount > 0 ? <em className="finding-signal"><AlertTriangle size={13} />{resource.findingCount}</em> : null}
        {resource.edgeCount > 0 ? <em className="edge-signal"><GitBranch size={13} />{resource.edgeCount}</em> : null}
        {!resource.findingCount && !resource.edgeCount ? <small>Quiet</small> : null}
      </span>
    </button>
  );
}

function ResourceInspector({
  resource,
  type,
  estate,
  activeTab,
  onTabChange,
  onClose,
  onSelectResource,
  onOpenTopology,
  onOpenFinding,
}: {
  resource: Resource;
  type?: ResourceType;
  estate: EstateSnapshot;
  activeTab: InspectorTab;
  onTabChange: (tab: InspectorTab) => void;
  onClose: () => void;
  onSelectResource: (id: string) => void;
  onOpenTopology: () => void;
  onOpenFinding: () => void;
}) {
  const relatedEdges = estate.edges.filter((edge) => edge.sourceId === resource.id || edge.targetId === resource.id);
  const relatedFindings = estate.findings.filter((finding) => finding.resourceId === resource.id);
  const tags = Object.entries(resource.tags ?? {});
  const tabs: InspectorTab[] = ["overview", "properties", "relationships"];

  function handleTabKeyDown(event: React.KeyboardEvent<HTMLButtonElement>, tab: InspectorTab) {
    const currentIndex = tabs.indexOf(tab);
    let nextIndex: number | undefined;
    if (event.key === "ArrowRight") nextIndex = (currentIndex + 1) % tabs.length;
    if (event.key === "ArrowLeft") nextIndex = (currentIndex - 1 + tabs.length) % tabs.length;
    if (event.key === "Home") nextIndex = 0;
    if (event.key === "End") nextIndex = tabs.length - 1;
    if (nextIndex === undefined) return;
    event.preventDefault();
    const next = tabs[nextIndex];
    onTabChange(next);
    event.currentTarget.parentElement?.querySelector<HTMLButtonElement>(`#resource-tab-${next}`)?.focus();
  }

  return (
    <aside className="inspector-pane" aria-label={`${resource.name} details`}>
      <header className="inspector-header">
        <button className="inspector-close" onClick={onClose} aria-label="Close resource details"><X size={17} /></button>
        <div className="inspector-resource">
          {type ? <img src={type.icon} alt="" /> : <Server size={30} />}
          <div>
            <h2>{resource.name}</h2>
            <span>{type?.displayName ?? resource.azureType}</span>
          </div>
        </div>
        <div className="inspector-path">
          <span>{resource.subscriptionId}</span><ChevronRight size={12} /><span>{resource.resourceGroup}</span>
        </div>
      </header>

      <div className="inspector-tabs" role="tablist" aria-label="Resource detail sections">
        {tabs.map((tab) => (
          <button
            key={tab}
            role="tab"
            id={`resource-tab-${tab}`}
            aria-selected={activeTab === tab}
            aria-controls={`resource-panel-${tab}`}
            tabIndex={activeTab === tab ? 0 : -1}
            className={activeTab === tab ? "active" : ""}
            onClick={() => onTabChange(tab)}
            onKeyDown={(event) => handleTabKeyDown(event, tab)}
          >
            {tab === "overview" ? "Overview" : tab === "properties" ? "Properties" : `Links ${relatedEdges.length}`}
          </button>
        ))}
      </div>

      <div className="inspector-scroll" role="tabpanel" id={`resource-panel-${activeTab}`} aria-labelledby={`resource-tab-${activeTab}`} tabIndex={0}>
        {activeTab === "overview" ? (
          <>
            {relatedFindings.length ? (
              <button className="resource-warning" onClick={onOpenFinding}>
                <AlertTriangle size={18} />
                <span><strong>{relatedFindings.length} audit {relatedFindings.length === 1 ? "finding" : "findings"}</strong><small>{relatedFindings[0].title}</small></span>
                <ChevronRight size={16} />
              </button>
            ) : (
              <div className="resource-clear"><CircleDot size={16} /><span>No audit findings are linked to this resource.</span></div>
            )}
            <dl className="detail-grid">
              <div><dt>Location</dt><dd>{resource.location ?? "Global"}</dd></div>
              <div><dt>Kind</dt><dd>{resource.kind ?? "Default"}</dd></div>
              <div><dt>Relationships</dt><dd>{relatedEdges.length}</dd></div>
              <div><dt>Tag count</dt><dd>{tags.length}</dd></div>
            </dl>
            <section className="inspector-section">
              <h3><Tags size={15} /> Tags</h3>
              {tags.length ? (
                <dl className="tag-list">
                  {tags.map(([key, value]) => <div key={key}><dt>{key}</dt><dd>{String(value)}</dd></div>)}
                </dl>
              ) : <p className="muted-copy">No tags stored on this resource.</p>}
            </section>
            <section className="inspector-section">
              <h3><Braces size={15} /> ARM identity</h3>
              <code className="arm-id">{resource.displayId}</code>
            </section>
          </>
        ) : null}

        {activeTab === "properties" ? (
          <section className="property-sheet">
            <div className="property-sheet-heading"><span>Stored Resource Graph properties</span><small>Snapshot evidence</small></div>
            <JsonTree value={{ kind: resource.kind, sku: resource.sku, identity: resource.identity, properties: resource.properties }} />
          </section>
        ) : null}

        {activeTab === "relationships" ? (
          <section className="relationship-sheet">
            <button className="topology-action" onClick={onOpenTopology}>
              <GitBranch size={17} /><span><strong>Open topology plate</strong><small>Trace this neighbourhood spatially</small></span><ExternalLink size={15} />
            </button>
            <div className="relationship-list">
              {relatedEdges.map((edge, index) => {
                const otherId = edge.sourceId === resource.id ? edge.targetId : edge.sourceId;
                const other = estate.resources.find((candidate) => candidate.id === otherId);
                const otherType = other ? estate.resourceTypes.find((candidate) => candidate.azureType === other.azureType) : undefined;
                return (
                  <button key={`${edge.kind}-${otherId}-${index}`} onClick={() => other && onSelectResource(other.id)} disabled={!other}>
                    <span className="relation-direction">{edge.sourceId === resource.id ? "OUT" : "IN"}</span>
                    {otherType ? <img src={otherType.icon} alt="" /> : <CircleDot size={22} />}
                    <span><strong>{other?.name ?? otherId.split("/").at(-1)}</strong><small>{prettyRelation(edge.kind)}</small></span>
                    <ChevronRight size={14} />
                  </button>
                );
              })}
              {!relatedEdges.length ? <p className="muted-copy">No derived relationships touch this resource.</p> : null}
            </div>
          </section>
        ) : null}
      </div>
    </aside>
  );
}

function JsonTree({ value }: { value: unknown }) {
  return <pre>{JSON.stringify(value, null, 2)}</pre>;
}

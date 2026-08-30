import { useEffect, useMemo, useRef, useState } from "react";
import {
  AlertTriangle,
  ArrowDownAZ,
  ChevronDown,
  ChevronRight,
  GitBranch,
  Layers3,
  MapPin,
  SearchX,
  X,
} from "lucide-react";
import { ALL_RESOURCES_ICON, RESOURCE_GROUP_ICON, SUBSCRIPTION_ICON } from "../azure-icons";
import { displayLocation } from "../azure-values";
import type { AzureMetadata, EstateSnapshot, Resource, ResourceType, ScopeSelection } from "../types";
import { ShowMore, useProgressiveList } from "./progressive-list";
import { EmptyState } from "./view-chrome";

type SortKey = "name" | "type" | "location" | "findings";

interface EstateExplorerProps {
  estate: EstateSnapshot;
  search: string;
  scope: ScopeSelection;
  onScopeChange: (scope: ScopeSelection) => void;
  onSelectResource: (id: string) => void;
}

function includesSearch(resource: Resource, search: string, metadata: AzureMetadata) {
  if (!search.trim()) return true;
  const value = search.toLowerCase();
  return [
    resource.name,
    resource.azureType,
    resource.location,
    displayLocation(metadata, resource.location),
    resource.resourceGroup,
    resource.subscriptionId,
    JSON.stringify(resource.tags ?? {}),
  ].some((candidate) => candidate?.toLowerCase().includes(value));
}

export function EstateExplorer({
  estate,
  search,
  scope,
  onScopeChange,
  onSelectResource,
}: EstateExplorerProps) {
  const [typeFilter, setTypeFilter] = useState("");
  const [locationFilter, setLocationFilter] = useState("");
  const [sortKey, setSortKey] = useState<SortKey>("name");
  const [expandedSubscriptions, setExpandedSubscriptions] = useState<Set<string>>(
    () => new Set(estate.subscriptions.length === 1 ? [estate.subscriptions[0].id] : []),
  );
  const [expandedGroups, setExpandedGroups] = useState<Set<string>>(() => new Set());
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
      return inSubscription && inGroup && hasType && inLocation && includesSearch(resource, search, estate.azureMetadata);
    });
    return matches.sort((a, b) => {
      if (sortKey === "findings") return b.findingCount - a.findingCount || a.name.localeCompare(b.name);
      if (sortKey === "type") return a.azureType.localeCompare(b.azureType) || a.name.localeCompare(b.name);
      if (sortKey === "location") return (a.location ?? "").localeCompare(b.location ?? "") || a.name.localeCompare(b.name);
      return a.name.localeCompare(b.name);
    });
  }, [estate.azureMetadata, estate.resources, locationFilter, scope, search, sortKey, typeFilter]);
  const list = useProgressiveList(filtered, [locationFilter, scope.resourceGroup, scope.subscriptionId, search, sortKey, typeFilter]);
  const visibleResources = list.visible;

  const activeScopeName = scope.resourceGroup
    ? scope.resourceGroup
    : scope.subscriptionId
      ? subscriptionMap.get(scope.subscriptionId)
      : "Entire estate";

  useEffect(() => {
    const subscriptionId = scope.subscriptionId;
    if (subscriptionId) {
      setExpandedSubscriptions((current) => {
        if (current.has(subscriptionId)) return current;
        const next = new Set(current);
        next.add(subscriptionId);
        return next;
      });
    }
    const resourceGroup = scope.resourceGroup;
    if (!subscriptionId || !resourceGroup) return;
    const group = estate.resourceGroups.find(
      (candidate) => candidate.subscriptionId === subscriptionId && candidate.name.toLowerCase() === resourceGroup,
    );
    if (!group) return;
    setExpandedGroups((current) => {
      if (current.has(group.id)) return current;
      const next = new Set(current);
      next.add(group.id);
      return next;
    });
  }, [estate.resourceGroups, scope.resourceGroup, scope.subscriptionId]);

  function toggleExpanded(setter: React.Dispatch<React.SetStateAction<Set<string>>>, id: string) {
    setter((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }


  function focusResource(index: number) {
    const resource = visibleResources[index];
    if (!resource) return;
    onSelectResource(resource.id);
    const option = resourceListRef.current?.querySelector<HTMLButtonElement>(`[data-resource-index="${index}"]`);
    option?.focus();
  }

  function handleResourceListKeyDown(event: React.KeyboardEvent<HTMLDivElement>) {
    if (visibleResources.length === 0) return;
    const activeIndex = Number((document.activeElement as HTMLElement | null)?.dataset.resourceIndex ?? 0);
    let nextIndex: number | undefined;
    if (event.key === "ArrowDown") nextIndex = Math.min(activeIndex + 1, visibleResources.length - 1);
    if (event.key === "ArrowUp") nextIndex = Math.max(activeIndex - 1, 0);
    if (event.key === "Home") nextIndex = 0;
    if (event.key === "End") nextIndex = visibleResources.length - 1;
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
          <img className="azure-entity-icon" src={ALL_RESOURCES_ICON} alt="" />
        </div>

        <div className="scope-tree">
          <button
            className={!scope.subscriptionId ? "scope-all active" : "scope-all"}
            onClick={() => onScopeChange({})}
          >
            <span><img src={ALL_RESOURCES_ICON} alt="" /> Entire estate</span>
            <b>{estate.totals.resources}</b>
          </button>
          {estate.subscriptions.map((subscription) => {
            const subResources = estate.resources.filter((resource) => resource.subscriptionId === subscription.id);
            const groups = estate.resourceGroups.filter((group) => group.subscriptionId === subscription.id);
            const subActive = scope.subscriptionId === subscription.id && !scope.resourceGroup;
            const expanded = expandedSubscriptions.has(subscription.id);
            return (
              <div className="subscription-branch" key={subscription.id}>
                <div className={subActive ? "tree-node subscription-row active" : "tree-node subscription-row"}>
                  <button
                    className="tree-disclosure"
                    onClick={() => toggleExpanded(setExpandedSubscriptions, subscription.id)}
                    aria-expanded={expanded}
                    aria-label={`${expanded ? "Collapse" : "Expand"} ${subscription.displayName}`}
                  >
                    {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                  </button>
                  <button className="tree-selection" onClick={() => onScopeChange({ subscriptionId: subscription.id })}>
                    <img src={SUBSCRIPTION_ICON} alt="" />
                    <span title={subscription.displayName}>{subscription.displayName}</span>
                    <b>{subResources.length}</b>
                  </button>
                </div>
                <div className={expanded ? "group-branches open" : "group-branches"}>
                  {(expanded ? groups : []).map((group) => {
                    const groupName = group.name.toLowerCase();
                    const groupResources = subResources
                      .filter((resource) => resource.resourceGroup === groupName)
                      .sort((left, right) => left.name.localeCompare(right.name) || left.id.localeCompare(right.id));
                    const count = groupResources.length;
                    const active = scope.subscriptionId === subscription.id && scope.resourceGroup === groupName;
                    const groupExpanded = expandedGroups.has(group.id);
                    return (
                      <div className="group-branch" key={group.id}>
                        <div className={active ? "tree-node group-row active" : "tree-node group-row"}>
                          <button
                            className="tree-disclosure"
                            onClick={() => toggleExpanded(setExpandedGroups, group.id)}
                            aria-expanded={groupExpanded}
                            aria-label={`${groupExpanded ? "Collapse" : "Expand"} ${group.name}`}
                          >
                            {groupExpanded ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
                          </button>
                          <button
                            className="tree-selection"
                            onClick={() => {
                              onScopeChange({ subscriptionId: subscription.id, resourceGroup: groupName });
                              setExpandedGroups((current) => new Set(current).add(group.id));
                            }}
                          >
                            <img src={RESOURCE_GROUP_ICON} alt="" />
                            <span title={group.name}>{group.name}</span>
                            <b>{count}</b>
                          </button>
                        </div>
                        {groupExpanded ? (
                          <div className="resource-branches">
                            {groupResources.length > 0
                              ? groupResources.map((resource) => {
                                  const resourceType = resourceTypeMap.get(resource.azureType);
                                  return (
                                    <button
                                  key={resource.id}
                                  className="tree-resource-row"
                                      onClick={() => {
                                        onScopeChange({ subscriptionId: subscription.id, resourceGroup: groupName });
                                        onSelectResource(resource.id);
                                      }}
                                      title={resource.name}
                                    >
                                      <img src={resourceType?.icon ?? ALL_RESOURCES_ICON} alt="" />
                                      <span>{resource.name}</span>
                                      {resource.findingCount > 0 ? <em>{resource.findingCount}</em> : null}
                                    </button>
                                  );
                                })
                              : (
                                <EmptyState
                                  className="tree-empty-row"
                                  icon={<img src={ALL_RESOURCES_ICON} alt="" />}
                                  detail="No stored resources"
                                />
                              )}
                          </div>
                        ) : null}
                      </div>
                    );
                  })}
                </div>
              </div>
            );
          })}
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
        </header>

        <div className="resource-controls">
          <label>
            <Layers3 size={14} />
            <select value={typeFilter} onChange={(event) => setTypeFilter(event.target.value)} aria-label="Filter by resource type">
              <option value="">All resource types</option>
              {estate.resourceTypes.map((type) => (
                <option key={type.azureType} value={type.azureType}>{type.displayName} ({type.count})</option>
              ))}
            </select>
          </label>
          <label>
            <MapPin size={14} />
            <select value={locationFilter} onChange={(event) => setLocationFilter(event.target.value)} aria-label="Filter by location">
              <option value="">All locations</option>
              {estate.locations.map((location) => (
                <option key={location.name} value={location.name}>
                  {displayLocation(estate.azureMetadata, location.name, "Not stored")} ({location.count})
                </option>
              ))}
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
          {visibleResources.map((resource, index) => (
            <ResourceRow
              key={resource.id}
              resource={resource}
              type={resourceTypeMap.get(resource.azureType)}
              locationName={displayLocation(estate.azureMetadata, resource.location, "Global")}
              index={index}
              tabIndex={index === 0 ? 0 : -1}
              onSelect={() => onSelectResource(resource.id)}
            />
          ))}
          {!filtered.length ? (
            <EmptyState
              className="no-results"
              icon={<SearchX size={28} />}
              title="No resources match this view"
              detail="Clear a filter or search the whole snapshot."
            />
          ) : null}
          {visibleResources.length < filtered.length ? (
            <ShowMore list={list} />
          ) : null}
        </div>
      </section>

    </div>
  );
}

function ResourceRow({
  resource,
  type,
  locationName,
  index,
  tabIndex,
  onSelect,
}: {
  resource: Resource;
  type?: ResourceType;
  locationName: string;
  index: number;
  tabIndex: number;
  onSelect: () => void;
}) {
  return (
    <button className="resource-row" data-resource-index={index} data-resource-id={resource.id} tabIndex={tabIndex} onClick={onSelect} role="option" aria-selected="false">
      <span className="resource-identity">
        <img src={type?.icon ?? ALL_RESOURCES_ICON} alt="" />
        <span>
          <strong>{resource.name}</strong>
          <small>{type?.displayName ?? resource.azureType}</small>
        </span>
      </span>
      <span className="resource-group-cell">{resource.resourceGroup ?? "—"}</span>
      <span className="resource-location-cell">{locationName}</span>
      <span className="signal-cell">
        {resource.findingCount > 0 ? <em className="finding-signal"><AlertTriangle size={13} />{resource.findingCount}</em> : null}
        {resource.edgeCount > 0 ? <em className="edge-signal"><GitBranch size={13} />{resource.edgeCount}</em> : null}
        {!resource.findingCount && !resource.edgeCount ? <small>Quiet</small> : null}
      </span>
    </button>
  );
}

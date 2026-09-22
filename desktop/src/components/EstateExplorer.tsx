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
  Tag,
  X,
} from "lucide-react";
import { ALL_RESOURCES_ICON, RESOURCE_GROUP_ICON, SUBSCRIPTION_ICON, resourceIcon } from "../azure-icons";
import { displayLocation } from "../azure-values";
import type { DashboardFilter, EstateSnapshot, QueryDefMeta, QueryRows, Resource, ResourceType, ScopeSelection } from "../types";
import { getQueryPackMetadata, getQueryRows } from "../api";
import { QueryRecord } from "./query-record";
import { cellText, detailColumns, isMachineShaped, joinRows, typeQueries, type JoinedRow } from "./resource-queries";
import { ShowMore } from "./progressive-list";
import { useProgressiveList, type ProgressiveList } from "./use-progressive-list";
import { EmptyState, ErrorStrip } from "./view-chrome";
import { errorMessage, fill, fillNodes, plural, spaced } from "../format";
import { useLabels } from "../labels";
import { matchesResourceSearch, useResourceTypeMap, useSubscriptionNames } from "../estate-lookups";
import { readPreference, tenantKey, writePreference } from "../preferences";

import { resourceMatchesDashboard } from "./dashboard-model";

type SortKey = "name" | "type" | "location" | "findings";

/** The local filters, remembered per tenant between sessions. */
interface ExplorerFilters {
  typeFilter: string;
  locationFilter: string;
  tagKey: string;
  tagValue: string;
  sortKey: SortKey;
}

const DEFAULT_FILTERS: ExplorerFilters = { typeFilter: "", locationFilter: "", tagKey: "", tagValue: "", sortKey: "name" };

function tagText(value: unknown) {
  return typeof value === "string" ? value : JSON.stringify(value ?? "");
}

interface EstateExplorerProps {
  dashboardFilter?: DashboardFilter;
  estate: EstateSnapshot;
  search: string;
  scope: ScopeSelection;
  onScopeChange: (scope: ScopeSelection) => void;
  onSelectResource: (id: string) => void;
}

export function EstateExplorer({
  estate,
  search,
  scope,
  onScopeChange,
  onSelectResource,
  dashboardFilter,
}: EstateExplorerProps) {
  const filtersKey = tenantKey(estate.tenantId, "explorer-filters");
  const [filters, setFilters] = useState<ExplorerFilters>(() => ({
    ...DEFAULT_FILTERS,
    ...readPreference<Partial<ExplorerFilters>>(filtersKey, {}),
  }));
  const { typeFilter, locationFilter, tagKey, tagValue, sortKey } = filters;
  const setFilter = (patch: Partial<ExplorerFilters>) => setFilters((current) => ({ ...current, ...patch }));
  useEffect(() => {
    writePreference(filtersKey, filters);
  }, [filters, filtersKey]);
  const [activeId, setActiveId] = useState<string>();
  const [expandedSubscriptions, setExpandedSubscriptions] = useState<Set<string>>(
    () => new Set(estate.subscriptions.length === 1 ? [estate.subscriptions[0].id] : []),
  );
  const [expandedGroups, setExpandedGroups] = useState<Set<string>>(() => new Set());
  const resourceListRef = useRef<HTMLDivElement>(null);

  const resourceTypeMap = useResourceTypeMap(estate);
  const subscriptionMap = useSubscriptionNames(estate);
  const { common, desktop: { estate: words, inventory: inventoryWords } } = useLabels();

  // Type-aware columns: once the view is down to one resource type, a query
  // that describes that type can supply the table. The pack says which; an
  // unreadable pack only means the summary columns stay.
  const [pack, setPack] = useState<QueryDefMeta[]>([]);
  useEffect(() => {
    let active = true;
    getQueryPackMetadata()
      .then((defs) => {
        if (active) setPack(defs);
      })
      .catch(() => undefined);
    return () => {
      active = false;
    };
  }, []);
  const selectedType = typeFilter || dashboardFilter?.azureType || "";
  const covering = useMemo(
    () => (selectedType ? typeQueries(pack, estate.queryRuns, selectedType) : []),
    [estate.queryRuns, pack, selectedType],
  );
  /** Per type: the chosen query, or "" for the summary columns. Unset means the default. */
  const [columnChoice, setColumnChoice] = useState<Record<string, string>>({});
  const choice = columnChoice[selectedType];
  const detailDef = choice === "" ? undefined : covering.find((def) => def.name === choice) ?? covering[0];
  const [detailRows, setDetailRows] = useState<QueryRows>();
  const [detailError, setDetailError] = useState<string>();
  useEffect(() => {
    if (!detailDef) return;
    let active = true;
    setDetailError(undefined);
    getQueryRows(detailDef.name, estate.id)
      .then((rows) => {
        if (active) setDetailRows(rows);
      })
      .catch((caught) => {
        if (active) setDetailError(fill(inventoryWords.rows_failed, { error: errorMessage(caught) }));
      });
    return () => {
      active = false;
    };
  }, [detailDef, estate.id, inventoryWords.rows_failed]);

  const filtered = useMemo(() => {
    const matches = estate.resources.filter((resource) => {
      const inSubscription = !scope.subscriptionId || resource.subscriptionId === scope.subscriptionId;
      const inGroup = !scope.resourceGroup || resource.resourceGroup === scope.resourceGroup;
      const hasType = !typeFilter || resource.azureType === typeFilter;
      const inLocation = !locationFilter || resource.location === locationFilter;
      const tags = resource.tags ?? {};
      const hasTag = !tagKey || (tagKey in tags && (!tagValue || tagText(tags[tagKey]) === tagValue));
      return resourceMatchesDashboard(resource, dashboardFilter) && inSubscription && inGroup && hasType && inLocation && hasTag && matchesResourceSearch(resource, search, estate.azureMetadata);
    });
    return matches.sort((a, b) => {
      if (sortKey === "findings") return b.findingCount - a.findingCount || a.name.localeCompare(b.name);
      if (sortKey === "type") return a.azureType.localeCompare(b.azureType) || a.name.localeCompare(b.name);
      if (sortKey === "location") return (a.location ?? "").localeCompare(b.location ?? "") || a.name.localeCompare(b.name);
      return a.name.localeCompare(b.name);
    });
  }, [dashboardFilter, estate.azureMetadata, estate.resources, locationFilter, scope, search, sortKey, tagKey, tagValue, typeFilter]);
  const list = useProgressiveList(filtered, [locationFilter, scope.resourceGroup, scope.subscriptionId, search, sortKey, tagKey, tagValue, typeFilter]);
  const loadedDetail = detailDef && detailRows?.queryName === detailDef.name ? detailRows : undefined;
  const detail = useMemo(
    () => (detailDef?.resourceColumn && loadedDetail ? joinRows(filtered, loadedDetail.rows, detailDef.resourceColumn) : undefined),
    [detailDef, filtered, loadedDetail],
  );
  const detailList = useProgressiveList(detail?.rows ?? [], [detailDef?.name, filtered], 250);
  const tagKeys = useMemo(
    () => [...new Set(estate.resources.flatMap((resource) => Object.keys(resource.tags ?? {})))].sort((a, b) => a.localeCompare(b)),
    [estate.resources],
  );
  const tagValues = useMemo(
    () => (tagKey ? [...new Set(estate.resources.flatMap((resource) => (resource.tags && tagKey in resource.tags ? [tagText(resource.tags[tagKey])] : [])))].sort((a, b) => a.localeCompare(b)) : []),
    [estate.resources, tagKey],
  );
  const visibleResources = list.visible;

  const activeScopeName = scope.resourceGroup
    ? scope.resourceGroup
    : scope.subscriptionId
      ? subscriptionMap.get(scope.subscriptionId)
      : words.entire_estate;

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
    setActiveId(resource.id);
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
      <aside className="scope-pane" aria-label={words.hierarchy_aria}>
        <div className="pane-heading">
          <div>
            <h1>{words.title}</h1>
            <span>{fill(words.summary, { subscriptions: estate.totals.subscriptions, groups: estate.totals.resourceGroups })}</span>
          </div>
          <img className="azure-entity-icon" src={ALL_RESOURCES_ICON} alt="" />
        </div>

        <div className="scope-tree">
          <button
            className={!scope.subscriptionId ? "scope-all active" : "scope-all"}
            onClick={() => onScopeChange({})}
          >
            <span><img src={ALL_RESOURCES_ICON} alt="" /> {words.entire_estate}</span>
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
                    aria-label={fill(expanded ? words.collapse : words.expand, { name: subscription.displayName })}
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
                            aria-label={fill(groupExpanded ? words.collapse : words.expand, { name: group.name })}
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
                                      <img src={resourceIcon(resourceType)} alt="" />
                                      <span>{resource.name}</span>
                                      {resource.findingCount > 0 ? <em>{resource.findingCount}</em> : null}
                                    </button>
                                  );
                                })
                              : (
                                <EmptyState
                                  className="tree-empty-row"
                                  icon={<img src={ALL_RESOURCES_ICON} alt="" />}
                                  detail={words.no_stored_resources}
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

      <section className="resource-pane" aria-label={words.inventory_aria}>
        <header className="resource-header">
          <div>
            <h2>{activeScopeName}</h2>
            <p>
              {fillNodes(words.of_total, { count: <strong>{filtered.length}</strong>, total: estate.totals.resources })}
              {search ? fill(words.matching, { search }) : null}
            </p>
          </div>
        </header>

        <div className="resource-controls">
          <label>
            <Layers3 size={14} />
            <select value={typeFilter} onChange={(event) => setFilter({ typeFilter: event.target.value })} aria-label={words.type_filter_aria}>
              <option value="">{words.all_types}</option>
              {estate.resourceTypes.map((type) => (
                <option key={type.azureType} value={type.azureType}>{type.displayName} ({type.count})</option>
              ))}
            </select>
          </label>
          <label>
            <MapPin size={14} />
            <select value={locationFilter} onChange={(event) => setFilter({ locationFilter: event.target.value })} aria-label={words.location_filter_aria}>
              <option value="">{words.all_locations}</option>
              {estate.locations.map((location) => (
                <option key={location.name} value={location.name}>
                  {displayLocation(estate.azureMetadata, location.name, words.location_not_stored)} ({location.count})
                </option>
              ))}
            </select>
          </label>
          {tagKeys.length > 0 ? (
            <label className="tag-facet">
              <Tag size={14} />
              <select value={tagKey} onChange={(event) => setFilter({ tagKey: event.target.value, tagValue: "" })} aria-label={words.tag_filter_aria}>
                <option value="">{words.all_tags}</option>
                {tagKeys.map((key) => (
                  <option key={key} value={key}>{key}</option>
                ))}
              </select>
              {tagKey ? (
                <select value={tagValue} onChange={(event) => setFilter({ tagValue: event.target.value })} aria-label={words.tag_value_aria}>
                  <option value="">{words.any_value}</option>
                  {tagValues.map((value) => (
                    <option key={value} value={value}>{value}</option>
                  ))}
                </select>
              ) : null}
            </label>
          ) : null}
          <label>
            <ArrowDownAZ size={14} />
            <select value={sortKey} onChange={(event) => setFilter({ sortKey: event.target.value as SortKey })} aria-label={words.sort_aria}>
              <option value="name">{words.sort_name}</option>
              <option value="type">{words.sort_type}</option>
              <option value="location">{words.sort_location}</option>
              <option value="findings">{words.sort_findings}</option>
            </select>
          </label>
          {(typeFilter || locationFilter || tagKey || search) ? (
            <button className="clear-filters" onClick={() => setFilter({ typeFilter: "", locationFilter: "", tagKey: "", tagValue: "" })}>
              <X size={13} /> {words.clear_filters}
            </button>
          ) : null}
        </div>

        {covering.length > 0 ? (
          <div className="column-picker" role="group" aria-label={words.columns_aria}>
            <button aria-pressed={!detailDef} onClick={() => setColumnChoice((current) => ({ ...current, [selectedType]: "" }))}>
              {words.columns_summary}
            </button>
            {covering.map((def) => (
              <button
                key={def.name}
                aria-pressed={detailDef?.name === def.name}
                onClick={() => setColumnChoice((current) => ({ ...current, [selectedType]: def.name }))}
                title={def.description}
              >
                {spaced(def.name)}
              </button>
            ))}
          </div>
        ) : null}

        {detailDef ? (
          <DetailTable
            def={detailDef}
            rows={loadedDetail}
            detail={detail}
            list={detailList}
            error={detailError}
            run={estate.queryRuns.find((run) => run.queryName === detailDef.name)}
            resourceTypeMap={resourceTypeMap}
            onDismissError={() => setDetailError(undefined)}
            onSelect={(id) => {
              setActiveId(id);
              onSelectResource(id);
            }}
          />
        ) : (
          <>
        <div className="resource-table-head" aria-hidden="true">
          <span>{common.columns.resource}</span>
          <span>{common.columns.resource_group}</span>
          <span>{common.columns.location}</span>
          <span>{words.signals}</span>
        </div>
        <div ref={resourceListRef} className="resource-list" role="listbox" aria-label={words.list_aria} onKeyDown={handleResourceListKeyDown}>
          {visibleResources.map((resource, index) => (
            <ResourceRow
              key={resource.id}
              resource={resource}
              type={resourceTypeMap.get(resource.azureType)}
              locationName={displayLocation(estate.azureMetadata, resource.location, words.location_global)}
              index={index}
              tabIndex={index === 0 ? 0 : -1}
              selected={activeId === resource.id}
              onSelect={() => {
                setActiveId(resource.id);
                onSelectResource(resource.id);
              }}
            />
          ))}
          {!filtered.length ? (
            <EmptyState
              className="no-results"
              icon={<SearchX size={28} />}
              title={words.empty_title}
              detail={words.empty_detail}
            />
          ) : null}
          {visibleResources.length < filtered.length ? (
            <ShowMore list={list} />
          ) : null}
        </div>
          </>
        )}
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
  selected,
  onSelect,
}: {
  resource: Resource;
  type?: ResourceType;
  locationName: string;
  index: number;
  tabIndex: number;
  selected: boolean;
  onSelect: () => void;
}) {
  const { common, desktop: { estate: words } } = useLabels();
  return (
    <button className={selected ? "resource-row selected" : "resource-row"} data-resource-index={index} data-resource-id={resource.id} tabIndex={tabIndex} onClick={onSelect} role="option" aria-selected={selected}>
      <span className="resource-identity">
        <img src={resourceIcon(type)} alt="" />
        <span>
          <strong>{resource.name}</strong>
          <small>{type?.displayName ?? resource.azureType}</small>
        </span>
      </span>
      <span className="resource-group-cell">{resource.resourceGroup ?? common.verdict.none}</span>
      <span className="resource-location-cell">{locationName}</span>
      <span className="signal-cell">
        {resource.findingCount > 0 ? <em className="finding-signal"><AlertTriangle size={13} />{resource.findingCount}</em> : null}
        {resource.edgeCount > 0 ? <em className="edge-signal"><GitBranch size={13} />{resource.edgeCount}</em> : null}
        {!resource.findingCount && !resource.edgeCount ? <small>{words.quiet}</small> : null}
      </span>
    </button>
  );
}

/**
 * One query's columns under the resources in view. The first cell is always the
 * resource, so a child row (a subnet, a rule) still says whose it is; a
 * resource with no stored row keeps an empty line rather than vanishing.
 */
function DetailTable({
  def,
  rows,
  detail,
  list,
  error,
  run,
  resourceTypeMap,
  onDismissError,
  onSelect,
}: {
  def: QueryDefMeta;
  rows?: QueryRows;
  detail?: ReturnType<typeof joinRows>;
  list: ProgressiveList<JoinedRow>;
  error?: string;
  run?: EstateSnapshot["queryRuns"][number];
  resourceTypeMap: Map<string, ResourceType>;
  onDismissError: () => void;
  onSelect: (id: string) => void;
}) {
  const { common, desktop: { estate: words, inventory: inventoryWords } } = useLabels();
  const none = common.verdict.none;
  const columns = rows ? detailColumns(rows.columns, def.resourceColumn ?? "id") : [];
  return (
    <div className="resource-detail-table">
      {error ? <ErrorStrip message={error} onDismiss={onDismissError} /> : null}
      <p className="query-description">{def.description}</p>
      <QueryRecord key={def.name} run={run} />
      <div className="data-grid-wrap">
        {!detail ? (
          <p className="muted-copy">{inventoryWords.reading}</p>
        ) : detail.rows.length === detail.missing ? (
          <p className="muted-copy">{words.detail_empty}</p>
        ) : (
          <table className="data-grid">
            <thead>
              <tr>
                <th>{common.columns.resource}</th>
                {columns.map((column) => <th key={column}>{column}</th>)}
              </tr>
            </thead>
            <tbody>
              {list.visible.map(({ resource, row, first }, index) => (
                <tr key={`${resource.id}:${index}`} className={first ? undefined : "continued"}>
                  <td>
                    {first ? (
                      <button className="detail-resource" onClick={() => onSelect(resource.id)} title={resource.name}>
                        <img src={resourceIcon(resourceTypeMap.get(resource.azureType))} alt="" />
                        <span>{resource.name}</span>
                      </button>
                    ) : null}
                  </td>
                  {columns.map((column) => (
                    <td
                      key={column}
                      className={isMachineShaped(column) ? "mono-cell" : undefined}
                      title={row ? cellText(row[column], none) : undefined}
                    >
                      {row ? cellText(row[column], none) : null}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
      <div className="grid-footer">
        <span>
          {detail ? plural(inventoryWords.row_count, detail.rows.length - detail.missing, { count: (detail.rows.length - detail.missing).toLocaleString() }) : ""}
          {detail?.missing ? ` · ${plural(words.detail_missing, detail.missing)}` : ""}
        </span>
        <ShowMore list={list} inline />
      </div>
    </div>
  );
}

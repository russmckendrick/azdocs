import { useEffect, useRef, type ReactNode } from "react";
import { ArrowUpRight, ChevronRight, X } from "lucide-react";
import {
  ALL_RESOURCES_ICON,
  AUDIT_ICON,
  RELATIONSHIPS_ICON,
  TAGS_ICON,
  resourceIcon,
} from "../azure-icons";
import type {
  DashboardDestination,
  DashboardFilter,
  EstateSnapshot,
  SnapshotComparison,
  SnapshotSummary,
} from "../types";
import { fill, plural, resourceName, spaced } from "../format";
import { useLabels } from "../labels";
import { stableCompare } from "../ordering";
import {
  findingMatchesDashboard,
  resourceMatchesDashboard,
} from "./dashboard-model";
import { ShowMore, useProgressiveList } from "./progressive-list";

export interface DashboardSelection {
  kind:
    | "resources"
    | "findings"
    | "tags"
    | "relationships"
    | "queries"
    | "changes"
    | "snapshot";
  title: string;
  scopeName: string;
  filter: DashboardFilter;
  snapshot?: SnapshotSummary;
}

export function DashboardModal({
  selection,
  estate,
  comparison,
  onClose,
  onOpenResults,
  onOpenResource,
  onOpenRelationships,
  onLoadSnapshot,
}: {
  selection: DashboardSelection;
  estate: EstateSnapshot;
  comparison?: SnapshotComparison;
  onClose: () => void;
  onOpenResults: (destination: DashboardDestination) => void;
  onOpenResource: (id: string) => void;
  onOpenRelationships: (id: string) => void;
  onLoadSnapshot: (id: string) => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const {
    common,
    desktop: {
      overview: { dashboard: d },
      topology,
      history,
    },
  } = useLabels();
  useEffect(() => {
    dialog.current?.showModal();
  }, []);
  const { kind, filter } = selection;
  const resourceById = new Map(
    estate.resources.map((resource) => [resource.id, resource]),
  );
  const typeById = new Map(
    estate.resourceTypes.map((type) => [type.azureType, type]),
  );
  function open(
    view: DashboardDestination["view"],
    constraints = filter,
    label = filter.subscriptionId
      ? `${selection.title} · ${selection.scopeName}`
      : selection.title,
  ) {
    onClose();
    onOpenResults({ view, filter: constraints, label });
  }
  function resourceRow(id: string, relationships = false): ReactNode {
    const resource = resourceById.get(id);
    if (!resource)
      return (
        <span className="dashboard-modal-record">
          <span>
            <strong>{resourceName(id)}</strong>
            <small className="mono">{id}</small>
          </span>
        </span>
      );
    return (
      <button
        className="dashboard-modal-record"
        onClick={() => {
          onClose();
          if (relationships) onOpenRelationships(id);
          else onOpenResource(id);
        }}
      >
        <img src={resourceIcon(typeById.get(resource.azureType))} alt="" />
        <span>
          <strong>{resource.name}</strong>
          <small>
            {typeById.get(resource.azureType)?.displayName ??
              resource.azureType}{" "}
            · {resource.resourceGroup}
          </small>
        </span>
        <ChevronRight size={16} />
      </button>
    );
  }
  let rows: ReactNode[] = [];
  let action = () => open("estate");
  let actionLabel = d.open_results;
  let note: string | undefined;
  let icon = ALL_RESOURCES_ICON;
  if (kind === "resources" || kind === "tags") {
    rows = estate.resources
      .filter((resource) => resourceMatchesDashboard(resource, filter))
      .sort(
        (a, b) => stableCompare(a.name, b.name) || stableCompare(a.id, b.id),
      )
      .map((resource) => resourceRow(resource.id));
    if (kind === "tags") {
      icon = TAGS_ICON;
      note = d.tags_note;
    }
  } else if (kind === "findings") {
    icon = AUDIT_ICON;
    rows = estate.findings
      .filter((finding) => findingMatchesDashboard(finding, estate, filter))
      .map((finding) => (
        <div className="dashboard-modal-finding">
          <button
            onClick={() =>
              open(
                "findings",
                {
                  ...filter,
                  queryName: finding.queryName,
                  severity: finding.severity,
                  ...(finding.resourceId
                    ? { resourceIds: [finding.resourceId] }
                    : {}),
                },
                finding.title,
              )
            }
          >
            <span className={`severity-label ${finding.severity}`}>
              {common.severity[finding.severity].name}
            </span>
            <strong>{finding.title}</strong>
            <ChevronRight size={16} />
          </button>
          {finding.resourceId && resourceById.has(finding.resourceId) ? (
            <button
              className="dashboard-resource-link"
              onClick={() => {
                onClose();
                onOpenResource(finding.resourceId!);
              }}
            >
              {resourceById.get(finding.resourceId)?.name}
              <ArrowUpRight size={13} />
            </button>
          ) : (
            <small>{spaced(finding.queryName)}</small>
          )}
        </div>
      ));
    action = () => open("findings");
  } else if (kind === "relationships") {
    icon = RELATIONSHIPS_ICON;
    note = d.relationships_note;
    const ids = new Set(
      estate.resources
        .filter((resource) => resourceMatchesDashboard(resource, filter))
        .map((resource) => resource.id),
    );
    rows = estate.edges
      .filter(
        (edge) =>
          !filter.subscriptionId ||
          ids.has(edge.sourceId) ||
          ids.has(edge.targetId),
      )
      .map((edge) => (
        <div className="dashboard-modal-edge">
          {resourceRow(edge.sourceId, true)}
          <span>
            {topology.edge_kinds[
              edge.kind as keyof typeof topology.edge_kinds
            ] ?? spaced(edge.kind)}
          </span>
          {resourceRow(edge.targetId, true)}
        </div>
      ));
    action = () => open("topology", {}, d.all_subscriptions);
    actionLabel = d.open_map;
  } else if (kind === "queries") {
    note = d.coverage_note;
    rows = estate.queryRuns
      .filter(
        (run) =>
          (!filter.category || run.category === filter.category) &&
          (!filter.queryName || run.queryName === filter.queryName),
      )
      .map((run) => (
        <button
          className="dashboard-modal-record"
          onClick={() => {
            if (run.error || !run.provenance)
              open(
                "history",
                {
                  category: run.category,
                  queryName: run.queryName,
                  healthOnly: true,
                },
                spaced(run.queryName),
              );
            else if (run.queryName === "all_resources")
              open("estate", {}, spaced(run.queryName));
            else
              open(
                run.provenance.kind === "finding" ? "findings" : "inventory",
                { queryName: run.queryName, category: run.category },
                spaced(run.queryName),
              );
          }}
        >
          <span>
            <strong>{spaced(run.queryName)}</strong>
            <small className={run.error ? "risk" : undefined}>
              {run.error ??
                (run.rowCount != null
                  ? plural(d.query_result, run.rowCount)
                  : d.query_unknown)}
            </small>
          </span>
          <ChevronRight size={16} />
        </button>
      ));
    action = () => open("history", { ...filter, healthOnly: true });
    actionLabel = d.all_queries;
  } else if (kind === "changes") {
    note = d.changes_note;
    rows = (comparison?.[filter.changeKind ?? "changed"] ?? []).map(
      (id) => resourceRow(id),
    );
    action = () => open("history");
  } else if (kind === "snapshot" && selection.snapshot) {
    const snapshot = selection.snapshot;
    note = d.snapshot_note;
    rows = [
      <dl className="dashboard-snapshot-stats">
        <div>
          <dt>{history.estate}</dt>
          <dd>{snapshot.resources}</dd>
        </div>
        <div>
          <dt>{d.audit}</dt>
          <dd>{snapshot.findings}</dd>
        </div>
        <div>
          <dt>{history.status}</dt>
          <dd>{snapshot.status}</dd>
        </div>
        <div>
          <dt>{d.scope}</dt>
          <dd>{snapshot.subscriptions}</dd>
        </div>
      </dl>,
    ];
    action = () => {
      onClose();
      onLoadSnapshot(snapshot.id);
    };
    actionLabel = d.open_snapshot;
  }
  const list = useProgressiveList(rows, [selection], 50);
  return (
    <dialog
      ref={dialog}
      className="dashboard-modal"
      aria-labelledby="dashboard-modal-title"
      aria-describedby="dashboard-modal-description"
      onClose={onClose}
      onKeyDown={(event) => {
        if (event.key === "Escape") event.stopPropagation();
      }}
      onCancel={(event) => event.stopPropagation()}
      onClick={(event) => {
        if (event.target === event.currentTarget) {
          const box = event.currentTarget.getBoundingClientRect();
          if (
            event.clientX < box.left ||
            event.clientX > box.right ||
            event.clientY < box.top ||
            event.clientY > box.bottom
          )
            event.currentTarget.close();
        }
      }}
    >
      <header>
        <img src={icon} alt="" />
        <div>
          <h2 id="dashboard-modal-title">{selection.title}</h2>
          <p id="dashboard-modal-description">
            {selection.scopeName}
            {kind !== "snapshot"
              ? ` · ${fill(d.details, { count: rows.length })}`
              : ""}
          </p>
        </div>
        <button
          className="icon-button"
          aria-label={d.close}
          onClick={() => dialog.current?.close()}
          autoFocus
        >
          <X size={20} />
        </button>
      </header>
      <div className="dashboard-modal-body">
        {note && <p className="dashboard-note">{note}</p>}
        <div className="dashboard-modal-records">
          {list.visible.map((row, index) => (
            <div key={index}>{row}</div>
          ))}
        </div>
        {!rows.length && <p className="dashboard-empty">{d.empty}</p>}
        <ShowMore list={list} />
      </div>
      <footer>
        {kind === "tags" && (
          <button
            className="quiet-button"
            onClick={() => open("governance", {}, d.all_subscriptions)}
          >
            {d.open_governance}
          </button>
        )}
        <button className="collect-button" onClick={action}>
          {actionLabel}
          <ArrowUpRight size={16} />
        </button>
      </footer>
    </dialog>
  );
}

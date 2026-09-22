import {
  useMemo,
  useState,
  type CSSProperties,
  type ReactNode,
} from "react";
import { ArrowUpRight, ChevronRight } from "lucide-react";
import {
  ALL_RESOURCES_ICON,
  AUDIT_ICON,
  LOCATION_ICON,
  RELATIONSHIPS_ICON,
  SUBSCRIPTION_ICON,
  TAGS_ICON,
} from "../azure-icons";
import { displayLocation } from "../azure-values";
import type {
  AppBootstrap,
  DashboardDestination,
  DashboardFilter,
  EstateSnapshot,
  SnapshotComparison,
} from "../types";
import {
  snapshotStatusLabel,
  capitalise,
  dayMonth,
  dateTime,
  fill,
  plural,
  spaced,
} from "../format";
import { useLabels } from "../labels";
import { SEVERITIES, SEVERITY_TOKEN } from "../ordering";
import { DatabaseStamp } from "./view-chrome";
import {
  dashboardComparison,
  dashboardData,
  dashboardHistory,
  queryCoverage,
} from "./dashboard-model";
import { useDashboardNumber } from "./dashboard-motion";
import { DashboardModal, type DashboardSelection } from "./DashboardModal";
import { MAP_HEIGHT, MAP_WIDTH, project } from "./world-map";
import { WorldMapBackdrop } from "./WorldMapBackdrop";
import { HistoryChart } from "./HistoryChart";

/**
 * One KPI card: the Azure icon in a soft tinted container, the metric label
 * beside it, the value, then the supporting line. `accent` names the design
 * token the container is tinted with — a semantic colour, never decoration.
 */
function Kpi({
  title,
  value,
  suffix,
  icon,
  accent,
  detail,
  signal,
  onClick,
}: {
  title: string;
  value: number;
  suffix?: string;
  icon: string;
  accent: string;
  detail: string;
  signal?: boolean;
  onClick: () => void;
}) {
  const number = useDashboardNumber(value);
  return (
    <button
      className="dashboard-kpi"
      style={{ "--kpi-accent": accent } as CSSProperties}
      aria-label={`${title} ${value.toLocaleString()}${suffix ?? ""} · ${detail}`}
      onClick={onClick}
      aria-haspopup="dialog"
    >
      <span className="dashboard-kpi-head">
        <span className="dashboard-kpi-icon">
          <img src={icon} alt="" />
        </span>
        {title}
      </span>
      <strong className={signal ? "risk" : undefined}>
        {number}
        {suffix}
      </strong>
      <span className="dashboard-kpi-detail">
        {detail}
        <ChevronRight size={16} />
      </span>
    </button>
  );
}

function Panel({
  title,
  icon,
  children,
  className = "",
  action,
}: {
  title: string;
  icon?: string;
  children: ReactNode;
  className?: string;
  action?: ReactNode;
}) {
  return (
    <section className={`dashboard-panel ${className}`}>
      <header>
        {icon && <img src={icon} alt="" />}
        <h2>{title}</h2>
        {action}
      </header>
      {children}
    </section>
  );
}

function DataBar({
  label,
  count,
  max,
  icon,
  color,
  onClick,
}: {
  label: string;
  count: number;
  max: number;
  icon?: string;
  color?: string;
  onClick: () => void;
}) {
  return (
    <button
      className="dashboard-bar"
      onClick={onClick}
      aria-haspopup="dialog"
      style={{ "--bar-color": color ?? "var(--az-primary)" } as CSSProperties}
    >
      {icon && <img src={icon} alt="" />}
      <span className="dashboard-bar-main">
        <span className="dashboard-bar-label">
          <span title={label}>{label}</span>
          <b>{count.toLocaleString()}</b>
        </span>
        <span className="dashboard-bar-track">
          <span style={{ transform: `scaleX(${max ? count / max : 0})` }} />
        </span>
      </span>
    </button>
  );
}

/**
 * Resource locations: an understated world map with one small pulsing
 * Azure-blue marker per region. Regions the catalogue cannot place are still
 * counted in the list beneath, so nothing is only on the map.
 */
function ResourceMap({
  locations,
  regions,
  label,
  nameOf,
  onSelect,
}: {
  locations: Array<{ name: string; count: number }>;
  regions: EstateSnapshot["azureMetadata"]["regions"];
  label: string;
  nameOf: (location: string) => string;
  onSelect: (location: { name: string; count: number }) => void;
}) {
  const markers = locations.flatMap((location) => {
    const region = regions[location.name.toLowerCase()];
    if (!region) return [];
    const [x, y] = project(region.longitude, region.latitude);
    return [{ ...location, x, y, radius: 3.5 }];
  });
  return (
    <div className="dashboard-map">
      <svg viewBox={`0 0 ${MAP_WIDTH} ${MAP_HEIGHT}`} role="img" aria-label={label}>
        <WorldMapBackdrop />
        {markers.map((marker) => {
          const name = nameOf(marker.name);
          return (
            <g
              key={marker.name}
              className="dashboard-map-marker"
              role="button"
              tabIndex={0}
              aria-haspopup="dialog"
              aria-label={`${name} · ${marker.count.toLocaleString()}`}
              onClick={() => onSelect(marker)}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  onSelect(marker);
                }
              }}
            >
              <title>
                {name} · {marker.count.toLocaleString()}
              </title>
              <circle cx={marker.x} cy={marker.y} r={marker.radius + 8} className="dashboard-map-target" />
              <circle cx={marker.x} cy={marker.y} r={marker.radius} className="map-pulse" />
              <circle cx={marker.x} cy={marker.y} r={marker.radius + 3} className="map-marker-ring" />
              <circle cx={marker.x} cy={marker.y} r={marker.radius} />
            </g>
          );
        })}
      </svg>
    </div>
  );
}

export function OverviewView({
  bootstrap,
  estate,
  comparison: previousComparison,
  onOpenResults,
  onOpenResource,
  onOpenRelationships,
  onLoadSnapshot,
  onOpenRegions,
}: {
  bootstrap: AppBootstrap;
  estate: EstateSnapshot;
  /** Fetched by the shell after the snapshot paints; undefined until then. */
  comparison?: SnapshotComparison;
  onOpenResults: (destination: DashboardDestination) => void;
  onOpenResource: (id: string) => void;
  onOpenRelationships: (id: string) => void;
  onLoadSnapshot: (id: string) => void;
  /** The Regions page: every datacentre on a full map, not a resource list. */
  onOpenRegions: () => void;
}) {
  const [subscriptionId, setSubscriptionId] = useState("");
  const [days, setDays] = useState(90);
  const [selection, setSelection] = useState<DashboardSelection>();
  const {
    common,
    desktop: { overview: words, history: historyWords },
  } = useLabels();
  const d = words.dashboard;
  const data = useMemo(
    () => dashboardData(estate, subscriptionId || undefined),
    [estate, subscriptionId],
  );
  const series = useMemo(
    () => dashboardHistory(bootstrap, estate, days),
    [bootstrap, estate, days],
  );
  const coverage = useMemo(() => queryCoverage(estate), [estate]);
  const comparison = dashboardComparison(bootstrap, estate, previousComparison);
  const scope = subscriptionId ? { subscriptionId } : {};
  const scopeName =
    estate.subscriptions.find(
      (subscription) => subscription.id === subscriptionId,
    )?.displayName ?? d.all_subscriptions;
  function inspect(
    kind: DashboardSelection["kind"],
    title: string,
    filter: DashboardFilter = {},
    scoped = true,
  ) {
    const constraints = { ...(scoped ? scope : {}), ...filter };
    const name =
      estate.subscriptions.find(
        (subscription) => subscription.id === constraints.subscriptionId,
      )?.displayName ?? (scoped ? scopeName : d.all_subscriptions);
    setSelection({ kind, title, scopeName: name, filter: constraints });
  }
  const severityShares = SEVERITIES.map((level) =>
    data.findings.length
      ? (data.severity[level] / data.findings.length) * 100
      : 0,
  );
  const severityStarts = severityShares.map((_, index) =>
    severityShares.slice(0, index).reduce((total, share) => total + share, 0),
  );
  return (
    <div className="overview-workspace">
      {/* No page header here: the dashboard is the orientation. One compact
          row scopes it and stamps the snapshot; the heading stays for
          assistive technology only. */}
      <h1 className="sr-only">{words.title}</h1>
      <div className="dashboard-toolbar">
        <label className="dashboard-scope">
          <span>{d.scope}</span>
          <select
            value={subscriptionId}
            onChange={(event) => setSubscriptionId(event.target.value)}
          >
            <option value="">{d.all_subscriptions}</option>
            {estate.subscriptions.map((subscription) => (
              <option key={subscription.id} value={subscription.id}>
                {subscription.displayName}
              </option>
            ))}
          </select>
        </label>
        <DatabaseStamp
          label={words.snapshot_stamp}
          value={
            <>
              {estate.id.slice(0, 8)} · {snapshotStatusLabel(estate.status)}
            </>
          }
        />
      </div>
      <div className="dashboard-kpis">
        <Kpi
          title={words.resources}
          value={data.resources.length}
          icon={ALL_RESOURCES_ICON}
          accent="var(--az-resource)"
          detail={plural(d.groups_context, data.groups)}
          onClick={() => inspect("resources", words.resources)}
        />
        <Kpi
          title={d.audit}
          value={data.findings.length}
          icon={AUDIT_ICON}
          accent={
            data.severity.high > 0 ? "var(--az-danger)" : "var(--az-primary)"
          }
          signal={data.severity.high > 0}
          detail={fill(d.high_context, { count: data.severity.high })}
          onClick={() => inspect("findings", d.audit)}
        />
        <Kpi
          title={words.tag_coverage}
          value={data.percent}
          suffix="%"
          icon={TAGS_ICON}
          accent="var(--az-governance)"
          detail={fill(d.tags_context, {
            tagged: data.tagged.length,
            total: data.resources.length,
          })}
          onClick={() =>
            inspect("tags", d.untagged, {
              resourceIds: data.resources
                .filter((resource) => !data.tagged.includes(resource))
                .map((resource) => resource.id),
            })
          }
        />
        <Kpi
          title={words.relationships}
          value={data.edges.length}
          icon={RELATIONSHIPS_ICON}
          accent="var(--az-relationship)"
          detail={d.links_context}
          onClick={() => inspect("relationships", words.relationships)}
        />
      </div>
      <div className="dashboard-grid">
        <Panel
          title={d.regions}
          icon={LOCATION_ICON}
          className="dashboard-history dashboard-map-panel"
          action={
            <button className="dashboard-panel-link" onClick={onOpenRegions}>
              {d.all_regions}
              <ArrowUpRight size={14} />
            </button>
          }
        >
          <ResourceMap
            locations={data.locations}
            regions={estate.azureMetadata.regions}
            label={words.regions_caption}
            nameOf={(location) =>
              displayLocation(
                estate.azureMetadata,
                location,
                words.location_not_stored,
              )
            }
            onSelect={(location) =>
              inspect(
                "resources",
                displayLocation(
                  estate.azureMetadata,
                  location.name,
                  words.location_not_stored,
                ),
                { location: location.name },
              )
            }
          />
          <ul className="dashboard-map-locations">
            {data.locations.slice(0, 6).map((location) => (
              <li key={location.name}>
                <button
                  aria-haspopup="dialog"
                  onClick={() =>
                    inspect(
                      "resources",
                      displayLocation(
                        estate.azureMetadata,
                        location.name,
                        words.location_not_stored,
                      ),
                      { location: location.name },
                    )
                  }
                >
                  <span>
                    {displayLocation(estate.azureMetadata, location.name, words.location_not_stored)}
                  </span>
                  <b>{location.count.toLocaleString()}</b>
                </button>
              </li>
            ))}
          </ul>
        </Panel>
        <Panel
          title={d.severity}
          className="dashboard-severity"
          icon={AUDIT_ICON}
        >
          <div className="dashboard-donut-wrap">
            <svg
              className="dashboard-donut"
              viewBox="0 0 160 160"
              role="img"
              aria-label={fill(words.findings, { high: data.severity.high })}
            >
              <circle
                className="dashboard-donut-track"
                cx="80"
                cy="80"
                r="61"
              />
              {SEVERITIES.map((level, index) => {
                const share = severityShares[index];
                const start = severityStarts[index];
                return (
                  <circle
                    key={level}
                    cx="80"
                    cy="80"
                    r="61"
                    pathLength="100"
                    stroke={SEVERITY_TOKEN[level]}
                    strokeDasharray={`${share} ${100 - share}`}
                    strokeDashoffset={-start}
                    transform="rotate(-90 80 80)"
                  />
                );
              })}
              <text
                x="80"
                y="80"
                textAnchor="middle"
                className="dashboard-donut-total"
              >
                {data.findings.length.toLocaleString()}
              </text>
              <text x="80" y="100" textAnchor="middle">
                {d.audit}
              </text>
            </svg>
            <div className="dashboard-severity-legend">
              {SEVERITIES.map((level) => (
                <button
                  key={level}
                  onClick={() =>
                    inspect(
                      "findings",
                      capitalise(common.severity[level].name),
                      { severity: level },
                    )
                  }
                  aria-haspopup="dialog"
                >
                  <i style={{ background: SEVERITY_TOKEN[level] }} />
                  <span>{capitalise(common.severity[level].name)}</span>
                  <b>{data.severity[level]}</b>
                  <ChevronRight size={14} />
                </button>
              ))}
            </div>
          </div>
        </Panel>
        <Panel
          title={d.types}
          icon={ALL_RESOURCES_ICON}
          className="dashboard-third"
        >
          <div className="dashboard-bars">
            {data.types.slice(0, 6).map((type) => (
              <DataBar
                key={type.azureType}
                label={type.displayName}
                icon={type.icon}
                count={type.count}
                color={type.color}
                max={data.types[0]?.count ?? 1}
                onClick={() =>
                  inspect("resources", type.displayName, {
                    azureType: type.azureType,
                  })
                }
              />
            ))}
          </div>
          <button
            className="dashboard-panel-link"
            onClick={() => inspect("resources", words.resources)}
          >
            {d.all_types}
            <ArrowUpRight size={14} />
          </button>
        </Panel>
        <Panel
          title={d.history}
          className="dashboard-third dashboard-trend"
          action={
            <select
              aria-label={d.history_range}
              value={days}
              onChange={(event) => setDays(Number(event.target.value))}
            >
              <option value={30}>{d.range_month}</option>
              <option value={90}>{d.range_quarter}</option>
              <option value={0}>{d.range_all}</option>
            </select>
          }
        >
          <HistoryChart
            key={`${estate.id}:${days}`}
            series={series}
            onSelect={(snapshot) =>
              setSelection({
                kind: "snapshot",
                title: dateTime(snapshot.createdAt),
                scopeName: d.all_subscriptions,
                filter: {},
                snapshot,
              })
            }
          />
          <p className="dashboard-note">{d.history_note}</p>
        </Panel>
        <Panel
          title={d.subscriptions}
          icon={TAGS_ICON}
          className="dashboard-third"
        >
          <div className="dashboard-adoption">
            {estate.governance.subscriptions
              .filter(
                (subscription) =>
                  !subscriptionId ||
                  subscription.subscriptionId === subscriptionId,
              )
              .map((subscription) => (
                <button
                  key={subscription.subscriptionId}
                  onClick={() => {
                    const subset = dashboardData(
                      estate,
                      subscription.subscriptionId,
                    );
                    inspect("tags", d.untagged, {
                      subscriptionId: subscription.subscriptionId,
                      resourceIds: subset.resources
                        .filter((resource) => !subset.tagged.includes(resource))
                        .map((resource) => resource.id),
                    });
                  }}
                  aria-haspopup="dialog"
                >
                  <span className="dashboard-adoption-value">
                    {subscription.percent}%
                  </span>
                  <span className="dashboard-adoption-track">
                    <span style={{ transform: `scaleX(${subscription.percent / 100})` }} />
                  </span>
                  <span className="dashboard-adoption-name">
                    <img src={SUBSCRIPTION_ICON} alt="" />
                    <span title={subscription.displayName}>
                      {subscription.displayName}
                    </span>
                  </span>
                </button>
              ))}
          </div>
          <p className="dashboard-note">{d.tags_note}</p>
        </Panel>
        <Panel
          title={d.coverage}
          className="dashboard-coverage"
          action={
            <span className="dashboard-caption">{d.all_subscriptions}</span>
          }
        >
          <div className="dashboard-coverage-bars">
            {coverage.map((category) => (
              <button
                key={category.category}
                className="dashboard-coverage-row"
                onClick={() =>
                  inspect(
                    "queries",
                    spaced(category.category),
                    { category: category.category },
                    false,
                  )
                }
                aria-haspopup="dialog"
              >
                <span>{spaced(category.category)}</span>
                <span className="dashboard-bar-track">
                  <span
                    style={{
                      transform: `scaleX(${category.total ? category.succeeded / category.total : 0})`,
                    }}
                  />
                </span>
                <b>
                  {fill(d.coverage_count, {
                    succeeded: category.succeeded,
                    total: category.total,
                  })}
                </b>
              </button>
            ))}
          </div>
          {!coverage.length && (
            <p className="dashboard-note">{d.coverage_empty}</p>
          )}
          <p className="dashboard-note">{d.coverage_note}</p>
        </Panel>
        <Panel title={d.changes} className="dashboard-changes">
          {comparison ? (
            <>
              <p className="dashboard-note">
                {fill(d.baseline, {
                  date: dayMonth(comparison.base.createdAt),
                })}
              </p>
              <div className="dashboard-change-counts">
                {(["added", "changed", "removed"] as const).map((kind) => (
                  <button
                    key={kind}
                    className={kind}
                    onClick={() =>
                      inspect(
                        "changes",
                        historyWords[kind],
                        { changeKind: kind },
                        false,
                      )
                    }
                    aria-haspopup="dialog"
                  >
                    <strong>{comparison.diff[kind].length}</strong>
                    <span>{historyWords[kind]}</span>
                  </button>
                ))}
              </div>
              <p className="dashboard-note">{d.changes_note}</p>
            </>
          ) : (
            <p className="dashboard-note">{d.changes_empty}</p>
          )}
        </Panel>
        <Panel
          title={d.attention}
          icon={AUDIT_ICON}
          className="dashboard-attention"
          action={
            <button
              className="dashboard-panel-link"
              onClick={() => inspect("findings", d.audit)}
            >
              {words.review_all}
              <ArrowUpRight size={14} />
            </button>
          }
        >
          {data.checks.slice(0, 3).map((check) => (
            <button
              className="dashboard-check"
              key={`${check.queryName}:${check.severity}`}
              onClick={() =>
                inspect("findings", spaced(check.queryName), {
                  queryName: check.queryName,
                  severity: check.severity,
                })
              }
              aria-haspopup="dialog"
            >
              <span className={`severity-label ${check.severity}`}>
                {common.severity[check.severity].name}
              </span>
              <strong>{spaced(check.queryName)}</strong>
              <span>{plural(d.check_count, check.count)}</span>
              <ChevronRight size={16} />
            </button>
          ))}
          {!data.checks.length && (
            <p className="dashboard-note">{words.no_findings}</p>
          )}
        </Panel>
      </div>
      {selection && (
        <DashboardModal
          selection={selection}
          estate={estate}
          comparison={comparison?.diff}
          onClose={() => setSelection(undefined)}
          onOpenResults={onOpenResults}
          onOpenResource={onOpenResource}
          onOpenRelationships={onOpenRelationships}
          onLoadSnapshot={onLoadSnapshot}
        />
      )}
    </div>
  );
}

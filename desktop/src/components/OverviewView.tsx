import {
  useEffect,
  useMemo,
  useRef,
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
  SnapshotSummary,
} from "../types";
import {
  capitalise,
  dayMonth,
  dateTime,
  fill,
  plural,
  spaced,
} from "../format";
import { useLabels } from "../labels";
import { SEVERITIES, SEVERITY_TOKEN } from "../ordering";
import { DatabaseStamp, ViewHeading } from "./view-chrome";
import {
  dashboardComparison,
  dashboardData,
  dashboardHistory,
  queryCoverage,
  resourceHistoryScale,
} from "./dashboard-model";
import { useDashboardNumber } from "./dashboard-motion";
import { DashboardModal, type DashboardSelection } from "./DashboardModal";

function Kpi({
  title,
  value,
  suffix,
  icon,
  detail,
  signal,
  onClick,
}: {
  title: string;
  value: number;
  suffix?: string;
  icon: string;
  detail: string;
  signal?: boolean;
  onClick: () => void;
}) {
  const number = useDashboardNumber(value);
  return (
    <button
      className="dashboard-kpi"
      aria-label={`${title} ${value.toLocaleString()}${suffix ?? ""} · ${detail}`}
      onClick={onClick}
      aria-haspopup="dialog"
    >
      <span className="dashboard-kpi-label">
        {title}
        <img src={icon} alt="" />
      </span>
      <strong className={signal ? "risk" : undefined}>
        {number}
        {suffix}
      </strong>
      <span className="dashboard-kpi-detail">
        {detail}
        <ArrowUpRight size={16} />
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
      style={{ "--bar-color": color ?? "var(--accent)" } as CSSProperties}
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

export function HistoryChart({
  series,
  onSelect,
}: {
  series: SnapshotSummary[];
  onSelect: (snapshot: SnapshotSummary) => void;
}) {
  const words = useLabels().desktop.overview;
  const chart = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(630);
  useEffect(() => {
    const element = chart.current;
    if (!element) return;
    const observer = new ResizeObserver((entries) =>
      setWidth(Math.max(240, entries[0].contentRect.width)),
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  const endX = width - 20;
  const complete = series.filter((snapshot) => snapshot.status === "complete");
  const scale = resourceHistoryScale(complete.map(snapshot => snapshot.resources));
  const tickLabels = scale.ticks.map(tick => tick.toLocaleString());
  const startX = Math.max(45, Math.max(...tickLabels.map(label => label.length)) * 7 + 12);
  const yPosition = (value: number) => 154 - ((value - scale.min) / (scale.max - scale.min)) * 120;
  const first = Date.parse(series[0]?.createdAt ?? "");
  const span = Math.max(1, Date.parse(series.at(-1)?.createdAt ?? "") - first);
  const points = series.map((snapshot) => ({
    snapshot,
    x:
      series.length === 1
        ? width / 2
        : startX + ((Date.parse(snapshot.createdAt) - first) / span) * (endX - startX),
    y: yPosition(snapshot.resources),
  }));
  let connected = false;
  const segments: (typeof points)[] = [];
  for (const point of points) {
    if (point.snapshot.status !== "complete") {
      connected = false;
      continue;
    }
    if (!connected) segments.push([]);
    segments.at(-1)?.push(point);
    connected = true;
  }
  connected = false;
  const path = points
    .map((point) => {
      if (point.snapshot.status !== "complete") {
        connected = false;
        return "";
      }
      const command = connected ? "L" : "M";
      connected = true;
      return `${command}${point.x},${point.y}`;
    })
    .join(" ");
  return (
    <div className="dashboard-history-chart" ref={chart}>
      <svg viewBox={`0 0 ${width} 190`} aria-label={words.growth_aria}>
        {scale.ticks.map((tick, index) => (
          <g key={tick}>
            <line
              x1={startX}
              x2={endX}
              y1={yPosition(tick)}
              y2={yPosition(tick)}
              className="dashboard-gridline"
            />
            <text x={startX - 10} y={yPosition(tick) + 4} textAnchor="end">
              {tickLabels[index]}
            </text>
          </g>
        ))}
        {segments
          .filter((segment) => segment.length > 1)
          .map((segment, index) => (
            <path
              key={index}
              className="dashboard-history-area"
              d={`M${segment[0].x},154 ${segment.map((point) => `L${point.x},${point.y}`).join(" ")} L${segment.at(-1)!.x},154 Z`}
            />
          ))}
        <path className="dashboard-history-line" d={path} pathLength="1" />
        {points.map(({ snapshot, x, y }) => (
          <g
            key={snapshot.id}
            role="button"
            tabIndex={0}
            aria-haspopup="dialog"
            aria-label={`${dateTime(snapshot.createdAt)} · ${snapshot.resources} · ${snapshot.status}`}
            onClick={() => onSelect(snapshot)}
            onKeyDown={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                onSelect(snapshot);
              }
            }}
            className="dashboard-history-point"
          >
            <title>
              {dateTime(snapshot.createdAt)} · {snapshot.resources} ·{" "}
              {snapshot.status}
            </title>
            <circle
              cx={x}
              cy={snapshot.status === "complete" ? y : 154}
              r="12"
              className="dashboard-point-target"
            />
            <circle
              cx={x}
              cy={snapshot.status === "complete" ? y : 154}
              r="4"
              className={
                snapshot.status === "complete"
                  ? "dashboard-point"
                  : "dashboard-point-gap"
              }
            />
            {snapshot.status !== "complete" && (
              <text x={x} y="142" textAnchor="middle">
                {snapshot.status}
              </text>
            )}
          </g>
        ))}
        <text x={startX} y="181">
          {dayMonth(series[0]?.createdAt)}
        </text>
        <text x={endX} y="181" textAnchor="end">
          {dayMonth(series.at(-1)?.createdAt)}
        </text>
      </svg>
      {complete.length < 2 && (
        <p className="dashboard-note">{words.dashboard.history_empty}</p>
      )}
    </div>
  );
}

export function OverviewView({
  bootstrap,
  estate,
  onOpenResults,
  onOpenResource,
  onOpenRelationships,
  onLoadSnapshot,
}: {
  bootstrap: AppBootstrap;
  estate: EstateSnapshot;
  onOpenResults: (destination: DashboardDestination) => void;
  onOpenResource: (id: string) => void;
  onOpenRelationships: (id: string) => void;
  onLoadSnapshot: (id: string) => void;
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
  const comparison = dashboardComparison(bootstrap, estate);
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
  let offset = 0;
  return (
    <div className="overview-workspace">
      <ViewHeading
        title={words.title}
        description={fill(d.recorded, { date: dateTime(estate.createdAt) })}
      >
        <div className="dashboard-heading-controls">
          <label>
            {d.scope}
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
                {estate.id.slice(0, 8)} · {estate.status}
              </>
            }
          />
        </div>
      </ViewHeading>
      <div className="dashboard-kpis">
        <Kpi
          title={words.resources}
          value={data.resources.length}
          icon={ALL_RESOURCES_ICON}
          detail={plural(d.groups_context, data.groups)}
          onClick={() => inspect("resources", words.resources)}
        />
        <Kpi
          title={d.audit}
          value={data.findings.length}
          icon={AUDIT_ICON}
          signal={data.severity.high > 0}
          detail={fill(d.high_context, { count: data.severity.high })}
          onClick={() => inspect("findings", d.audit)}
        />
        <Kpi
          title={words.tag_coverage}
          value={data.percent}
          suffix="%"
          icon={TAGS_ICON}
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
          detail={d.links_context}
          onClick={() => inspect("relationships", words.relationships)}
        />
      </div>
      <div className="dashboard-grid">
        <Panel
          title={d.history}
          className="dashboard-history"
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
              {SEVERITIES.map((level) => {
                const share = data.findings.length
                  ? (data.severity[level] / data.findings.length) * 100
                  : 0;
                const start = offset;
                offset += share;
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
          title={d.regions}
          icon={LOCATION_ICON}
          className="dashboard-third"
        >
          <div className="dashboard-bars">
            {data.locations.slice(0, 6).map((location) => (
              <DataBar
                key={location.name}
                label={displayLocation(
                  estate.azureMetadata,
                  location.name,
                  words.location_not_stored,
                )}
                count={location.count}
                max={data.locations[0]?.count ?? 1}
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
              />
            ))}
          </div>
          <button
            className="dashboard-panel-link"
            onClick={() => inspect("resources", d.regions)}
          >
            {d.all_regions}
            <ArrowUpRight size={14} />
          </button>
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
                    <span style={{ transform: `scaleY(${subscription.percent / 100})` }} />
                  </span>
                  <img src={SUBSCRIPTION_ICON} alt="" />
                  <span title={subscription.displayName}>
                    {subscription.displayName}
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

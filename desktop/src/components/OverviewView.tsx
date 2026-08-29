import { useMemo } from "react";
import type { AppBootstrap, EstateSnapshot, ViewId } from "../types";

const SEVERITY_SWATCH: Record<string, string> = {
  high: "var(--coral)",
  medium: "var(--amber)",
  low: "var(--muted)",
  info: "var(--accent)",
};

function sparkPath(values: number[], width: number, height: number) {
  const min = Math.min(...values);
  const max = Math.max(...values);
  const range = Math.max(1, max - min);
  const step = values.length > 1 ? (width - 8) / (values.length - 1) : 0;
  const points = values.map((value, index) => ({
    x: 4 + index * step,
    y: height - 18 - ((value - min) / range) * (height - 34),
  }));
  return { points, line: points.map((p, i) => `${i === 0 ? "M" : "L"}${p.x},${p.y}`).join(" ") };
}

export function OverviewView({
  bootstrap,
  estate,
  onOpenView,
  onOpenResource,
}: {
  bootstrap: AppBootstrap;
  estate: EstateSnapshot;
  onOpenView: (view: ViewId) => void;
  onOpenResource: (id: string) => void;
}) {
  // Oldest first, so growth reads left to right.
  const series = useMemo(
    () => [...bootstrap.snapshots].reverse(),
    [bootstrap.snapshots],
  );
  const topTypes = estate.resourceTypes.slice(0, 8);
  const maxTypeCount = Math.max(1, ...topTypes.map((type) => type.count));
  const topLocations = estate.locations.slice(0, 6);
  const maxLocationCount = Math.max(1, ...topLocations.map((location) => location.count));
  const attention = useMemo(
    () =>
      [...estate.findings]
        .sort((a, b) => severityRank(a.severity) - severityRank(b.severity))
        .slice(0, 4),
    [estate.findings],
  );
  const previous = series.length > 1 ? series[series.length - 2] : undefined;
  const findingDelta = previous ? estate.totals.findings - previous.findings : 0;
  const failedQueries = estate.queryRuns.filter((run) => run.error).length;
  const growth = series.length > 1 ? sparkPath(series.map((entry) => entry.resources), 560, 120) : undefined;

  return (
    <div className="overview-workspace">
      <header className="view-heading">
        <div>
          <h1>Overview</h1>
          <p>
            {estate.tenantId} · {estate.totals.subscriptions} subscriptions · {estate.totals.resourceGroups} resource
            groups · {estate.locations.length} regions
          </p>
        </div>
        <div className="database-stamp">
          <span>
            <small>Snapshot</small>
            <strong>{estate.id.slice(0, 8)} · {estate.status}</strong>
          </span>
        </div>
      </header>

      <div className="stat-strip">
        <button className="stat-cell" onClick={() => onOpenView("estate")}>
          <strong>{estate.totals.resources}</strong>
          <span>Resources</span>
        </button>
        <button className="stat-cell" onClick={() => onOpenView("estate")}>
          <strong>{estate.totals.resourceGroups}</strong>
          <span>Resource groups</span>
        </button>
        <button className="stat-cell" onClick={() => onOpenView("findings")}>
          <strong className="risk">{estate.totals.findings}</strong>
          <span>Findings · {estate.severityCounts.high} high</span>
        </button>
        <button className="stat-cell" onClick={() => onOpenView("governance")}>
          <strong>{estate.tagCoverage.percent}%</strong>
          <span>Tag coverage</span>
        </button>
        <button className="stat-cell" onClick={() => onOpenView("topology")}>
          <strong>{estate.edges.length}</strong>
          <span>Relationships</span>
        </button>
      </div>

      <div className="overview-columns">
        <div className="overview-figures">
          <div className="figure-block">
            {topTypes.map((type) => (
              <div className="bar-row" key={type.azureType}>
                <span>{type.displayName}</span>
                <div className="bar-track">
                  <div
                    className="bar-fill"
                    style={{ width: `${Math.max(3, (type.count / maxTypeCount) * 100)}%`, background: type.color }}
                  />
                  <span className="bar-val">{type.count}</span>
                </div>
              </div>
            ))}
            <div className="fig-caption">
              Most common resource types — one fixed colour per service category, everywhere in the app.
            </div>
          </div>

          {growth ? (
            <div className="figure-block">
              <svg className="spark" viewBox="0 0 560 120" role="img" aria-label="Resource count across stored snapshots">
                <line x1="4" y1="102" x2="556" y2="102" stroke="var(--line-strong)" strokeWidth="1" />
                <path d={growth.line} fill="none" stroke="var(--accent)" strokeWidth="2" />
                {growth.points.map((point, index) => (
                  <circle
                    key={index}
                    cx={point.x}
                    cy={point.y}
                    r={index === growth.points.length - 1 ? 4 : 3}
                    fill="var(--accent)"
                  />
                ))}
                <text x={growth.points.at(-1)?.x ?? 0} y={(growth.points.at(-1)?.y ?? 12) - 8} textAnchor="end" fontSize="12" fontWeight="600" fill="var(--ink)">
                  {series.at(-1)?.resources}
                </text>
                <text x="4" y="116" fontSize="10.5" fill="var(--faintest)">
                  {shortDate(series[0]?.createdAt)}
                </text>
                <text x="556" y="116" textAnchor="end" fontSize="10.5" fill="var(--faintest)">
                  {shortDate(series.at(-1)?.createdAt)}
                </text>
              </svg>
              <div className="fig-caption">Resource count over the stored snapshots — one measure, so one hue.</div>
            </div>
          ) : null}

          <div className="figure-block">
            {topLocations.map((location) => (
              <div className="bar-row" key={location.name}>
                <span className="mono">{location.name}</span>
                <div className="bar-track">
                  <div
                    className="bar-fill"
                    style={{ width: `${Math.max(3, (location.count / maxLocationCount) * 100)}%`, height: 10 }}
                  />
                  <span className="bar-val">{location.count}</span>
                </div>
              </div>
            ))}
            <div className="fig-caption">Resources by region.</div>
          </div>
        </div>

        <div className="overview-side">
          <section>
            <h2>Findings that need attention</h2>
            <div className="ledger-rows">
              {attention.map((finding, index) => (
                <button
                  key={`${finding.queryName}-${index}`}
                  onClick={() => (finding.resourceId ? onOpenResource(finding.resourceId) : onOpenView("findings"))}
                >
                  <span className="sev" style={{ color: SEVERITY_SWATCH[finding.severity] }}>
                    {finding.severity.toUpperCase().slice(0, 4)}
                  </span>
                  <span>
                    <strong>{finding.title}</strong>
                    <small>
                      {finding.category} · <span className="mono">{finding.resourceId?.split("/").at(-1) ?? "estate-level"}</span>
                    </small>
                  </span>
                </button>
              ))}
              {attention.length === 0 ? <div><small>No findings in this snapshot.</small></div> : null}
            </div>
          </section>

          <section>
            <h2>Severity ledger</h2>
            <div className="severity-ledger">
              {(["high", "medium", "low", "info"] as const).map((severity) => (
                <div key={severity}>
                  <span className="swatch" style={{ background: SEVERITY_SWATCH[severity] }} />
                  <span>{severity[0].toUpperCase() + severity.slice(1)}</span>
                  <b>{estate.severityCounts[severity]}</b>
                </div>
              ))}
            </div>
            {previous && findingDelta !== 0 ? (
              <p className="trend-note" style={findingDelta > 0 ? { color: "var(--coral)" } : undefined}>
                {findingDelta > 0 ? `▲ ${findingDelta} more` : `▾ ${Math.abs(findingDelta)} fewer`} than the previous
                snapshot
              </p>
            ) : null}
          </section>

          <section>
            <h2>Collection health</h2>
            <p className="muted-copy" style={{ margin: "6px 0 0" }}>
              {estate.queryRuns.length - failedQueries} of {estate.queryRuns.length} queries succeeded
              {failedQueries > 0 ? ` · ${failedQueries} failed` : ""} — the full ledger lives under Changes.
            </p>
          </section>
        </div>
      </div>
    </div>
  );
}

function severityRank(severity: string) {
  return ["high", "medium", "low", "info"].indexOf(severity);
}

function shortDate(value?: string) {
  if (!value) return "";
  return new Intl.DateTimeFormat(undefined, { day: "2-digit", month: "short" }).format(new Date(value));
}

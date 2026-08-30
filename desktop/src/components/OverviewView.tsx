import { useMemo } from "react";
import { displayLocation } from "../azure-values";
import type { AppBootstrap, EstateSnapshot, ViewId } from "../types";
import { dayMonth, resourceName } from "../format";
import { SEVERITY_TOKEN, severityRank } from "../ordering";
import { DatabaseStamp, ViewHeading } from "./view-chrome";

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
  const topTypes = estate.resourceTypes.slice(0, 6);
  const maxTypeCount = Math.max(1, ...topTypes.map((type) => type.count));
  const topLocations = estate.locations.slice(0, 5);
  const maxLocationCount = Math.max(1, ...topLocations.map((location) => location.count));
  const attention = useMemo(
    () =>
      [...estate.findings]
        .sort((a, b) => severityRank(a.severity) - severityRank(b.severity))
        .slice(0, 5),
    [estate.findings],
  );
  const growth = series.length > 1 ? sparkPath(series.map((entry) => entry.resources), 560, 120) : undefined;

  return (
    <div className="overview-workspace">
      <ViewHeading
        title="Overview"
        description={
          <>
            {estate.tenantId} · {estate.totals.subscriptions} subscriptions · {estate.totals.resourceGroups} resource
            groups · {estate.locations.length} regions
          </>
        }
      >
        <DatabaseStamp
          label="Snapshot"
          value={<>{estate.id.slice(0, 8)} · {estate.status}</>}
        />
      </ViewHeading>

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
                  {dayMonth(series[0]?.createdAt)}
                </text>
                <text x="556" y="116" textAnchor="end" fontSize="10.5" fill="var(--faintest)">
                  {dayMonth(series.at(-1)?.createdAt)}
                </text>
              </svg>
              <div className="fig-caption">Resource count over the stored snapshots — one measure, so one hue.</div>
            </div>
          ) : null}

          <div className="figure-block">
            {topLocations.map((location) => (
              <div className="bar-row" key={location.name}>
                <span>{displayLocation(estate.azureMetadata, location.name, "Not stored")}</span>
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
                  <span className="sev" style={{ color: SEVERITY_TOKEN[finding.severity] }}>
                    {finding.severity.toUpperCase().slice(0, 4)}
                  </span>
                  <span>
                    <strong>{finding.title}</strong>
                    <small>
                      {finding.category} · <span className="mono">{finding.resourceId ? resourceName(finding.resourceId) : "estate-level"}</span>
                    </small>
                  </span>
                </button>
              ))}
              {attention.length === 0 ? <div><small>No findings in this snapshot.</small></div> : null}
            </div>
          </section>

          <button className="overview-all-findings" onClick={() => onOpenView("findings")}>Review all findings</button>
        </div>
      </div>
    </div>
  );
}



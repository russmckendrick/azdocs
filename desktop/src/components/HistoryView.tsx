import { useEffect, useMemo, useState } from "react";
import { CheckCircle2, CircleDashed, GitCompareArrows, Rows3 } from "lucide-react";
import { compareSnapshots } from "../api";
import type { AppBootstrap, DashboardFilter, EstateSnapshot, FieldChange, SnapshotComparison } from "../types";
import { dateTime, dayMonth, errorMessage, fill, plural, resourceName } from "../format";
import { useLabels } from "../labels";
import { ShowMore, useProgressiveList } from "./progressive-list";
import { DatabaseStamp, ErrorStrip, ViewHeading } from "./view-chrome";

type ChangeKind = "added" | "changed" | "removed";

function cell(value: unknown): string {
  if (value === null || value === undefined) return "—";
  return typeof value === "string" ? value : JSON.stringify(value);
}

function fieldPath(change: FieldChange) {
  return change.path ? `${change.field}.${change.path}` : change.field;
}

export function HistoryView({ bootstrap, estate, previousComparison, onLoadSnapshot, dashboardFilter, onOpenResource }: { bootstrap: AppBootstrap; estate: EstateSnapshot; previousComparison?: SnapshotComparison; onLoadSnapshot: (id: string) => void; dashboardFilter?: DashboardFilter; onOpenResource: (id: string) => void }) {
  const older = useMemo(
    () => bootstrap.snapshots.filter((snapshot) => snapshot.id !== estate.id),
    [bootstrap.snapshots, estate.id],
  );
  const [baseId, setBaseId] = useState<string>();
  // The last good comparison stays on screen when a later one fails.
  const [comparison, setComparison] = useState<SnapshotComparison | undefined>(previousComparison);
  const [comparing, setComparing] = useState(false);
  const [compareError, setCompareError] = useState<string>();
  const resourceById = useMemo(
    () => new Map(estate.resources.map((resource) => [resource.id, resource])),
    [estate.resources],
  );
  const { common, desktop: { history: words } } = useLabels();

  const activeBase = baseId ?? previousComparison?.baseSnapshotId;

  useEffect(() => {
    setBaseId(undefined);
    setCompareError(undefined);
    setComparison(previousComparison);
  }, [estate.id, previousComparison]);

  useEffect(() => {
    if (!baseId) return;
    let active = true;
    setComparing(true);
    setCompareError(undefined);
    compareSnapshots(baseId, estate.id)
      .then((next) => {
        if (active) setComparison(next);
      })
      .catch((caught) => {
        if (active) setCompareError(fill(words.compare_failed, { error: errorMessage(caught) }));
      })
      .finally(() => {
        if (active) setComparing(false);
      });
    return () => {
      active = false;
    };
  }, [baseId, estate.id, words.compare_failed]);

  const changes = useMemo(() => {
    if (!comparison) return [];
    const describe = (id: string, kind: ChangeKind) => {
      const resource = resourceById.get(id);
      return {
        id,
        kind,
        name: resource?.name ?? resourceName(id),
        detail: resource
          ? `${resource.azureType} · ${resource.resourceGroup ?? common.verdict.none}`
          : id.split("/providers/").at(-1) ?? id,
        fields: kind === "changed" ? (comparison.fields[id] ?? []) : [],
      };
    };
    return [
      ...comparison.added.map((id) => describe(id, "added")),
      ...comparison.changed.map((id) => describe(id, "changed")),
      ...comparison.removed.map((id) => describe(id, "removed")),
    ];
  }, [comparison, resourceById, common.verdict.none]);
  const failedQueries = estate.queryRuns.filter((run) => run.error).length;
  const list = useProgressiveList(changes.filter(change => !dashboardFilter?.changeKind || change.kind === dashboardFilter.changeKind), [comparison?.baseSnapshotId, comparison?.targetSnapshotId, dashboardFilter], 250);
  const visibleChanges = list.visible;
  // The trend never includes running or failed snapshots; the Rust side
  // already filtered, so nothing here decides eligibility.
  const trend = estate.trend;

  return (
    <div className="history-workspace">
      <ViewHeading
        title={words.title}
        description={words.description}
        modifier="history-heading"
      >
        <DatabaseStamp
          icon={<Rows3 size={17} />}
          label={words.source_stamp}
          value={resourceName(bootstrap.databasePath)}
        />
      </ViewHeading>
      {compareError ? <ErrorStrip message={compareError} onDismiss={() => setCompareError(undefined)} /> : null}
      <div className="history-columns">
        <section className="snapshot-ledger">
          <div className="snapshot-head"><span>{words.captured}</span><span>{words.estate}</span><span>{words.findings}</span><span>{words.status}</span></div>
          {bootstrap.snapshots.map((snapshot) => (
            <button key={snapshot.id} className={snapshot.id === estate.id ? "snapshot-row active" : "snapshot-row"} aria-current={snapshot.id === estate.id ? "true" : undefined} onClick={() => onLoadSnapshot(snapshot.id)}>
              <span className="snapshot-time"><i /><span><strong>{dateTime(snapshot.createdAt)}</strong><small>{snapshot.notes ?? snapshot.id.slice(0, 8)}</small></span></span>
              <span>{fill(words.resources, { count: snapshot.resources })}<small>{fill(words.subscriptions, { count: snapshot.subscriptions })}</small></span>
              <span>{snapshot.findings}<small>{words.audit_signals}</small></span>
              <span className={`snapshot-status ${snapshot.status}`}>{snapshot.status === "complete" ? <CheckCircle2 size={14} /> : <CircleDashed size={14} />}{snapshot.status}</span>
            </button>
          ))}
          {trend.length > 1 ? (
            <div className="history-trend">
              <h3>{words.trend}</h3>
              <p className="muted-copy">{words.trend_note}</p>
              <div className="data-grid-wrap">
                <table className="data-grid">
                  <thead>
                    <tr>
                      <th>{common.cover.collected}</th>
                      <th>{common.columns.status}</th>
                      <th className="numeric">{common.columns.resources}</th>
                      <th className="numeric">{common.columns.tagged}</th>
                      <th className="numeric">{common.columns.findings}</th>
                      <th className="numeric">{common.severity.high.label}</th>
                      <th className="numeric">{common.severity.medium.label}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {trend.map((point) => (
                      <tr key={point.snapshotId} className={point.snapshotId === estate.id ? "current" : undefined}>
                        <td>{dateTime(point.createdAt)}</td>
                        <td>{point.status}</td>
                        <td className="numeric mono-cell">{point.resources}</td>
                        <td className="numeric mono-cell">{point.tagged}</td>
                        <td className="numeric mono-cell">{point.findings}</td>
                        <td className="numeric mono-cell">{point.high}</td>
                        <td className="numeric mono-cell">{point.medium}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          ) : null}
        </section>
        <aside className="change-plate">
          <header>
            <GitCompareArrows size={18} />
            <div>
              <h2>{words.what_changed}</h2>
              <p>{comparison ? `${comparison.baseSnapshotId.slice(0, 8)} → ${comparison.targetSnapshotId.slice(0, 8)}` : words.earliest}</p>
            </div>
          </header>
          {older.length > 0 ? (
            <div className="compare-controls">
              <span>{words.base}</span>
              <select
                value={activeBase ?? ""}
                onChange={(event) => setBaseId(event.target.value)}
                aria-label={words.base_aria}
              >
                {older.map((snapshot) => (
                  <option key={snapshot.id} value={snapshot.id}>
                    {fill(words.base_option, { date: dayMonth(snapshot.createdAt), count: snapshot.resources })}
                  </option>
                ))}
              </select>
              <span>{fill(words.target, { date: dayMonth(estate.createdAt) })}</span>
              {comparing ? <span>{words.comparing}</span> : null}
            </div>
          ) : null}
          {comparison ? (
            <>
              <div className="change-totals">
                <div className="added"><strong>{comparison.added.length}</strong><span>{words.added}</span></div>
                <div className="changed"><strong>{comparison.changed.length}</strong><span>{words.changed}</span></div>
                <div className="removed"><strong>{comparison.removed.length}</strong><span>{words.removed}</span></div>
              </div>
              <div className="change-list">
                {visibleChanges.map((item) => (
                  <div key={`${item.kind}-${item.id}`} className={item.fields.length ? "with-fields" : undefined}>
                    <i className={item.kind} />
                    {item.kind === "changed" ? (
                      <details className="change-fields">
                        <summary>
                          <span><strong>{resourceById.has(item.id) ? <button className="text-link" onClick={(event) => { event.preventDefault(); onOpenResource(item.id); }}>{item.name}</button> : item.name}</strong><small>{item.detail}</small></span>
                          <em>{item.fields.length ? plural(words.field_count, item.fields.length) : words.kinds[item.kind]}</em>
                        </summary>
                        {item.fields.length ? (
                          <table className="data-grid">
                            <thead><tr><th>{common.columns.field}</th><th>{common.columns.before}</th><th>{common.columns.after}</th></tr></thead>
                            <tbody>
                              {item.fields.map((change) => (
                                <tr key={fieldPath(change)}>
                                  <td className="mono-cell">{fieldPath(change)}</td>
                                  <td>{cell(change.before)}</td>
                                  <td>{cell(change.after)}</td>
                                </tr>
                              ))}
                            </tbody>
                          </table>
                        ) : <p className="muted-copy">{words.no_field_changes}</p>}
                      </details>
                    ) : (
                      <>
                        <span><strong>{resourceById.has(item.id) ? <button className="text-link" onClick={() => onOpenResource(item.id)}>{item.name}</button> : item.name}</strong><small>{item.detail}</small></span>
                        <em>{words.kinds[item.kind] ?? item.kind}</em>
                      </>
                    )}
                  </div>
                ))}
                <ShowMore list={list} />
              </div>
              {(["findingsAdded", "findingsResolved"] as const).map((key) =>
                comparison[key].length ? (
                  <section key={key} className="change-findings">
                    <h3>{key === "findingsAdded" ? words.new_findings : words.resolved_findings}</h3>
                    {comparison[key].map((finding, index) => (
                      <div key={`${finding.queryName}-${finding.resourceId}-${index}`}>
                        <span className={`severity-marker ${finding.severity}`}>{common.severity[finding.severity].name}</span>
                        <span><strong>{finding.title}</strong><small>{finding.category} · {finding.resourceId ? resourceName(finding.resourceId) : words.earliest}</small></span>
                      </div>
                    ))}
                  </section>
                ) : null,
              )}
              {comparison.edgesAdded.length || comparison.edgesRemoved.length ? (
                <section className="change-findings">
                  {comparison.edgesAdded.length ? <h3>{words.relationships_added}</h3> : null}
                  {comparison.edgesAdded.map((edge) => (
                    <div key={`a-${edge.sourceId}-${edge.kind}-${edge.targetId}`}><span><strong>{resourceName(edge.sourceId)}</strong><small>{common.columns.kind}: {edge.kind} → {resourceName(edge.targetId)}</small></span></div>
                  ))}
                  {comparison.edgesRemoved.length ? <h3>{words.relationships_removed}</h3> : null}
                  {comparison.edgesRemoved.map((edge) => (
                    <div key={`r-${edge.sourceId}-${edge.kind}-${edge.targetId}`}><span><strong>{resourceName(edge.sourceId)}</strong><small>{common.columns.kind}: {edge.kind} → {resourceName(edge.targetId)}</small></span></div>
                  ))}
                </section>
              ) : null}
            </>
          ) : <p className="muted-copy">{words.no_older}</p>}
          <details className="query-health" open={dashboardFilter?.healthOnly || undefined}>
            <summary>
              {words.health}
              <span>{fill(words.health_summary, { succeeded: estate.queryRuns.length - failedQueries, total: estate.queryRuns.length })}</span>
            </summary>
            <div className="query-health-list">
              {estate.queryRuns.filter(run => (!dashboardFilter?.category || run.category === dashboardFilter.category) && (!dashboardFilter?.queryName || run.queryName === dashboardFilter.queryName)).map((run) => <div key={run.queryName}><span><i className={run.error ? "failed" : "complete"} />{run.queryName}</span><small>{run.error ?? fill(words.run_detail, { rows: run.rowCount ?? 0, ms: run.durationMs ?? 0 })}</small></div>)}
            </div>
          </details>
        </aside>
      </div>
    </div>
  );
}

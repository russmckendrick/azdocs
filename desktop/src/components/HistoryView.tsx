import { useEffect, useMemo, useState } from "react";
import { CheckCircle2, CircleDashed, GitCompareArrows, Rows3 } from "lucide-react";
import { compareSnapshots } from "../api";
import type { AppBootstrap, EstateSnapshot, SnapshotComparison } from "../types";
import { dateTime, dayMonth, fill, resourceName } from "../format";
import { useLabels } from "../labels";
import { ShowMore, useProgressiveList } from "./progressive-list";
import { DatabaseStamp, ViewHeading } from "./view-chrome";






export function HistoryView({ bootstrap, estate, onLoadSnapshot }: { bootstrap: AppBootstrap; estate: EstateSnapshot; onLoadSnapshot: (id: string) => void }) {
  const older = useMemo(
    () => bootstrap.snapshots.filter((snapshot) => snapshot.id !== estate.id),
    [bootstrap.snapshots, estate.id],
  );
  const [baseId, setBaseId] = useState<string>();
  const [comparison, setComparison] = useState<SnapshotComparison | undefined>(estate.previousDiff ?? undefined);
  const [comparing, setComparing] = useState(false);
  const resourceById = useMemo(
    () => new Map(estate.resources.map((resource) => [resource.id, resource])),
    [estate.resources],
  );
  const { common, desktop: { history: words } } = useLabels();

  const activeBase = baseId ?? estate.previousDiff?.baseSnapshotId;

  useEffect(() => {
    setBaseId(undefined);
    setComparison(estate.previousDiff ?? undefined);
  }, [estate.id, estate.previousDiff]);

  useEffect(() => {
    if (!baseId) return;
    let active = true;
    setComparing(true);
    compareSnapshots(baseId, estate.id)
      .then((next) => {
        if (active) setComparison(next);
      })
      .catch(() => {
        if (active) setComparison(undefined);
      })
      .finally(() => {
        if (active) setComparing(false);
      });
    return () => {
      active = false;
    };
  }, [baseId, estate.id]);

  const changes = useMemo(() => {
    if (!comparison) return [];
    const describe = (id: string, kind: "added" | "changed" | "removed") => {
      const resource = resourceById.get(id);
      return {
        id,
        kind,
        name: resource?.name ?? resourceName(id),
        detail: resource
          ? `${resource.azureType} · ${resource.resourceGroup ?? common.verdict.none}`
          : id.split("/providers/").at(-1) ?? id,
      };
    };
    return [
      ...comparison.added.map((id) => describe(id, "added")),
      ...comparison.changed.map((id) => describe(id, "changed")),
      ...comparison.removed.map((id) => describe(id, "removed")),
    ];
  }, [comparison, resourceById, common.verdict.none]);
  const failedQueries = estate.queryRuns.filter((run) => run.error).length;
  const list = useProgressiveList(changes, [comparison?.baseSnapshotId, comparison?.targetSnapshotId], 250);
  const visibleChanges = list.visible;

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
      <div className="history-columns">
        <section className="snapshot-ledger">
          <div className="snapshot-head"><span>{words.captured}</span><span>{words.estate}</span><span>{words.findings}</span><span>{words.status}</span></div>
          {bootstrap.snapshots.map((snapshot) => (
            <button key={snapshot.id} className={snapshot.id === estate.id ? "snapshot-row active" : "snapshot-row"} onClick={() => onLoadSnapshot(snapshot.id)}>
              <span className="snapshot-time"><i /><span><strong>{dateTime(snapshot.createdAt)}</strong><small>{snapshot.notes ?? snapshot.id.slice(0, 8)}</small></span></span>
              <span>{fill(words.resources, { count: snapshot.resources })}<small>{fill(words.subscriptions, { count: snapshot.subscriptions })}</small></span>
              <span>{snapshot.findings}<small>{words.audit_signals}</small></span>
              <span className={`snapshot-status ${snapshot.status}`}>{snapshot.status === "complete" ? <CheckCircle2 size={14} /> : <CircleDashed size={14} />}{snapshot.status}</span>
            </button>
          ))}
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
                  <div key={`${item.kind}-${item.id}`}><i className={item.kind} /><span><strong>{item.name}</strong><small>{item.detail}</small></span><em>{words.kinds[item.kind] ?? item.kind}</em></div>
                ))}
                <ShowMore list={list} />
              </div>
            </>
          ) : <p className="muted-copy">{words.no_older}</p>}
          <details className="query-health">
            <summary>
              {words.health}
              <span>{fill(words.health_summary, { succeeded: estate.queryRuns.length - failedQueries, total: estate.queryRuns.length })}</span>
            </summary>
            <div className="query-health-list">
              {estate.queryRuns.map((run) => <div key={run.queryName}><span><i className={run.error ? "failed" : "complete"} />{run.queryName}</span><small>{run.error ?? fill(words.run_detail, { rows: run.rowCount ?? 0, ms: run.durationMs ?? 0 })}</small></div>)}
            </div>
          </details>
        </aside>
      </div>
    </div>
  );
}

import { useEffect, useMemo, useState } from "react";
import { CheckCircle2, CircleDashed, GitCompareArrows, Rows3 } from "lucide-react";
import { compareSnapshots } from "../api";
import type { AppBootstrap, EstateSnapshot, SnapshotComparison } from "../types";

const CHANGE_BATCH = 250;

function dateTime(value: string) {
  return new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" }).format(new Date(value));
}

function compactDate(value: string) {
  return new Intl.DateTimeFormat(undefined, { day: "2-digit", month: "short" }).format(new Date(value));
}

export function HistoryView({ bootstrap, estate, onLoadSnapshot }: { bootstrap: AppBootstrap; estate: EstateSnapshot; onLoadSnapshot: (id: string) => void }) {
  const older = useMemo(
    () => bootstrap.snapshots.filter((snapshot) => snapshot.id !== estate.id),
    [bootstrap.snapshots, estate.id],
  );
  const [baseId, setBaseId] = useState<string>();
  const [comparison, setComparison] = useState<SnapshotComparison | undefined>(estate.previousDiff);
  const [comparing, setComparing] = useState(false);
  const [visibleChangeCount, setVisibleChangeCount] = useState(CHANGE_BATCH);
  const resourceById = useMemo(
    () => new Map(estate.resources.map((resource) => [resource.id, resource])),
    [estate.resources],
  );

  const activeBase = baseId ?? estate.previousDiff?.baseSnapshotId;

  useEffect(() => {
    setBaseId(undefined);
    setComparison(estate.previousDiff);
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
    const describe = (id: string, kind: string) => {
      const resource = resourceById.get(id);
      return {
        id,
        kind,
        name: resource?.name ?? id.split("/").at(-1) ?? id,
        detail: resource
          ? `${resource.azureType} · ${resource.resourceGroup ?? "—"}`
          : id.split("/providers/").at(-1) ?? id,
      };
    };
    return [
      ...comparison.added.map((id) => describe(id, "added")),
      ...comparison.changed.map((id) => describe(id, "changed")),
      ...comparison.removed.map((id) => describe(id, "removed")),
    ];
  }, [comparison, resourceById]);
  const failedQueries = estate.queryRuns.filter((run) => run.error).length;
  const visibleChanges = changes.slice(0, visibleChangeCount);

  useEffect(() => setVisibleChangeCount(CHANGE_BATCH), [comparison?.baseSnapshotId, comparison?.targetSnapshotId]);

  return (
    <div className="history-workspace">
      <header className="view-heading history-heading">
        <div>
          <h1>Changes</h1>
          <p>Immutable estate observations, newest first. Compare any two without querying Azure.</p>
        </div>
        <div className="database-stamp"><Rows3 size={17} /><span><small>SQLite source</small><strong>{bootstrap.databasePath.split("/").at(-1)}</strong></span></div>
      </header>
      <div className="history-columns">
        <section className="snapshot-ledger">
          <div className="snapshot-head"><span>Captured</span><span>Estate</span><span>Findings</span><span>Status</span></div>
          {bootstrap.snapshots.map((snapshot) => (
            <button key={snapshot.id} className={snapshot.id === estate.id ? "snapshot-row active" : "snapshot-row"} onClick={() => onLoadSnapshot(snapshot.id)}>
              <span className="snapshot-time"><i /><span><strong>{dateTime(snapshot.createdAt)}</strong><small>{snapshot.notes ?? snapshot.id.slice(0, 8)}</small></span></span>
              <span>{snapshot.resources} resources<small>{snapshot.subscriptions} subscriptions</small></span>
              <span>{snapshot.findings}<small>audit signals</small></span>
              <span className={`snapshot-status ${snapshot.status}`}>{snapshot.status === "complete" ? <CheckCircle2 size={14} /> : <CircleDashed size={14} />}{snapshot.status}</span>
            </button>
          ))}
        </section>
        <aside className="change-plate">
          <header>
            <GitCompareArrows size={18} />
            <div>
              <h2>What changed</h2>
              <p>{comparison ? `${comparison.baseSnapshotId.slice(0, 8)} → ${comparison.targetSnapshotId.slice(0, 8)}` : "This is the earliest stored snapshot"}</p>
            </div>
          </header>
          {older.length > 0 ? (
            <div className="compare-controls">
              <span>Base</span>
              <select
                value={activeBase ?? ""}
                onChange={(event) => setBaseId(event.target.value)}
                aria-label="Base snapshot to compare against"
              >
                {older.map((snapshot) => (
                  <option key={snapshot.id} value={snapshot.id}>
                    {compactDate(snapshot.createdAt)} · {snapshot.resources} resources
                  </option>
                ))}
              </select>
              <span>→ {compactDate(estate.createdAt)} (open)</span>
              {comparing ? <span>comparing…</span> : null}
            </div>
          ) : null}
          {comparison ? (
            <>
              <div className="change-totals">
                <div className="added"><strong>{comparison.added.length}</strong><span>Added</span></div>
                <div className="changed"><strong>{comparison.changed.length}</strong><span>Changed</span></div>
                <div className="removed"><strong>{comparison.removed.length}</strong><span>Removed</span></div>
              </div>
              <div className="change-list">
                {visibleChanges.map((item) => (
                  <div key={`${item.kind}-${item.id}`}><i className={item.kind} /><span><strong>{item.name}</strong><small>{item.detail}</small></span><em>{item.kind}</em></div>
                ))}
                {visibleChanges.length < changes.length ? (
                  <button className="load-more" onClick={() => setVisibleChangeCount((current) => current + CHANGE_BATCH)}>
                    Show {Math.min(CHANGE_BATCH, changes.length - visibleChanges.length)} more
                    <span>{visibleChanges.length.toLocaleString()} of {changes.length.toLocaleString()} loaded</span>
                  </button>
                ) : null}
              </div>
            </>
          ) : <p className="muted-copy">There is no older snapshot to compare against.</p>}
          <details className="query-health">
            <summary>
              Collection health
              <span>{estate.queryRuns.length - failedQueries} of {estate.queryRuns.length} queries succeeded</span>
            </summary>
            <div className="query-health-list">
              {estate.queryRuns.map((run) => <div key={run.queryName}><span><i className={run.error ? "failed" : "complete"} />{run.queryName}</span><small>{run.error ?? `${run.rowCount ?? 0} rows · ${run.durationMs ?? 0} ms`}</small></div>)}
            </div>
          </details>
        </aside>
      </div>
    </div>
  );
}

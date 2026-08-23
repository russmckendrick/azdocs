import { ArrowDown, ArrowRight, ArrowUp, CheckCircle2, CircleDashed, GitCompareArrows, Rows3 } from "lucide-react";
import type { AppBootstrap, EstateSnapshot } from "../types";

function dateTime(value: string) {
  return new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" }).format(new Date(value));
}

export function HistoryView({ bootstrap, estate, onLoadSnapshot }: { bootstrap: AppBootstrap; estate: EstateSnapshot; onLoadSnapshot: (id: string) => void }) {
  const diff = estate.previousDiff;
  return (
    <div className="history-workspace">
      <header className="view-heading history-heading">
        <div><h1>Snapshot history</h1><p>Immutable estate observations, newest first. Compare what changed without querying Azure.</p></div>
        <div className="database-stamp"><Rows3 size={18} /><span><small>SQLite source</small><strong>{bootstrap.databasePath.split("/").at(-1)}</strong></span></div>
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
          <header><GitCompareArrows size={20} /><div><h2>Change since previous</h2><p>{diff ? `${diff.baseSnapshotId.slice(0, 8)} → ${diff.targetSnapshotId.slice(0, 8)}` : "This is the earliest stored snapshot"}</p></div></header>
          {diff ? (
            <>
              <div className="change-totals">
                <div className="added"><ArrowUp size={17} /><strong>{diff.added.length}</strong><span>Added</span></div>
                <div className="changed"><ArrowRight size={17} /><strong>{diff.changed.length}</strong><span>Changed</span></div>
                <div className="removed"><ArrowDown size={17} /><strong>{diff.removed.length}</strong><span>Removed</span></div>
              </div>
              <div className="change-list">
                {[...diff.added.map((id) => ({ id, kind: "added" })), ...diff.changed.map((id) => ({ id, kind: "changed" })), ...diff.removed.map((id) => ({ id, kind: "removed" }))].map((item) => (
                  <div key={`${item.kind}-${item.id}`}><i className={item.kind} /><span><strong>{item.id.split("/").at(-1)}</strong><small>{item.id.split("/providers/").at(-1)}</small></span><em>{item.kind}</em></div>
                ))}
              </div>
            </>
          ) : <p className="muted-copy">There is no older snapshot to compare against.</p>}
          <section className="query-health"><h3>Collection health</h3>{estate.queryRuns.map((run) => <div key={run.queryName}><span><i className={run.error ? "failed" : "complete"} />{run.queryName}</span><small>{run.error ?? `${run.rowCount ?? 0} rows · ${run.durationMs ?? 0} ms`}</small></div>)}</section>
        </aside>
      </div>
    </div>
  );
}

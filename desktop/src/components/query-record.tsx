import type { QueryRun } from "../types";
import { useLabels } from "../labels";

/**
 * The exact KQL, scope and sources a stored table came from. Shared by the
 * Estate type tables and the Query results browser so both cite a query the
 * same way.
 */
export function QueryRecord({ run }: { run?: QueryRun }) {
  const { desktop: { inventory: words } } = useLabels();
  return (
    <details className="inventory-evidence">
      <summary>{words.provenance}</summary>
      {run?.provenance ? (
        <div className="inventory-evidence-body">
          <dl>
            <dt>{words.scope}</dt>
            <dd>{run.provenance.authorizationScope}</dd>
            <dt>{words.subscriptions}</dt>
            <dd>{run.provenance.subscriptions.join(", ") || words.all_visible}</dd>
            <dt>{words.query_hash}</dt>
            <dd className="mono">{run.provenance.kqlSha256}</dd>
            {!!run.provenance.sourceUrls.length && (
              <>
                <dt>{words.source}</dt>
                <dd>{run.provenance.sourceUrls.map((url) => <p key={url}>{url}</p>)}</dd>
              </>
            )}
            {run.provenance.reviewedOn && (
              <>
                <dt>{words.source_reviewed}</dt>
                <dd>{run.provenance.reviewedOn}</dd>
              </>
            )}
          </dl>
          <pre>{run.provenance.kql}</pre>
        </div>
      ) : (
        <p className="muted-copy">{words.provenance_missing}</p>
      )}
    </details>
  );
}

import type { EstateSnapshot } from "../types";
import { fill, fillNodes } from "../format";
import { useLabels } from "../labels";
import { ViewHeading } from "./view-chrome";

/**
 * Tag governance, drawn from `estate.governance`.
 *
 * The analysis behind this — the key and subscription coverage, who is missing
 * a required tag, which groups are worst and which are past the threshold — is
 * `azdocs::report::governance`, the same code the printed report renders. This
 * view used to compute all of it in a `useMemo` against the *current* config,
 * so editing `required_tags` changed what an old snapshot appeared to contain
 * while the stored findings said otherwise. `requiredTags` survives here only
 * to say whether a sweep is configured at all, which is genuinely config.
 */
export function GovernanceView({
  estate,
  requiredTags,
  onOpenFindings,
}: {
  estate: EstateSnapshot;
  requiredTags: string[];
  onOpenFindings: () => void;
}) {
  const { governance } = estate;
  const enforced = requiredTags.length > 0;
  const governanceFindings = estate.findings.filter((finding) => finding.category === "governance").length;
  const { common, desktop: { governance: words } } = useLabels();
  const columns = common.columns;

  return (
    <div className="governance-workspace">
      <ViewHeading
        title={words.title}
        description={
          enforced
            ? fillNodes(words.required_tags_from, {
                tags: <span className="mono">{requiredTags.join(" · ")}</span>,
              })
            : words.not_enforced
        }
      />

      <div className="stat-strip">
        <div className="stat-cell">
          <strong>{estate.tagCoverage.percent}%</strong>
          <span>{columns.tag_coverage}</span>
        </div>
        <div className="stat-cell">
          <strong>{governance.distinctKeys}</strong>
          <span>{columns.distinct_keys}</span>
        </div>
        <div className="stat-cell">
          <strong className={governance.nonCompliant > 0 ? "risk" : undefined}>
            {enforced ? governance.nonCompliant : common.verdict.none}
          </strong>
          <span>{columns.non_compliant}</span>
        </div>
        <button className="stat-cell" onClick={onOpenFindings}>
          <strong>{governanceFindings}</strong>
          <span>{words.governance_findings}</span>
        </button>
      </div>

      <div className="governance-columns">
        <div className="figure-block">
          <h2 className="figure-title">{common.governance.coverage_by_key}</h2>
          {governance.topKeys.map((entry) => (
            <div className="meter-row" key={entry.key}>
              <span className="meter-key" title={entry.key}>{entry.key}</span>
              <div className="meter">
                <div style={{ width: `${Math.max(2, entry.percent)}%` }} />
              </div>
              <span className="meter-val">{entry.percent}%</span>
            </div>
          ))}
          {governance.topKeys.length === 0 ? <p className="muted-copy">{common.governance.no_tags}</p> : null}
          <div className="fig-caption">{fill(words.key_caption, { tagged: estate.tagCoverage.tagged })}</div>
        </div>
        <div className="figure-block">
          <h2 className="figure-title">{common.governance.coverage_by_subscription}</h2>
          {governance.subscriptions.map((entry) => (
            <div className="meter-row" key={entry.subscriptionId}>
              <span className="meter-key sans" title={entry.displayName}>{entry.displayName}</span>
              <div className="meter">
                <div
                  style={{
                    width: `${Math.max(2, entry.percent)}%`,
                    background: entry.healthy ? "var(--green)" : "var(--amber)",
                  }}
                />
              </div>
              <span className="meter-val">{entry.percent}%</span>
            </div>
          ))}
          <div className="fig-caption">{words.subscription_caption}</div>
        </div>
      </div>

      {enforced ? (
        <div className="gov-table-wrap">
          <table className="data-grid">
            <thead>
              <tr>
                <th>{columns.resource_group}</th>
                <th>{columns.subscription}</th>
                <th className="numeric">{columns.resources}</th>
                <th className="numeric">{columns.non_compliant}</th>
                <th>{columns.missed_tags}</th>
              </tr>
            </thead>
            <tbody>
              {governance.worstGroups.map((group) => (
                <tr key={`${group.subscriptionName}-${group.name}`}>
                  <td>{group.name}</td>
                  <td className="secondary">{group.subscriptionName}</td>
                  <td className="mono-cell numeric">{group.resources}</td>
                  <td className={group.flagged ? "numeric emphatic flagged" : "numeric emphatic"}>
                    {group.nonCompliant}
                  </td>
                  <td className="missed-tags">
                    {group.missedTags.map((tag) => (
                      <span key={tag} className="tag-chip missing">
                        {tag}
                      </span>
                    ))}
                  </td>
                </tr>
              ))}
              {governance.worstGroups.length === 0 ? (
                <tr>
                  <td colSpan={5} className="all-clear">{words.all_clear}</td>
                </tr>
              ) : null}
            </tbody>
          </table>
          <div className="fig-caption spaced">
            {fillNodes(words.worst_caption, { audit: <span className="mono upright">missing_required_tags</span> })}
          </div>
        </div>
      ) : null}
    </div>
  );
}

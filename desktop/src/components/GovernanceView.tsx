import type { EstateSnapshot } from "../types";
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

  return (
    <div className="governance-workspace">
      <ViewHeading
        title="Governance & tags"
        description={
          enforced ? (
            <>
              Required tags from azdocs.toml: <span className="mono">{requiredTags.join(" · ")}</span>
            </>
          ) : (
            "No required tags are configured — coverage is reported, compliance is not enforced."
          )
        }
      />

      <div className="stat-strip">
        <div className="stat-cell">
          <strong>{estate.tagCoverage.percent}%</strong>
          <span>Tag coverage</span>
        </div>
        <div className="stat-cell">
          <strong>{governance.distinctKeys}</strong>
          <span>Distinct keys</span>
        </div>
        <div className="stat-cell">
          <strong className={governance.nonCompliant > 0 ? "risk" : undefined}>
            {enforced ? governance.nonCompliant : "—"}
          </strong>
          <span>Non-compliant</span>
        </div>
        <button className="stat-cell" onClick={onOpenFindings}>
          <strong>{governanceFindings}</strong>
          <span>Governance findings</span>
        </button>
      </div>

      <div className="governance-columns">
        <div className="figure-block">
          <h2 className="figure-title">Coverage by tag key</h2>
          {governance.topKeys.map((entry) => (
            <div className="meter-row" key={entry.key}>
              <span className="meter-key" title={entry.key}>{entry.key}</span>
              <div className="meter">
                <div style={{ width: `${Math.max(2, entry.percent)}%` }} />
              </div>
              <span className="meter-val">{entry.percent}%</span>
            </div>
          ))}
          {governance.topKeys.length === 0 ? <p className="muted-copy">No tags are stored in this snapshot.</p> : null}
          <div className="fig-caption">
            Share of the {estate.tagCoverage.tagged} tagged resources carrying each key. One measure, one hue.
          </div>
        </div>
        <div className="figure-block">
          <h2 className="figure-title">Coverage by subscription</h2>
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
          <div className="fig-caption">Any tag counts here; the required-tag sweep below is stricter.</div>
        </div>
      </div>

      {enforced ? (
        <div className="gov-table-wrap">
          <table className="data-grid">
            <thead>
              <tr>
                <th>Resource group</th>
                <th>Subscription</th>
                <th className="numeric">Resources</th>
                <th className="numeric">Non-compliant</th>
                <th>Missed tags</th>
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
                  <td colSpan={5} className="all-clear">
                    Every resource carries all required tags.
                  </td>
                </tr>
              ) : null}
            </tbody>
          </table>
          <div className="fig-caption spaced">
            Least compliant resource groups, from the <span className="mono upright">missing_required_tags</span> audit rules.
          </div>
        </div>
      ) : null}
    </div>
  );
}

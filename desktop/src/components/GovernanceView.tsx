import { useMemo } from "react";
import type { EstateSnapshot } from "../types";
import { ViewHeading } from "./view-chrome";
import { useSubscriptionNames } from "../estate-lookups";

interface GroupCompliance {
  name: string;
  subscriptionName: string;
  resources: number;
  nonCompliant: number;
  missedTags: string[];
}

export function GovernanceView({
  estate,
  requiredTags,
  onOpenFindings,
}: {
  estate: EstateSnapshot;
  requiredTags: string[];
  onOpenFindings: () => void;
}) {
  const subscriptionNames = useSubscriptionNames(estate);

  const analysis = useMemo(() => {
    const keyCounts = new Map<string, number>();
    let tagged = 0;
    const subscriptionTotals = new Map<string, { total: number; tagged: number }>();
    const groupStats = new Map<string, GroupCompliance>();
    let nonCompliant = 0;

    for (const resource of estate.resources) {
      const tags = Object.keys(resource.tags ?? {});
      const hasTags = tags.length > 0;
      if (hasTags) tagged += 1;
      for (const key of tags) keyCounts.set(key, (keyCounts.get(key) ?? 0) + 1);

      const subscription = subscriptionTotals.get(resource.subscriptionId) ?? { total: 0, tagged: 0 };
      subscription.total += 1;
      if (hasTags) subscription.tagged += 1;
      subscriptionTotals.set(resource.subscriptionId, subscription);

      const missing = requiredTags.filter((required) => !tags.includes(required));
      if (requiredTags.length > 0 && missing.length > 0) {
        nonCompliant += 1;
        const groupKey = `${resource.subscriptionId}/${resource.resourceGroup ?? "—"}`;
        const group = groupStats.get(groupKey) ?? {
          name: resource.resourceGroup ?? "—",
          subscriptionName: subscriptionNames.get(resource.subscriptionId) ?? resource.subscriptionId,
          resources: 0,
          nonCompliant: 0,
          missedTags: [],
        };
        group.nonCompliant += 1;
        for (const tag of missing) if (!group.missedTags.includes(tag)) group.missedTags.push(tag);
        groupStats.set(groupKey, group);
      }
    }

    // Group sizes come from the full resource list, not just the offenders.
    for (const resource of estate.resources) {
      const groupKey = `${resource.subscriptionId}/${resource.resourceGroup ?? "—"}`;
      const group = groupStats.get(groupKey);
      if (group) group.resources += 1;
    }

    const topKeys = [...keyCounts.entries()]
      .sort(([, a], [, b]) => b - a)
      .slice(0, 6)
      .map(([key, count]) => ({ key, count, percent: tagged > 0 ? Math.round((count / tagged) * 100) : 0 }));

    const subscriptions = [...subscriptionTotals.entries()].map(([id, totals]) => ({
      name: subscriptionNames.get(id) ?? id,
      percent: totals.total > 0 ? Math.round((totals.tagged / totals.total) * 100) : 0,
    }));

    const worstGroups = [...groupStats.values()]
      .sort((a, b) => b.nonCompliant - a.nonCompliant || a.name.localeCompare(b.name))
      .slice(0, 6);

    return { keyCount: keyCounts.size, tagged, topKeys, subscriptions, nonCompliant, worstGroups };
  }, [estate.resources, requiredTags, subscriptionNames]);

  const governanceFindings = estate.findings.filter((finding) => finding.category === "governance").length;

  return (
    <div className="governance-workspace">
      <ViewHeading
        title="Governance & tags"
        description={
          requiredTags.length > 0 ? (
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
          <strong>{analysis.keyCount}</strong>
          <span>Distinct keys</span>
        </div>
        <div className="stat-cell">
          <strong className={analysis.nonCompliant > 0 ? "risk" : undefined}>
            {requiredTags.length > 0 ? analysis.nonCompliant : "—"}
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
          {analysis.topKeys.map((entry) => (
            <div className="meter-row" key={entry.key}>
              <span className="meter-key" title={entry.key}>{entry.key}</span>
              <div className="meter">
                <div style={{ width: `${Math.max(2, entry.percent)}%` }} />
              </div>
              <span className="meter-val">{entry.percent}%</span>
            </div>
          ))}
          {analysis.topKeys.length === 0 ? <p className="muted-copy">No tags are stored in this snapshot.</p> : null}
          <div className="fig-caption">
            Share of the {analysis.tagged} tagged resources carrying each key. One measure, one hue.
          </div>
        </div>
        <div className="figure-block">
          <h2 className="figure-title">Coverage by subscription</h2>
          {analysis.subscriptions.map((entry) => (
            <div className="meter-row" key={entry.name}>
              <span className="meter-key sans" title={entry.name}>{entry.name}</span>
              <div className="meter">
                <div
                  style={{
                    width: `${Math.max(2, entry.percent)}%`,
                    background: entry.percent >= 60 ? "var(--green)" : "var(--amber)",
                  }}
                />
              </div>
              <span className="meter-val">{entry.percent}%</span>
            </div>
          ))}
          <div className="fig-caption">Any tag counts here; the required-tag sweep below is stricter.</div>
        </div>
      </div>

      {requiredTags.length > 0 ? (
        <div className="gov-table-wrap">
          <table className="data-grid">
            <thead>
              <tr>
                <th>Resource group</th>
                <th>Subscription</th>
                <th style={{ textAlign: "right" }}>Resources</th>
                <th style={{ textAlign: "right" }}>Non-compliant</th>
                <th>Missed tags</th>
              </tr>
            </thead>
            <tbody>
              {analysis.worstGroups.map((group) => (
                <tr key={`${group.subscriptionName}-${group.name}`}>
                  <td>{group.name}</td>
                  <td style={{ color: "var(--muted)" }}>{group.subscriptionName}</td>
                  <td className="mono-cell" style={{ textAlign: "right" }}>{group.resources}</td>
                  <td style={{ textAlign: "right", fontWeight: 600, color: group.nonCompliant > group.resources / 2 ? "var(--coral)" : "var(--ink)" }}>
                    {group.nonCompliant}
                  </td>
                  <td>
                    {group.missedTags.map((tag) => (
                      <span key={tag} className="tag-chip missing" style={{ marginRight: 6 }}>
                        {tag}
                      </span>
                    ))}
                  </td>
                </tr>
              ))}
              {analysis.worstGroups.length === 0 ? (
                <tr>
                  <td colSpan={5} style={{ color: "var(--green)", fontWeight: 600 }}>
                    Every resource carries all required tags.
                  </td>
                </tr>
              ) : null}
            </tbody>
          </table>
          <div className="fig-caption" style={{ marginTop: 8 }}>
            Least compliant resource groups, from the <span className="mono" style={{ fontStyle: "normal" }}>missing_required_tags</span> audit rules.
          </div>
        </div>
      ) : null}
    </div>
  );
}

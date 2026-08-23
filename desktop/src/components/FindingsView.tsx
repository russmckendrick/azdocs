import { useMemo, useState } from "react";
import { AlertOctagon, ArrowRight, CheckCircle2, Filter, ShieldAlert } from "lucide-react";
import type { EstateSnapshot, Finding, Severity } from "../types";

const severities: Array<Severity | "all"> = ["all", "high", "medium", "low", "info"];

export function FindingsView({
  estate,
  search,
  onOpenResource,
}: {
  estate: EstateSnapshot;
  search: string;
  onOpenResource: (id: string) => void;
}) {
  const [severity, setSeverity] = useState<Severity | "all">("all");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const filtered = useMemo(
    () => estate.findings.filter((finding) => {
      const matchesSeverity = severity === "all" || finding.severity === severity;
      const needle = search.toLowerCase();
      const matchesSearch = !needle || [finding.title, finding.category, finding.queryName, JSON.stringify(finding.detail ?? {})].some((value) => value.toLowerCase().includes(needle));
      return matchesSeverity && matchesSearch;
    }),
    [estate.findings, search, severity],
  );
  const selected = filtered[Math.min(selectedIndex, Math.max(0, filtered.length - 1))];

  return (
    <div className="findings-workspace">
      <header className="view-heading findings-heading">
        <div>
          <h1>Audit findings</h1>
          <p>Stored evidence ordered by severity, linked back to exact resources.</p>
        </div>
        <div className="severity-tally" aria-label="Finding severity counts">
          {(["high", "medium", "low", "info"] as Severity[]).map((item) => (
            <button key={item} className={severity === item ? `severity-box ${item} active` : `severity-box ${item}`} onClick={() => setSeverity(severity === item ? "all" : item)}>
              <strong>{estate.severityCounts[item]}</strong><span>{item}</span>
            </button>
          ))}
        </div>
      </header>
      <div className="findings-columns">
        <section className="finding-ledger">
          <div className="finding-toolbar">
            <span><ShieldAlert size={16} /> {filtered.length} findings</span>
            <label><Filter size={14} /><select value={severity} onChange={(event) => setSeverity(event.target.value as Severity | "all")}>{severities.map((item) => <option key={item} value={item}>{item === "all" ? "All severities" : item}</option>)}</select></label>
          </div>
          <div className="finding-list">
            {filtered.map((finding, index) => (
              <button key={`${finding.queryName}-${finding.resourceId}-${index}`} className={selected === finding ? "finding-row selected" : "finding-row"} onClick={() => setSelectedIndex(index)}>
                <span className={`severity-marker ${finding.severity}`}>{finding.severity.slice(0, 1).toUpperCase()}</span>
                <span className="finding-copy"><strong>{finding.title}</strong><small>{finding.category} · {resourceName(finding)}</small></span>
                <ArrowRight size={15} />
              </button>
            ))}
            {!filtered.length ? <div className="no-findings"><CheckCircle2 size={30} /><strong>No findings match this view</strong><span>Try another severity or search phrase.</span></div> : null}
          </div>
        </section>
        <FindingEvidence finding={selected} estate={estate} onOpenResource={onOpenResource} />
      </div>
    </div>
  );
}

function resourceName(finding: Finding) {
  return finding.resourceId?.split("/").at(-1) ?? "Estate-level";
}

function FindingEvidence({ finding, estate, onOpenResource }: { finding?: Finding; estate: EstateSnapshot; onOpenResource: (id: string) => void }) {
  if (!finding) return <aside className="finding-evidence empty"><AlertOctagon size={32} /><strong>Select a finding</strong></aside>;
  const resource = estate.resources.find((item) => item.id === finding.resourceId);
  const type = resource ? estate.resourceTypes.find((item) => item.azureType === resource.azureType) : undefined;
  return (
    <aside className="finding-evidence">
      <header>
        <span className={`severity-label ${finding.severity}`}>{finding.severity}</span>
        <h2>{finding.title}</h2>
        <p>{finding.category} · {finding.queryName}</p>
      </header>
      {resource ? (
        <button className="finding-resource-link" onClick={() => onOpenResource(resource.id)}>
          {type ? <img src={type.icon} alt="" /> : null}
          <span><small>Affected resource</small><strong>{resource.name}</strong><em>{type?.displayName ?? resource.azureType}</em></span>
          <ArrowRight size={16} />
        </button>
      ) : null}
      <section>
        <h3>Stored evidence</h3>
        <pre>{JSON.stringify(finding.detail ?? { message: "No additional detail was stored." }, null, 2)}</pre>
      </section>
      <section className="finding-context">
        <h3>Audit context</h3>
        <dl><div><dt>Query</dt><dd>{finding.queryName}</dd></div><div><dt>Category</dt><dd>{finding.category}</dd></div><div><dt>Snapshot</dt><dd>{estate.id.slice(0, 8)}</dd></div></dl>
      </section>
    </aside>
  );
}

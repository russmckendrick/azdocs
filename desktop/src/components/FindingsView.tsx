import { useEffect, useMemo, useState } from "react";
import { ArrowRight, CheckCircle2, Filter, ShieldAlert, X } from "lucide-react";
import type { EstateSnapshot, Finding, Severity } from "../types";
import { fill, resourceName } from "../format";
import { useLabels } from "../labels";
import { ShowMore, useProgressiveList } from "./progressive-list";
import { SEVERITIES } from "../ordering";
import { EmptyState, ViewHeading } from "./view-chrome";
import { useEscapeKey } from "../estate-lookups";

const severities: Array<Severity | "all"> = ["all", ...SEVERITIES];

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
  const [selectedIndex, setSelectedIndex] = useState<number>();
  const filtered = useMemo(
    () => estate.findings.filter((finding) => {
      const matchesSeverity = severity === "all" || finding.severity === severity;
      const needle = search.toLowerCase();
      const matchesSearch = !needle || [finding.title, finding.category, finding.queryName, JSON.stringify(finding.detail ?? {})].some((value) => value.toLowerCase().includes(needle));
      return matchesSeverity && matchesSearch;
    }),
    [estate.findings, search, severity],
  );
  const list = useProgressiveList(filtered, [search, severity]);
  const visibleFindings = list.visible;
  const selected = selectedIndex === undefined ? undefined : visibleFindings[selectedIndex];
  const { common, desktop: { findings: words } } = useLabels();

  useEffect(() => {
    setSelectedIndex(undefined);
  }, [search, severity]);

  useEscapeKey(Boolean(selected), () => setSelectedIndex(undefined));

  return (
    <div className="findings-workspace">
      <ViewHeading
        title={words.title}
        description={words.description}
        modifier="findings-heading"
      />
      <div className="severity-tally" aria-label={words.severity_counts_aria}>
        {SEVERITIES.map((item) => (
          <button key={item} className={severity === item ? `severity-box ${item} active` : `severity-box ${item}`} onClick={() => setSeverity(severity === item ? "all" : item)}>
            <strong>{estate.severityCounts[item]}</strong><span>{common.severity[item].name}</span>
          </button>
        ))}
      </div>
      <div className="findings-columns">
        <section className="finding-ledger">
          <div className="finding-toolbar">
            <span><ShieldAlert size={16} /> {fill(words.count, { count: filtered.length })}</span>
            <label><Filter size={14} /><select value={severity} onChange={(event) => setSeverity(event.target.value as Severity | "all")}>{severities.map((item) => <option key={item} value={item}>{item === "all" ? words.all_severities : common.severity[item].name}</option>)}</select></label>
          </div>
          <div className="finding-list" role="listbox" aria-label={words.list_aria}>
            {visibleFindings.map((finding, index) => (
              <button key={`${finding.queryName}-${finding.resourceId}-${index}`} role="option" aria-selected={selected === finding} className={selected === finding ? "finding-row selected" : "finding-row"} onClick={() => setSelectedIndex(index)}>
                <span className={`severity-marker ${finding.severity}`}>{common.severity[finding.severity].name}</span>
                <span className="finding-copy"><strong>{finding.title}</strong><small>{finding.category} · {findingSubject(finding, words.estate_level)}</small></span>
                <ArrowRight size={15} />
              </button>
            ))}
            {!filtered.length ? (
              <EmptyState
                className="no-findings"
                icon={<CheckCircle2 size={30} />}
                title={words.empty_title}
                detail={words.empty_detail}
              />
            ) : null}
            <ShowMore list={list} />
          </div>
        </section>
        <FindingEvidence finding={selected} estate={estate} onOpenResource={onOpenResource} onClose={() => setSelectedIndex(undefined)} />
      </div>
    </div>
  );
}

/** A finding without a resource id is about the estate, not a resource. */
function findingSubject(finding: Finding, estateLevel: string) {
  return finding.resourceId ? resourceName(finding.resourceId) : estateLevel;
}

function FindingEvidence({ finding, estate, onOpenResource, onClose }: { finding?: Finding; estate: EstateSnapshot; onOpenResource: (id: string) => void; onClose: () => void }) {
  const { common, desktop: { findings: words } } = useLabels();
  if (!finding) return null;
  const resource = estate.resources.find((item) => item.id === finding.resourceId);
  const type = resource ? estate.resourceTypes.find((item) => item.azureType === resource.azureType) : undefined;
  return (
    <aside className="finding-evidence">
      <header>
        <button className="evidence-close" onClick={onClose} aria-label={words.close_evidence} autoFocus><X size={18} /></button>
        <span className={`severity-label ${finding.severity}`}>{common.severity[finding.severity].name}</span>
        <h2>{finding.title}</h2>
        <p>{finding.category} · {finding.queryName}</p>
      </header>
      {resource ? (
        <button className="finding-resource-link" onClick={() => onOpenResource(resource.id)}>
          {type ? <img src={type.icon} alt="" /> : null}
          <span><small>{words.affected_resource}</small><strong>{resource.name}</strong><em>{type?.displayName ?? resource.azureType}</em></span>
          <ArrowRight size={16} />
        </button>
      ) : null}
      <section>
        <h3>{words.stored_evidence}</h3>
        <pre>{JSON.stringify(finding.detail ?? { message: words.no_detail }, null, 2)}</pre>
      </section>
      <section className="finding-context">
        <h3>{words.audit_context}</h3>
        <dl><div><dt>{words.query}</dt><dd>{finding.queryName}</dd></div><div><dt>{words.category}</dt><dd>{finding.category}</dd></div><div><dt>{words.snapshot}</dt><dd>{estate.id.slice(0, 8)}</dd></div></dl>
      </section>
    </aside>
  );
}

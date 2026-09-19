import { useEffect, useMemo, useState } from "react";
import { ArrowRight, CheckCircle2, ClipboardCopy, Download, Filter, ShieldAlert, X } from "lucide-react";
import { copyText, saveTextFile } from "../api";
import type { DashboardFilter, EstateSnapshot, Finding, Severity } from "../types";
import { errorMessage, fill, plural, resourceName } from "../format";
import { useLabels } from "../labels";
import { ShowMore } from "./progressive-list";
import { useProgressiveList } from "./use-progressive-list";
import { SEVERITIES } from "../ordering";
import { EmptyState, ErrorStrip, ViewHeading } from "./view-chrome";
import { useEscapeKey } from "../estate-lookups";

import { findingMatchesDashboard } from "./dashboard-model";
import { findingsCsv } from "./findings-csv";

const severities: Array<Severity | "all"> = ["all", ...SEVERITIES];

/** Identity for a selection: the stored finding rows carry no id of their own. */
function findingKey(finding: Finding) {
  return [finding.queryName, finding.resourceId ?? "", finding.title].join("|");
}

export function FindingsView({
  estate,
  search,
  onOpenResource,
  dashboardFilter,
}: {
  dashboardFilter?: DashboardFilter;
  estate: EstateSnapshot;
  search: string;
  onOpenResource: (id: string) => void;
}) {
  const [severity, setSeverity] = useState<Severity | "all">("all");
  const [selectedIndex, setSelectedIndex] = useState<number>();
  const [checked, setChecked] = useState<Set<string>>(() => new Set());
  const [notice, setNotice] = useState<string>();
  const [actionError, setActionError] = useState<string>();
  const filtered = useMemo(
    () => estate.findings.filter((finding) => {
      const matchesSeverity = severity === "all" || finding.severity === severity;
      const needle = search.toLowerCase();
      const matchesSearch = !needle || [finding.title, finding.category, finding.queryName, JSON.stringify(finding.detail ?? {})].some((value) => value.toLowerCase().includes(needle));
      return findingMatchesDashboard(finding, estate, dashboardFilter) && matchesSeverity && matchesSearch;
    }),
    [dashboardFilter, estate, search, severity],
  );
  const list = useProgressiveList(filtered, [dashboardFilter, search, severity]);
  const visibleFindings = list.visible;
  const selected = selectedIndex === undefined ? undefined : visibleFindings[selectedIndex];
  const { common, desktop: { findings: words } } = useLabels();
  const csvColumns = common.columns;

  useEffect(() => {
    setSelectedIndex(undefined);
    setChecked(new Set());
    setNotice(undefined);
  }, [dashboardFilter, search, severity, estate.id]);

  useEscapeKey(Boolean(selected), () => setSelectedIndex(undefined));

  const checkedFindings = filtered.filter((finding) => checked.has(findingKey(finding)));
  const allVisibleChecked = visibleFindings.length > 0 && visibleFindings.every((finding) => checked.has(findingKey(finding)));

  function toggle(finding: Finding) {
    setChecked((current) => {
      const next = new Set(current);
      const key = findingKey(finding);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  }
  function toggleAllVisible() {
    setChecked((current) => {
      const next = new Set(current);
      if (allVisibleChecked) visibleFindings.forEach((finding) => next.delete(findingKey(finding)));
      else visibleFindings.forEach((finding) => next.add(findingKey(finding)));
      return next;
    });
  }
  async function copyIds() {
    const ids = [...new Set(checkedFindings.flatMap((finding) => (finding.resourceId ? [finding.resourceId] : [])))];
    try {
      await copyText(ids.join("\n"));
      setNotice(fill(words.copied, { count: ids.length }));
    } catch (caught) {
      setActionError(errorMessage(caught));
    }
  }
  async function exportCsv() {
    const csv = findingsCsv(checkedFindings, [csvColumns.severity, csvColumns.category, csvColumns.check, csvColumns.title, csvColumns.resource]);
    try {
      if (await saveTextFile(words.csv_name, csv)) setNotice(fill(words.csv_saved, { count: checkedFindings.length }));
    } catch (caught) {
      setActionError(errorMessage(caught));
    }
  }

  return (
    <div className="findings-workspace">
      <ViewHeading
        title={words.title}
        description={words.description}
        modifier="findings-heading"
      />
      {actionError ? <ErrorStrip message={actionError} onDismiss={() => setActionError(undefined)} /> : null}
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
          <div className="finding-select-bar">
            <label>
              <input type="checkbox" checked={allVisibleChecked} onChange={toggleAllVisible} disabled={visibleFindings.length === 0} />
              <span>{words.select_all}</span>
            </label>
            <span className="finding-selected-count" role="status">
              {checkedFindings.length ? plural(words.selected, checkedFindings.length) : (notice ?? "")}
            </span>
            <button className="quiet-button" disabled={!checkedFindings.some((finding) => finding.resourceId)} onClick={() => void copyIds()}>
              <ClipboardCopy size={14} /> {words.copy_ids}
            </button>
            <button className="quiet-button" disabled={!checkedFindings.length} onClick={() => void exportCsv()}>
              <Download size={14} /> {words.export_csv}
            </button>
          </div>
          <div className="finding-list" role="listbox" aria-label={words.list_aria} aria-multiselectable="true">
            {visibleFindings.map((finding, index) => (
              <div key={`${finding.queryName}-${finding.resourceId}-${index}`} className={selected === finding ? "finding-row selected" : "finding-row"} role="option" aria-selected={selected === finding}>
                <input type="checkbox" aria-label={words.select_aria} checked={checked.has(findingKey(finding))} onChange={() => toggle(finding)} />
                <button className="finding-row-open" onClick={() => setSelectedIndex(index)}>
                  <span className={`severity-marker ${finding.severity}`}>{common.severity[finding.severity].name}</span>
                  <span className="finding-copy"><strong>{finding.title}</strong><small>{finding.category} · {findingSubject(finding, words.estate_level)}</small></span>
                  <ArrowRight size={15} />
                </button>
              </div>
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

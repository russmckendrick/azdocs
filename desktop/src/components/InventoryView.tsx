import { useEffect, useMemo, useState } from "react";
import { Table2 } from "lucide-react";
import { getQueryPackMetadata, getQueryRows } from "../api";
import type { DashboardFilter, EstateSnapshot, QueryDefMeta, QueryRows } from "../types";
import { ShowMore } from "./progressive-list";
import { useProgressiveList } from "./use-progressive-list";
import { DatabaseStamp, EmptyState, ErrorStrip, ViewHeading } from "./view-chrome";
import { capitalise, errorMessage, fill, plural, spaced } from "../format";
import { useLabels } from "../labels";


/// Category dot colours follow the design language: chart slots for the big
/// categories, the wider category_color() family for the rest, neutral
/// otherwise.
const CATEGORY_VARS: Record<string, string> = {
  compute: "--cat-compute",
  networking: "--cat-networking",
  storage: "--cat-storage",
  databases: "--cat-databases",
  "app services": "--cat-appservices",
  integration: "--cat-appservices",
  security: "--coral",
  identity: "--kind-identity",
  monitoring: "--kind-monitoring",
};

function categoryVar(category: string) {
  return CATEGORY_VARS[category] ?? "--cat-other";
}

function cellText(value: unknown, none: string): string {
  if (value === null || value === undefined) return none;
  if (typeof value === "object") return JSON.stringify(value);
  return String(value);
}

function isMachineShaped(column: string) {
  const needle = column.toLowerCase();
  return needle === "location" || needle.endsWith("id") || needle.includes("version") || needle.includes("address");
}

function EvidenceSummary({ evidence }: { evidence: EstateSnapshot["evidenceSummaries"][number] }) {
  // Large assignment registers reveal more rows explicitly, matching inventory behaviour.
  const list = useProgressiveList(evidence.rows, [evidence.key], 100);
  return <section>
    <h3>{evidence.title}</h3>
    <p>{evidence.status}</p>
    <p className="muted-copy">{evidence.note}</p>
    {!!evidence.rows.length && <><div className="data-grid-wrap"><table className="data-grid">
      <thead><tr>{evidence.columns.map((column) => <th key={column}>{column}</th>)}</tr></thead>
      <tbody>{list.visible.map((row, index) => <tr key={index}>{row.map((cell, column) => <td key={column}>{cell}</td>)}</tr>)}</tbody>
    </table></div><ShowMore list={list} inline /></>}
  </section>;
}

export function InventoryView({ estate, search, dashboardFilter }: { estate: EstateSnapshot; search: string; dashboardFilter?: DashboardFilter }) {
  const [pack, setPack] = useState<QueryDefMeta[]>();
  const [packError, setPackError] = useState<string>();
  const [category, setCategory] = useState<string | undefined>(dashboardFilter?.category);
  const [queryName, setQueryName] = useState<string | undefined>(dashboardFilter?.queryName);
  const [result, setResult] = useState<QueryRows>();
  const [rowsLoading, setRowsLoading] = useState(false);
  const [rowsError, setRowsError] = useState<string>();
  const { common, desktop: { inventory: words } } = useLabels();
  const none = common.verdict.none;

  const runByName = useMemo(
    () => new Map(estate.queryRuns.map((run) => [run.queryName, run])),
    [estate.queryRuns],
  );

  useEffect(() => {
    let active = true;
    getQueryPackMetadata()
      .then((defs) => {
        if (active) setPack(defs);
      })
      .catch((error) => {
        if (active) setPackError(errorMessage(error, words.pack_unreadable));
      });
    return () => {
      active = false;
    };
  }, [words.pack_unreadable]);

  // Only inventory queries that actually ran for this snapshot are listed —
  // recorded definitions survive changes to the current pack.
  const inventory = useMemo(
    () => estate.queryRuns.flatMap((run) => {
      // Prefer the snapshot's definition: an override may have changed or vanished.
      const recorded = run.provenance;
      const def = recorded
        ? { name: run.queryName, category: run.category, kind: recorded.kind, description: recorded.description }
        : pack?.find((entry) => entry.name === run.queryName);
      return def?.kind === "inventory" && def.name !== "all_resources"
        ? [{ ...def, kind: "inventory" as const }]
        : [];
    }),
    [pack, estate.queryRuns],
  );
  const categories = useMemo(() => {
    const byCategory = new Map<string, QueryDefMeta[]>();
    for (const def of inventory) {
      byCategory.set(def.category, [...(byCategory.get(def.category) ?? []), def]);
    }
    return [...byCategory.entries()].sort(([a], [b]) => a.localeCompare(b));
  }, [inventory]);

  const activeCategory = category && categories.some(([name]) => name === category)
    ? category
    : categories[0]?.[0];
  const queries = categories.find(([name]) => name === activeCategory)?.[1] ?? [];
  const activeQuery = queryName && queries.some((def) => def.name === queryName)
    ? queryName
    : queries[0]?.name;
  const activeDef = queries.find((def) => def.name === activeQuery);

  useEffect(() => {
    if (!activeQuery) return;
    let active = true;
    setRowsLoading(true);
    setRowsError(undefined);
    getQueryRows(activeQuery, estate.id)
      .then((rows) => {
        if (!active) return;
        setResult(rows);
      })
      .catch((caught) => {
        // The previous table stays up; a failed read is said, not blanked.
        if (active) setRowsError(fill(words.rows_failed, { error: errorMessage(caught) }));
      })
      .finally(() => {
        if (active) setRowsLoading(false);
      });
    return () => {
      active = false;
    };
  }, [activeQuery, estate.id, words.rows_failed]);

  const columns = (result?.columns ?? []).filter((column) => column !== "id");
  const filteredRows = useMemo(() => {
    const rows = result?.rows ?? [];
    const needle = search.trim().toLowerCase();
    if (!needle) return rows;
    return rows.filter((row) => Object.values(row).some((value) => cellText(value, none).toLowerCase().includes(needle)));
  }, [result, search, none]);
  const run = activeQuery ? runByName.get(activeQuery) : undefined;
  const list = useProgressiveList(filteredRows, [activeQuery, search], 250);
  const visibleRows = list.visible;


  // Recorded definitions keep historical evidence usable if today's pack is invalid.
  if (packError && inventory.length === 0) {
    return (
      <div className="inventory-workspace">
        <ViewHeading title={words.title} description={fill(words.pack_failed, { error: packError })} />
      </div>
    );
  }

  return (
    <div className="inventory-workspace">
      <ViewHeading
        title={words.title}
        description={words.description}
      >
        <DatabaseStamp
          icon={<Table2 size={17} />}
          label={words.collected_stamp}
          value={fill(words.collected_value, { inventory: inventory.length, total: estate.queryRuns.length })}
        />
      </ViewHeading>
      {rowsError ? <ErrorStrip message={rowsError} onDismiss={() => setRowsError(undefined)} /> : null}

      {!!estate.evidenceSummaries?.length && (
        <details className="inventory-evidence">
          <summary>{words.summaries}</summary>
          <div className="inventory-evidence-body">
            {/* Rust owns coverage and age decisions; these are the report's labelled cells. */}
            {estate.evidenceSummaries.map((evidence) => (
              <EvidenceSummary key={evidence.key} evidence={evidence} />
            ))}
          </div>
        </details>
      )}

      {categories.length === 0 ? (
        <EmptyState
          className="inventory-empty"
          icon={<Table2 size={30} strokeWidth={1.4} />}
          title={words.empty_title}
          detail={words.empty_detail}
          detailClassName="muted-copy"
        />
      ) : (
        <div className="inventory-columns">
          <nav className="inv-cats" aria-label={words.categories_aria}>
            {categories.map(([name, defs]) => (
              <button
                key={name}
                className={name === activeCategory ? "cat-row active" : "cat-row"}
                onClick={() => {
                  setCategory(name);
                  setQueryName(undefined);
                }}
              >
                <span className="cat-dot" style={{ background: `var(${categoryVar(name)})` }} />
                {capitalise(name)}
                <b>{defs.length}</b>
              </button>
            ))}
          </nav>

          <section className="inv-main" tabIndex={0} aria-label={words.title}>
            <div className="query-picker">
              <label>
                <span>{words.table}</span>
                <select value={activeQuery ?? ""} onChange={(event) => setQueryName(event.target.value)}>
                  {queries.map((def) => (
                    <option key={def.name} value={def.name}>
                      {fill(words.query_option, { name: spaced(def.name), rows: runByName.get(def.name)?.rowCount ?? 0 })}
                    </option>
                  ))}
                </select>
              </label>
              <span className="query-position">{fill(words.position, { index: queries.findIndex((def) => def.name === activeQuery) + 1, total: queries.length, category: activeCategory ?? "" })}</span>
            </div>
            <div className="query-description">
              {activeDef?.description}
              {run?.error ? <span style={{ color: "var(--coral)" }}>{fill(words.failed, { error: run.error })}</span> : null}
            </div>

            <details className="inventory-evidence" key={activeQuery}>
              <summary>{words.provenance}</summary>
              {run?.provenance ? <div className="inventory-evidence-body">
                <dl>
                  <dt>{words.scope}</dt><dd>{run.provenance.authorizationScope}</dd>
                  <dt>{words.subscriptions}</dt><dd>{run.provenance.subscriptions.join(", ") || words.all_visible}</dd>
                  <dt>{words.query_hash}</dt><dd className="mono">{run.provenance.kqlSha256}</dd>
                  {!!run.provenance.sourceUrls.length && <><dt>{words.source}</dt><dd>{run.provenance.sourceUrls.map((url) => <p key={url}>{url}</p>)}</dd></>}
                  {run.provenance.reviewedOn && <><dt>{words.source_reviewed}</dt><dd>{run.provenance.reviewedOn}</dd></>}
                </dl>
                <pre>{run.provenance.kql}</pre>
              </div> : <p className="muted-copy">{words.provenance_missing}</p>}
            </details>

            <div className="data-grid-wrap">
              {rowsLoading ? (
                <p className="muted-copy">{words.reading}</p>
              ) : filteredRows.length === 0 ? (
                <p className="muted-copy">
                  {search.trim() ? words.no_search_rows : words.no_rows}
                </p>
              ) : (
                <table className="data-grid">
                  <thead>
                    <tr>
                      {columns.map((column) => (
                        <th key={column}>{column}</th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {visibleRows.map((row, index) => (
                      <tr key={index}>
                        {columns.map((column) => (
                          <td key={column} className={isMachineShaped(column) ? "mono-cell" : undefined} title={cellText(row[column], none)}>
                            {cellText(row[column], none)}
                          </td>
                        ))}
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </div>

            <div className="grid-footer">
              <span>
                {visibleRows.length < filteredRows.length ? fill(words.visible_of, { visible: visibleRows.length.toLocaleString() }) : ""}{plural(words.row_count, filteredRows.length, { count: filteredRows.length.toLocaleString() })}
                {search.trim() ? fill(words.matching, { search: search.trim() }) : ""}
                {run?.durationMs != null ? fill(words.collected_in, { ms: run.durationMs }) : ""}
              </span>
              <ShowMore list={list} inline />
              <span className="mono" style={{ marginLeft: "auto", color: "var(--faintest)" }}>
                queries/{activeCategory}/{activeQuery}.toml
              </span>
            </div>
          </section>
        </div>
      )}
    </div>
  );
}

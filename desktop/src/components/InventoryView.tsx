import { useEffect, useMemo, useState } from "react";
import { Table2 } from "lucide-react";
import { getQueryPackMetadata, getQueryRows } from "../api";
import type { EstateSnapshot, QueryDefMeta, QueryRows } from "../types";
import { ShowMore, useProgressiveList } from "./progressive-list";
import { DatabaseStamp, EmptyState, ViewHeading } from "./view-chrome";
import { capitalise, spaced } from "../format";


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

function cellText(value: unknown): string {
  if (value === null || value === undefined) return "—";
  if (typeof value === "object") return JSON.stringify(value);
  return String(value);
}

function isMachineShaped(column: string) {
  const needle = column.toLowerCase();
  return needle === "location" || needle.endsWith("id") || needle.includes("version") || needle.includes("address");
}

export function InventoryView({ estate, search }: { estate: EstateSnapshot; search: string }) {
  const [pack, setPack] = useState<QueryDefMeta[]>();
  const [packError, setPackError] = useState<string>();
  const [category, setCategory] = useState<string>();
  const [queryName, setQueryName] = useState<string>();
  const [result, setResult] = useState<QueryRows>();
  const [rowsLoading, setRowsLoading] = useState(false);

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
        if (active) setPackError(error instanceof Error ? error.message : String(error));
      });
    return () => {
      active = false;
    };
  }, []);

  // Only inventory queries that actually ran for this snapshot are listed —
  // the pack is the catalogue, query_runs is the evidence.
  const inventory = useMemo(
    () =>
      (pack ?? []).filter(
        (def) => def.kind === "inventory" && def.name !== "all_resources" && runByName.has(def.name),
      ),
    [pack, runByName],
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
    getQueryRows(activeQuery, estate.id)
      .then((rows) => {
        if (!active) return;
        setResult(rows);
      })
      .catch(() => {
        if (active) setResult({ queryName: activeQuery, columns: [], rows: [] });
      })
      .finally(() => {
        if (active) setRowsLoading(false);
      });
    return () => {
      active = false;
    };
  }, [activeQuery, estate.id]);

  const columns = (result?.columns ?? []).filter((column) => column !== "id");
  const filteredRows = useMemo(() => {
    const rows = result?.rows ?? [];
    const needle = search.trim().toLowerCase();
    if (!needle) return rows;
    return rows.filter((row) => Object.values(row).some((value) => cellText(value).toLowerCase().includes(needle)));
  }, [result, search]);
  const run = activeQuery ? runByName.get(activeQuery) : undefined;
  const list = useProgressiveList(filteredRows, [activeQuery, search], 250);
  const visibleRows = list.visible;


  if (packError) {
    return (
      <div className="inventory-workspace">
        <ViewHeading title="Inventory" description={`The query pack could not be loaded: ${packError}`} />
      </div>
    );
  }

  return (
    <div className="inventory-workspace">
      <ViewHeading
        title="Inventory"
        description="The shaped rows every collected query stored for this snapshot — the same tables the reports print, browsable per category."
      >
        <DatabaseStamp
          icon={<Table2 size={17} />}
          label="Collected queries"
          value={<>{inventory.length} inventory · {estate.queryRuns.length} total</>}
        />
      </ViewHeading>

      {categories.length === 0 ? (
        <EmptyState
          className="inventory-empty"
          icon={<Table2 size={30} strokeWidth={1.4} />}
          title="No shaped inventory rows in this snapshot"
          detail="Collect a snapshot to fill the per-query tables."
          detailClassName="muted-copy"
        />
      ) : (
        <div className="inventory-columns">
          <nav className="inv-cats" aria-label="Query categories">
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

          <section className="inv-main">
            <div className="query-picker">
              <label>
                <span>Table</span>
                <select value={activeQuery ?? ""} onChange={(event) => setQueryName(event.target.value)}>
                  {queries.map((def) => (
                    <option key={def.name} value={def.name}>
                      {spaced(def.name)} · {runByName.get(def.name)?.rowCount ?? 0} rows
                    </option>
                  ))}
                </select>
              </label>
              <span className="query-position">{queries.findIndex((def) => def.name === activeQuery) + 1} of {queries.length} in {activeCategory}</span>
            </div>
            <div className="query-description">
              {activeDef?.description}
              {run?.error ? <span style={{ color: "var(--coral)" }}> — this query failed: {run.error}</span> : null}
            </div>

            <div className="data-grid-wrap">
              {rowsLoading ? (
                <p className="muted-copy">Reading stored rows…</p>
              ) : filteredRows.length === 0 ? (
                <p className="muted-copy">
                  {search.trim() ? "No rows match the current search." : "This query stored no rows for the snapshot."}
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
                          <td key={column} className={isMachineShaped(column) ? "mono-cell" : undefined} title={cellText(row[column])}>
                            {cellText(row[column])}
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
                {visibleRows.length < filteredRows.length ? `${visibleRows.length.toLocaleString()} of ` : ""}{filteredRows.length.toLocaleString()} row{filteredRows.length === 1 ? "" : "s"}
                {search.trim() ? ` matching “${search.trim()}”` : ""}
                {run?.durationMs !== undefined ? ` · collected in ${run.durationMs} ms` : ""}
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

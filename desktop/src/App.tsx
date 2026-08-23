import { lazy, Suspense, useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  AlertTriangle,
  Boxes,
  ChevronDown,
  Database,
  FileClock,
  FolderSearch2,
  GitBranch,
  LoaderCircle,
  PanelLeftClose,
  RefreshCw,
  Search,
  Settings2,
  ShieldCheck,
} from "lucide-react";
import { chooseDatabase, collectEstate, getBootstrap, getSnapshot, isTauri } from "./api";
import { EstateExplorer } from "./components/EstateExplorer";
import { FindingsView } from "./components/FindingsView";
import { HistoryView } from "./components/HistoryView";
import type {
  AppBootstrap,
  CollectionEvent,
  EstateSnapshot,
  ScopeSelection,
  ViewId,
} from "./types";

const TopologyView = lazy(() =>
  import("./components/TopologyView").then((module) => ({ default: module.TopologyView })),
);

const views: Array<{
  id: ViewId;
  label: string;
  icon: typeof Boxes;
}> = [
  { id: "estate", label: "Estate", icon: Boxes },
  { id: "topology", label: "Relationships", icon: GitBranch },
  { id: "findings", label: "Findings", icon: ShieldCheck },
  { id: "history", label: "Snapshots", icon: FileClock },
];

function compactDate(value: string) {
  return new Intl.DateTimeFormat(undefined, {
    day: "2-digit",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(value));
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

export default function App() {
  const [bootstrap, setBootstrap] = useState<AppBootstrap>();
  const [estate, setEstate] = useState<EstateSnapshot>();
  const [view, setView] = useState<ViewId>("estate");
  const [scope, setScope] = useState<ScopeSelection>({});
  const [selectedResourceId, setSelectedResourceId] = useState<string>();
  const [search, setSearch] = useState("");
  const [searchOpen, setSearchOpen] = useState(false);
  const [activeSearchIndex, setActiveSearchIndex] = useState(0);
  const [topologyFocusRequest, setTopologyFocusRequest] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>();
  const [collectionMessage, setCollectionMessage] = useState<string>();
  const [collecting, setCollecting] = useState(false);
  const searchRef = useRef<HTMLInputElement>(null);

  const loadSnapshot = useCallback(async (snapshotId?: string) => {
    setLoading(true);
    setError(undefined);
    try {
      const next = await getSnapshot(snapshotId);
      setEstate(next);
      setSelectedResourceId((current) =>
        current && next.resources.some((resource) => resource.id === current)
          ? current
          : next.resources.find((resource) => resource.findingCount > 0)?.id ??
            next.resources[0]?.id,
      );
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    let active = true;
    async function start() {
      setLoading(true);
      try {
        const nextBootstrap = await getBootstrap();
        if (!active) return;
        setBootstrap(nextBootstrap);
        if (nextBootstrap.latestSnapshotId) {
          const nextEstate = await getSnapshot(nextBootstrap.latestSnapshotId);
          if (!active) return;
          setEstate(nextEstate);
          setSelectedResourceId(
            nextEstate.resources.find((resource) => resource.findingCount > 0)?.id ??
              nextEstate.resources[0]?.id,
          );
        }
      } catch (caught) {
        if (active) setError(errorMessage(caught));
      } finally {
        if (active) setLoading(false);
      }
    }
    void start();
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    function handleShortcut(event: KeyboardEvent) {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        searchRef.current?.focus();
      }
    }
    window.addEventListener("keydown", handleShortcut);
    return () => window.removeEventListener("keydown", handleShortcut);
  }, []);

  const selectedResource = useMemo(
    () => estate?.resources.find((resource) => resource.id === selectedResourceId),
    [estate, selectedResourceId],
  );
  const resourceTypeMap = useMemo(
    () => new Map(estate?.resourceTypes.map((type) => [type.azureType, type]) ?? []),
    [estate],
  );
  const searchMatches = useMemo(() => {
    const value = search.trim().toLowerCase();
    if (!estate || !value) return [];
    return estate.resources
      .filter((resource) => [
        resource.name,
        resource.azureType,
        resource.resourceGroup,
        resource.location,
        JSON.stringify(resource.tags ?? {}),
      ].some((candidate) => candidate?.toLowerCase().includes(value)))
      .slice(0, 8);
  }, [estate, search]);
  const shortcutLabel = navigator.platform.toLowerCase().includes("mac") ? "⌘ K" : "Ctrl K";

  function chooseSearchResult(index: number) {
    const resource = searchMatches[index];
    if (!resource) return;
    setSelectedResourceId(resource.id);
    if (view === "topology") setTopologyFocusRequest((current) => current + 1);
    else setView("estate");
    setSearch("");
    setSearchOpen(false);
    setActiveSearchIndex(0);
  }

  function handleSearchKeyDown(event: React.KeyboardEvent<HTMLInputElement>) {
    if (event.key === "ArrowDown" && searchMatches.length > 0) {
      event.preventDefault();
      setSearchOpen(true);
      setActiveSearchIndex((current) => (current + 1) % searchMatches.length);
    } else if (event.key === "ArrowUp" && searchMatches.length > 0) {
      event.preventDefault();
      setSearchOpen(true);
      setActiveSearchIndex((current) => (current - 1 + searchMatches.length) % searchMatches.length);
    } else if (event.key === "Enter" && searchOpen) {
      event.preventDefault();
      chooseSearchResult(activeSearchIndex);
    } else if (event.key === "Escape") {
      setSearchOpen(false);
    }
  }

  async function handleDatabase() {
    setError(undefined);
    try {
      const next = await chooseDatabase();
      if (!next) return;
      setBootstrap(next);
      setEstate(undefined);
      setSelectedResourceId(undefined);
      if (next.latestSnapshotId) await loadSnapshot(next.latestSnapshotId);
    } catch (caught) {
      setError(errorMessage(caught));
    }
  }

  async function handleCollect() {
    if (collecting) return;
    setCollecting(true);
    setError(undefined);
    setCollectionMessage("Preparing read-only collection");
    try {
      const result = await collectEstate((event: CollectionEvent) => {
        if (event.event === "phase") setCollectionMessage(event.data.message);
        if (event.event === "complete") setCollectionMessage("Snapshot stored; rebuilding estate view");
        if (event.event === "failed") setCollectionMessage(event.data.message);
      });
      const nextBootstrap = await getBootstrap();
      setBootstrap(nextBootstrap);
      await loadSnapshot(result.snapshotId);
      setCollectionMessage(`Collected ${result.rowsIngested.toLocaleString()} rows into a ${result.status} snapshot`);
      window.setTimeout(() => setCollectionMessage(undefined), 4800);
    } catch (caught) {
      setError(errorMessage(caught));
      setCollectionMessage(undefined);
    } finally {
      setCollecting(false);
    }
  }

  function openResource(resourceId: string, destination: ViewId = "estate") {
    setSelectedResourceId(resourceId);
    setView(destination);
    if (destination === "topology") setTopologyFocusRequest((current) => current + 1);
  }

  return (
    <div className="app-shell">
      <aside className="command-rail" aria-label="Primary navigation">
        <button className="brand-mark" aria-label="azdocs estate explorer" onClick={() => setView("estate")}>
          <img src="/icons/other/10018-icon-service-Azure-A.svg" alt="" />
        </button>
        <nav className="rail-nav">
          {views.map((item) => {
            const Icon = item.icon;
            const badge = item.id === "findings" ? estate?.totals.findings : undefined;
            return (
              <button
                key={item.id}
                className={view === item.id ? "rail-action active" : "rail-action"}
                onClick={() => setView(item.id)}
                aria-label={item.label}
                aria-current={view === item.id ? "page" : undefined}
                title={item.label}
              >
                <Icon size={21} strokeWidth={1.8} />
                {badge ? <span className="rail-badge">{badge}</span> : null}
                <span>{item.label}</span>
              </button>
            );
          })}
        </nav>
        <div className="rail-footer">
          <button className="rail-action" onClick={handleDatabase} aria-label="Open database" title="Open database">
            <Database size={20} strokeWidth={1.8} />
            <span>Database</span>
          </button>
        </div>
      </aside>

      <header className="topbar">
        <div className="product-lockup">
          <strong>azdocs</strong>
          <span>Azure estate intelligence</span>
          {!isTauri ? <em>Illustrative workspace</em> : null}
        </div>
        <label className="snapshot-control">
          <span>Snapshot</span>
          <select
            value={estate?.id ?? ""}
            onChange={(event) => void loadSnapshot(event.target.value)}
            disabled={!bootstrap?.snapshots.length || loading}
            aria-label="Active snapshot"
          >
            {bootstrap?.snapshots.map((snapshot) => (
              <option key={snapshot.id} value={snapshot.id}>
                {compactDate(snapshot.createdAt)} · {snapshot.resources} resources
              </option>
            ))}
          </select>
          <ChevronDown size={15} aria-hidden="true" />
        </label>
        <div className="global-search">
          <Search size={17} aria-hidden="true" />
          <input
            ref={searchRef}
            value={search}
            onChange={(event) => {
              setSearch(event.target.value);
              setSearchOpen(Boolean(event.target.value.trim()));
              setActiveSearchIndex(0);
            }}
            onFocus={() => setSearchOpen(Boolean(search.trim()))}
            onKeyDown={handleSearchKeyDown}
            placeholder="Search resources, types, groups, tags…"
            aria-label="Search the active snapshot"
            role="combobox"
            aria-autocomplete="list"
            aria-expanded={searchOpen && searchMatches.length > 0}
            aria-controls="global-resource-results"
            aria-activedescendant={searchOpen && searchMatches.length > 0 ? `global-resource-result-${activeSearchIndex}` : undefined}
          />
          <kbd>{shortcutLabel}</kbd>
          {searchOpen && search ? (
            <div className="global-search-results" id="global-resource-results" role="listbox" aria-label="Matching resources">
              {searchMatches.map((resource, index) => {
                const type = resourceTypeMap.get(resource.azureType);
                return (
                  <button
                    key={resource.id}
                    id={`global-resource-result-${index}`}
                    role="option"
                    aria-selected={index === activeSearchIndex}
                    className={index === activeSearchIndex ? "active" : ""}
                    onMouseDown={(event) => event.preventDefault()}
                    onClick={() => chooseSearchResult(index)}
                  >
                    {type ? <img src={type.icon} alt="" /> : <Boxes size={22} />}
                    <span><strong>{resource.name}</strong><small>{type?.displayName ?? resource.azureType} · {resource.resourceGroup}</small></span>
                  </button>
                );
              })}
              {searchMatches.length === 0 ? <p>No matching resources in this snapshot.</p> : null}
            </div>
          ) : null}
        </div>
        <div className="topbar-actions">
          <button className="quiet-button" onClick={handleDatabase} title={bootstrap?.databasePath}>
            <FolderSearch2 size={16} />
            Open data
          </button>
          <button
            className="collect-button"
            onClick={() => void handleCollect()}
            disabled={collecting || !bootstrap?.hasCredentials}
            title={bootstrap?.hasCredentials ? "Collect a new snapshot" : `Configure credentials in ${bootstrap?.configPath ?? "azdocs.toml"}`}
          >
            {collecting ? <LoaderCircle className="spin" size={16} /> : <RefreshCw size={16} />}
            {collecting ? "Collecting" : "Collect snapshot"}
          </button>
        </div>
      </header>

      <main className={view === "topology" ? "workspace workspace-immersive" : "workspace"}>
        {collectionMessage ? (
          <div className="collection-strip" role="status">
            <LoaderCircle className={collecting ? "spin" : ""} size={16} />
            <span>{collectionMessage}</span>
            <small>Azure Resource Graph · read-only</small>
          </div>
        ) : null}
        {error ? (
          <div className="error-strip" role="alert">
            <AlertTriangle size={17} />
            <span>{error}</span>
            <button onClick={() => setError(undefined)}>Dismiss</button>
          </div>
        ) : null}

        {loading && !estate ? <LoadingWorkspace /> : null}
        {!loading && !estate && !error ? (
          <EmptyWorkspace
            canCollect={Boolean(bootstrap?.hasCredentials)}
            onCollect={() => void handleCollect()}
            onOpen={handleDatabase}
          />
        ) : null}
        {estate ? (
          <>
            {view === "estate" ? (
              <EstateExplorer
                estate={estate}
                search={search}
                scope={scope}
                onScopeChange={setScope}
                selectedResourceId={selectedResourceId}
                onSelectResource={setSelectedResourceId}
                onOpenTopology={(id) => openResource(id, "topology")}
                onOpenFinding={() => setView("findings")}
              />
            ) : null}
            {view === "topology" ? (
              <Suspense fallback={<LoadingWorkspace />}>
                <TopologyView
                  estate={estate}
                  selectedResourceId={selectedResourceId}
                  focusRequestNonce={topologyFocusRequest}
                  onSelectResource={setSelectedResourceId}
                  onInspect={(id) => openResource(id, "estate")}
                />
              </Suspense>
            ) : null}
            {view === "findings" ? (
              <FindingsView estate={estate} search={search} onOpenResource={(id) => openResource(id)} />
            ) : null}
            {view === "history" && bootstrap ? (
              <HistoryView
                bootstrap={bootstrap}
                estate={estate}
                onLoadSnapshot={(id) => void loadSnapshot(id)}
              />
            ) : null}
          </>
        ) : null}
      </main>

      <div className="statusbar" aria-label="Application status">
        <span className={`status-dot ${estate?.status ?? "unknown"}`} />
        <span>{estate ? `${estate.status} snapshot` : "No snapshot loaded"}</span>
        <span className="status-divider" />
        <span>{bootstrap?.databasePath ?? "Resolving database…"}</span>
        <span className="status-spacer" />
        <span>{selectedResource ? `${selectedResource.edgeCount} relationships · ${selectedResource.findingCount} findings` : "No resource selected"}</span>
        <Settings2 size={13} aria-hidden="true" />
      </div>
    </div>
  );
}

function LoadingWorkspace() {
  return (
    <div className="loading-workspace" role="status">
      <div className="loading-cabinet">
        <span />
        <span />
        <span />
      </div>
      <strong>Opening the estate cabinet</strong>
      <p>Reading subscriptions, resources, relationships, and findings from SQLite.</p>
    </div>
  );
}

function EmptyWorkspace({
  canCollect,
  onCollect,
  onOpen,
}: {
  canCollect: boolean;
  onCollect: () => void;
  onOpen: () => void;
}) {
  return (
    <div className="empty-workspace">
      <PanelLeftClose size={42} strokeWidth={1.3} />
      <h1>No stored snapshots yet</h1>
      <p>
        Open an existing azdocs database or collect a read-only Azure snapshot. Exploration and exports remain offline after collection.
      </p>
      <div>
        <button className="collect-button" onClick={onOpen}>Open database</button>
        <button className="quiet-button" onClick={onCollect} disabled={!canCollect}>Collect first snapshot</button>
      </div>
    </div>
  );
}

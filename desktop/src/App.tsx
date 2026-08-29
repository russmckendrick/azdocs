import { lazy, Suspense, useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  AlertTriangle,
  Boxes,
  ChevronDown,
  FileClock,
  FolderSearch2,
  GitBranch,
  LayoutGrid,
  LoaderCircle,
  PanelLeftClose,
  RefreshCw,
  Search,
  Settings2,
  ShieldCheck,
  Table2,
  Tags,
} from "lucide-react";
import { chooseDatabase, collectEstate, getBootstrap, getSnapshot, isTauri } from "./api";
import { ALL_RESOURCES_ICON } from "./azure-icons";
import { displayLocation } from "./azure-values";
import { EstateExplorer } from "./components/EstateExplorer";
import { FindingsView } from "./components/FindingsView";
import { GovernanceView } from "./components/GovernanceView";
import { HistoryView } from "./components/HistoryView";
import { InventoryView } from "./components/InventoryView";
import { OverviewView } from "./components/OverviewView";
import { ResourceDetailView } from "./components/ResourceDetailView";
import { SettingsView } from "./components/SettingsView";
import type {
  AppBootstrap,
  CollectionEvent,
  EstateSnapshot,
  ScopeSelection,
  ThemePreference,
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
  { id: "overview", label: "Overview", icon: LayoutGrid },
  { id: "estate", label: "Estate", icon: Boxes },
  { id: "topology", label: "Relationships", icon: GitBranch },
  { id: "inventory", label: "Inventory", icon: Table2 },
  { id: "findings", label: "Findings", icon: ShieldCheck },
  { id: "governance", label: "Governance", icon: Tags },
  { id: "history", label: "Changes", icon: FileClock },
];

const THEME_STORAGE_KEY = "azdocs-theme";

function readThemePreference(): ThemePreference {
  try {
    const stored = window.localStorage.getItem(THEME_STORAGE_KEY);
    return stored === "light" || stored === "dark" ? stored : "system";
  } catch {
    return "system";
  }
}

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
  const [view, setView] = useState<ViewId>("overview");
  const [scope, setScope] = useState<ScopeSelection>({});
  const [selectedResourceId, setSelectedResourceId] = useState<string>();
  const [resourceReturnView, setResourceReturnView] = useState<ViewId>("estate");
  const [search, setSearch] = useState("");
  const [searchOpen, setSearchOpen] = useState(false);
  const [activeSearchIndex, setActiveSearchIndex] = useState(0);
  const [topologyFocusRequest, setTopologyFocusRequest] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>();
  const [collectionMessage, setCollectionMessage] = useState<string>();
  const [collecting, setCollecting] = useState(false);
  const [themePreference, setThemePreference] = useState<ThemePreference>(readThemePreference);
  const [systemDark, setSystemDark] = useState(
    () => window.matchMedia("(prefers-color-scheme: dark)").matches,
  );
  const searchRef = useRef<HTMLInputElement>(null);

  const resolvedTheme: "light" | "dark" =
    themePreference === "system" ? (systemDark ? "dark" : "light") : themePreference;

  useEffect(() => {
    const preference = window.matchMedia("(prefers-color-scheme: dark)");
    const update = () => setSystemDark(preference.matches);
    preference.addEventListener("change", update);
    return () => preference.removeEventListener("change", update);
  }, []);

  useEffect(() => {
    // System preference leaves the attribute off so prefers-color-scheme wins;
    // an explicit choice pins it for both directions.
    if (themePreference === "system") delete document.documentElement.dataset.theme;
    else document.documentElement.dataset.theme = themePreference;
    try {
      window.localStorage.setItem(THEME_STORAGE_KEY, themePreference);
    } catch {
      // Preference persistence is a convenience; the session still themes.
    }
  }, [themePreference]);

  const loadSnapshot = useCallback(async (snapshotId?: string) => {
    setLoading(true);
    setError(undefined);
    try {
      const next = await getSnapshot(snapshotId);
      setEstate(next);
      setSelectedResourceId(undefined);
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
          setSelectedResourceId(undefined);
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
        displayLocation(estate.azureMetadata, resource.location),
        JSON.stringify(resource.tags ?? {}),
      ].some((candidate) => candidate?.toLowerCase().includes(value)))
      .slice(0, 8);
  }, [estate, search]);
  const shortcutLabel = navigator.platform.toLowerCase().includes("mac") ? "⌘ K" : "Ctrl K";
  const highFindings = estate?.severityCounts.high ?? 0;

  function chooseSearchResult(index: number) {
    const resource = searchMatches[index];
    if (!resource) return;
    setSelectedResourceId(resource.id);
    if (view === "topology") setTopologyFocusRequest((current) => current + 1);
    else {
      setResourceReturnView(view);
      setView("estate");
    }
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
    if (destination === "estate") setResourceReturnView(view);
    setSelectedResourceId(resourceId);
    setView(destination);
    if (destination === "topology") setTopologyFocusRequest((current) => current + 1);
  }

  function closeResourceRecord() {
    if (resourceReturnView === "topology") {
      setView("topology");
      return;
    }
    setSelectedResourceId(undefined);
    setView(resourceReturnView);
  }

  return (
    <div className="app-shell">
      <header className="masthead">
        <div className="brand">
          <img src="/icons/other/10018-icon-service-Azure-A.svg" alt="" />
          <strong>azdocs</strong>
          <span>Estate field report</span>
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
          <ChevronDown size={14} aria-hidden="true" />
        </label>
        <div className="global-search">
          <Search size={15} aria-hidden="true" />
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
                    <img src={type?.icon ?? ALL_RESOURCES_ICON} alt="" />
                    <span><strong>{resource.name}</strong><small>{type?.displayName ?? resource.azureType} · {resource.resourceGroup}</small></span>
                  </button>
                );
              })}
              {searchMatches.length === 0 ? <p>No matching resources in this snapshot.</p> : null}
            </div>
          ) : null}
        </div>
        <div className="masthead-actions">
          <button className="quiet-button" onClick={handleDatabase} title={bootstrap?.databasePath}>
            <FolderSearch2 size={15} />
            Open data
          </button>
          <button
            className="collect-button"
            onClick={() => void handleCollect()}
            disabled={collecting || !bootstrap?.hasCredentials}
            title={bootstrap?.hasCredentials ? "Collect a new snapshot" : `Configure credentials in ${bootstrap?.configPath ?? "azdocs.toml"}`}
          >
            {collecting ? <LoaderCircle className="spin" size={15} /> : <RefreshCw size={15} />}
            {collecting ? "Collecting" : "Collect snapshot"}
          </button>
        </div>
      </header>

      <div className="app-body">
        <nav className="side-nav" aria-label="Primary navigation">
          {views.map((item) => {
            const Icon = item.icon;
            const badge = item.id === "findings" && highFindings > 0 ? highFindings : undefined;
            return (
              <button
                key={item.id}
                className={view === item.id ? "nav-row active" : "nav-row"}
                onClick={() => {
                  setView(item.id);
                  if (item.id !== "topology") setSelectedResourceId(undefined);
                }}
                aria-current={view === item.id ? "page" : undefined}
                title={item.label}
              >
                <Icon size={15} strokeWidth={1.6} />
                <span>{item.label}</span>
                {badge ? <span className="nav-badge">{badge}</span> : null}
              </button>
            );
          })}
          <div className="nav-spacer" />
          <button
            className={view === "settings" ? "nav-row active" : "nav-row"}
            onClick={() => setView("settings")}
            aria-current={view === "settings" ? "page" : undefined}
            title="Settings"
          >
            <Settings2 size={15} strokeWidth={1.6} />
            <span>Settings</span>
          </button>
        </nav>

        <main className="workspace">
          {collectionMessage ? (
            <div className="collection-strip" role="status">
              <LoaderCircle className={collecting ? "spin" : ""} size={15} />
              <span>{collectionMessage}</span>
              <small>Azure Resource Graph · read-only</small>
            </div>
          ) : null}
          {error ? (
            <div className="error-strip" role="alert">
              <AlertTriangle size={16} />
              <span>{error}</span>
              <button onClick={() => setError(undefined)}>Dismiss</button>
            </div>
          ) : null}

          {loading && !estate ? <LoadingWorkspace /> : null}
          {!loading && !estate && !error && view !== "settings" ? (
            <EmptyWorkspace
              canCollect={Boolean(bootstrap?.hasCredentials)}
              onCollect={() => void handleCollect()}
              onOpen={handleDatabase}
            />
          ) : null}
          {view === "settings" && bootstrap ? (
            <SettingsView
              bootstrap={bootstrap}
              estate={estate}
              themePreference={themePreference}
              resolvedTheme={resolvedTheme}
              onThemeChange={setThemePreference}
              onOpenDatabase={handleDatabase}
            />
          ) : null}
          {estate && view !== "settings" ? (
            <>
              {view === "overview" && bootstrap ? (
                <OverviewView
                  bootstrap={bootstrap}
                  estate={estate}
                  onOpenView={(nextView) => {
                    setSelectedResourceId(undefined);
                    setView(nextView);
                  }}
                  onOpenResource={(id) => openResource(id)}
                />
              ) : null}
              {view === "estate" ? (
                selectedResource ? (
                  <ResourceDetailView
                    resource={selectedResource}
                    type={resourceTypeMap.get(selectedResource.azureType)}
                    estate={estate}
                    backLabel={resourceReturnView === "estate" ? "Back to estate" : `Back to ${resourceReturnView === "settings" ? "Settings" : views.find((item) => item.id === resourceReturnView)?.label ?? "estate"}`}
                    onBack={closeResourceRecord}
                    onSelectResource={setSelectedResourceId}
                    onOpenTopology={() => openResource(selectedResource.id, "topology")}
                    onOpenFindings={() => {
                      setSelectedResourceId(undefined);
                      setView("findings");
                    }}
                  />
                ) : (
                  <EstateExplorer
                    estate={estate}
                    search={search}
                    scope={scope}
                    onScopeChange={setScope}
                    onSelectResource={(id) => {
                      setResourceReturnView("estate");
                      setSelectedResourceId(id);
                    }}
                  />
                )
              ) : null}
              {view === "topology" ? (
                <Suspense fallback={<LoadingWorkspace />}>
                  <TopologyView
                    estate={estate}
                    theme={resolvedTheme}
                    selectedResourceId={selectedResourceId}
                    focusRequestNonce={topologyFocusRequest}
                    onSelectResource={setSelectedResourceId}
                    onInspect={(id) => openResource(id, "estate")}
                  />
                </Suspense>
              ) : null}
              {view === "inventory" ? (
                <InventoryView estate={estate} search={search} />
              ) : null}
              {view === "findings" ? (
                <FindingsView estate={estate} search={search} onOpenResource={(id) => openResource(id)} />
              ) : null}
              {view === "governance" && bootstrap ? (
                <GovernanceView
                  estate={estate}
                  requiredTags={bootstrap.requiredTags}
                  onOpenFindings={() => setView("findings")}
                />
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
      </div>

      <footer className="statusbar" aria-label="Application status">
        <span className={`status-dot ${estate?.status ?? "unknown"}`} />
        <span>{estate ? `${estate.status} snapshot` : "No snapshot loaded"}</span>
        <span className="status-divider" />
        <span className="mono">{bootstrap?.databasePath ?? "Resolving database…"}</span>
        <span className="status-spacer" />
        <span>{selectedResource ? `${selectedResource.edgeCount} relationships · ${selectedResource.findingCount} findings` : "No resource selected"}</span>
      </footer>
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
      <strong>Opening the estate record</strong>
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
      <PanelLeftClose size={38} strokeWidth={1.3} />
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

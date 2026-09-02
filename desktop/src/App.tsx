import { lazy, Suspense, useCallback, useEffect, useMemo, useReducer, useRef, useState } from "react";
import {
  AlertTriangle,
  Boxes,
  ChevronDown,
  FileClock,
  FileOutput,
  FolderSearch2,
  LayoutGrid,
  LoaderCircle,
  Map as MapIcon,
  PanelLeftClose,
  RefreshCw,
  Search,
  Settings2,
  ShieldCheck,
  Table2,
  Tags,
} from "lucide-react";
import { chooseDatabase, collectEstate, getBootstrap, getSnapshot, isTauri } from "./api";
import { resourceIcon } from "./azure-icons";
import {
  initialNavigationState,
  navigationReducer,
  type NavigationFrame,
  type RelationshipWorkspaceState,
} from "./navigation-state";
import { EstateExplorer } from "./components/EstateExplorer";
import { ExportsView } from "./components/ExportsView";
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
import { dayMonthTime, errorMessage, fill } from "./format";
import { installLabels, useLabels, type Labels } from "./labels";
import { matchesResourceSearch, useResourceTypeMap } from "./estate-lookups";

const TopologyView = lazy(() =>
  import("./components/TopologyView").then((module) => ({ default: module.TopologyView })),
);

/** The side-nav entries; labels come from `desktop.nav` under the same ids. */
const views: Array<{
  id: Exclude<ViewId, "settings">;
  icon: typeof Boxes;
}> = [
  { id: "overview", icon: LayoutGrid },
  { id: "estate", icon: Boxes },
  { id: "topology", icon: MapIcon },
  { id: "inventory", icon: Table2 },
  { id: "findings", icon: ShieldCheck },
  { id: "governance", icon: Tags },
  { id: "history", icon: FileClock },
  { id: "exports", icon: FileOutput },
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

function frameLabel(
  frame: NavigationFrame | undefined,
  nav: Labels["desktop"]["nav"],
  estate?: EstateSnapshot,
) {
  if (!frame) return nav.previous_view;
  if (frame.surface.kind === "resource") {
    const resourceId = frame.surface.resourceId;
    return estate?.resources.find((resource) => resource.id === resourceId)?.name
      ?? nav.resource;
  }
  if (frame.section === "topology") {
    const location = frame.relationships.location;
    if (location.kind === "group") {
      return estate?.resourceGroups.find((group) => group.id === location.groupId)?.name
        ?? nav.resource_group;
    }
    if (location.kind === "neighbourhood") {
      const name = estate?.resources.find((resource) => resource.id === location.resourceId)?.name;
      return name ? fill(nav.neighbourhood_of, { name }) : nav.neighbourhood;
    }
    return nav.topology;
  }
  return nav[frame.section] ?? nav.previous_view;
}

export default function App() {
  const [bootstrap, setBootstrap] = useState<AppBootstrap>();
  const [estate, setEstate] = useState<EstateSnapshot>();
  const [navigation, dispatchNavigation] = useReducer(navigationReducer, undefined, initialNavigationState);
  const [scope, setScope] = useState<ScopeSelection>({});
  const [search, setSearch] = useState("");
  const [searchOpen, setSearchOpen] = useState(false);
  const [activeSearchIndex, setActiveSearchIndex] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>();
  const [collectionMessage, setCollectionMessage] = useState<string>();
  const [collecting, setCollecting] = useState(false);
  const [themePreference, setThemePreference] = useState<ThemePreference>(readThemePreference);
  const [systemDark, setSystemDark] = useState(
    () => window.matchMedia("(prefers-color-scheme: dark)").matches,
  );
  const searchRef = useRef<HTMLInputElement>(null);
  const { desktop: { nav, shell } } = useLabels();
  const view = navigation.section;
  const resourceRecordId = navigation.surface.kind === "resource"
    ? navigation.surface.resourceId
    : undefined;

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
      dispatchNavigation({ type: "reset-snapshot" });
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
        // Before setBootstrap: the re-render it triggers repaints the chrome
        // with the installed words.
        installLabels(nextBootstrap.labels);
        document.title = nextBootstrap.labels.desktop.app.window_title;
        setBootstrap(nextBootstrap);
        if (nextBootstrap.latestSnapshotId) {
          const nextEstate = await getSnapshot(nextBootstrap.latestSnapshotId);
          if (!active) return;
          setEstate(nextEstate);
          dispatchNavigation({ type: "reset-snapshot" });
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
    () => estate?.resources.find((resource) => resource.id === resourceRecordId),
    [estate, resourceRecordId],
  );
  const relationshipResource = useMemo(() => {
    const location = navigation.relationships.location;
    return location.kind === "neighbourhood"
      ? estate?.resources.find((resource) => resource.id === location.resourceId)
      : undefined;
  }, [estate, navigation.relationships.location]);
  const resourceTypeMap = useResourceTypeMap(estate);
  const searchMatches = useMemo(() => {
    const value = search.trim().toLowerCase();
    if (!estate || !value) return [];
    return estate.resources
      .filter((resource) => matchesResourceSearch(resource, value, estate.azureMetadata))
      .slice(0, 8);
  }, [estate, search]);
  const shortcutLabel = navigator.platform.toLowerCase().includes("mac")
    ? shell.shortcut_mac
    : shell.shortcut_other;
  const highFindings = estate?.severityCounts.high ?? 0;

  function chooseSearchResult(index: number) {
    const resource = searchMatches[index];
    if (!resource) return;
    dispatchNavigation({ type: "open-resource", resourceId: resource.id });
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
      installLabels(next.labels);
      setBootstrap(next);
      setEstate(undefined);
      dispatchNavigation({ type: "reset-snapshot" });
      if (next.latestSnapshotId) await loadSnapshot(next.latestSnapshotId);
    } catch (caught) {
      setError(errorMessage(caught));
    }
  }

  async function handleCollect() {
    if (collecting) return;
    setCollecting(true);
    setError(undefined);
    setCollectionMessage(shell.preparing_collection);
    try {
      const result = await collectEstate((event: CollectionEvent) => {
        if (event.event === "phase") setCollectionMessage(event.data.message);
        if (event.event === "complete") setCollectionMessage(shell.snapshot_stored);
        if (event.event === "failed") setCollectionMessage(event.data.message);
      });
      const nextBootstrap = await getBootstrap();
      installLabels(nextBootstrap.labels);
      setBootstrap(nextBootstrap);
      await loadSnapshot(result.snapshotId);
      setCollectionMessage(
        fill(shell.collected, { rows: result.rowsIngested.toLocaleString(), status: result.status }),
      );
      window.setTimeout(() => setCollectionMessage(undefined), 4800);
    } catch (caught) {
      setError(errorMessage(caught));
      setCollectionMessage(undefined);
    } finally {
      setCollecting(false);
    }
  }

  const openSection = useCallback((section: ViewId) => {
    dispatchNavigation({ type: "open-section", section });
  }, []);

  const openResource = useCallback((resourceId: string) => {
    dispatchNavigation({ type: "open-resource", resourceId });
  }, []);

  const openRelationships = useCallback((resourceId: string) => {
    dispatchNavigation({ type: "open-relationships", resourceId });
  }, []);

  const updateRelationships = useCallback((workspace: RelationshipWorkspaceState) => {
    dispatchNavigation({ type: "update-relationships", workspace });
  }, []);

  const navigateRelationships = useCallback((workspace: RelationshipWorkspaceState) => {
    dispatchNavigation({ type: "navigate-relationships", workspace });
  }, []);

  const navigateBack = useCallback(() => {
    dispatchNavigation({ type: "back" });
  }, []);

  return (
    <div className="app-shell">
      <header className="masthead">
        <div className="brand">
          <div className="brand-product" role="img" aria-label="azdocs">
            <span className="brand-mark" aria-hidden="true" />
            <strong aria-hidden="true">zdocs</strong>
          </div>
          <span>{shell.tagline}</span>
          {!isTauri ? <em>{shell.preview_badge}</em> : null}
        </div>
        <label className="snapshot-control">
          <span>{shell.snapshot}</span>
          <select
            value={estate?.id ?? ""}
            onChange={(event) => void loadSnapshot(event.target.value)}
            disabled={!bootstrap?.snapshots.length || loading}
            aria-label={shell.snapshot_picker}
          >
            {bootstrap?.snapshots.map((snapshot) => (
              <option key={snapshot.id} value={snapshot.id}>
                {fill(shell.snapshot_option, { date: dayMonthTime(snapshot.createdAt), count: snapshot.resources })}
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
            placeholder={shell.search_placeholder}
            aria-label={shell.search_aria}
            role="combobox"
            aria-autocomplete="list"
            aria-expanded={searchOpen && searchMatches.length > 0}
            aria-controls="global-resource-results"
            aria-activedescendant={searchOpen && searchMatches.length > 0 ? `global-resource-result-${activeSearchIndex}` : undefined}
          />
          <kbd>{shortcutLabel}</kbd>
          {searchOpen && search ? (
            <div className="global-search-results" id="global-resource-results" role="listbox" aria-label={shell.search_results}>
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
                    <img src={resourceIcon(type)} alt="" />
                    <span><strong>{resource.name}</strong><small>{type?.displayName ?? resource.azureType} · {resource.resourceGroup}</small></span>
                  </button>
                );
              })}
              {searchMatches.length === 0 ? <p>{shell.search_no_matches}</p> : null}
            </div>
          ) : null}
        </div>
        <div className="masthead-actions">
          <button className="quiet-button" onClick={handleDatabase} title={bootstrap?.databasePath}>
            <FolderSearch2 size={15} />
            {shell.open_data}
          </button>
          <button
            className="collect-button"
            onClick={() => void handleCollect()}
            disabled={collecting || !bootstrap?.hasCredentials}
            title={bootstrap?.hasCredentials
              ? shell.collect_hint
              : fill(shell.configure_credentials, { path: bootstrap?.configPath ?? "azdocs.toml" })}
          >
            {collecting ? <LoaderCircle className="spin" size={15} /> : <RefreshCw size={15} />}
            {collecting ? shell.collecting : shell.collect}
          </button>
        </div>
      </header>

      <div className="app-body">
        <nav className="side-nav" aria-label={shell.primary_navigation}>
          {views.map((item) => {
            const Icon = item.icon;
            const badge = item.id === "findings" && highFindings > 0 ? highFindings : undefined;
            return (
              <button
                key={item.id}
                className={view === item.id ? "nav-row active" : "nav-row"}
                onClick={() => openSection(item.id)}
                aria-current={view === item.id ? "page" : undefined}
                title={nav[item.id]}
              >
                <Icon size={15} strokeWidth={1.6} />
                <span>{nav[item.id]}</span>
                {badge ? <span className="nav-badge">{badge}</span> : null}
              </button>
            );
          })}
          <div className="nav-spacer" />
          <button
            className={view === "settings" ? "nav-row active" : "nav-row"}
            onClick={() => openSection("settings")}
            aria-current={view === "settings" ? "page" : undefined}
            title={nav.settings}
          >
            <Settings2 size={15} strokeWidth={1.6} />
            <span>{nav.settings}</span>
          </button>
        </nav>

        <main className={view === "topology" ? "workspace workspace-topology" : "workspace"}>
          {collectionMessage ? (
            <div className="collection-strip" role="status">
              <LoaderCircle className={collecting ? "spin" : ""} size={15} />
              <span>{collectionMessage}</span>
              <small>{shell.source_note}</small>
            </div>
          ) : null}
          {error ? (
            <div className="error-strip" role="alert">
              <AlertTriangle size={16} />
              <span>{error}</span>
              <button onClick={() => setError(undefined)}>{shell.dismiss}</button>
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
              {view === "overview" && bootstrap && !selectedResource ? (
                <OverviewView
                  bootstrap={bootstrap}
                  estate={estate}
                  onOpenView={openSection}
                  onOpenResource={openResource}
                />
              ) : null}
              {view === "estate" && !selectedResource ? (
                <EstateExplorer
                  estate={estate}
                  search={search}
                  scope={scope}
                  onScopeChange={setScope}
                  onSelectResource={openResource}
                />
              ) : null}
              {view === "topology" ? (
                <Suspense fallback={<LoadingWorkspace />}>
                  <TopologyView
                    estate={estate}
                    theme={resolvedTheme}
                    workspace={navigation.relationships}
                    active={!selectedResource}
                    backLabel={navigation.history.length > 0
                      ? fill(nav.back_to, { target: frameLabel(navigation.history.at(-1), nav, estate) })
                      : undefined}
                    onBack={navigation.history.length > 0 ? navigateBack : undefined}
                    onNavigate={navigateRelationships}
                    onWorkspaceChange={updateRelationships}
                    onInspect={openResource}
                  />
                </Suspense>
              ) : null}
              {view === "inventory" && !selectedResource ? (
                <InventoryView estate={estate} search={search} />
              ) : null}
              {view === "findings" && !selectedResource ? (
                <FindingsView estate={estate} search={search} onOpenResource={openResource} />
              ) : null}
              {view === "governance" && bootstrap && !selectedResource ? (
                <GovernanceView
                  estate={estate}
                  requiredTags={bootstrap.requiredTags}
                  onOpenFindings={() => openSection("findings")}
                />
              ) : null}
              {view === "history" && bootstrap && !selectedResource ? (
                <HistoryView
                  bootstrap={bootstrap}
                  estate={estate}
                  onLoadSnapshot={(id) => void loadSnapshot(id)}
                />
              ) : null}
              {view === "exports" && bootstrap && !selectedResource ? (
                <ExportsView estate={estate} />
              ) : null}
              {selectedResource ? (
                <div className={view === "topology" ? "resource-record-overlay" : "resource-record-surface"}>
                  <ResourceDetailView
                    resource={selectedResource}
                    type={resourceTypeMap.get(selectedResource.azureType)}
                    estate={estate}
                    backLabel={fill(nav.back_to, { target: frameLabel(navigation.history.at(-1), nav, estate) })}
                    onBack={navigateBack}
                    onSelectResource={openResource}
                    onOpenTopology={() => openRelationships(selectedResource.id)}
                    onOpenFindings={() => openSection("findings")}
                  />
                </div>
              ) : null}
            </>
          ) : null}
        </main>
      </div>

      <footer className="statusbar" aria-label={shell.status_aria}>
        <span className={`status-dot ${estate?.status ?? "unknown"}`} />
        <span>{estate ? fill(shell.status_snapshot, { status: estate.status }) : shell.status_none}</span>
        <span className="status-divider" />
        <span className="mono">{bootstrap?.databasePath ?? shell.status_resolving}</span>
        <span className="status-spacer" />
        <span>{selectedResource ?? relationshipResource
          ? fill(shell.status_selection, {
              edges: (selectedResource ?? relationshipResource)?.edgeCount ?? 0,
              findings: (selectedResource ?? relationshipResource)?.findingCount ?? 0,
            })
            : view === "topology" && estate
              ? fill(shell.status_stored_relationships, { count: estate.edges.length })
            : view === "exports" && estate
              ? fill(shell.status_export_ready, { id: estate.id })
            : shell.status_no_selection}</span>
      </footer>
    </div>
  );
}

function LoadingWorkspace() {
  const { shell } = useLabels().desktop;
  return (
    <div className="loading-workspace" role="status">
      <div className="loading-cabinet">
        <span />
        <span />
        <span />
      </div>
      <strong>{shell.loading_title}</strong>
      <p>{shell.loading_detail}</p>
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
  const { shell } = useLabels().desktop;
  return (
    <div className="empty-workspace">
      <PanelLeftClose size={38} strokeWidth={1.3} />
      <h1>{shell.empty_title}</h1>
      <p>{shell.empty_detail}</p>
      <div>
        <button className="collect-button" onClick={onOpen}>{shell.open_database}</button>
        <button className="quiet-button" onClick={onCollect} disabled={!canCollect}>{shell.collect_first}</button>
      </div>
    </div>
  );
}

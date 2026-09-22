import { WebsiteContext, useWebsiteCapture } from "./website-capture";
import { collectionFeedback } from "./collection-feedback";
import { CollectionDialog } from "./components/CollectionDialog";
import { ShortcutsDialog } from "./components/ShortcutsDialog";
import { readPreference, writePreference } from "./preferences";
import {
  lazy,
  Suspense,
  useCallback,
  useEffect,
  useMemo,
  useReducer,
  useRef,
  useState,
} from "react";
import {
  ChevronDown,
  Compass,
  FolderSearch2,
  Globe,
  History,
  Layers,
  LayoutDashboard,
  LoaderCircle,
  type LucideIcon,
  PanelLeftClose,
  PanelLeftOpen,
  RefreshCw,
  Search,
  Settings,
  ShieldAlert,
  Tags,
  Upload,
} from "lucide-react";
import {
  cancelCollect,
  chooseDatabase,
  collectEstate,
  compareSnapshots,
  type CollectOptions,
  getBootstrap,
  getSnapshot,
  isTauri,
  selectTenant,
} from "./api";
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
import { RegionsView } from "./components/RegionsView";
import { ResourceDetailView } from "./components/ResourceDetailView";
import { SettingsView } from "./components/SettingsView";
import type {
  AppBootstrap,
  DashboardDestination,
  CollectionEvent,
  EstateSnapshot,
  ScopeSelection,
  SnapshotComparison,
  ThemePreference,
  ViewId,
} from "./types";
import { dayMonthTime, errorMessage, fill, snapshotStatusLabel } from "./format";
import { installLabels, useLabels, type Labels } from "./labels";
import { matchesResourceSearch, useResourceTypeMap } from "./estate-lookups";
import { ErrorStrip, EstateTabs } from "./components/view-chrome";

const TopologyView = lazy(() =>
  import("./components/TopologyView").then((module) => ({
    default: module.TopologyView,
  })),
);

/** Text-size steps for mod +/-/0; the CSS scales rem-based type and spacing. */
const UI_SCALES = [1, 1.1, 1.2, 1.3, 1.5] as const;

function readUiScale(): number {
  const stored = readPreference<number>("ui-scale", 1);
  return UI_SCALES.includes(stored as (typeof UI_SCALES)[number]) ? stored : 1;
}

/**
 * The side-nav entries; labels come from `desktop.nav` under the same ids.
 * Navigation uses one outline icon set. Azure artwork stays in the workspace,
 * where it identifies real subscriptions, groups and resources.
 */
const views: Array<{ id: Exclude<ViewId, "settings">; icon: LucideIcon }> = [
  { id: "overview", icon: LayoutDashboard },
  { id: "estate", icon: Layers },
  { id: "topology", icon: Compass },
  { id: "regions", icon: Globe },
  { id: "findings", icon: ShieldAlert },
  { id: "governance", icon: Tags },
  { id: "history", icon: History },
  { id: "exports", icon: Upload },
];

/** Query results is Estate's second tab, so the rail lights Estate for it. */
function railSection(view: ViewId): ViewId {
  return view === "inventory" ? "estate" : view;
}

function readThemePreference(): ThemePreference {
  const stored = readPreference<string>("theme", "system");
  return stored === "light" || stored === "dark" ? stored : "system";
}

function frameLabel(
  frame: NavigationFrame | undefined,
  nav: Labels["desktop"]["nav"],
  estate?: EstateSnapshot,
) {
  if (!frame) return nav.previous_view;
  if (frame.surface.kind === "resource") {
    const resourceId = frame.surface.resourceId;
    return (
      estate?.resources.find((resource) => resource.id === resourceId)?.name ??
      nav.resource
    );
  }
  if (frame.section === "topology") {
    const location = frame.relationships.location;
    if (location.kind === "group") {
      return (
        estate?.resourceGroups.find((group) => group.id === location.groupId)
          ?.name ?? nav.resource_group
      );
    }
    if (location.kind === "neighbourhood") {
      const name = estate?.resources.find(
        (resource) => resource.id === location.resourceId,
      )?.name;
      return name ? fill(nav.neighbourhood_of, { name }) : nav.neighbourhood;
    }
    return nav.topology;
  }
  return nav[frame.section] ?? nav.previous_view;
}

export default function App() {
  const [sidebarCollapsed, setSidebarCollapsed] = useState(() =>
    readPreference("sidebar-collapsed", false),
  );
  function toggleSidebar() {
    setSidebarCollapsed((current) => {
      writePreference("sidebar-collapsed", !current);
      return !current;
    });
  }
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const [uiScale, setUiScale] = useState(readUiScale);
  useEffect(() => {
    document.documentElement.style.setProperty("--ui-scale", String(uiScale));
    writePreference("ui-scale", uiScale);
  }, [uiScale]);
  function stepUiScale(direction: 1 | -1 | 0) {
    setUiScale((current) => {
      if (direction === 0) return 1;
      const index = UI_SCALES.indexOf(current as (typeof UI_SCALES)[number]);
      const next = UI_SCALES[Math.min(UI_SCALES.length - 1, Math.max(0, index + direction))];
      return next ?? 1;
    });
  }
  const [settingsDirty, setSettingsDirty] = useState(false);
  const snapshotRequest = useRef(0);
  const [bootstrap, setBootstrap] = useState<AppBootstrap>();
  const [estate, setEstate] = useState<EstateSnapshot>();
  // The diff against the previous usable snapshot is fetched after the
  // estate has painted: it loads a second snapshot, and nothing on the
  // first screen needs it.
  const [comparison, setComparison] = useState<SnapshotComparison>();
  const [navigation, dispatchNavigation] = useReducer(
    navigationReducer,
    undefined,
    initialNavigationState,
  );
  const [scope, setScope] = useState<ScopeSelection>({});
  const [search, setSearch] = useState("");
  const [searchOpen, setSearchOpen] = useState(false);
  const [activeSearchIndex, setActiveSearchIndex] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>();
  const [collectionMessage, setCollectionMessage] = useState<string>();
  const [collecting, setCollecting] = useState(false);
  const [collectionOpen, setCollectionOpen] = useState(false);
  const [collectionError, setCollectionError] = useState<string>();
  const [feedback, updateFeedback] = useReducer(collectionFeedback, undefined);
  const [screenshotPhase, setScreenshotPhase] = useState(false);
  const websites = useWebsiteCapture(estate?.id, collecting);
  const [themePreference, setThemePreference] =
    useState<ThemePreference>(readThemePreference);
  const [systemDark, setSystemDark] = useState(
    () => window.matchMedia("(prefers-color-scheme: dark)").matches,
  );
  const searchRef = useRef<HTMLInputElement>(null);
  const {
    common: { websites: websiteWords },
    desktop: { nav, shell, overview: overviewWords },
  } = useLabels();
  const settingsWords = useLabels().desktop.settings.editor;
  const view = navigation.section;
  const resourceRecordId =
    navigation.surface.kind === "resource"
      ? navigation.surface.resourceId
      : undefined;

  const resolvedTheme: "light" | "dark" =
    themePreference === "system"
      ? systemDark
        ? "dark"
        : "light"
      : themePreference;

  useEffect(() => {
    const preference = window.matchMedia("(prefers-color-scheme: dark)");
    const update = () => setSystemDark(preference.matches);
    preference.addEventListener("change", update);
    return () => preference.removeEventListener("change", update);
  }, []);

  useEffect(() => {
    // System preference leaves the attribute off so prefers-color-scheme wins;
    // an explicit choice pins it for both directions.
    if (themePreference === "system")
      delete document.documentElement.dataset.theme;
    else document.documentElement.dataset.theme = themePreference;
    writePreference("theme", themePreference);
  }, [themePreference]);

  useEffect(() => {
    // On macOS the native title bar overlays the sidebar (tauri.conf.json
    // `titleBarStyle: Overlay`); the stylesheet pads the frame below the
    // traffic lights when this attribute is present.
    if (isTauri && navigator.platform.toLowerCase().includes("mac"))
      document.documentElement.dataset.titlebar = "overlay";
  }, []);

  const loadSnapshot = useCallback(async (snapshotId?: string) => {
    const request = ++snapshotRequest.current;
    setLoading(true);
    setError(undefined);
    try {
      const next = await getSnapshot(snapshotId);
      if (request !== snapshotRequest.current) return;
      setEstate(next);
      dispatchNavigation({ type: "reset-snapshot" });
    } catch (caught) {
      if (request === snapshotRequest.current) setError(errorMessage(caught));
    } finally {
      if (request === snapshotRequest.current) setLoading(false);
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
    setComparison(undefined);
    const previous = estate?.previousSnapshotId;
    const target = estate?.id;
    if (!previous || !target) return;
    let active = true;
    compareSnapshots(previous, target)
      .then((next) => {
        if (active) setComparison(next);
      })
      .catch(() => {
        // The overview and history say there is nothing to compare yet;
        // the history view can still request a comparison explicitly.
      });
    return () => {
      active = false;
    };
  }, [estate?.id, estate?.previousSnapshotId]);

  useEffect(() => {
    function handleShortcut(event: KeyboardEvent) {
      if (!(event.metaKey || event.ctrlKey)) return;
      if (event.key.toLowerCase() === "k") {
        event.preventDefault();
        searchRef.current?.focus();
      } else if (event.key === "/") {
        event.preventDefault();
        setShortcutsOpen((current) => !current);
      } else if (event.key === "=" || event.key === "+") {
        event.preventDefault();
        stepUiScale(1);
      } else if (event.key === "-") {
        event.preventDefault();
        stepUiScale(-1);
      } else if (event.key === "0") {
        event.preventDefault();
        stepUiScale(0);
      }
    }
    window.addEventListener("keydown", handleShortcut);
    return () => window.removeEventListener("keydown", handleShortcut);
  }, []);

  const selectedResource = useMemo(
    () =>
      estate?.resources.find((resource) => resource.id === resourceRecordId),
    [estate, resourceRecordId],
  );
  const relationshipResource = useMemo(() => {
    const location = navigation.relationships.location;
    return location.kind === "neighbourhood"
      ? estate?.resources.find(
          (resource) => resource.id === location.resourceId,
        )
      : undefined;
  }, [estate, navigation.relationships.location]);
  const resourceTypeMap = useResourceTypeMap(estate);
  const searchMatches = useMemo(() => {
    const value = search.trim().toLowerCase();
    if (!estate || !value) return [];
    return estate.resources
      .filter((resource) =>
        matchesResourceSearch(resource, value, estate.azureMetadata),
      )
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
      setActiveSearchIndex(
        (current) =>
          (current - 1 + searchMatches.length) % searchMatches.length,
      );
    } else if (event.key === "Enter" && searchOpen) {
      event.preventDefault();
      chooseSearchResult(activeSearchIndex);
    } else if (event.key === "Escape") {
      setSearchOpen(false);
    }
  }

  async function applyConfiguration(next: AppBootstrap) {
    snapshotRequest.current++;
    installLabels(next.labels);
    document.title = next.labels.desktop.app.window_title;
    setBootstrap(next);
    setEstate(undefined);
    setScope({});
    setSearch("");
    setError(undefined);
    setCollectionMessage(undefined);
    setCollectionError(undefined);
    updateFeedback({ type: "reset" });
    setCollectionOpen(false);
    dispatchNavigation({ type: "reset-snapshot" });
    if (next.latestSnapshotId) await loadSnapshot(next.latestSnapshotId);
    else setLoading(false);
  }
  async function handleTenant(tenantId: string) {
    if (settingsDirty || websites.blocked) return;
    snapshotRequest.current++;
    setLoading(true);
    setEstate(undefined);
    try {
      await applyConfiguration(await selectTenant(tenantId));
    } catch (caught) {
      setError(errorMessage(caught));
      setLoading(false);
    }
  }
  async function handleDatabase() {
    if (settingsDirty) {
      setError(bootstrap?.labels.desktop.settings.editor.leave_detail);
      return;
    }
    setError(undefined);
    try {
      const next = await chooseDatabase();
      if (next) await applyConfiguration(next);
    } catch (caught) {
      setError(errorMessage(caught));
    }
  }

  async function handleCollect(options: CollectOptions) {
    if (websites.blocked || settingsDirty) return;
    let active = true;
    setCollecting(true);
    updateFeedback({ type: "start", at: Date.now() });
    setScreenshotPhase(false);
    setCollectionError(undefined);
    setError(undefined);
    setCollectionMessage(shell.preparing_collection);
    try {
      const result = await collectEstate((event: CollectionEvent) => {
        if (!active) return;
        updateFeedback({ type: "event", event });
        if (event.event === "screenshots") {
          setScreenshotPhase(true);
          setCollectionMessage(
            fill(websiteWords.progress, {
              completed: event.data.progress.completed,
              total: event.data.progress.total,
              url: event.data.progress.url ?? "",
            }),
          );
        }
        if (event.event === "phase") setCollectionMessage(event.data.message);
        if (event.event === "complete")
          setCollectionMessage(shell.snapshot_stored);
        if (event.event === "cancelled")
          setCollectionMessage(shell.collection_cancelled);
        if (event.event === "failed") setCollectionMessage(event.data.message);
      }, options);
      updateFeedback(
        result.status === "cancelled"
          ? { type: "cancel", at: Date.now() }
          : { type: "finish", at: Date.now(), result },
      );
      const nextBootstrap = await getBootstrap();
      installLabels(nextBootstrap.labels);
      setBootstrap(nextBootstrap);
      await loadSnapshot(result.snapshotId);
      setCollectionMessage(
        [
          fill(result.status === "warnings" ? shell.collected_warnings : shell.collected, {
            rows: result.rowsIngested.toLocaleString(),
            status: result.status,
            failed: result.queriesFailed,
          }),
          result.screenshots
            ? (result.screenshots.error ??
              (result.screenshots.cancelled
                ? websiteWords.cancelled
                : fill(websiteWords.complete, {
                    captured: result.screenshots.captured,
                    failed: result.screenshots.failed,
                    skipped: result.screenshots.skipped,
                  })))
            : "",
        ]
          .filter(Boolean)
          .join(" "),
      );
    } catch (caught) {
      updateFeedback({ type: "fail", at: Date.now() });
      setCollectionError(errorMessage(caught));
      setCollectionMessage(undefined);
    } finally {
      active = false;
      setCollecting(false);
      setScreenshotPhase(false);
    }
  }

  const openSection = useCallback(
    (section: ViewId) => {
      if (settingsDirty) {
        setError(bootstrap?.labels.desktop.settings.editor.leave_detail);
        return;
      }
      dispatchNavigation({ type: "open-section", section });
    },
    [settingsDirty, bootstrap],
  );

  const openDashboardResults = useCallback(
    (destination: DashboardDestination) => {
      setSearch("");
      setScope({});
      dispatchNavigation({ type: "open-results", destination });
    },
    [],
  );

  const openResource = useCallback((resourceId: string) => {
    dispatchNavigation({ type: "open-resource", resourceId });
  }, []);

  /** Findings for one resource, through the same result chip the dashboard uses. */
  const openResourceFindings = useCallback(
    (resourceId: string, name: string) => {
      setSearch("");
      dispatchNavigation({
        type: "open-results",
        destination: {
          view: "findings",
          label: name,
          filter: { resourceIds: [resourceId] },
        },
      });
    },
    [],
  );

  const openRelationships = useCallback((resourceId: string) => {
    dispatchNavigation({ type: "open-relationships", resourceId });
  }, []);

  const updateRelationships = useCallback(
    (workspace: RelationshipWorkspaceState) => {
      dispatchNavigation({ type: "update-relationships", workspace });
    },
    [],
  );

  const navigateRelationships = useCallback(
    (workspace: RelationshipWorkspaceState) => {
      dispatchNavigation({ type: "navigate-relationships", workspace });
    },
    [],
  );

  const navigateBack = useCallback(() => {
    dispatchNavigation({ type: "back" });
  }, []);

  return (
    <WebsiteContext.Provider value={websites}>
      <div className="app-shell">
        <div
          className={
            sidebarCollapsed ? "app-body sidebar-collapsed" : "app-body"
          }
        >
          {/* The whole rail moves the window: "deep" makes every descendant a
              drag handle, and Tauri still lets buttons take their clicks. */}
          <nav
            className="side-nav"
            id="primary-navigation"
            aria-label={shell.primary_navigation}
            data-tauri-drag-region="deep"
          >
            <div className="sidebar-brand">
              <div className="brand-product" role="img" aria-label="azdocs">
                <span className="brand-mark" aria-hidden="true" />
                <span className="brand-copy" aria-hidden="true">
                  <strong>azdocs</strong>
                  <small>{shell.tagline}</small>
                </span>
              </div>
            </div>
            <div className="nav-list">
              {views.map((item) => {
                const Icon = item.icon;
                const badge =
                  item.id === "findings" && highFindings > 0
                    ? highFindings
                    : undefined;
                return (
                  <button
                    key={item.id}
                    className={railSection(view) === item.id ? "nav-row active" : "nav-row"}
                    onClick={() => openSection(item.id)}
                    aria-current={railSection(view) === item.id ? "page" : undefined}
                    title={nav[item.id]}
                    aria-label={
                      badge ? `${nav[item.id]} · ${badge}` : nav[item.id]
                    }
                  >
                    <Icon size={18} aria-hidden="true" />
                    <span>{nav[item.id]}</span>
                    {badge ? <span className="nav-badge">{badge}</span> : null}
                  </button>
                );
              })}
            </div>
            <div className="nav-spacer" />
            <div className="sidebar-footer">
              <button
                className={view === "settings" ? "nav-row active" : "nav-row"}
                onClick={() => openSection("settings")}
                aria-current={view === "settings" ? "page" : undefined}
                title={nav.settings}
                aria-label={nav.settings}
              >
                <Settings size={18} aria-hidden="true" />
                <span>{nav.settings}</span>
              </button>
            </div>
          </nav>

          <div className="app-main">
            <header className="commandbar" data-tauri-drag-region>
              <button
                className="icon-button sidebar-toggle"
                onClick={toggleSidebar}
                aria-expanded={!sidebarCollapsed}
                aria-controls="primary-navigation"
                aria-label={
                  sidebarCollapsed
                    ? overviewWords.dashboard.expand_sidebar
                    : overviewWords.dashboard.collapse_sidebar
                }
                title={
                  sidebarCollapsed
                    ? overviewWords.dashboard.expand_sidebar
                    : overviewWords.dashboard.collapse_sidebar
                }
              >
                {sidebarCollapsed ? (
                  <PanelLeftOpen size={18} />
                ) : (
                  <PanelLeftClose size={18} />
                )}
              </button>
              <label className="snapshot-control tenant-control">
                <span>{settingsWords.tenant}</span>
                <select
                  value={bootstrap?.activeTenantId ?? ""}
                  disabled={loading || websites.blocked || settingsDirty}
                  onChange={(event) => void handleTenant(event.target.value)}
                  aria-label={settingsWords.select_tenant}
                >
                  <option value="" disabled>
                    {settingsWords.select_tenant}
                  </option>
                  {bootstrap?.tenants.map((tenant) => (
                    <option key={tenant.tenantId} value={tenant.tenantId}>
                      {tenant.name}
                      {tenant.configured
                        ? ""
                        : ` · ${settingsWords.tenant_offline}`}
                    </option>
                  ))}
                </select>
                <ChevronDown size={14} aria-hidden="true" />
              </label>
              <label className="snapshot-control">
                <span>{shell.snapshot}</span>
                <select
                  value={estate?.id ?? ""}
                  onChange={(event) => void loadSnapshot(event.target.value)}
                  disabled={
                    !bootstrap?.snapshots.length ||
                    loading ||
                    websites.blocked ||
                    settingsDirty
                  }
                  aria-label={shell.snapshot_picker}
                >
                  {bootstrap?.snapshots.map((snapshot) => (
                    <option key={snapshot.id} value={snapshot.id}>
                      {fill(shell.snapshot_option, {
                        date: dayMonthTime(snapshot.createdAt),
                        count: snapshot.resources,
                      })}
                    </option>
                  ))}
                </select>
                <ChevronDown size={14} aria-hidden="true" />
              </label>
              <div className="global-search">
                <Search size={15} aria-hidden="true" />
                <input
                  ref={searchRef}
                  disabled={settingsDirty}
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
                  aria-activedescendant={
                    searchOpen && searchMatches.length > 0
                      ? `global-resource-result-${activeSearchIndex}`
                      : undefined
                  }
                />
                <kbd>{shortcutLabel}</kbd>
                {searchOpen && search ? (
                  <div
                    className="global-search-results"
                    id="global-resource-results"
                    role="listbox"
                    aria-label={shell.search_results}
                  >
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
                          <span>
                            <strong>{resource.name}</strong>
                            <small>
                              {type?.displayName ?? resource.azureType} ·{" "}
                              {resource.resourceGroup}
                            </small>
                          </span>
                        </button>
                      );
                    })}
                    {searchMatches.length === 0 ? (
                      <p>{shell.search_no_matches}</p>
                    ) : null}
                  </div>
                ) : null}
              </div>
              {!isTauri ? (
                <em className="preview-badge">{shell.preview_badge}</em>
              ) : null}
              <div className="commandbar-actions">
                <button
                  className="quiet-button"
                  disabled={websites.blocked}
                  onClick={handleDatabase}
                  title={bootstrap?.databasePath}
                >
                  <FolderSearch2 size={15} />
                  {shell.open_data}
                </button>
                <button
                  className="collect-button"
                  onClick={() => setCollectionOpen(true)}
                  aria-haspopup="dialog"
                  disabled={!bootstrap}
                  title={
                    bootstrap?.hasCredentials
                      ? `${shell.collect_hint} ${websiteWords.collect_note}`
                      : fill(shell.configure_credentials, {
                          path: bootstrap?.configPath ?? "azdocs.toml",
                        })
                  }
                >
                  {collecting || websites.busy ? (
                    <LoaderCircle className="spin" size={15} />
                  ) : (
                    <RefreshCw size={15} />
                  )}
                  {collecting
                    ? shell.collecting
                    : websites.busy
                      ? websiteWords.states.running
                      : shell.collect}
                </button>
              </div>
            </header>

            <main
              className={
                view === "topology" ? "workspace workspace-topology" : "workspace"
              }
            >
              {collecting && !collectionOpen && collectionMessage ? (
                <div className="collection-strip" role="status">
                  <LoaderCircle className={collecting ? "spin" : ""} size={15} />
                  <span>{collectionMessage}</span>
                  <button
                    className="quiet-button"
                    onClick={() => setCollectionOpen(true)}
                  >
                    {websiteWords.show_collection}
                  </button>
                </div>
              ) : null}
              {error ? (
                <ErrorStrip message={error} onDismiss={() => setError(undefined)} />
              ) : null}

              {navigation.result &&
              navigation.result.view === view &&
              !selectedResource ? (
                <div className="dashboard-result-scope">
                  <button className="quiet-button" onClick={navigateBack}>
                    {overviewWords.dashboard.back}
                  </button>
                  <span>
                    {fill(overviewWords.dashboard.filter, {
                      selection: navigation.result.label,
                    })}
                  </span>
                  <button
                    className="quiet-button"
                    onClick={() => dispatchNavigation({ type: "clear-results" })}
                  >
                    {overviewWords.dashboard.clear_filter}
                  </button>
                </div>
              ) : null}
              {loading && !estate ? <LoadingWorkspace /> : null}
              {!loading && !estate && !error && view !== "settings" ? (
                <EmptyWorkspace
                  canCollect={Boolean(bootstrap?.hasCredentials)}
                  onCollect={() => setCollectionOpen(true)}
                  onOpen={handleDatabase}
                  onSetup={() => openSection("settings")}
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
                  onConfigChange={applyConfiguration}
                  onDirtyChange={setSettingsDirty}
                  onShowShortcuts={() => setShortcutsOpen(true)}
                  blocked={websites.blocked}
                />
              ) : null}
              {estate && view !== "settings" ? (
                <>
                  {view === "overview" && bootstrap && !selectedResource ? (
                    <OverviewView
                      bootstrap={bootstrap}
                      estate={estate}
                      comparison={comparison}
                      onOpenResults={openDashboardResults}
                      onOpenResource={openResource}
                      onOpenRelationships={openRelationships}
                      onLoadSnapshot={(id) => void loadSnapshot(id)}
                      onOpenRegions={() => openSection("regions")}
                    />
                  ) : null}
                  {(view === "estate" || view === "inventory") && !selectedResource ? (
                    <EstateTabs active={view} onSelect={openSection} />
                  ) : null}
                  {view === "estate" && !selectedResource ? (
                    <EstateExplorer
                      estate={estate}
                      search={search}
                      scope={scope}
                      dashboardFilter={navigation.result?.filter}
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
                        backLabel={
                          navigation.history.length > 0
                            ? fill(nav.back_to, {
                                target: frameLabel(
                                  navigation.history.at(-1),
                                  nav,
                                  estate,
                                ),
                              })
                            : undefined
                        }
                        onBack={
                          navigation.history.length > 0 ? navigateBack : undefined
                        }
                        onNavigate={navigateRelationships}
                        onWorkspaceChange={updateRelationships}
                        onInspect={openResource}
                      />
                    </Suspense>
                  ) : null}
                  {view === "regions" && !selectedResource ? (
                    <RegionsView
                      estate={estate}
                      onOpenResults={openDashboardResults}
                    />
                  ) : null}
                  {view === "inventory" && !selectedResource ? (
                    <InventoryView
                      estate={estate}
                      search={search}
                      dashboardFilter={navigation.result?.filter}
                    />
                  ) : null}
                  {view === "findings" && !selectedResource ? (
                    <FindingsView
                      estate={estate}
                      search={search}
                      dashboardFilter={navigation.result?.filter}
                      onOpenResource={openResource}
                    />
                  ) : null}
                  {view === "governance" && bootstrap && !selectedResource ? (
                    <GovernanceView
                      estate={estate}
                      requiredTags={bootstrap.requiredTags}
                      onOpenFindings={(destination) =>
                        destination
                          ? openDashboardResults(destination)
                          : openSection("findings")
                      }
                    />
                  ) : null}
                  {view === "history" && bootstrap && !selectedResource ? (
                    <HistoryView
                      bootstrap={bootstrap}
                      estate={estate}
                      previousComparison={comparison}
                      onLoadSnapshot={(id) => void loadSnapshot(id)}
                      dashboardFilter={navigation.result?.filter}
                      onOpenResource={openResource}
                    />
                  ) : null}
                  {view === "exports" && bootstrap && !selectedResource ? (
                    <ExportsView estate={estate} />
                  ) : null}
                  {selectedResource ? (
                    <div
                      className={
                        view === "topology"
                          ? "resource-record-overlay"
                          : "resource-record-surface"
                      }
                    >
                      <ResourceDetailView
                        resource={selectedResource}
                        type={resourceTypeMap.get(selectedResource.azureType)}
                        estate={estate}
                        backLabel={fill(nav.back_to, {
                          target: frameLabel(
                            navigation.history.at(-1),
                            nav,
                            estate,
                          ),
                        })}
                        onBack={navigateBack}
                        onSelectResource={openResource}
                        onOpenTopology={() =>
                          openRelationships(selectedResource.id)
                        }
                        onOpenFindings={() =>
                          openResourceFindings(
                            selectedResource.id,
                            selectedResource.name,
                          )
                        }
                      />
                    </div>
                  ) : null}
                </>
              ) : null}
            </main>
          </div>
        </div>

        <footer className="statusbar" aria-label={shell.status_aria}>
          <span className={`status-dot ${estate?.status ?? "unknown"}`} />
          <span>
            {estate
              ? (estate.status === "warnings" ? snapshotStatusLabel(estate.status) : fill(shell.status_snapshot, { status: estate.status }))
              : shell.status_none}
          </span>
          <span className="status-divider" />
          <span className="mono">
            {bootstrap?.databasePath ?? shell.status_resolving}
          </span>
          {bootstrap ? (
            <>
              <span className="status-divider" />
              <span className="mono">
                {fill(shell.version, { version: bootstrap.appVersion })}
              </span>
            </>
          ) : null}
          <span className="status-spacer" />
          <span>
            {(selectedResource ?? relationshipResource)
              ? fill(shell.status_selection, {
                  edges:
                    (selectedResource ?? relationshipResource)?.edgeCount ?? 0,
                  findings:
                    (selectedResource ?? relationshipResource)?.findingCount ??
                    0,
                })
              : view === "topology" && estate
                ? fill(shell.status_stored_relationships, {
                    count: estate.edges.length,
                  })
                : view === "exports" && estate
                  ? fill(shell.status_export_ready, { id: estate.id })
                  : shell.status_no_selection}
          </span>
        </footer>
        <CollectionDialog
          open={collectionOpen}
          collecting={collecting}
          autoCapturing={screenshotPhase}
          canCollect={Boolean(bootstrap?.hasCredentials)}
          credentialsHint={fill(shell.configure_credentials, {
            path: bootstrap?.configPath ?? "azdocs.toml",
          })}
          snapshotLabel={
            estate
              ? fill(shell.snapshot_option, {
                  date: dayMonthTime(estate.createdAt),
                  count: estate.resources.length,
                })
              : undefined
          }
          subscriptions={estate?.subscriptions ?? []}
          message={collectionMessage}
          error={collectionError}
          feedback={feedback}
          onClose={() => setCollectionOpen(false)}
          onCollect={(options) => void handleCollect(options)}
          onCancel={() => void cancelCollect()}
        />
        <ShortcutsDialog
          open={shortcutsOpen}
          onClose={() => setShortcutsOpen(false)}
        />
      </div>
    </WebsiteContext.Provider>
  );
}

function LoadingWorkspace() {
  const { shell } = useLabels().desktop;
  return (
    <div className="loading-workspace" role="status" aria-live="polite">
      <div className="skeleton skeleton-head" />
      <div className="skeleton-row">
        {[0, 1, 2, 3].map((slot) => (
          <div key={slot} className="skeleton skeleton-card" />
        ))}
      </div>
      <div className="skeleton-panels">
        <div className="skeleton skeleton-panel" />
        <div className="skeleton skeleton-panel" />
      </div>
      <strong className="sr-only">{shell.loading_title}</strong>
      <p className="sr-only">{shell.loading_detail}</p>
    </div>
  );
}

function EmptyWorkspace({
  canCollect,
  onCollect,
  onOpen,
  onSetup,
}: {
  canCollect: boolean;
  onCollect: () => void;
  onOpen: () => void;
  onSetup: () => void;
}) {
  const { shell, settings } = useLabels().desktop;
  return (
    <div className="empty-workspace">
      <h1>{shell.empty_title}</h1>
      <p>{shell.empty_detail}</p>
      <div>
        <button className="collect-button" onClick={onOpen}>
          {shell.open_database}
        </button>
        <button
          className="quiet-button"
          onClick={canCollect ? onCollect : onSetup}
        >
          {canCollect ? shell.collect_first : settings.editor.setup}
        </button>
      </div>
    </div>
  );
}

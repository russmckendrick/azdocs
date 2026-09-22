import { Channel, invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  AppBootstrap,
  CollectionEvent,
  CollectResult,
  ExportEvent,
  ExportRequest,
  ExportResult,
  EstateSnapshot,
  QueryDefMeta,
  QueryRows,
  ResourceDetail,
  SnapshotComparison,
  TopologyGraph,
  TopologyRequest,
  WebsiteState,
  WebsiteCaptureRequest,
  WebsiteProgress,
  WebsiteBatchResult,
} from "./types";
import { labels } from "./labels";

export const isTauri = "__TAURI_INTERNALS__" in window;

/**
 * False in a Tauri build (vite.config.ts sets it from TAURI_ENV_PLATFORM).
 *
 * Every `if (!PREVIEW || isTauri) return invoke(...)` below then folds to an
 * unconditional return, so the dynamic `import("./mock-data")` after it becomes
 * unreachable and Rollup drops the mock estate and the fallback topology
 * builder — ~800 lines that can never execute inside the app — from the shipped
 * bundle. A plain `vite build` keeps them, so the browser demo still works.
 */
declare const __BROWSER_PREVIEW__: boolean;
const PREVIEW = __BROWSER_PREVIEW__;

const pause = (milliseconds = 240) =>
  new Promise((resolve) => window.setTimeout(resolve, milliseconds));

export async function getBootstrap(): Promise<AppBootstrap> {
  if (!PREVIEW || isTauri) return invoke<AppBootstrap>("bootstrap");
  await pause();
  return (await import("./mock-data")).mockBootstrap;
}

export async function getSnapshot(
  snapshotId?: string,
): Promise<EstateSnapshot> {
  if (!PREVIEW || isTauri)
    return invoke<EstateSnapshot>("load_snapshot", { snapshotId });
  await pause(340);
  const { mockEstate } = await import("./mock-data");
  return { ...mockEstate, id: snapshotId ?? mockEstate.id };
}

export async function getTopology(
  request: TopologyRequest,
): Promise<TopologyGraph> {
  if (!PREVIEW || isTauri)
    return invoke<TopologyGraph>("topology_graph", { request });
  await pause(120);
  const [{ mockEstate }, { buildFallbackTopology }] = await Promise.all([
    import("./mock-data"),
    import("./components/topology-fallback"),
  ]);
  return buildFallbackTopology(mockEstate, request);
}

export async function chooseDatabase(): Promise<AppBootstrap | undefined> {
  if (PREVIEW && !isTauri) return (await import("./mock-data")).mockBootstrap;
  const words = labels().desktop.dialogs;
  const path = await open({
    title: words.open_database_title,
    multiple: false,
    directory: false,
    filters: [
      { name: words.sqlite_filter, extensions: ["db", "sqlite", "sqlite3"] },
    ],
  });
  if (!path) return undefined;
  return invoke<AppBootstrap>("open_database", { path });
}

export async function chooseExportDirectory(): Promise<string | undefined> {
  if (PREVIEW && !isTauri) return "/Users/demo/Documents/azdocs-exports";
  const path = await open({
    title: labels().desktop.dialogs.export_directory_title,
    multiple: false,
    directory: true,
  });
  return typeof path === "string" ? path : undefined;
}

export async function getQueryPackMetadata(): Promise<QueryDefMeta[]> {
  if (!PREVIEW || isTauri) return invoke<QueryDefMeta[]>("query_pack_metadata");
  await pause();
  return (await import("./mock-data")).mockQueryPack;
}

export async function getQueryRows(
  queryName: string,
  snapshotId?: string,
): Promise<QueryRows> {
  if (!PREVIEW || isTauri)
    return invoke<QueryRows>("query_rows", { snapshotId, queryName });
  await pause(180);
  return (await import("./mock-data")).mockQueryRows(queryName);
}

/** Every inventory row describing one resource, grouped by query. */
export async function getResourceQueryRows(
  snapshotId: string,
  resourceId: string,
): Promise<QueryRows[]> {
  if (!PREVIEW || isTauri)
    return invoke<QueryRows[]>("resource_query_rows", {
      snapshotId,
      resourceId,
    });
  await pause(140);
  return (await import("./mock-data")).mockResourceQueryRows(resourceId);
}

export async function compareSnapshots(
  baseSnapshotId: string,
  targetSnapshotId: string,
): Promise<SnapshotComparison> {
  if (!PREVIEW || isTauri) {
    return invoke<SnapshotComparison>("compare_snapshots", {
      baseSnapshotId,
      targetSnapshotId,
    });
  }
  await pause();
  return (await import("./mock-data")).mockComparison;
}

/** The stored bags of one resource, read when its record opens. */
export async function getResourceDetail(
  snapshotId: string,
  resourceId: string,
): Promise<ResourceDetail | null> {
  if (!PREVIEW || isTauri)
    return invoke<ResourceDetail | null>("resource_detail", {
      snapshotId,
      resourceId,
    });
  await pause(120);
  return (await import("./mock-data")).mockResourceDetails[resourceId] ?? null;
}

export async function copyText(text: string): Promise<void> {
  if (!PREVIEW || isTauri) return invoke("copy_text", { text });
  await navigator.clipboard?.writeText(text);
}

/** Saves text through the native picker; false when the user cancelled. */
export async function saveTextFile(
  suggestedName: string,
  contents: string,
): Promise<boolean> {
  if (!PREVIEW || isTauri)
    return invoke<boolean>("save_text_file", { suggestedName, contents });
  const url = URL.createObjectURL(new Blob([contents], { type: "text/plain" }));
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = suggestedName;
  anchor.click();
  URL.revokeObjectURL(url);
  return true;
}

export async function revealExportPath(path: string): Promise<void> {
  if (!PREVIEW || isTauri) return invoke("reveal_export_path", { path });
}

export async function openExportFolder(path: string): Promise<void> {
  if (!PREVIEW || isTauri) return invoke("open_export_folder", { path });
}

export async function openDocs(): Promise<void> {
  if (!PREVIEW || isTauri) return invoke("open_docs");
  window.open("https://github.com/russmckendrick/azdocs/tree/main/docs", "_blank");
}

export interface CollectOptions {
  /** Empty means every subscription the credential can see. */
  subscriptions: string[];
  notes?: string;
}

export async function collectEstate(
  onUpdate: (event: CollectionEvent) => void,
  options: CollectOptions = { subscriptions: [] },
): Promise<CollectResult> {
  if (PREVIEW && !isTauri) {
    const { mockEstate } = await import("./mock-data");
    onUpdate({ event: "stage", data: { stage: "inventory" } });
    onUpdate({
      event: "phase",
      data: { message: labels().desktop.dialogs.preview_collecting },
    });
    let rows = 0;
    for (let i = 0; i <= mockEstate.queryRuns.length; i++) {
      const query = mockEstate.queryRuns[i - 1];
      rows += query?.rowCount ?? 0;
      onUpdate({
        event: "queries",
        data: {
          progress: {
            completed: i,
            total: mockEstate.queryRuns.length,
            rows,
            failed: 0,
            latestQuery: query?.queryName ?? null,
          },
        },
      });
      await pause(400);
    }
    onUpdate({ event: "stage", data: { stage: "discovery" } });
    onUpdate({
      event: "phase",
      data: { message: labels().common.websites.discovering },
    });
    await pause(1000);
    onUpdate({ event: "stage", data: { stage: "capture" } });
    const { default: previewImage } = await import("./fixtures/website.png?inline");
    onUpdate({
      event: "screenshots",
      data: {
        progress: {
          completed: 0,
          total: 1,
          url: "https://example.test/",
          previewImage,
          captured: 0,
          failed: 0,
          cancelled: false,
        },
      },
    });
    await pause(3000);
    onUpdate({
      event: "screenshots",
      data: {
        progress: {
          completed: 1,
          total: 1,
          url: null,
          captured: 1,
          failed: 0,
          cancelled: false,
        },
      },
    });
    const result = {
      snapshotId: mockEstate.id,
      status: "complete",
      queriesRun: mockEstate.queryRuns.length,
      queriesFailed: 0,
      rowsIngested: rows,
      screenshots: {
        captured: 1,
        failed: 0,
        skipped: 0,
        cancelled: false,
        error: null,
      },
    };
    onUpdate({ event: "complete", data: { snapshotId: result.snapshotId } });
    return result;
  }
  const channel = new Channel<CollectionEvent>();
  channel.onmessage = onUpdate;
  return invoke<CollectResult>("collect_snapshot", {
    request: {
      subscriptions: options.subscriptions,
      notes: options.notes?.trim() || labels().desktop.dialogs.collect_notes,
    },
    onEvent: channel,
  });
}

export async function getWebsiteState(
  snapshotId: string,
): Promise<WebsiteState> {
  if (!PREVIEW || isTauri) return invoke("website_state", { snapshotId });
  const { mockEstate } = await import("./mock-data");
  const resource = mockEstate.resources.find(
    (r) => r.azureType === "microsoft.web/sites",
  );
  return {
    endpoints: resource
      ? [
          {
            resourceId: resource.id,
            resourceName: resource.name,
            source: "default",
            hostname: "example.test",
            url: "https://example.test/",
            status: "ready",
          },
        ]
      : [],
    captures: resource
      ? [
          {
            url: "https://example.test/",
            finalUrl: "https://example.test/",
            capturedAt: "2026-09-12T10:00:00Z",
            attemptedAt: "2026-09-12T10:00:00Z",
            status: "captured",
            error: null,
            renderer: "WebKit",
            width: 1440,
            height: 900,
          },
        ]
      : [],
    evidenceErrors: [],
  };
}

export async function getWebsiteImage(
  snapshotId: string,
  url: string,
): Promise<string | null> {
  if (!PREVIEW || isTauri) return invoke("website_image", { snapshotId, url });
  return (await import("./fixtures/website.png?inline")).default;
}

export async function captureWebsites(
  request: WebsiteCaptureRequest,
  onUpdate: (event: WebsiteProgress) => void,
): Promise<WebsiteBatchResult> {
  if (PREVIEW && !isTauri)
    return {
      captured: 0,
      failed: 0,
      skipped: 0,
      cancelled: false,
      error: labels().common.websites.preview_only,
    };
  const channel = new Channel<WebsiteProgress>();
  channel.onmessage = onUpdate;
  return invoke("capture_websites", { request, onEvent: channel });
}

export async function cancelCollect(): Promise<void> {
  if (!PREVIEW || isTauri) await invoke("cancel_collect");
}

export async function cancelExport(): Promise<void> {
  if (!PREVIEW || isTauri) await invoke("cancel_export");
}

export async function cancelWebsiteCapture(): Promise<void> {
  if (!PREVIEW || isTauri) await invoke("cancel_website_capture");
}

export async function saveWebsiteImage(
  snapshotId: string,
  url: string,
): Promise<boolean> {
  if (!PREVIEW || isTauri)
    return invoke("save_website_image", { snapshotId, url });
  return false;
}

export async function getReportThemes(
  snapshotId: string,
): Promise<import("./types").ReportThemes> {
  if (!PREVIEW || isTauri) return invoke("report_themes", { snapshotId });
  // The built-in themes against the default branding, written by the
  // bindings test, so the preview shows the colours an export would carry.
  return (await import("./generated-report-themes.json"))
    .default as import("./types").ReportThemes;
}

export async function exportSnapshot(
  request: ExportRequest,
  onUpdate: (event: ExportEvent) => void,
): Promise<ExportResult> {
  if (PREVIEW && !isTauri) {
    onUpdate({
      event: "phase",
      data: { message: labels().desktop.dialogs.preview_exporting },
    });
    await pause(720);
    const outputs = (await import("./mock-data")).mockExportOutputs(request);
    onUpdate({ event: "complete", data: { outputCount: outputs.length } });
    return { destination: request.destination, outputs, cancelled: false };
  }
  const channel = new Channel<ExportEvent>();
  channel.onmessage = onUpdate;
  return invoke<ExportResult>("export_snapshot", { request, onEvent: channel });
}

export async function getSettings(): Promise<
  import("./types").SettingsDocumentDto
> {
  if (!PREVIEW || isTauri) return invoke("settings_load");
  return structuredClone((await import("./settings-preview")).previewSettings);
}
export async function saveSettings(
  request: import("./types").SettingsSaveRequest,
): Promise<AppBootstrap> {
  if (!PREVIEW || isTauri) return invoke("settings_save", { request });
  return (await import("./settings-preview")).savePreview(request.values);
}
export async function testSettings(
  request: import("./types").SettingsTestRequest,
): Promise<import("./types").SettingsTestResult> {
  if (!PREVIEW || isTauri) return invoke("settings_test", { request });
  await pause(600);
  return {
    check: structuredClone((await import("./settings-preview")).previewCheck),
    secretToken: null,
  };
}
export async function selectTenant(tenantId: string): Promise<AppBootstrap> {
  if (!PREVIEW || isTauri) return invoke("select_tenant", { tenantId });
  return (await import("./settings-preview")).selectPreviewTenant(tenantId);
}
export async function loadSettingsConfig(
  choose = false,
): Promise<AppBootstrap | undefined> {
  const words = labels().desktop.settings.editor;
  let path: string | null = null;
  if (choose && (!PREVIEW || isTauri)) {
    const picked = await open({
      title: words.choose_file,
      multiple: false,
      filters: [{ name: words.toml_filter, extensions: ["toml"] }],
    });
    if (typeof picked !== "string") return undefined;
    path = picked;
  }
  if (!PREVIEW || isTauri) return invoke("settings_load_config", { path });
  return (await import("./mock-data")).mockBootstrap;
}
export async function migrateSettings(
  reference: string,
  name: string,
): Promise<AppBootstrap> {
  if (PREVIEW && !isTauri) return (await import("./mock-data")).mockBootstrap;
  return invoke("settings_migrate", { reference, name });
}
export async function exportSettings(): Promise<boolean> {
  if (!PREVIEW || isTauri) return invoke("settings_export");
  const values = (await getSettings()).values;
  const url = URL.createObjectURL(
    new Blob([JSON.stringify(values, null, 2)], { type: "application/json" }),
  );
  const link = document.createElement("a");
  link.href = url;
  link.download = "azdocs-settings-redacted.json";
  link.click();
  URL.revokeObjectURL(url);
  return true;
}
export async function chooseBrandingPath(
  directory = false,
): Promise<string | undefined> {
  if (PREVIEW && !isTauri) return undefined;
  const selected = await open({
    title: labels().desktop.settings.editor.choose_branding_file,
    directory,
    multiple: false,
  });
  return typeof selected === "string" ? selected : undefined;
}

export async function discardSettingsSecrets(tokens: string[]): Promise<void> {
  if ((!PREVIEW || isTauri) && tokens.length)
    await invoke("settings_discard_secrets", { tokens });
}

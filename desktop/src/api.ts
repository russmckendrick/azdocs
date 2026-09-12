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
  return (await import("./mock-data")).mockEstate.previousDiff!;
}

export async function collectEstate(
  onUpdate: (event: CollectionEvent) => void,
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
    onUpdate({
      event: "screenshots",
      data: {
        progress: {
          completed: 0,
          total: 1,
          url: "https://example.test/",
          captured: 0,
          failed: 0,
          cancelled: false,
        },
      },
    });
    await pause(1600);
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
      subscriptions: [],
      notes: labels().desktop.dialogs.collect_notes,
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
    return { destination: request.destination, outputs };
  }
  const channel = new Channel<ExportEvent>();
  channel.onmessage = onUpdate;
  return invoke<ExportResult>("export_snapshot", { request, onEvent: channel });
}

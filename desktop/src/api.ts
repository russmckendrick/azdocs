import { Channel, invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { mockBootstrap, mockEstate } from "./mock-data";
import { buildFallbackTopology } from "./components/topology-fallback";
import type {
  AppBootstrap,
  CollectionEvent,
  CollectResult,
  EstateSnapshot,
  SnapshotComparison,
  TopologyGraph,
  TopologyRequest,
} from "./types";

export const isTauri = "__TAURI_INTERNALS__" in window;

const pause = (milliseconds = 240) =>
  new Promise((resolve) => window.setTimeout(resolve, milliseconds));

export async function getBootstrap(): Promise<AppBootstrap> {
  if (isTauri) return invoke<AppBootstrap>("bootstrap");
  await pause();
  return mockBootstrap;
}

export async function getSnapshot(snapshotId?: string): Promise<EstateSnapshot> {
  if (isTauri) return invoke<EstateSnapshot>("load_snapshot", { snapshotId });
  await pause(340);
  return { ...mockEstate, id: snapshotId ?? mockEstate.id };
}

export async function getTopology(request: TopologyRequest): Promise<TopologyGraph> {
  if (isTauri) return invoke<TopologyGraph>("topology_graph", { request });
  await pause(120);
  return buildFallbackTopology(mockEstate, request);
}

export async function chooseDatabase(): Promise<AppBootstrap | undefined> {
  if (!isTauri) return mockBootstrap;
  const path = await open({
    title: "Open an azdocs SQLite database",
    multiple: false,
    directory: false,
    filters: [{ name: "SQLite database", extensions: ["db", "sqlite", "sqlite3"] }],
  });
  if (!path) return undefined;
  return invoke<AppBootstrap>("open_database", { path });
}

export async function compareSnapshots(
  baseSnapshotId: string,
  targetSnapshotId: string,
): Promise<SnapshotComparison> {
  if (isTauri) {
    return invoke<SnapshotComparison>("compare_snapshots", {
      baseSnapshotId,
      targetSnapshotId,
    });
  }
  await pause();
  return mockEstate.previousDiff!;
}

export async function collectEstate(
  onUpdate: (event: CollectionEvent) => void,
): Promise<CollectResult> {
  if (!isTauri) {
    onUpdate({ event: "phase", data: { message: "Running read-only Azure queries" } });
    await pause(1000);
    const result = {
      snapshotId: mockEstate.id,
      status: "complete",
      queriesRun: mockEstate.queryRuns.length,
      queriesFailed: 0,
      rowsIngested: mockEstate.resources.length,
    };
    onUpdate({ event: "complete", data: { snapshotId: result.snapshotId } });
    return result;
  }
  const channel = new Channel<CollectionEvent>();
  channel.onmessage = onUpdate;
  return invoke<CollectResult>("collect_snapshot", {
    request: { subscriptions: [], notes: "Collected from azdocs desktop" },
    onEvent: channel,
  });
}

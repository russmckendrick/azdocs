/**
 * The `api` module as component tests see it: every call answers from the
 * illustrative estate, and the tests override individual functions with
 * `vi.mocked(api.x).mockResolvedValue(...)` where a case needs it.
 *
 * Kept as a plain module so each test's `vi.mock("../api", ...)` factory is
 * one line and the mock cannot drift from the real surface: a new export on
 * `api.ts` that a component calls will fail loudly here, not silently.
 */
import { vi } from "vitest";
import {
  mockBootstrap,
  mockComparison,
  mockEstate,
  mockQueryPack,
  mockQueryRows,
  mockResourceDetails,
} from "./mock-data";
import { previewCheck, previewSettings } from "./settings-preview";
import type { CollectResult, ExportResult, WebsiteState } from "./types";

const emptyWebsites: WebsiteState = { endpoints: [], captures: [], evidenceErrors: [] };

export function apiMock() {
  return {
    isTauri: false,
    getBootstrap: vi.fn(async () => structuredClone(mockBootstrap)),
    getSnapshot: vi.fn(async () => structuredClone(mockEstate)),
    getTopology: vi.fn(async () => ({ nodes: [], edges: [], counts: {} })),
    chooseDatabase: vi.fn(async () => structuredClone(mockBootstrap)),
    chooseExportDirectory: vi.fn(async () => "/exports"),
    getQueryPackMetadata: vi.fn(async () => mockQueryPack),
    getQueryRows: vi.fn(async (name: string) => mockQueryRows(name)),
    compareSnapshots: vi.fn(async () => structuredClone(mockComparison)),
    getResourceDetail: vi.fn(async (_snapshot: string, id: string) => mockResourceDetails[id] ?? null),
    copyText: vi.fn(async () => {}),
    saveTextFile: vi.fn(async () => true),
    revealExportPath: vi.fn(async () => {}),
    openExportFolder: vi.fn(async () => {}),
    openDocs: vi.fn(async () => {}),
    collectEstate: vi.fn(
      async (): Promise<CollectResult> => ({
        snapshotId: mockEstate.id,
        status: "complete",
        queriesRun: 1,
        queriesFailed: 0,
        rowsIngested: 1,
        screenshots: null,
      }),
    ),
    getWebsiteState: vi.fn(async () => emptyWebsites),
    getWebsiteImage: vi.fn(async () => null),
    captureWebsites: vi.fn(async () => ({ captured: 0, failed: 0, skipped: 0, cancelled: false, error: null })),
    cancelCollect: vi.fn(async () => {}),
    cancelExport: vi.fn(async () => {}),
    cancelWebsiteCapture: vi.fn(async () => {}),
    saveWebsiteImage: vi.fn(async () => true),
    exportSnapshot: vi.fn(
      async (): Promise<ExportResult> => ({ destination: "/exports", outputs: [], cancelled: false }),
    ),
    getSettings: vi.fn(async () => structuredClone(previewSettings)),
    saveSettings: vi.fn(async () => structuredClone(mockBootstrap)),
    testSettings: vi.fn(async () => ({ check: structuredClone(previewCheck), secretToken: null })),
    selectTenant: vi.fn(async () => structuredClone(mockBootstrap)),
    loadSettingsConfig: vi.fn(async () => structuredClone(mockBootstrap)),
    migrateSettings: vi.fn(async () => structuredClone(mockBootstrap)),
    exportSettings: vi.fn(async () => true),
    chooseBrandingPath: vi.fn(async () => undefined),
    discardSettingsSecrets: vi.fn(async () => {}),
  };
}

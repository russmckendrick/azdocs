import { describe, expect, it } from "vitest";
import { collectionFeedback, elapsedTime } from "./collection-feedback";
import type { CollectResult } from "./types";

const result: CollectResult = {
  snapshotId: "snapshot",
  status: "partial",
  queriesRun: 70,
  queriesFailed: 1,
  rowsIngested: 320,
  screenshots: {
    captured: 20,
    failed: 2,
    skipped: 1,
    cancelled: false,
    error: null,
  },
};

describe("collection feedback", () => {
  it("keeps Azure and screenshot outcomes separate at completion", () => {
    const start = collectionFeedback(undefined, { type: "start", at: 1000 });
    const query = collectionFeedback(start, {
      type: "event",
      event: {
        event: "queries",
        data: {
          progress: {
            completed: 70,
            total: 70,
            rows: 320,
            failed: 1,
            latestQuery: "Web app inventory",
          },
        },
      },
    });
    const captured = collectionFeedback(query, {
      type: "event",
      event: {
        event: "screenshots",
        data: {
          progress: {
            completed: 22,
            total: 22,
            url: null,
            captured: 20,
            failed: 2,
            cancelled: false,
          },
        },
      },
    });
    const end = collectionFeedback(captured, {
      type: "finish",
      at: 5000,
      result,
    });
    expect(end?.queries?.failed).toBe(1);
    expect(end?.screenshots?.failed).toBe(2);
    expect(end?.result?.status).toBe("partial");
    expect(end?.endedAt).toBe(5000);
  });
  it("ignores late channel events after completion and resets for another collection", () => {
    const end = collectionFeedback(
      collectionFeedback(undefined, { type: "start", at: 0 }),
      { type: "finish", at: 100, result },
    );
    expect(
      collectionFeedback(end, {
        type: "event",
        event: { event: "stage", data: { stage: "capture" } },
      }),
    ).toBe(end);
    expect(collectionFeedback(end, { type: "start", at: 200 })).toEqual({
      startedAt: 200,
      stage: "inventory",
    });
  });
  it("formats elapsed time without estimating a finish time", () => {
    expect(elapsedTime(1000, 62000)).toBe("01:01");
    expect(elapsedTime(2000, 1000)).toBe("00:00");
  });
});

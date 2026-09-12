import { describe, expect, it } from "vitest";
import { mockExportOutputs } from "./mock-data";
import type { ExportRequest } from "./types";

const request: ExportRequest = {
  snapshotId: "fixture", destination: "/exports/", exportKind: "reports",
  formats: ["pdf", "docx", "csv"],
};

describe("export reference receipts", () => {
  it("omits reference documents by default", () => {
    expect(mockExportOutputs(request)).toEqual([
      "/exports/report.pdf", "/exports/report.docx", "/exports/inventory.csv", "/exports/findings.csv",
    ]);
  });
  it("adds companions only for selected print formats", () => {
    expect(mockExportOutputs({ ...request, includeReference: true })).toEqual([
      "/exports/report.pdf", "/exports/technical-reference.pdf",
      "/exports/report.docx", "/exports/technical-reference.docx",
      "/exports/inventory.csv", "/exports/findings.csv",
    ]);
  });
});

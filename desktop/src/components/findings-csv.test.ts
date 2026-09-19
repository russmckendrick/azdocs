import { describe, expect, it } from "vitest";
import { findingsCsv } from "./findings-csv";
import { mockEstate } from "../mock-data";

describe("findings csv", () => {
  it("matches the CLI column order and quotes commas and quotes", () => {
    const finding = { ...mockEstate.findings[0], title: 'Says "open", really' };
    const csv = findingsCsv([finding], ["Severity", "Category", "Check", "Title", "Resource"]);
    const [header, row] = csv.trimEnd().split("\n");
    expect(header).toBe("Severity,Category,Check,Title,Resource");
    expect(
      row.startsWith(`${finding.severity},${finding.category},${finding.queryName},"Says ""open"", really",`),
    ).toBe(true);
  });
});
